//! Установка локальных `.deb` в rootfs.
//!
//! Пакеты BSP не лежат ни в одном apt-репозитории, поэтому ставятся файлами.
//! Отдельный установщик нужен именно из-за этого: apt резолвит зависимости из
//! архива, а `dpkg` работает с тем, что уже есть в rootfs, и падает явно, если
//! зависимости не были объявлены в `packages.toml`.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use thiserror::Error;
use tracing::{info, warn};

use crate::chroot::{Chroot, ChrootError};

/// Каталог внутри rootfs, куда копируются устанавливаемые пакеты.
///
/// `/tmp` выбран намеренно: он существует в любом Ubuntu Base и очищается
/// первым же запуском системы, даже если сборка прервётся до уборки.
const STAGING_DIRECTORY: &str = "tmp/platinum-debs";

/// Расширение файлов, которые принимает установщик.
const DEB_EXTENSION: &str = "deb";

/// Ошибки установки локальных пакетов.
#[derive(Debug, Error)]
pub enum DpkgError {
    /// Пустой список означал бы stage без результата.
    #[error("список устанавливаемых пакетов не должен быть пустым")]
    Empty,
    /// Файла нет: inventory BSP дал путь, которого не существует.
    #[error("пакет не найден: {path}")]
    MissingPackage {
        /// Путь, полученный от предыдущего stage.
        path: PathBuf,
    },
    /// Файл не является Debian-пакетом.
    #[error("файл `{path}` не является пакетом `.deb`")]
    NotADebianPackage {
        /// Отклонённый путь.
        path: PathBuf,
    },
    /// Пакет не удалось положить внутрь rootfs.
    #[error("не удалось скопировать пакет в rootfs `{path}`: {source}")]
    Stage {
        /// Путь внутри rootfs.
        path: PathBuf,
        /// Исходная ошибка файловой системы.
        #[source]
        source: io::Error,
    },
    /// Ошибка подготовки chroot или самого `dpkg`.
    #[error(transparent)]
    Chroot(#[from] ChrootError),
}

/// Установщик локальных Debian-пакетов в подготовленный rootfs.
#[derive(Debug, Clone)]
pub struct DpkgInstaller {
    packages: Vec<PathBuf>,
}

impl DpkgInstaller {
    /// Создаёт установщик, проверив, что все пакеты существуют.
    ///
    /// Проверка идёт до входа в chroot: смонтированный `/proc` ради заведомо
    /// отсутствующего файла — лишний привилегированный шаг.
    pub fn new(packages: Vec<PathBuf>) -> Result<Self, DpkgError> {
        if packages.is_empty() {
            return Err(DpkgError::Empty);
        }

        for package in &packages {
            if package.extension().and_then(|extension| extension.to_str()) != Some(DEB_EXTENSION) {
                return Err(DpkgError::NotADebianPackage {
                    path: package.clone(),
                });
            }

            if !package.is_file() {
                return Err(DpkgError::MissingPackage {
                    path: package.clone(),
                });
            }
        }

        Ok(Self { packages })
    }

    /// Копирует пакеты внутрь rootfs и устанавливает их через `dpkg`.
    pub fn install(&self, chroot: &Chroot) -> Result<(), DpkgError> {
        let staged = self.stage(chroot.root())?;

        let result = self.run_dpkg(chroot, &staged);

        // Пакеты убираются в любом случае: десятки мегабайт `.deb` внутри
        // образа не нужны ни успешной сборке, ни диагностике неуспешной.
        remove_staging_directory(chroot.root());

        result?;

        info!(
            packages = self.packages.len(),
            "local packages installed into rootfs"
        );

        Ok(())
    }

    /// Копирует пакеты в rootfs и возвращает их пути внутри chroot.
    fn stage(&self, root: &Path) -> Result<Vec<String>, DpkgError> {
        let directory = root.join(STAGING_DIRECTORY);
        fs::create_dir_all(&directory).map_err(|source| DpkgError::Stage {
            path: directory.clone(),
            source,
        })?;

        let mut staged = Vec::with_capacity(self.packages.len());
        for package in &self.packages {
            // file_name существует: конструктор уже отверг пути без `.deb`.
            let Some(name) = package.file_name() else {
                return Err(DpkgError::NotADebianPackage {
                    path: package.clone(),
                });
            };

            let target = directory.join(name);
            fs::copy(package, &target).map_err(|source| DpkgError::Stage {
                path: target.clone(),
                source,
            })?;

            staged.push(format!("/{STAGING_DIRECTORY}/{}", name.to_string_lossy()));
        }

        Ok(staged)
    }

    /// Выполняет `dpkg --install` для подготовленных путей внутри rootfs.
    fn run_dpkg(&self, chroot: &Chroot, staged: &[String]) -> Result<(), DpkgError> {
        let session = chroot.enter()?;

        let mut arguments = vec!["--install"];
        arguments.extend(staged.iter().map(String::as_str));

        session.run("dpkg", &arguments)?;

        Ok(())
    }
}

/// Удаляет каталог с временными пакетами из rootfs.
fn remove_staging_directory(root: &Path) {
    let directory = root.join(STAGING_DIRECTORY);
    if let Err(error) = fs::remove_dir_all(&directory)
        && error.kind() != io::ErrorKind::NotFound
    {
        warn!(path = %directory.display(), %error, "не удалось удалить временные пакеты из rootfs");
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{DpkgError, DpkgInstaller, remove_staging_directory};

    fn temporary_directory(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "platinum-dpkg-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("системное время должно быть позже Unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&path).expect("временный каталог должен создаваться");

        path
    }

    #[test]
    fn rejects_a_file_that_is_not_a_debian_package() {
        let root = temporary_directory("extension");
        let archive = root.join("linux-image.tar.gz");
        fs::write(&archive, b"payload").expect("файл должен записываться");

        let error = DpkgInstaller::new(vec![archive])
            .expect_err("не-deb должен отклоняться до входа в chroot");

        assert!(matches!(error, DpkgError::NotADebianPackage { .. }));

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }

    #[test]
    fn rejects_a_package_that_inventory_did_not_produce() {
        let root = temporary_directory("missing");

        let error = DpkgInstaller::new(vec![root.join("linux-image.deb")])
            .expect_err("отсутствующий пакет должен отклоняться");

        assert!(matches!(error, DpkgError::MissingPackage { .. }));

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }

    #[test]
    fn stages_packages_under_a_path_visible_inside_the_chroot() {
        let root = temporary_directory("stage");
        let package = root.join("linux-image-vendor.deb");
        fs::write(&package, b"deb").expect("пакет должен записываться");
        let rootfs = root.join("rootfs");
        fs::create_dir_all(&rootfs).expect("каталог rootfs должен создаваться");

        let installer =
            DpkgInstaller::new(vec![package]).expect("корректный пакет должен приниматься");
        let staged = installer.stage(&rootfs).expect("пакет должен копироваться");

        assert_eq!(staged, ["/tmp/platinum-debs/linux-image-vendor.deb"]);
        assert!(
            rootfs
                .join("tmp/platinum-debs/linux-image-vendor.deb")
                .is_file()
        );

        remove_staging_directory(&rootfs);
        assert!(!rootfs.join("tmp/platinum-debs").exists());

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }
}
