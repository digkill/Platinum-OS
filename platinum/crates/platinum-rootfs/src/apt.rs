//! Установка пакетов Platinum поверх Ubuntu Base через apt.

use std::{fs, path::Path};

use thiserror::Error;
use tracing::{info, warn};

use crate::{
    chroot::{Chroot, ChrootError},
    packages::PackageSet,
};

/// Каталог кэша индексов apt внутри rootfs.
const APT_LISTS: &str = "var/lib/apt/lists";

/// Ошибки установки пакетов.
#[derive(Debug, Error)]
pub enum AptError {
    /// Не удалось подготовить окружение или выполнить команду в rootfs.
    #[error(transparent)]
    Chroot(#[from] ChrootError),
}

/// Установщик пакетов в подготовленный rootfs.
///
/// Источники пакетов не переопределяются: Ubuntu Base приходит с собственным
/// `sources.list` своего выпуска, и подмена suite вручную рассинхронизировала бы
/// userspace с базовым архивом.
#[derive(Debug, Clone)]
pub struct AptInstaller {
    packages: PackageSet,
    install_recommends: bool,
}

impl AptInstaller {
    /// Создаёт установщик для проверенного набора пакетов.
    pub fn new(packages: PackageSet, install_recommends: bool) -> Self {
        Self {
            packages,
            install_recommends,
        }
    }

    /// Обновляет индексы и устанавливает пакеты внутри rootfs.
    pub fn install(&self, chroot: &Chroot) -> Result<(), AptError> {
        let session = chroot.enter()?;

        session.run("apt-get", &["update"])?;

        let mut arguments = vec!["install", "--yes"];
        if !self.install_recommends {
            // Recommends в Ubuntu тянут desktop-зависимости в headless образ,
            // поэтому по умолчанию состав userspace задаётся явным списком.
            arguments.push("--no-install-recommends");
        }
        arguments.extend(self.packages.names().iter().map(String::as_str));

        session.run("apt-get", &arguments)?;
        session.run("apt-get", &["clean"])?;

        // Сессия снимается до очистки кэша: пока смонтирован /proc, каталоги
        // rootfs нельзя считать пригодными для файловых операций хоста.
        drop(session);

        prune_apt_lists(chroot.root());

        info!(
            packages = self.packages.names().len(),
            "packages installed into rootfs"
        );

        Ok(())
    }
}

/// Удаляет скачанные индексы apt из готового rootfs.
///
/// Индексы занимают десятки мегабайт и устаревают к первому включению
/// устройства, поэтому в образ они не попадают.
fn prune_apt_lists(root: &Path) {
    let lists = root.join(APT_LISTS);
    let Ok(entries) = fs::read_dir(&lists) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let removed = if path.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };

        if let Err(error) = removed {
            warn!(path = %path.display(), %error, "не удалось очистить кэш индексов apt");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::prune_apt_lists;

    #[test]
    fn removes_downloaded_package_indexes() {
        let root = std::env::temp_dir().join(format!(
            "platinum-apt-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("системное время должно быть позже Unix epoch")
                .as_nanos()
        ));
        let lists: PathBuf = root.join("var/lib/apt/lists/partial");
        fs::create_dir_all(&lists).expect("каталог индексов должен создаваться");
        fs::write(
            root.join("var/lib/apt/lists/ports.ubuntu.com_Packages"),
            b"index",
        )
        .expect("индекс должен записываться");

        prune_apt_lists(&root);

        let remaining = fs::read_dir(root.join("var/lib/apt/lists"))
            .expect("каталог индексов должен сохраняться")
            .count();
        assert_eq!(remaining, 0);

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }
}
