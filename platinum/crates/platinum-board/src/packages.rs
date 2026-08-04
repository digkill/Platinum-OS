//! Загрузка состава userspace из `packages.toml`.
//!
//! Список пакетов отделён от `board.toml` намеренно: состав Platinum userspace
//! общий для всех устройств, а `board.toml` описывает железо. Так одна и та же
//! конфигурация пакетов переиспользуется новыми платами без копирования.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::BoardError;

/// Имя файла состава userspace рядом с `board.toml`.
const PACKAGES_FILE: &str = "packages.toml";

/// Пакеты, устанавливаемые поверх Ubuntu Base.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PackagesConfig {
    /// Ставить ли Recommends вместе с явными пакетами.
    ///
    /// По умолчанию выключено: Recommends в Ubuntu тянут desktop-зависимости и
    /// делают размер образа непредсказуемым.
    #[serde(default)]
    pub install_recommends: bool,
    /// Имена пакетов apt в порядке установки.
    pub install: Vec<String>,
}

impl PackagesConfig {
    /// Загружает состав userspace из TOML-файла.
    pub fn load(path: &Path) -> Result<Self, BoardError> {
        let contents = fs::read_to_string(path).map_err(|source| BoardError::Read {
            path: path.display().to_string(),
            source,
        })?;

        toml::from_str(&contents).map_err(|source| BoardError::Parse {
            path: path.display().to_string(),
            source,
        })
    }

    /// Возвращает `packages.toml`, лежащий рядом с указанным `board.toml`.
    pub fn default_path(board_path: &Path) -> PathBuf {
        board_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(PACKAGES_FILE)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::PackagesConfig;

    fn write_packages(label: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "platinum-packages-{label}-{}.toml",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("системное время должно быть позже Unix epoch")
                .as_nanos()
        ));
        fs::write(&path, contents).expect("временная конфигурация должна записываться");

        path
    }

    #[test]
    fn loads_packages_without_recommends_by_default() {
        let path = write_packages("default", "install = [\"systemd\", \"sudo\"]\n");

        let packages = PackagesConfig::load(&path).expect("корректный TOML должен читаться");

        assert!(!packages.install_recommends);
        assert_eq!(packages.install, ["systemd".to_owned(), "sudo".to_owned()]);

        fs::remove_file(path).expect("временная конфигурация должна удаляться");
    }

    #[test]
    fn resolves_the_sibling_of_a_board_configuration() {
        assert_eq!(
            PackagesConfig::default_path(Path::new("boards/orangepi-zero3w/board.toml")),
            PathBuf::from("boards/orangepi-zero3w/packages.toml")
        );
    }
}
