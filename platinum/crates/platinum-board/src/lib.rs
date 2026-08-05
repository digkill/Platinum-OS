//! Загрузка и проверка board-конфигураций Platinum OS One.
//!
//! Конфигурация живёт в TOML, а не в BuildEngine. Поэтому добавление платы
//! меняет данные и не требует branch-логики в универсальном pipeline.

use std::{fs, path::Path};

use serde::Deserialize;
use thiserror::Error;

mod bootloader;
mod firmware;
mod packages;
mod partitions;
mod system;

pub use bootloader::{BootScriptConfig, BootloaderConfig, RaspberryPiConfig};
pub use firmware::{FIRMWARE_DIRECTORY, FirmwareConfig};
pub use packages::PackagesConfig;
pub use partitions::{PartitionConfig, PartitionsConfig};
pub use system::{
    BootConfig, CloudInitConfig, FilesystemConfig, NetworkConfig, ShellConfig, SystemConfig,
    UserConfig, WifiConfig,
};

/// Ошибки чтения или разбора board.toml.
#[derive(Debug, Error)]
pub enum BoardError {
    /// Конфигурация не была доступна на filesystem.
    #[error("не удалось прочитать board-конфигурацию `{path}`: {source}")]
    Read {
        /// Путь, который пытался открыть загрузчик.
        path: String,
        /// Исходная ошибка файловой системы.
        #[source]
        source: std::io::Error,
    },
    /// TOML не соответствует обязательной схеме платы.
    #[error("некорректный TOML board-конфигурации `{path}`: {source}")]
    Parse {
        /// Путь к некорректному файлу.
        path: String,
        /// Ошибка TOML parser.
        #[source]
        source: toml::de::Error,
    },
    /// Архитектура rootfs не совпадает с архитектурой платы.
    #[error("плата объявлена как `{board}`, а rootfs как `{rootfs}`")]
    ArchitectureMismatch {
        /// Архитектура из корня конфигурации.
        board: String,
        /// Архитектура из секции `[rootfs]`.
        rootfs: String,
    },
}

/// Описание платы, необходимое независимым stages сборки.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BoardConfig {
    /// Уникальное машинное имя, используемое в путях и CLI.
    pub id: String,
    /// Человекочитаемое имя платы.
    pub name: String,
    /// Архитектура userspace и toolchain.
    pub architecture: String,
    /// SoC платы; полезен для диагностики и выбора BSP family.
    pub soc: String,
    /// Семейство vendor BSP, не зависящее от имени платы.
    pub bsp_family: String,
    /// Установленный объём RAM в mebibyte.
    pub memory_mib: u32,
    /// Путь DTB внутри boot-артефактов BSP.
    pub dtb: String,
    /// Способ, которым vendor-загрузчик платы находит ядро.
    #[serde(default)]
    pub bootloader: BootloaderConfig,
    /// Модули ядра, загружаемые при старте (`/etc/modules`).
    ///
    /// Список — свойство железа, а не политики системы, поэтому живёт здесь, а
    /// не в `system.toml`: vendor-драйверы Allwinner не имеют modalias и без
    /// явной загрузки не поднимаются.
    #[serde(default)]
    pub modules: Vec<String>,
    /// Vendor-firmware платы, если он нужен помимо архива Ubuntu.
    #[serde(default)]
    pub firmware: Option<FirmwareConfig>,
    /// Pin и параметры Armbian Build, если BSP платы собирается через Armbian.
    ///
    /// Отсутствует у плат, чьё ядро приходит из архива Ubuntu обычным `apt`:
    /// для них Armbian не нужен вовсе.
    #[serde(default)]
    pub armbian: Option<ArmbianConfig>,
    /// Базовый userspace, поверх которого ставятся пакеты Platinum.
    pub rootfs: RootfsConfig,
}

/// Источник базового Ubuntu userspace для образа платы.
///
/// URL и SHA-256 хранятся рядом: без контрольной суммы сборка приняла бы любой
/// ответ зеркала, а платформа потеряла бы воспроизводимость.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RootfsConfig {
    /// Выпуск Ubuntu Base, например `26.04`.
    pub release: String,
    /// Архитектура rootfs; должна совпадать с архитектурой платы.
    pub architecture: String,
    /// URL опубликованного base-архива.
    pub url: String,
    /// SHA-256 архива из официального `SHA256SUMS`.
    pub sha256: String,
}

/// Детерминированные параметры upstream Armbian Build.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArmbianConfig {
    /// Git URL upstream Armbian Build.
    pub repository: String,
    /// Неизменяемый 40-символьный Git commit.
    pub revision: String,
    /// Идентификатор платы, который принимает `compile.sh`.
    pub board: String,
    /// Ветка kernel, определённая Armbian для этой платы.
    pub kernel_branch: String,
}

impl BoardConfig {
    /// Загружает одну board-конфигурацию из TOML-файла.
    pub fn load(path: &Path) -> Result<Self, BoardError> {
        let contents = fs::read_to_string(path).map_err(|source| BoardError::Read {
            path: path.display().to_string(),
            source,
        })?;

        let board: Self = toml::from_str(&contents).map_err(|source| BoardError::Parse {
            path: path.display().to_string(),
            source,
        })?;

        board.validate()?;

        Ok(board)
    }

    /// Проверяет инварианты, которые не выражаются схемой TOML.
    ///
    /// Несовпадение архитектур обнаруживается здесь, а не после загрузки
    /// 33-мегабайтного архива: rootfs чужой архитектуры даст незагружаемый образ.
    fn validate(&self) -> Result<(), BoardError> {
        if self.rootfs.architecture != self.architecture {
            return Err(BoardError::ArchitectureMismatch {
                board: self.architecture.clone(),
                rootfs: self.rootfs.architecture.clone(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use std::path::PathBuf;

    use super::{BoardConfig, BoardError};

    const SHA256: &str = "b2b46a37324ea1954e93f293fe6d7c2241daf2fc298c4022e6e4caceeed74cab";

    fn write_board(label: &str, rootfs_architecture: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "platinum-board-{label}-{}.toml",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("системное время должно быть позже Unix epoch")
                .as_nanos()
        ));
        fs::write(
            &path,
            format!(
                r#"
                id = "test-board"
                name = "Test Board"
                architecture = "arm64"
                soc = "Test SoC"
                bsp_family = "test-family"
                memory_mib = 1024
                dtb = "test-board.dtb"

                [armbian]
                repository = "https://example.test/armbian.git"
                revision = "0123456789abcdef0123456789abcdef01234567"
                board = "test-board"
                kernel_branch = "vendor"

                [rootfs]
                release = "26.04"
                architecture = "{rootfs_architecture}"
                url = "https://example.test/ubuntu-base-26.04-base-{rootfs_architecture}.tar.gz"
                sha256 = "{SHA256}"
            "#
            ),
        )
        .expect("временная конфигурация должна записываться");

        path
    }

    #[test]
    fn loads_a_valid_board_configuration() {
        let path = write_board("valid", "arm64");

        let board = BoardConfig::load(&path).expect("корректный TOML должен читаться");

        assert_eq!(board.id, "test-board");
        assert_eq!(
            board.armbian.as_ref().map(|armbian| armbian.board.as_str()),
            Some("test-board")
        );
        assert_eq!(board.rootfs.release, "26.04");

        fs::remove_file(path).expect("временная конфигурация должна удаляться");
    }

    /// Ключ платы, оказавшийся внутри таблицы, обязан быть ошибкой.
    ///
    /// TOML относит любой ключ после заголовка таблицы к ней, поэтому
    /// `modules`, дописанный ниже `[bootloader]`, становится полем загрузчика.
    /// Без `deny_unknown_fields` serde молча его отбрасывал: сборка проходила,
    /// а на устройстве не поднимался Wi-Fi — это и случилось 2026-08-04.
    #[test]
    fn rejects_a_board_key_placed_inside_a_table() {
        let path = std::env::temp_dir().join(format!(
            "platinum-board-misplaced-{}.toml",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("системное время должно быть позже Unix epoch")
                .as_nanos()
        ));
        fs::write(
            &path,
            format!(
                r#"
                id = "test-board"
                name = "Test Board"
                architecture = "arm64"
                soc = "Test SoC"
                bsp_family = "test-family"
                memory_mib = 1024
                dtb = "test-board.dtb"

                [bootloader]
                method = "boot-script"
                script = "boot-test.cmd"
                env = "test.txt"
                initrd_arch = "arm"
                modules = ["driver"]

                [armbian]
                repository = "https://example.test/armbian.git"
                revision = "0123456789abcdef0123456789abcdef01234567"
                board = "test-board"
                kernel_branch = "vendor"

                [rootfs]
                release = "26.04"
                architecture = "arm64"
                url = "https://example.test/ubuntu-base-26.04-base-arm64.tar.gz"
                sha256 = "{SHA256}"
            "#
            ),
        )
        .expect("временная конфигурация должна записываться");

        let error = BoardConfig::load(&path).expect_err("ключ в чужой таблице должен отклоняться");

        assert!(matches!(error, BoardError::Parse { .. }));

        fs::remove_file(path).expect("временная конфигурация должна удаляться");
    }

    #[test]
    fn rejects_a_rootfs_of_another_architecture() {
        let path = write_board("mismatch", "amd64");

        let error =
            BoardConfig::load(&path).expect_err("чужая архитектура rootfs должна отклоняться");

        assert!(matches!(error, BoardError::ArchitectureMismatch { .. }));

        fs::remove_file(path).expect("временная конфигурация должна удаляться");
    }
}
