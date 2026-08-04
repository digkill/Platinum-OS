//! Firmware устройств платы, не входящий в архив Ubuntu.
//!
//! Wi-Fi и Bluetooth Allwinner требуют vendor-blob, которого нет ни в
//! `linux-firmware` Ubuntu, ни в пакете ядра. Armbian поставляет их пакетом
//! `armbian-firmware`, но Platinum берёт только объявленные каталоги: пакет
//! Armbian несёт firmware всех поддерживаемых плат и добавил бы к образу
//! сотни мебибайт ради нескольких файлов.
//!
//! Источник фиксируется commit, а не веткой: `artifact-firmware.sh` Armbian
//! использует плавающий `master`, и образ, собранный дважды, содержал бы разный
//! firmware.

use std::collections::BTreeMap;

use serde::Deserialize;

/// Каталог firmware внутри rootfs.
pub const FIRMWARE_DIRECTORY: &str = "lib/firmware";

/// Источник и состав firmware платы.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FirmwareConfig {
    /// Git URL репозитория firmware.
    pub repository: String,
    /// Неизменяемый 40-символьный Git commit.
    pub revision: String,
    /// Каталоги репозитория, копируемые в `/lib/firmware` тем же путём.
    pub directories: Vec<String>,
    /// Симлинки внутри `/lib/firmware`: имя — цель.
    ///
    /// Нужны там, где драйвер ищет firmware по плоскому пути, а репозиторий
    /// хранит его деревом. Значения относительные: абсолютный симлинк указывал
    /// бы на каталог хоста сборки, а не устройства.
    #[serde(default)]
    pub links: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::FirmwareConfig;

    #[test]
    fn parses_directories_and_links() {
        let config: FirmwareConfig = toml::from_str(
            r#"
            repository = "https://github.com/armbian/firmware"
            revision = "d9846710f54da5e4383e2d67311819659ac2cf5c"
            directories = ["aic8800/SDIO/aic8800D80"]

            [links]
            aic8800d80 = "aic8800/SDIO/aic8800D80"
            "#,
        )
        .expect("описание firmware должно читаться");

        assert_eq!(config.directories, ["aic8800/SDIO/aic8800D80"]);
        assert_eq!(
            config.links.get("aic8800d80").map(String::as_str),
            Some("aic8800/SDIO/aic8800D80")
        );
    }

    /// Список симлинков не обязателен: не всякой плате нужен плоский путь.
    #[test]
    fn accepts_firmware_without_links() {
        let config: FirmwareConfig = toml::from_str(
            r#"
            repository = "https://example.test/firmware"
            revision = "0123456789abcdef0123456789abcdef01234567"
            directories = ["brcm"]
            "#,
        )
        .expect("firmware без симлинков должен читаться");

        assert!(config.links.is_empty());
    }
}
