//! Загрузка системной конфигурации образа из `system.toml`.
//!
//! Как и `packages.toml`, файл отделён от `board.toml`: hostname, локаль и
//! учётные записи — это политика продукта, а не свойство железа.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::BoardError;

/// Имя файла системной конфигурации рядом с `board.toml`.
const SYSTEM_FILE: &str = "system.toml";

/// Системная конфигурация готового образа.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SystemConfig {
    /// Имя устройства.
    pub hostname: String,
    /// Часовой пояс из tzdata.
    pub timezone: String,
    /// Основная локаль системы.
    pub locale: String,
    /// Учётные записи, создаваемые сборкой.
    #[serde(default)]
    pub users: Vec<UserConfig>,
    /// Записи `/etc/fstab`.
    #[serde(default)]
    pub filesystems: Vec<FilesystemConfig>,
    /// Сетевые настройки.
    #[serde(default)]
    pub network: NetworkConfig,
    /// Параметры загрузки.
    #[serde(default)]
    pub boot: BootConfig,
    /// Графическая оболочка, если образ её запускает.
    #[serde(default)]
    pub shell: Option<ShellConfig>,
    /// Расширять ли корень на весь носитель при первом запуске.
    ///
    /// Включено по умолчанию: образ выпускается под самую маленькую карту, и
    /// без расширения остаток носителя остался бы недоступным.
    #[serde(default = "enabled")]
    pub expand_rootfs: bool,
}

/// Параметры загрузки готовой системы.
///
/// Имена ядра, initramfs и DTB здесь отсутствуют: их определяет установленный
/// пакет ядра, и сборка читает их из `/boot`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BootConfig {
    /// Задержка меню U-Boot в десятых долях секунды.
    #[serde(default = "default_boot_timeout")]
    pub timeout_deciseconds: u32,
    /// Дополнительные аргументы командной строки ядра.
    ///
    /// `console=` здесь не задаётся по умолчанию: vendor-DTB объявляет
    /// `stdout-path`, и жёстко прописанное устройство разошлось бы с ним.
    #[serde(default)]
    pub extra_cmdline: Vec<String>,
}

impl Default for BootConfig {
    fn default() -> Self {
        Self {
            timeout_deciseconds: default_boot_timeout(),
            extra_cmdline: Vec::new(),
        }
    }
}

/// Задержка меню по умолчанию: три секунды на прерывание загрузки.
fn default_boot_timeout() -> u32 {
    30
}

/// Графическая оболочка готовой системы.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShellConfig {
    /// Имя `.desktop`-сессии Wayland, например `plasma-mobile`.
    pub session: String,
    /// Пользователь автоматического входа.
    ///
    /// Без него устройство с сенсорным экраном упирается в экран логина, на
    /// котором нечем набрать пароль.
    #[serde(default)]
    pub autologin_user: Option<String>,
    /// Каталог QML собственного домашнего экрана.
    ///
    /// Путь относительный к файлу конфигурации, как и остальные пути данных.
    /// Пусто — образ использует оболочку из пакета, без своей.
    #[serde(default)]
    pub homescreen: Option<String>,
}

/// Учётная запись готовой системы.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UserConfig {
    /// Имя пользователя.
    pub name: String,
    /// Хеш пароля crypt(3); открытый пароль сборка не принимает.
    pub password_hash: String,
    /// Дополнительные группы, например `sudo`.
    #[serde(default)]
    pub groups: Vec<String>,
    /// Оболочка входа; по умолчанию `/bin/bash`.
    #[serde(default)]
    pub shell: Option<String>,
    /// Требовать ли смену пароля при первом входе.
    ///
    /// По умолчанию включено: пароль из репозитория не должен пережить первую
    /// загрузку устройства.
    #[serde(default = "enabled")]
    pub force_password_change: bool,
}

/// Запись таблицы монтирования.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FilesystemConfig {
    /// Источник: `LABEL=`, `UUID=` или устройство.
    pub source: String,
    /// Точка монтирования.
    pub mount_point: String,
    /// Тип файловой системы.
    pub filesystem: String,
    /// Опции монтирования.
    #[serde(default = "default_mount_options")]
    pub options: String,
    /// Поле dump.
    #[serde(default)]
    pub dump: u8,
    /// Порядок проверки fsck.
    #[serde(default)]
    pub pass: u8,
}

/// Сетевые настройки образа.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NetworkConfig {
    /// Проводные интерфейсы, получающие адрес по DHCP.
    #[serde(default)]
    pub dhcp_interfaces: Vec<String>,
    /// Сети Wi-Fi, к которым подключается устройство.
    ///
    /// В репозитории этот список пуст: SSID и PSK — секреты развёртывания.
    /// Задаётся своим `system.toml` через `--system <path>`, как и пароли.
    #[serde(default)]
    pub wifi: Vec<WifiConfig>,
}

/// Одна сеть Wi-Fi.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WifiConfig {
    /// Имя сети.
    pub ssid: String,
    /// PSK — 64 шестнадцатеричных символа.
    ///
    /// Открытый пароль сборка не принимает, как и у учётных записей. PSK
    /// считается из SSID и пароля, поэтому пароль сети в файле не появляется:
    ///
    /// ```sh
    /// wpa_passphrase <ssid> <пароль>
    /// ```
    pub psk: String,
    /// Скрыта ли сеть (не вещает SSID).
    #[serde(default)]
    pub hidden: bool,
}

/// Значение по умолчанию для булевых полей, включённых по умолчанию.
fn enabled() -> bool {
    true
}

/// Опции монтирования по умолчанию.
fn default_mount_options() -> String {
    "defaults".to_owned()
}

impl SystemConfig {
    /// Загружает системную конфигурацию из TOML-файла.
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

    /// Возвращает `system.toml`, лежащий рядом с указанным `board.toml`.
    pub fn default_path(board_path: &Path) -> PathBuf {
        board_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(SYSTEM_FILE)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::SystemConfig;

    // Метка обязательна: разрешение системных часов на macOS грубее
    // наносекунды, и два параллельных теста получили бы один и тот же файл.
    fn write_system(label: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "platinum-system-{label}-{}.toml",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("системное время должно быть позже Unix epoch")
                .as_nanos()
        ));
        fs::write(&path, contents).expect("временная конфигурация должна записываться");

        path
    }

    #[test]
    fn requires_a_password_change_by_default() {
        let path = write_system(
            "users",
            r#"
            hostname = "platinum"
            timezone = "Etc/UTC"
            locale = "en_US.UTF-8"

            [[users]]
            name = "platinum"
            password_hash = "$6$salt$hash"
            groups = ["sudo"]
            "#,
        );

        let system = SystemConfig::load(&path).expect("корректный TOML должен читаться");

        assert!(system.users[0].force_password_change);
        assert_eq!(system.users[0].shell, None);
        assert!(system.network.dhcp_interfaces.is_empty());

        fs::remove_file(path).expect("временная конфигурация должна удаляться");
    }

    #[test]
    fn uses_default_mount_options() {
        let path = write_system(
            "filesystems",
            r#"
            hostname = "platinum"
            timezone = "Etc/UTC"
            locale = "en_US.UTF-8"

            [[filesystems]]
            source = "LABEL=platinum-root"
            mount_point = "/"
            filesystem = "ext4"
            pass = 1
            "#,
        );

        let system = SystemConfig::load(&path).expect("корректный TOML должен читаться");

        assert_eq!(system.filesystems[0].options, "defaults");
        assert_eq!(system.filesystems[0].pass, 1);

        fs::remove_file(path).expect("временная конфигурация должна удаляться");
    }
}
