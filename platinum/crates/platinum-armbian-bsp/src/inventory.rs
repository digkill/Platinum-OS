use std::{
    fs, io,
    path::{Path, PathBuf},
};

use platinum_board::BoardConfig;
use thiserror::Error;
use tracing::info;

/// Ошибки поиска артефактов после Armbian Build.
#[derive(Debug, Error)]
pub enum InventoryError {
    /// Каталога с пакетами нет: kernel target ещё не выполнялся.
    #[error("каталог артефактов Armbian отсутствует: {path}")]
    MissingOutputDirectory {
        /// Ожидавшийся каталог `output/debs`.
        path: PathBuf,
    },
    /// Каталог существует, но не читается.
    #[error("не удалось прочитать каталог артефактов `{path}`: {source}")]
    ReadOutputDirectory {
        /// Проблемный каталог.
        path: PathBuf,
        /// Исходная ошибка файловой системы.
        #[source]
        source: io::Error,
    },
    /// Пакета с ожидаемым префиксом нет среди собранных.
    #[error("не найден пакет `{prefix}*.deb` в `{path}`; доступны: {available}")]
    ArtifactNotFound {
        /// Префикс имени пакета, который искала сборка.
        prefix: String,
        /// Каталог, в котором выполнялся поиск.
        path: PathBuf,
        /// Перечень фактически найденных пакетов для диагностики.
        available: String,
    },
    /// Несколько пакетов подходят под префикс: выбор наугад сделал бы сборку
    /// невоспроизводимой.
    #[error("префиксу `{prefix}` соответствует несколько пакетов: {matches}")]
    AmbiguousArtifact {
        /// Префикс имени пакета.
        prefix: String,
        /// Список конфликтующих пакетов.
        matches: String,
    },
}

/// Пути к kernel-артефактам, собранным Armbian для одной платы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelArtifacts {
    /// Пакет с ядром и initramfs-хуками.
    pub image_deb: PathBuf,
    /// Пакет с Device Tree blobs платы.
    pub dtb_deb: PathBuf,
    /// Пакет с заголовками ядра; нужен не всем сборкам, поэтому опционален.
    pub headers_deb: Option<PathBuf>,
}

/// Поиск артефактов Armbian Build в каталоге `output/debs`.
///
/// Имена пакетов Armbian содержат версию, которую Platinum не задаёт и не может
/// знать заранее, поэтому inventory ищет по стабильному префиксу
/// `<тип>-<branch>-<family>_` и отказывается выбирать между несколькими
/// кандидатами.
#[derive(Debug, Clone)]
pub struct BspInventory {
    debs_dir: PathBuf,
    kernel_branch: String,
    bsp_family: String,
    armbian_board: String,
}

impl BspInventory {
    /// Создаёт inventory для checkout и параметров конкретной платы.
    pub fn new(
        checkout_dir: &Path,
        kernel_branch: String,
        bsp_family: String,
        armbian_board: String,
    ) -> Self {
        Self {
            debs_dir: checkout_dir.join("output").join("debs"),
            kernel_branch,
            bsp_family,
            armbian_board,
        }
    }

    /// Создаёт inventory по board-конфигурации.
    ///
    /// Возвращает `None` для плат без Armbian: их ядро приходит из архива
    /// Ubuntu, и каталога собранных `.deb` у них не бывает.
    pub fn for_board(checkout_dir: &Path, board: &BoardConfig) -> Option<Self> {
        let armbian = board.armbian.as_ref()?;

        Some(Self::new(
            checkout_dir,
            armbian.kernel_branch.clone(),
            board.bsp_family.clone(),
            armbian.board.clone(),
        ))
    }

    /// Возвращает каталог, в котором Armbian оставляет `.deb` пакеты.
    pub fn debs_dir(&self) -> &Path {
        &self.debs_dir
    }

    /// Находит kernel, DTB и, если он собран, headers пакет.
    pub fn kernel_artifacts(&self) -> Result<KernelArtifacts, InventoryError> {
        let packages = self.packages()?;

        let image_deb = self.find_single(&packages, "linux-image")?;
        let dtb_deb = self.find_single(&packages, "linux-dtb")?;
        let headers_deb = match self.find_single(&packages, "linux-headers") {
            Ok(path) => Some(path),
            Err(InventoryError::ArtifactNotFound { .. }) => None,
            Err(error) => return Err(error),
        };

        info!(
            image = %image_deb.display(),
            dtb = %dtb_deb.display(),
            "bsp kernel artifacts found"
        );

        Ok(KernelArtifacts {
            image_deb,
            dtb_deb,
            headers_deb,
        })
    }

    /// Находит пакет U-Boot, собранный target `uboot`.
    ///
    /// Пакет ищется по двум порядкам `BOARD`/`BRANCH` в имени: Armbian менял их
    /// местами между релизами, а pin платы может указывать на любой из них.
    /// Совпадение обязано быть единственным — выбор наугад сделал бы образ
    /// незагружаемым способом, который заметен только на живом устройстве.
    pub fn uboot_artifact(&self) -> Result<PathBuf, InventoryError> {
        let packages = self.packages()?;

        let prefixes = [
            format!(
                "linux-u-boot-{}-{}_",
                self.armbian_board, self.kernel_branch
            ),
            format!(
                "linux-u-boot-{}-{}_",
                self.kernel_branch, self.armbian_board
            ),
        ];

        let matches: Vec<&String> = packages
            .iter()
            .filter(|name| prefixes.iter().any(|prefix| name.starts_with(prefix)))
            .collect();

        match matches.as_slice() {
            [] => Err(InventoryError::ArtifactNotFound {
                prefix: prefixes.join(" | "),
                path: self.debs_dir.clone(),
                available: join_names(packages.iter().map(String::as_str)),
            }),
            [single] => {
                let path = self.debs_dir.join(single.as_str());
                info!(uboot = %path.display(), "bsp uboot artifact found");

                Ok(path)
            }
            _ => Err(InventoryError::AmbiguousArtifact {
                prefix: prefixes.join(" | "),
                matches: join_names(matches.into_iter().map(String::as_str)),
            }),
        }
    }

    /// Читает отсортированный список имён пакетов в каталоге вывода.
    ///
    /// Порядок `read_dir` зависит от файловой системы, поэтому сортировка нужна
    /// для повторяемых сообщений об ошибках и предсказуемого выбора.
    fn packages(&self) -> Result<Vec<String>, InventoryError> {
        if !self.debs_dir.is_dir() {
            return Err(InventoryError::MissingOutputDirectory {
                path: self.debs_dir.clone(),
            });
        }

        let entries =
            fs::read_dir(&self.debs_dir).map_err(|source| InventoryError::ReadOutputDirectory {
                path: self.debs_dir.clone(),
                source,
            })?;

        let mut packages = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|source| InventoryError::ReadOutputDirectory {
                path: self.debs_dir.clone(),
                source,
            })?;

            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".deb") {
                packages.push(name);
            }
        }

        packages.sort();

        Ok(packages)
    }

    /// Возвращает единственный пакет, начинающийся с ожидаемого префикса.
    ///
    /// Префикс завершается `_`, поэтому `linux-image-vendor-sun60iw2_` не
    /// совпадает с производными пакетами вроде `...-sun60iw2-dbg_`.
    fn find_single(&self, packages: &[String], kind: &str) -> Result<PathBuf, InventoryError> {
        let prefix = format!("{kind}-{}-{}_", self.kernel_branch, self.bsp_family);

        let matches: Vec<&String> = packages
            .iter()
            .filter(|name| name.starts_with(&prefix))
            .collect();

        match matches.as_slice() {
            [] => Err(InventoryError::ArtifactNotFound {
                prefix,
                path: self.debs_dir.clone(),
                available: join_names(packages.iter().map(String::as_str)),
            }),
            [single] => Ok(self.debs_dir.join(single.as_str())),
            _ => Err(InventoryError::AmbiguousArtifact {
                prefix,
                matches: join_names(matches.into_iter().map(String::as_str)),
            }),
        }
    }
}

/// Собирает имена в одну строку для сообщения об ошибке.
fn join_names<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let joined = names.collect::<Vec<_>>().join(", ");

    if joined.is_empty() {
        "пакетов нет".to_owned()
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{BspInventory, InventoryError};

    fn checkout_with_packages(label: &str, packages: &[&str]) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "platinum-bsp-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("системное время должно быть позже Unix epoch")
                .as_nanos()
        ));
        let debs = root.join("output").join("debs");
        fs::create_dir_all(&debs).expect("каталог артефактов должен создаваться");

        for package in packages {
            fs::write(debs.join(package), b"").expect("тестовый пакет должен записываться");
        }

        root
    }

    #[test]
    fn finds_kernel_and_dtb_packages() {
        let checkout = checkout_with_packages(
            "found",
            &[
                "linux-image-vendor-sun60iw2_26.5.0_arm64.deb",
                "linux-dtb-vendor-sun60iw2_26.5.0_arm64.deb",
                "linux-headers-vendor-sun60iw2_26.5.0_arm64.deb",
                "linux-libc-dev_26.5.0_arm64.deb",
            ],
        );

        let artifacts = BspInventory::new(
            &checkout,
            "vendor".into(),
            "sun60iw2".into(),
            "orangepizero3w".into(),
        )
        .kernel_artifacts()
        .expect("артефакты kernel должны находиться");

        assert!(
            artifacts
                .image_deb
                .ends_with("linux-image-vendor-sun60iw2_26.5.0_arm64.deb")
        );
        assert!(
            artifacts
                .dtb_deb
                .ends_with("linux-dtb-vendor-sun60iw2_26.5.0_arm64.deb")
        );
        assert!(artifacts.headers_deb.is_some());

        fs::remove_dir_all(checkout).expect("временный каталог должен удаляться");
    }

    #[test]
    fn ignores_derived_packages_with_the_same_family() {
        let checkout = checkout_with_packages(
            "derived",
            &[
                "linux-image-vendor-sun60iw2_26.5.0_arm64.deb",
                "linux-image-vendor-sun60iw2-dbg_26.5.0_arm64.deb",
                "linux-dtb-vendor-sun60iw2_26.5.0_arm64.deb",
            ],
        );

        let artifacts = BspInventory::new(
            &checkout,
            "vendor".into(),
            "sun60iw2".into(),
            "orangepizero3w".into(),
        )
        .kernel_artifacts()
        .expect("debug-пакет не должен мешать поиску");

        assert!(
            artifacts
                .image_deb
                .ends_with("linux-image-vendor-sun60iw2_26.5.0_arm64.deb")
        );
        assert!(artifacts.headers_deb.is_none());

        fs::remove_dir_all(checkout).expect("временный каталог должен удаляться");
    }

    #[test]
    fn rejects_several_kernel_packages_of_the_same_kind() {
        let checkout = checkout_with_packages(
            "ambiguous",
            &[
                "linux-image-vendor-sun60iw2_26.5.0_arm64.deb",
                "linux-image-vendor-sun60iw2_26.8.0_arm64.deb",
                "linux-dtb-vendor-sun60iw2_26.5.0_arm64.deb",
            ],
        );

        let error = BspInventory::new(
            &checkout,
            "vendor".into(),
            "sun60iw2".into(),
            "orangepizero3w".into(),
        )
        .kernel_artifacts()
        .expect_err("две версии ядра не должны выбираться молча");

        assert!(matches!(error, InventoryError::AmbiguousArtifact { .. }));

        fs::remove_dir_all(checkout).expect("временный каталог должен удаляться");
    }

    #[test]
    fn finds_a_uboot_package_in_either_naming_order() {
        for package in [
            "linux-u-boot-orangepizero3w-vendor_26.5.0_arm64.deb",
            "linux-u-boot-vendor-orangepizero3w_26.5.0_arm64.deb",
        ] {
            let checkout = checkout_with_packages("uboot", &[package]);

            let uboot = BspInventory::new(
                &checkout,
                "vendor".into(),
                "sun60iw2".into(),
                "orangepizero3w".into(),
            )
            .uboot_artifact()
            .expect("пакет U-Boot должен находиться в любом порядке имён");

            assert!(uboot.ends_with(package));

            fs::remove_dir_all(checkout).expect("временный каталог должен удаляться");
        }
    }

    #[test]
    fn rejects_several_uboot_packages() {
        let checkout = checkout_with_packages(
            "uboot-ambiguous",
            &[
                "linux-u-boot-orangepizero3w-vendor_26.5.0_arm64.deb",
                "linux-u-boot-vendor-orangepizero3w_26.8.0_arm64.deb",
            ],
        );

        let error = BspInventory::new(
            &checkout,
            "vendor".into(),
            "sun60iw2".into(),
            "orangepizero3w".into(),
        )
        .uboot_artifact()
        .expect_err("два пакета загрузчика не должны выбираться молча");

        assert!(matches!(error, InventoryError::AmbiguousArtifact { .. }));

        fs::remove_dir_all(checkout).expect("временный каталог должен удаляться");
    }

    #[test]
    fn reports_a_missing_output_directory() {
        let checkout = std::env::temp_dir().join("platinum-bsp-absent-checkout");

        let error = BspInventory::new(
            &checkout,
            "vendor".into(),
            "sun60iw2".into(),
            "orangepizero3w".into(),
        )
        .kernel_artifacts()
        .expect_err("отсутствующий каталог вывода должен быть ошибкой");

        assert!(matches!(
            error,
            InventoryError::MissingOutputDirectory { .. }
        ));
    }
}
