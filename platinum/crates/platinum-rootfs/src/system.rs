//! Проверенное описание системной конфигурации образа.
//!
//! Значения проверяются до входа в chroot: неверное имя пользователя или
//! пароль в открытом виде должны отклоняться раньше, чем сборка начнёт менять
//! `/etc` целевой системы.

use thiserror::Error;

/// Оболочка пользователя по умолчанию.
const DEFAULT_SHELL: &str = "/bin/bash";

/// Максимальная длина метки hostname по RFC 1123.
const HOSTNAME_MAX_LENGTH: usize = 63;

/// Ошибки системной конфигурации.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SystemError {
    /// Hostname попадает в `/etc/hostname` и в DNS-имя устройства.
    #[error("недопустимый hostname `{hostname}`")]
    InvalidHostname {
        /// Отклонённое значение.
        hostname: String,
    },
    /// Часовой пояс превращается в путь внутри rootfs, поэтому обязан быть
    /// относительным и без переходов вверх по дереву.
    #[error("недопустимый часовой пояс `{timezone}`")]
    InvalidTimezone {
        /// Отклонённое значение.
        timezone: String,
    },
    /// Без charset `locale-gen` не знает, какую локаль генерировать.
    #[error("локаль `{locale}` должна содержать charset, например `en_US.UTF-8`")]
    InvalidLocale {
        /// Отклонённое значение.
        locale: String,
    },
    /// Имя пользователя по политике Debian.
    #[error("недопустимое имя пользователя `{name}`")]
    InvalidUserName {
        /// Отклонённое значение.
        name: String,
    },
    /// Хранить пароль в открытом виде в репозитории недопустимо.
    #[error("пароль пользователя `{name}` должен быть хешем crypt(3), а не открытым текстом")]
    PlaintextPassword {
        /// Пользователь с некорректным паролем.
        name: String,
    },
    /// Имя группы по политике Debian.
    #[error("недопустимое имя группы `{group}` у пользователя `{name}`")]
    InvalidGroup {
        /// Пользователь, которому назначалась группа.
        name: String,
        /// Отклонённая группа.
        group: String,
    },
    /// Оболочка задаётся абсолютным путём внутри целевой системы.
    #[error("оболочка `{shell}` пользователя `{name}` должна быть абсолютным путём")]
    InvalidShell {
        /// Пользователь с некорректной оболочкой.
        name: String,
        /// Отклонённая оболочка.
        shell: String,
    },
    /// Точка монтирования в fstab обязана быть абсолютной.
    #[error("точка монтирования `{mount_point}` должна быть абсолютным путём")]
    InvalidMountPoint {
        /// Отклонённое значение.
        mount_point: String,
    },
    /// Поля fstab разделяются пробелами, поэтому пустых значений быть не может.
    #[error("поле `{field}` записи fstab не должно быть пустым или содержать пробелы")]
    InvalidFstabField {
        /// Имя проблемного поля.
        field: &'static str,
    },
    /// Имя сетевого интерфейса попадает в netplan как ключ YAML.
    #[error("недопустимое имя сетевого интерфейса `{interface}`")]
    InvalidInterface {
        /// Отклонённое значение.
        interface: String,
    },
    /// Имя сети Wi-Fi попадает в netplan строкой в кавычках.
    #[error("недопустимое имя сети Wi-Fi `{ssid}`")]
    InvalidSsid {
        /// Отклонённое значение.
        ssid: String,
    },
    /// Пароль сети в открытом виде оказался бы в образе и в истории сборки.
    #[error(
        "для сети `{ssid}` нужен PSK из 64 шестнадцатеричных символов, а не пароль; \
         посчитайте его командой `wpa_passphrase {ssid} <пароль>`"
    )]
    PlaintextWifiPassword {
        /// Сеть с некорректным ключом.
        ssid: String,
    },
}

/// Сеть Wi-Fi готовой системы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiNetwork {
    /// Имя сети.
    pub ssid: String,
    /// PSK — 64 шестнадцатеричных символа.
    pub psk: String,
    /// Скрыта ли сеть.
    pub hidden: bool,
    /// Имя беспроводного интерфейса.
    pub interface: String,
}

impl WifiNetwork {
    /// Создаёт проверенное описание сети.
    ///
    /// PSK принимается только в виде хеша: открытый пароль сети попал бы в
    /// образ, в логи сборки и в историю команд, а сеть у пользователя обычно
    /// одна на всё вокруг.
    pub fn new(
        ssid: String,
        psk: String,
        hidden: bool,
        interface: String,
    ) -> Result<Self, SystemError> {
        if !is_valid_interface(&interface) {
            return Err(SystemError::InvalidInterface { interface });
        }

        // SSID уходит в YAML ключом в кавычках: кавычка или перевод строки
        // сломали бы файл, и netplan молча не применил бы конфигурацию.
        if ssid.is_empty() || ssid.len() > 32 || ssid.contains(['"', '\n', '\r', '\\']) {
            return Err(SystemError::InvalidSsid { ssid });
        }

        if psk.len() != 64 || !psk.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(SystemError::PlaintextWifiPassword { ssid });
        }

        Ok(Self {
            ssid,
            psk,
            hidden,
            interface,
        })
    }
}

/// Учётная запись, создаваемая в готовой системе.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    /// Имя пользователя.
    pub name: String,
    /// Хеш пароля в формате crypt(3), например из `openssl passwd -6`.
    pub password_hash: String,
    /// Дополнительные группы, например `sudo`.
    pub groups: Vec<String>,
    /// Оболочка входа.
    pub shell: String,
    /// Требовать ли смену пароля при первом входе.
    pub force_password_change: bool,
}

impl User {
    /// Создаёт проверенную учётную запись.
    ///
    /// Пароль принимается только как хеш: открытый пароль в `system.toml`
    /// оказался бы в истории git и стал бы постоянным credential устройства.
    pub fn new(
        name: String,
        password_hash: String,
        groups: Vec<String>,
        shell: Option<String>,
        force_password_change: bool,
    ) -> Result<Self, SystemError> {
        if !is_valid_unix_name(&name) {
            return Err(SystemError::InvalidUserName { name });
        }

        if !password_hash.starts_with('$') {
            return Err(SystemError::PlaintextPassword { name });
        }

        for group in &groups {
            if !is_valid_unix_name(group) {
                return Err(SystemError::InvalidGroup {
                    name,
                    group: group.clone(),
                });
            }
        }

        let shell = shell.unwrap_or_else(|| DEFAULT_SHELL.to_owned());
        if !shell.starts_with('/') {
            return Err(SystemError::InvalidShell { name, shell });
        }

        Ok(Self {
            name,
            password_hash,
            groups,
            shell,
            force_password_change,
        })
    }
}

/// Запись `/etc/fstab` готовой системы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filesystem {
    /// Источник: `LABEL=`, `UUID=` или устройство.
    pub source: String,
    /// Точка монтирования.
    pub mount_point: String,
    /// Тип файловой системы.
    pub filesystem: String,
    /// Опции монтирования.
    pub options: String,
    /// Поле dump.
    pub dump: u8,
    /// Порядок проверки fsck.
    pub pass: u8,
}

impl Filesystem {
    /// Создаёт проверенную запись fstab.
    pub fn new(
        source: String,
        mount_point: String,
        filesystem: String,
        options: String,
        dump: u8,
        pass: u8,
    ) -> Result<Self, SystemError> {
        if !mount_point.starts_with('/') {
            return Err(SystemError::InvalidMountPoint { mount_point });
        }

        for (field, value) in [
            ("source", &source),
            ("mount_point", &mount_point),
            ("filesystem", &filesystem),
            ("options", &options),
        ] {
            // Пробел внутри поля сдвинул бы все последующие колонки fstab, и
            // система смонтировала бы не то, что описано в конфигурации.
            if value.is_empty() || value.contains(char::is_whitespace) {
                return Err(SystemError::InvalidFstabField { field });
            }
        }

        Ok(Self {
            source,
            mount_point,
            filesystem,
            options,
            dump,
            pass,
        })
    }
}

/// Полная системная конфигурация образа.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemSpec {
    /// Имя устройства.
    pub hostname: String,
    /// Часовой пояс из tzdata, например `Etc/UTC`.
    pub timezone: String,
    /// Основная локаль системы.
    pub locale: String,
    /// Учётные записи, создаваемые сборкой.
    pub users: Vec<User>,
    /// Записи `/etc/fstab`.
    pub filesystems: Vec<Filesystem>,
    /// Интерфейсы, которым netplan выдаёт DHCP-конфигурацию.
    pub dhcp_interfaces: Vec<String>,
    /// Модули ядра, загружаемые при старте (`/etc/modules`).
    pub modules: Vec<String>,
    /// Расширять ли корень на весь носитель при первом запуске.
    pub expand_rootfs: bool,
    /// Сети Wi-Fi, к которым подключается устройство.
    pub wifi: Vec<WifiNetwork>,
    /// Графическая оболочка, если образ её запускает.
    pub shell: Option<crate::shell::ShellSpec>,
    /// Настройка первой загрузки, если образ её поддерживает.
    pub cloud_init: Option<crate::cloudinit::CloudInitSpec>,
}

impl SystemSpec {
    /// Создаёт конфигурацию с обязательными hostname, часовым поясом и локалью.
    pub fn new(hostname: String, timezone: String, locale: String) -> Result<Self, SystemError> {
        if !is_valid_hostname(&hostname) {
            return Err(SystemError::InvalidHostname { hostname });
        }

        if !is_valid_timezone(&timezone) {
            return Err(SystemError::InvalidTimezone { timezone });
        }

        if locale.contains(char::is_whitespace) || !locale.contains('.') {
            return Err(SystemError::InvalidLocale { locale });
        }

        Ok(Self {
            hostname,
            timezone,
            locale,
            users: Vec::new(),
            filesystems: Vec::new(),
            dhcp_interfaces: Vec::new(),
            modules: Vec::new(),
            expand_rootfs: false,
            wifi: Vec::new(),
            shell: None,
            cloud_init: None,
        })
    }

    /// Добавляет модули ядра, загружаемые при старте.
    pub fn with_modules(mut self, modules: Vec<String>) -> Self {
        self.modules = modules;

        self
    }

    /// Включает настройку первой загрузки.
    pub fn with_cloud_init(mut self, cloud_init: Option<crate::cloudinit::CloudInitSpec>) -> Self {
        self.cloud_init = cloud_init;

        self
    }

    /// Включает графическую оболочку.
    pub fn with_shell(mut self, shell: Option<crate::shell::ShellSpec>) -> Self {
        self.shell = shell;

        self
    }

    /// Добавляет сети Wi-Fi.
    pub fn with_wifi(mut self, wifi: Vec<WifiNetwork>) -> Self {
        self.wifi = wifi;

        self
    }

    /// Включает расширение корня при первом запуске.
    pub fn with_rootfs_expansion(mut self, expand: bool) -> Self {
        self.expand_rootfs = expand;

        self
    }

    /// Добавляет учётные записи.
    pub fn with_users(mut self, users: Vec<User>) -> Self {
        self.users = users;

        self
    }

    /// Добавляет записи fstab.
    pub fn with_filesystems(mut self, filesystems: Vec<Filesystem>) -> Self {
        self.filesystems = filesystems;

        self
    }

    /// Добавляет интерфейсы, настраиваемые по DHCP.
    pub fn with_dhcp_interfaces(mut self, interfaces: Vec<String>) -> Result<Self, SystemError> {
        for interface in &interfaces {
            if !is_valid_interface(interface) {
                return Err(SystemError::InvalidInterface {
                    interface: interface.clone(),
                });
            }
        }

        self.dhcp_interfaces = interfaces;

        Ok(self)
    }

    /// Возвращает charset локали для `locale.gen`.
    ///
    /// Существование charset уже проверено конструктором, поэтому здесь
    /// достаточно взять часть после точки.
    pub fn locale_charset(&self) -> &str {
        self.locale.rsplit('.').next().unwrap_or("UTF-8")
    }
}

/// Проверяет hostname по RFC 1123: буквы, цифры и дефис внутри метки.
fn is_valid_hostname(hostname: &str) -> bool {
    !hostname.is_empty()
        && hostname.len() <= HOSTNAME_MAX_LENGTH
        && !hostname.starts_with('-')
        && !hostname.ends_with('-')
        && hostname
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
}

/// Проверяет, что часовой пояс безопасно использовать как путь в rootfs.
fn is_valid_timezone(timezone: &str) -> bool {
    !timezone.is_empty()
        && !timezone.starts_with('/')
        && !timezone.split('/').any(|segment| {
            segment.is_empty()
                || segment == "."
                || segment == ".."
                || segment.contains(char::is_whitespace)
        })
}

/// Проверяет имя пользователя или группы по политике Debian.
fn is_valid_unix_name(name: &str) -> bool {
    let mut characters = name.chars();

    let starts_correctly = matches!(
        characters.next(),
        Some(first) if first.is_ascii_lowercase() || first == '_'
    );

    starts_correctly
        && name.len() <= 32
        && name.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '_' | '-')
        })
}

/// Проверяет имя сетевого интерфейса.
fn is_valid_interface(interface: &str) -> bool {
    !interface.is_empty()
        && interface.len() <= 15
        && interface
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

#[cfg(test)]
mod tests {
    use super::{Filesystem, SystemError, SystemSpec, User};

    fn spec() -> SystemSpec {
        SystemSpec::new("platinum".into(), "Etc/UTC".into(), "en_US.UTF-8".into())
            .expect("базовая конфигурация должна быть корректной")
    }

    #[test]
    fn extracts_the_charset_of_the_locale() {
        assert_eq!(spec().locale_charset(), "UTF-8");
    }

    #[test]
    fn rejects_a_hostname_with_a_dot() {
        let error = SystemSpec::new("platinum.local".into(), "Etc/UTC".into(), "C.UTF-8".into())
            .expect_err("hostname с точкой должен отклоняться");

        assert!(matches!(error, SystemError::InvalidHostname { .. }));
    }

    #[test]
    fn rejects_a_timezone_escaping_the_zoneinfo_directory() {
        let error = SystemSpec::new(
            "platinum".into(),
            "../../etc/shadow".into(),
            "C.UTF-8".into(),
        )
        .expect_err("выход за пределы zoneinfo должен отклоняться");

        assert!(matches!(error, SystemError::InvalidTimezone { .. }));
    }

    #[test]
    fn rejects_a_locale_without_a_charset() {
        let error = SystemSpec::new("platinum".into(), "Etc/UTC".into(), "en_US".into())
            .expect_err("локаль без charset должна отклоняться");

        assert!(matches!(error, SystemError::InvalidLocale { .. }));
    }

    #[test]
    fn rejects_a_password_stored_in_plaintext() {
        let error = User::new("platinum".into(), "hunter2".into(), Vec::new(), None, true)
            .expect_err("открытый пароль не должен приниматься");

        assert_eq!(
            error,
            SystemError::PlaintextPassword {
                name: "platinum".into()
            }
        );
    }

    #[test]
    fn uses_bash_as_the_default_shell() {
        let user = User::new(
            "platinum".into(),
            "$6$salt$hash".into(),
            vec!["sudo".into()],
            None,
            true,
        )
        .expect("корректный пользователь должен приниматься");

        assert_eq!(user.shell, "/bin/bash");
    }

    #[test]
    fn rejects_an_fstab_field_with_a_space() {
        let error = Filesystem::new(
            "LABEL=platinum root".into(),
            "/".into(),
            "ext4".into(),
            "defaults".into(),
            0,
            1,
        )
        .expect_err("пробел внутри поля сдвинул бы колонки fstab");

        assert!(matches!(error, SystemError::InvalidFstabField { .. }));
    }

    #[test]
    fn rejects_a_relative_mount_point() {
        let error = Filesystem::new(
            "LABEL=platinum-root".into(),
            "boot".into(),
            "ext4".into(),
            "defaults".into(),
            0,
            2,
        )
        .expect_err("относительная точка монтирования должна отклоняться");

        assert!(matches!(error, SystemError::InvalidMountPoint { .. }));
    }

    #[test]
    fn rejects_an_interface_name_that_is_not_a_yaml_key() {
        let error = spec()
            .with_dhcp_interfaces(vec!["end0: {}".into()])
            .expect_err("имя интерфейса должно отклоняться");

        assert!(matches!(error, SystemError::InvalidInterface { .. }));
    }
}
