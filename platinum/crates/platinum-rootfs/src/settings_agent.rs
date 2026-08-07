//! Применение системных настроек, запрошенных оболочкой.
//!
//! Оболочка написана на чистом QML: в образ входит только `qml6-module-qtquick`,
//! а запуск процессов есть лишь в расширениях на C++. Сменить часовой пояс она
//! поэтому не может — это делает `timedatectl`, и делает от root.
//!
//! Канал между ними — файл. Оболочка пишет запрос в `/run/platinum/system.conf`,
//! systemd замечает изменение и запускает скрипт, который единственный
//! превращает запрос в действие. Файл лежит в tmpfs: это заявка, а не хранилище
//! настроек, и переживать перезагрузку ей незачем — применённый пояс живёт в
//! `/etc/localtime`.
//!
//! Содержимое запроса проверяется. Файл доступен на запись оболочке, то есть
//! любому коду, запущенному под её пользователем, и доверять ему нельзя.

use std::{
    fs, io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use thiserror::Error;
use tracing::info;

/// Скрипт, применяющий запрос.
const AGENT_PATH: &str = "usr/libexec/platinum-apply-settings";

/// Каталог юнитов systemd.
const UNITS_DIRECTORY: &str = "usr/lib/systemd/system";

/// Юнит слежения за файлом запроса.
const PATH_UNIT: &str = "platinum-settings.path";

/// Юнит, применяющий запрос.
const SERVICE_UNIT: &str = "platinum-settings.service";

/// Правило создания каталога запроса при старте.
const TMPFILES_PATH: &str = "usr/lib/tmpfiles.d/platinum.conf";

/// Каталог, в котором systemd ищет включённые юниты.
const WANTS_DIRECTORY: &str = "etc/systemd/system/multi-user.target.wants";

/// Ошибки установки посредника.
#[derive(Debug, Error)]
pub enum SettingsAgentError {
    /// Имя пользователя попадает в правило tmpfiles.
    #[error("недопустимое имя пользователя `{user}`")]
    InvalidUser {
        /// Отклонённое значение.
        user: String,
    },
    /// Файл не удалось записать.
    #[error("не удалось записать `{path}`: {source}")]
    Write {
        /// Проблемный путь.
        path: PathBuf,
        /// Исходная ошибка файловой системы.
        #[source]
        source: io::Error,
    },
}

/// Ставит посредника между оболочкой и системными настройками.
#[derive(Debug, Clone)]
pub struct SettingsAgent {
    /// Пользователь оболочки: ему принадлежит каталог запроса.
    user: String,
}

impl SettingsAgent {
    /// Создаёт установщик для пользователя оболочки.
    pub fn new(user: String) -> Result<Self, SettingsAgentError> {
        if user.is_empty()
            || !user
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        {
            return Err(SettingsAgentError::InvalidUser { user });
        }

        Ok(Self { user })
    }

    /// Пишет скрипт, юниты и правило tmpfiles в rootfs.
    pub fn apply(&self, rootfs: &Path) -> Result<(), SettingsAgentError> {
        write(&rootfs.join(AGENT_PATH), AGENT)?;
        let agent = rootfs.join(AGENT_PATH);
        fs::set_permissions(&agent, fs::Permissions::from_mode(0o755)).map_err(|source| {
            SettingsAgentError::Write {
                path: agent.clone(),
                source,
            }
        })?;

        write(
            &rootfs.join(UNITS_DIRECTORY).join(PATH_UNIT),
            PATH_UNIT_FILE,
        )?;
        write(
            &rootfs.join(UNITS_DIRECTORY).join(SERVICE_UNIT),
            SERVICE_UNIT_FILE,
        )?;
        write(&rootfs.join(TMPFILES_PATH), &render_tmpfiles(&self.user))?;

        // Юниты включаются симлинком, а не `systemctl enable`: systemd целевой
        // системы в chroot не запущен, а результат должен быть виден в diff.
        //
        // Включены оба: слежение за заявкой и сам применяющий юнит. Второй
        // нужен при загрузке, когда заявки ещё нет, — он публикует текущее
        // состояние системы, иначе оболочке неоткуда узнать часовой пояс.
        for unit in [PATH_UNIT, SERVICE_UNIT] {
            let wants = rootfs.join(WANTS_DIRECTORY).join(unit);
            if let Some(parent) = wants.parent() {
                fs::create_dir_all(parent).map_err(|source| SettingsAgentError::Write {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            let _ = fs::remove_file(&wants);
            std::os::unix::fs::symlink(format!("/{UNITS_DIRECTORY}/{unit}"), &wants).map_err(
                |source| SettingsAgentError::Write {
                    path: wants.clone(),
                    source,
                },
            )?;
        }

        info!(user = %self.user, "system settings agent installed");

        Ok(())
    }
}

/// Пишет файл, создавая родительские каталоги.
fn write(path: &Path, contents: &str) -> Result<(), SettingsAgentError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| SettingsAgentError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    fs::write(path, contents).map_err(|source| SettingsAgentError::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Формирует правило создания каталога запроса.
fn render_tmpfiles(user: &str) -> String {
    format!(
        "# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.\n\
         # Каталог заявок оболочки: она пишет сюда запрос на смену системных\n\
         # настроек, которые сама применить не может.\n\
         d /run/platinum 0755 {user} {user} -\n"
    )
}

/// Скрипт, применяющий запрос.
const AGENT: &str = r#"#!/bin/sh
# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.
#
# Применяет системные настройки, запрошенные оболочкой. Это единственное место,
# где заявка из /run превращается в действие, поэтому проверка значений здесь
# обязательна: файл доступен на запись пользователю оболочки.
set -eu

REQUEST=/run/platinum/system.conf
STATE=/run/platinum/state.conf

# Qt пишет INI: `ключ=значение`, значение с пробелами — в кавычках.
value() {
    sed -n "s/^$1=//p" "$REQUEST" | tail -1 | tr -d '\r' | sed 's/^"//; s/"$//'
}

# Что получилось на самом деле. Оболочка читает этот файл и показывает его, а
# не свою заявку: заявка могла быть отклонена, и показывать её как факт значило
# бы врать пользователю.
publish() {
    mkdir -p /run/platinum
    {
        printf '[state]\n'
        printf 'timezone=%s\n' "$(timedatectl show -p Timezone --value)"
        printf 'ntp=%s\n' "$(timedatectl show -p NTPSynchronized --value)"
        printf 'ntp_enabled=%s\n' "$(timedatectl show -p NTP --value)"
    } > "$STATE"
    chmod 0644 "$STATE"
}

if [ ! -r "$REQUEST" ]; then
    # Заявки нет — значит это первый запуск после загрузки: публикуем состояние
    # и выходим.
    publish
    exit 0
fi

timezone=$(value timezone)
if [ -n "$timezone" ]; then
    case "$timezone" in
        # Только имя пояса и ничего больше: ни абсолютного пути, ни выхода
        # вверх по дереву, ни точки с запятой.
        /* | *..* | *[!A-Za-z0-9/_+-]*)
            echo "platinum-settings: отклонён часовой пояс '$timezone'" >&2
            ;;
        *)
            if [ -f "/usr/share/zoneinfo/$timezone" ]; then
                timedatectl set-timezone "$timezone"
            else
                echo "platinum-settings: нет такого пояса '$timezone'" >&2
            fi
            ;;
    esac
fi

# Отказ здесь не должен ронять скрипт: без службы синхронизации команда
# завершится ошибкой, а часовой пояс к этому моменту уже применён, и
# состояние ниже обязано опубликоваться.
ntp=$(value ntp)
case "$ntp" in
    true) timedatectl set-ntp true || echo "platinum-settings: нет службы синхронизации" >&2 ;;
    false) timedatectl set-ntp false || true ;;
esac

# При включённой синхронизации timedatectl откажется ставить время сам, и это
# правильно: иначе ручная правка молча уехала бы обратно через секунду.
moment=$(value time)
case "$moment" in
    [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]" "[0-9][0-9]:[0-9][0-9]:[0-9][0-9])
        timedatectl set-time "$moment" || true
        ;;
    "") ;;
    *)
        echo "platinum-settings: отклонено время '$moment'" >&2
        ;;
esac

publish
"#;

/// Юнит слежения за файлом запроса.
const PATH_UNIT_FILE: &str = "\
# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.
[Unit]
Description=Заявка оболочки Platinum на смену системных настроек

[Path]
# PathChanged, а не PathModified: заявка пишется целиком и закрывается, и
# реагировать нужно один раз, а не на каждую запись.
PathChanged=/run/platinum/system.conf
Unit=platinum-settings.service

[Install]
WantedBy=multi-user.target
";

/// Юнит, применяющий запрос.
const SERVICE_UNIT_FILE: &str = "\
# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.
[Unit]
Description=Применение системных настроек Platinum
After=systemd-timedated.service

[Service]
Type=oneshot
ExecStart=/usr/libexec/platinum-apply-settings

# Юнит включён и сам по себе: при загрузке заявки ещё нет, но состояние системы
# нужно опубликовать, иначе оболочка не знает, какой пояс стоит.
[Install]
WantedBy=multi-user.target
";

#[cfg(test)]
mod tests {
    use super::{AGENT, SettingsAgent, render_tmpfiles};

    #[test]
    fn rejects_a_user_name_that_would_break_the_tmpfiles_rule() {
        assert!(SettingsAgent::new(String::from("platinum user")).is_err());
        assert!(SettingsAgent::new(String::new()).is_err());
        assert!(SettingsAgent::new(String::from("platinum")).is_ok());
    }

    #[test]
    fn owns_the_request_directory_by_the_shell_user() {
        let rule = render_tmpfiles("platinum");

        assert!(rule.contains("d /run/platinum 0755 platinum platinum -\n"));
    }

    /// Заявка приходит из-под пользователя оболочки, поэтому скрипт обязан
    /// проверять её, а не подставлять в команду как есть.
    #[test]
    fn validates_the_requested_timezone() {
        assert!(AGENT.contains("/usr/share/zoneinfo/$timezone"));
        assert!(AGENT.contains("*[!A-Za-z0-9/_+-]*"));
        assert!(AGENT.contains("*..*"));
    }
}
