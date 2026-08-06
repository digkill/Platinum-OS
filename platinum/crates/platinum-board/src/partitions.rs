//! Загрузка разметки образа из `partitions.toml`.
//!
//! Разметка отделена от `system.toml`, но метки разделов связывают их: fstab
//! готовой системы строится из этих же данных, поэтому разойтись они не могут.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::BoardError;

/// Имя файла разметки рядом с `board.toml`.
const PARTITIONS_FILE: &str = "partitions.toml";

/// Разметка дискового образа платы.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PartitionsConfig {
    /// Область в начале образа, куда BSP пишет загрузчик, в mebibyte.
    ///
    /// Значение обязательно и не имеет умолчания: загрузчик пишется мимо
    /// таблицы разделов, поэтому раздел, попавший в эту область, повреждается
    /// без единой ошибки при сборке. Умолчание «безопасного» размера здесь
    /// невозможно — смещения записи задаёт family BSP конкретной платы.
    pub reserved_mib: u64,
    /// Разделы в порядке возрастания смещения.
    pub partitions: Vec<PartitionConfig>,
}

/// Один раздел образа.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PartitionConfig {
    /// Логическое имя раздела.
    pub name: String,
    /// Метка файловой системы; по ней раздел находит fstab.
    pub label: String,
    /// Файловая система раздела.
    pub filesystem: String,
    /// Смещение начала раздела в mebibyte.
    pub start_mib: u64,
    /// Размер раздела в mebibyte.
    pub size_mib: u64,
    /// Точка монтирования в готовой системе.
    #[serde(default)]
    pub mount_point: Option<String>,
    /// Отмечать ли раздел активным в таблице разделов.
    #[serde(default)]
    pub bootable: bool,
    /// Является ли раздел ESP — системным разделом UEFI.
    ///
    /// Меняет только код типа в таблице разделов: прошивка ищет ESP именно по
    /// нему, а раздел с обычным типом FAT пропустит.
    #[serde(default)]
    pub esp: bool,
    /// Опции монтирования для fstab.
    #[serde(default = "default_mount_options")]
    pub options: String,
    /// Порядок проверки fsck.
    #[serde(default)]
    pub pass: u8,
}

/// Опции монтирования по умолчанию.
///
/// `noatime` выбран осознанно: на SD-картах запись времени доступа заметно
/// сокращает ресурс носителя.
fn default_mount_options() -> String {
    "defaults,noatime".to_owned()
}

impl PartitionsConfig {
    /// Загружает разметку из TOML-файла.
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

    /// Возвращает `partitions.toml`, лежащий рядом с указанным `board.toml`.
    pub fn default_path(board_path: &Path) -> PathBuf {
        board_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(PARTITIONS_FILE)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::PartitionsConfig;

    #[test]
    fn loads_a_partition_with_default_mount_options() {
        let path: PathBuf = std::env::temp_dir().join(format!(
            "platinum-partitions-{}.toml",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("системное время должно быть позже Unix epoch")
                .as_nanos()
        ));
        fs::write(
            &path,
            r#"
            reserved_mib = 32

            [[partitions]]
            name = "root"
            label = "platinum-root"
            filesystem = "ext4"
            start_mib = 32
            size_mib = 3072
            mount_point = "/"
            bootable = true
            pass = 1
            "#,
        )
        .expect("временная конфигурация должна записываться");

        let partitions = PartitionsConfig::load(&path).expect("корректный TOML должен читаться");

        assert_eq!(partitions.partitions[0].options, "defaults,noatime");
        assert_eq!(partitions.reserved_mib, 32);
        assert_eq!(partitions.partitions[0].start_mib, 32);
        assert!(partitions.partitions[0].bootable);

        fs::remove_file(path).expect("временная конфигурация должна удаляться");
    }
}
