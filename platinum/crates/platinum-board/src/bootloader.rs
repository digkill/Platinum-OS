//! Способ, которым U-Boot платы находит ядро.
//!
//! Способ загрузки — свойство vendor-загрузчика, а не политика продукта,
//! поэтому он живёт в `board.toml` рядом с BSP pin, а не в `system.toml`.
//! Ветвление по способу происходит при сборке pipeline, но выбор делают данные:
//! `if board == ...` в engine превратил бы каждую новую плату в правку кода.

use serde::Deserialize;

/// Как загрузчик платы получает ядро, initramfs и DTB.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "method", rename_all = "kebab-case")]
pub enum BootloaderConfig {
    /// Стандартный `extlinux.conf`, который U-Boot читает сам.
    Extlinux,
    /// Скомпилированный `boot.scr` из boot-скрипта Armbian.
    ///
    /// Нужен там, где vendor U-Boot старше поддержки extlinux либо где скрипт
    /// платы переопределяет адреса загрузки. Скрипт не переписывается на
    /// стороне Platinum: он берётся из pinned checkout Armbian как есть, иначе
    /// расхождение с upstream пришлось бы отслеживать вручную.
    BootScript(BootScriptConfig),
    /// Прошивка Raspberry Pi: `config.txt` и `cmdline.txt` на FAT-разделе.
    ///
    /// Загрузчик живёт в SPI EEPROM платы, поэтому в сырые сектора образа не
    /// пишется ничего, а ядро и DTB читает сама прошивка ещё до запуска ядра.
    RaspberryPi(RaspberryPiConfig),
    /// Прошивка UEFI и GRUB на разделе ESP.
    ///
    /// Способ для машин, где загрузчик приходит с прошивкой: виртуальных
    /// (Parallels, QEMU с EDK2) и обычных arm64-компьютеров.
    Uefi(UefiBootConfig),
}

impl BootloaderConfig {
    /// Сообщает, пишется ли загрузчик в сырые сектора образа.
    ///
    /// U-Boot живёт до таблицы разделов и переносится сборкой; прошивка
    /// Raspberry Pi лежит в EEPROM платы, и записывать в образ нечего.
    pub fn writes_raw_sectors(&self) -> bool {
        !matches!(self, Self::RaspberryPi(_) | Self::Uefi(_))
    }
}

/// Данные загрузки через UEFI.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UefiBootConfig {
    /// Точка монтирования раздела ESP.
    pub esp_mount_point: String,
}

/// Данные загрузки платы Raspberry Pi.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RaspberryPiConfig {
    /// Точка монтирования FAT-раздела, который читает прошивка.
    pub firmware_mount_point: String,
    /// Строки `config.txt` помимо тех, что сборка выводит из данных платы.
    #[serde(default)]
    pub config: Vec<String>,
}

impl Default for BootloaderConfig {
    /// Платы без секции `[bootloader]` грузятся через `extlinux.conf`.
    fn default() -> Self {
        Self::Extlinux
    }
}

/// Данные boot-скрипта, взятые из family-конфигурации Armbian.
///
/// Значения дублируют `config/sources/families/<family>.conf` pinned checkout.
/// Разбор bash-файла Armbian был бы хрупче явной записи в данных платы, а
/// расхождение обнаруживается сборкой: отсутствующий файл скрипта — ошибка.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BootScriptConfig {
    /// Имя файла в `config/bootscripts` checkout Armbian (`BOOTSCRIPT`).
    pub script: String,
    /// Имя файла в `config/bootenv` checkout Armbian (`BOOTENV_FILE`).
    pub env: String,
    /// Архитектура заголовка uImage для initramfs (`INITRD_ARCH`).
    ///
    /// Может не совпадать с архитектурой платы: U-Boot проверяет это поле, а
    /// vendor-загрузчик бывает 32-битным при arm64 userspace.
    pub initrd_arch: String,
    /// Префикс имён DT overlay (`OVERLAY_PREFIX`), если плата их использует.
    #[serde(default)]
    pub overlay_prefix: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{BootScriptConfig, BootloaderConfig};

    #[test]
    fn parses_a_boot_script_bootloader() {
        let config: BootloaderConfig = toml::from_str(
            r#"
            method = "boot-script"
            script = "boot-sun60iw2.cmd"
            env = "sun60iw2.txt"
            initrd_arch = "arm"
            overlay_prefix = "sun60i-a733"
            "#,
        )
        .expect("описание boot-скрипта должно читаться");

        assert_eq!(
            config,
            BootloaderConfig::BootScript(BootScriptConfig {
                script: "boot-sun60iw2.cmd".into(),
                env: "sun60iw2.txt".into(),
                initrd_arch: "arm".into(),
                overlay_prefix: Some("sun60i-a733".into()),
            })
        );
    }

    #[test]
    fn parses_extlinux_without_extra_data() {
        let config: BootloaderConfig =
            toml::from_str(r#"method = "extlinux""#).expect("extlinux должен читаться");

        assert_eq!(config, BootloaderConfig::Extlinux);
    }

    /// Способ загрузки без обязательных данных не должен молча стать extlinux.
    #[test]
    fn rejects_a_boot_script_without_its_source_files() {
        toml::from_str::<BootloaderConfig>(r#"method = "boot-script""#)
            .expect_err("boot-script без имён файлов должен отклоняться");
    }
}
