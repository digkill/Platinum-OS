//! Применение системной конфигурации к готовому rootfs.
//!
//! Файлы `/etc` пишутся с хоста, а не командами внутри chroot: результат не
//! зависит от того, какие утилиты оказались в базовом архиве, и его видно в
//! diff сборки. В chroot уходит только то, что обязано выполняться целевой
//! системой: генерация локали и создание пользователей.

use std::{
    fs, io,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
};

use thiserror::Error;
use tracing::{info, warn};

use crate::{
    chroot::{Chroot, ChrootError},
    resize::{ResizeError, RootfsExpander},
    shell::{ShellConfigurator, ShellError},
    system::{Filesystem, SystemSpec, User, WifiNetwork},
};

/// Файл имени устройства.
const HOSTNAME_FILE: &str = "etc/hostname";

/// Таблица локальных имён.
const HOSTS_FILE: &str = "etc/hosts";

/// Файл выбранного часового пояса.
const TIMEZONE_FILE: &str = "etc/timezone";

/// Симлинк на данные часового пояса.
const LOCALTIME_LINK: &str = "etc/localtime";

/// Каталог tzdata внутри rootfs.
const ZONEINFO_DIRECTORY: &str = "usr/share/zoneinfo";

/// Таблица монтирования.
const FSTAB_FILE: &str = "etc/fstab";

/// Список модулей, загружаемых при старте.
const MODULES_FILE: &str = "etc/modules";

/// Список генерируемых локалей.
const LOCALE_GEN_FILE: &str = "etc/locale.gen";

/// Конфигурация netplan, создаваемая сборкой.
const NETPLAN_FILE: &str = "etc/netplan/10-platinum.yaml";

/// База данных учётных записей.
const PASSWD_FILE: &str = "etc/passwd";

/// Ошибки применения системной конфигурации.
#[derive(Debug, Error)]
pub enum ConfigureError {
    /// Файл конфигурации не удалось записать.
    #[error("не удалось записать `{path}`: {source}")]
    Write {
        /// Проблемный путь.
        path: PathBuf,
        /// Исходная ошибка файловой системы.
        #[source]
        source: io::Error,
    },
    /// В rootfs нет данных запрошенного часового пояса.
    #[error(
        "часовой пояс `{timezone}` отсутствует в rootfs: нет `{path}`; добавьте tzdata в packages.toml"
    )]
    MissingTimezone {
        /// Запрошенный часовой пояс.
        timezone: String,
        /// Ожидавшийся путь внутри rootfs.
        path: PathBuf,
    },
    /// Ошибка chroot или команды внутри него.
    #[error(transparent)]
    Chroot(#[from] ChrootError),
    /// Не удалось установить службу расширения корня.
    #[error(transparent)]
    Resize(ResizeError),
    /// Не удалось включить графическую оболочку.
    #[error(transparent)]
    Shell(ShellError),
}

/// Применяет проверенную системную конфигурацию к rootfs.
#[derive(Debug, Clone)]
pub struct SystemConfigurator {
    spec: SystemSpec,
}

impl SystemConfigurator {
    /// Создаёт конфигуратор для проверенной спецификации.
    pub fn new(spec: SystemSpec) -> Self {
        Self { spec }
    }

    /// Записывает конфигурацию в rootfs и выполняет то, что требует chroot.
    pub fn apply(&self, chroot: &Chroot) -> Result<(), ConfigureError> {
        let root = chroot.root();

        write_file(
            &root.join(HOSTNAME_FILE),
            &format!("{}\n", self.spec.hostname),
        )?;
        write_file(&root.join(HOSTS_FILE), &render_hosts(&self.spec.hostname))?;
        self.link_timezone(root)?;
        self.write_fstab(root)?;
        self.write_netplan(root)?;
        self.write_modules(root)?;
        self.install_rootfs_expansion(root)?;
        self.enable_shell(root)?;
        write_file(
            &root.join(LOCALE_GEN_FILE),
            &render_locale_gen(&self.spec.locale, self.spec.locale_charset()),
        )?;

        let session = chroot.enter()?;

        // locale-gen компилирует локаль под целевую архитектуру, поэтому это
        // единственный способ получить рабочий LANG в готовом образе.
        session.run("locale-gen", &[])?;
        session.run("update-locale", &[&format!("LANG={}", self.spec.locale)])?;

        for user in &self.spec.users {
            create_user(&session, root, user)?;
        }

        info!(
            hostname = %self.spec.hostname,
            users = self.spec.users.len(),
            "system configured"
        );

        Ok(())
    }

    /// Связывает `/etc/localtime` с данными выбранного часового пояса.
    fn link_timezone(&self, root: &Path) -> Result<(), ConfigureError> {
        let zoneinfo = root.join(ZONEINFO_DIRECTORY).join(&self.spec.timezone);
        if !zoneinfo.exists() {
            return Err(ConfigureError::MissingTimezone {
                timezone: self.spec.timezone.clone(),
                path: zoneinfo,
            });
        }

        write_file(
            &root.join(TIMEZONE_FILE),
            &format!("{}\n", self.spec.timezone),
        )?;

        let link = root.join(LOCALTIME_LINK);
        if link.symlink_metadata().is_ok() {
            fs::remove_file(&link).map_err(|source| ConfigureError::Write {
                path: link.clone(),
                source,
            })?;
        }

        // Симлинк относительный: абсолютный указывал бы на zoneinfo хоста,
        // пока rootfs ещё не смонтирован как корень.
        let target = format!("../{ZONEINFO_DIRECTORY}/{}", self.spec.timezone);
        symlink(&target, &link).map_err(|source| ConfigureError::Write { path: link, source })
    }

    /// Включает графическую оболочку, если образ её запускает.
    fn enable_shell(&self, root: &Path) -> Result<(), ConfigureError> {
        let Some(shell) = &self.spec.shell else {
            return Ok(());
        };

        ShellConfigurator::new(shell.clone())
            .apply(root)
            .map_err(ConfigureError::Shell)
    }

    /// Ставит одноразовую службу расширения корня, если она включена.
    fn install_rootfs_expansion(&self, root: &Path) -> Result<(), ConfigureError> {
        if !self.spec.expand_rootfs {
            return Ok(());
        }

        RootfsExpander::new()
            .install(root)
            .map_err(ConfigureError::Resize)
    }

    /// Записывает `/etc/modules`, если плата объявила модули.
    ///
    /// Пустой список не создаёт файл: vendor-драйверы нужны не всякой плате, а
    /// пустой `/etc/modules` перезаписал бы то, что положил пакет.
    fn write_modules(&self, root: &Path) -> Result<(), ConfigureError> {
        if self.spec.modules.is_empty() {
            return Ok(());
        }

        write_file(
            &root.join(MODULES_FILE),
            &format!("{}\n", self.spec.modules.join("\n")),
        )
    }

    /// Записывает `/etc/fstab`, если конфигурация описывает разделы.
    ///
    /// Пустой список означает, что разметка ещё не определена: перезаписать
    /// fstab пустым файлом было бы хуже, чем не трогать его.
    fn write_fstab(&self, root: &Path) -> Result<(), ConfigureError> {
        if self.spec.filesystems.is_empty() {
            warn!("fstab не записан: в system.toml нет ни одной файловой системы");

            return Ok(());
        }

        write_file(
            &root.join(FSTAB_FILE),
            &render_fstab(&self.spec.filesystems),
        )
    }

    /// Записывает конфигурацию netplan для интерфейсов с DHCP.
    fn write_netplan(&self, root: &Path) -> Result<(), ConfigureError> {
        if self.spec.dhcp_interfaces.is_empty() && self.spec.wifi.is_empty() {
            return Ok(());
        }

        let path = root.join(NETPLAN_FILE);
        write_file(
            &path,
            &render_netplan(&self.spec.dhcp_interfaces, &self.spec.wifi),
        )?;

        // netplan отказывается применять конфигурацию, доступную на чтение
        // всем: файл может содержать сетевые секреты.
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|source| ConfigureError::Write { path, source })
    }
}

/// Создаёт учётную запись, если её ещё нет в rootfs.
fn create_user(
    session: &crate::chroot::ChrootSession<'_>,
    root: &Path,
    user: &User,
) -> Result<(), ConfigureError> {
    // Пользователь мог остаться от прошлой сборки: work-dir переиспользуется.
    // Тогда его настройки приводятся к конфигурации, а не пропускаются —
    // иначе смена пароля в `system.toml` молча не попала бы в образ.
    let existing = user_exists(root, &user.name);

    let mut arguments = vec![
        "--shell".to_owned(),
        user.shell.clone(),
        "--password".to_owned(),
        user.password_hash.clone(),
    ];

    if !user.groups.is_empty() {
        arguments.push("--groups".to_owned());
        arguments.push(user.groups.join(","));
    }

    arguments.push(user.name.clone());

    let borrowed: Vec<&str> = arguments.iter().map(String::as_str).collect();

    if existing {
        info!(user = %user.name, "пользователь уже существует, настройки приводятся к конфигурации");
        session.run("usermod", &borrowed)?;
    } else {
        let mut create = vec!["--create-home"];
        create.extend_from_slice(&borrowed);
        session.run("useradd", &create)?;
    }

    if user.force_password_change {
        // `chage --lastday 0` заставляет сменить пароль при первом входе:
        // пароль из репозитория не должен пережить первую загрузку устройства.
        session.run("chage", &["--lastday", "0", &user.name])?;
    } else if existing {
        // Снятие требования тоже обязано доезжать: иначе образ, пересобранный
        // с `force_password_change = false`, всё равно просил бы смену.
        session.run("chage", &["--lastday", "-1", &user.name])?;
    }

    Ok(())
}

/// Сообщает, есть ли пользователь в `/etc/passwd` целевой системы.
fn user_exists(root: &Path, name: &str) -> bool {
    let Ok(passwd) = fs::read_to_string(root.join(PASSWD_FILE)) else {
        return false;
    };

    passwd
        .lines()
        .any(|line| line.split(':').next() == Some(name))
}

/// Записывает файл, создавая недостающие каталоги.
fn write_file(path: &Path, contents: &str) -> Result<(), ConfigureError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ConfigureError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    fs::write(path, contents).map_err(|source| ConfigureError::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Формирует `/etc/hosts` с локальными именами устройства.
fn render_hosts(hostname: &str) -> String {
    format!(
        "127.0.0.1\tlocalhost\n\
         127.0.1.1\t{hostname}\n\
         \n\
         ::1\tlocalhost ip6-localhost ip6-loopback\n\
         ff02::1\tip6-allnodes\n\
         ff02::2\tip6-allrouters\n"
    )
}

/// Формирует `/etc/fstab` из описанных разделов.
fn render_fstab(filesystems: &[Filesystem]) -> String {
    let mut fstab = String::from(
        "# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.\n\
         # <источник> <точка> <тип> <опции> <dump> <pass>\n",
    );

    for filesystem in filesystems {
        fstab.push_str(&format!(
            "{} {} {} {} {} {}\n",
            filesystem.source,
            filesystem.mount_point,
            filesystem.filesystem,
            filesystem.options,
            filesystem.dump,
            filesystem.pass
        ));
    }

    fstab
}

/// Формирует конфигурацию netplan для интерфейсов с DHCP.
fn render_netplan(interfaces: &[String], wifi: &[WifiNetwork]) -> String {
    let mut netplan = String::from(
        "# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.\n\
         network:\n  version: 2\n  renderer: networkd\n",
    );

    if !interfaces.is_empty() {
        netplan.push_str("  ethernets:\n");
        for interface in interfaces {
            netplan.push_str(&format!("    {interface}:\n      dhcp4: true\n"));
        }
    }

    if !wifi.is_empty() {
        // Интерфейс задаётся шаблоном `wl*`, а не именем: systemd переименовывает
        // сетевые устройства по пути в шине, и жёсткий `wlan0` разошёлся бы с
        // фактическим именем на другой плате или после смены ядра.
        netplan.push_str(
            "  wifis:\n    platinum-wifi:\n      match:\n        name: \"wl*\"\n\
             \x20     dhcp4: true\n      access-points:\n",
        );

        for network in wifi {
            netplan.push_str(&format!("        \"{}\":\n", network.ssid));
            // PSK, а не пароль: netplan отдаёт 64 шестнадцатеричных символа
            // wpa_supplicant как есть, без строки с паролем в открытом виде.
            netplan.push_str(&format!("          password: \"{}\"\n", network.psk));

            if network.hidden {
                netplan.push_str("          hidden: true\n");
            }
        }
    }

    netplan
}

/// Формирует `/etc/locale.gen` с единственной локалью системы.
fn render_locale_gen(locale: &str, charset: &str) -> String {
    format!(
        "# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.\n\
         {locale} {charset}\n"
    )
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::system::Filesystem;

    use super::{render_fstab, render_hosts, render_locale_gen, render_netplan, user_exists};
    use crate::system::WifiNetwork;

    #[test]
    fn renders_hosts_with_the_device_name() {
        let hosts = render_hosts("platinum");

        assert!(hosts.contains("127.0.1.1\tplatinum\n"));
        assert!(hosts.contains("127.0.0.1\tlocalhost\n"));
    }

    #[test]
    fn renders_fstab_columns_in_order() {
        let fstab = render_fstab(&[Filesystem::new(
            "LABEL=platinum-root".into(),
            "/".into(),
            "ext4".into(),
            "defaults,noatime".into(),
            0,
            1,
        )
        .expect("запись fstab должна быть корректной")]);

        assert!(fstab.contains("LABEL=platinum-root / ext4 defaults,noatime 0 1\n"));
    }

    /// PSK обязан попадать в netplan вместо пароля сети.
    #[test]
    fn renders_wifi_with_a_psk_and_an_interface_pattern() {
        let network = WifiNetwork::new(
            "test-network".into(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
            false,
        )
        .expect("сеть должна быть корректной");

        let netplan = render_netplan(&[], &[network]);

        assert!(netplan.contains("  wifis:\n"));
        // Имя интерфейса шаблоном: systemd переименовывает устройства.
        assert!(netplan.contains("name: \"wl*\""));
        assert!(netplan.contains("\"test-network\":\n"));
        assert!(netplan.contains(
            "password: \"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\""
        ));
        // Без проводных интерфейсов пустая секция ethernets ломала бы netplan.
        assert!(!netplan.contains("ethernets:"));
    }

    /// Пароль сети в открытом виде обязан отклоняться, как и пароль учётной
    /// записи: он попал бы в образ, в лог сборки и в историю команд.
    #[test]
    fn rejects_a_wifi_password_in_plaintext() {
        let error = WifiNetwork::new(
            "test-network".into(),
            "пароль-в-открытом-виде".into(),
            false,
        )
        .expect_err("открытый пароль сети не должен приниматься");

        assert!(matches!(
            error,
            crate::system::SystemError::PlaintextWifiPassword { .. }
        ));
    }

    #[test]
    fn renders_netplan_for_each_interface() {
        let netplan = render_netplan(&["end0".to_owned()], &[]);

        assert!(netplan.contains("    end0:\n      dhcp4: true\n"));
        assert!(netplan.contains("renderer: networkd"));
    }

    #[test]
    fn renders_locale_gen_with_the_charset() {
        assert!(render_locale_gen("en_US.UTF-8", "UTF-8").contains("en_US.UTF-8 UTF-8\n"));
    }

    #[test]
    fn detects_an_already_created_user() {
        let root: PathBuf = std::env::temp_dir().join(format!(
            "platinum-configure-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("системное время должно быть позже Unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(root.join("etc")).expect("каталог etc должен создаваться");
        fs::write(
            root.join("etc/passwd"),
            "root:x:0:0:root:/root:/bin/bash\nplatinum:x:1000:1000::/home/platinum:/bin/bash\n",
        )
        .expect("passwd должен записываться");

        assert!(user_exists(&root, "platinum"));
        assert!(!user_exists(&root, "unknown"));

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }
}
