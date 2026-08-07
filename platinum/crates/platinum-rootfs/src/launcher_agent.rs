//! Запуск приложений, запрошенных оболочкой.
//!
//! Оболочка на чистом QML процессы запускать не умеет, поэтому приложение для
//! неё запускает служба уровня пользователя — как [`crate::console_agent`],
//! без повышения прав: всё, что запускается через неё, пользователь может
//! запустить и сам.
//!
//! Приложение подключается к Wayland-сокету самой оболочки, а не cage: с
//! ступени 3 оболочка — вложенный композитор, и окно должно попасть в её
//! сцену, а не лечь поверх неё во весь экран.
//!
//! Команда передаётся в base64 по той же причине, что и у консоли: сырую
//! строку пришлось бы разбирать из INI, который пишет Qt, и кавычки с
//! процентами внутри команды ломали бы разбор.

use std::{
    fs, io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use thiserror::Error;
use tracing::info;

/// Скрипт, запускающий приложение.
const AGENT_PATH: &str = "usr/libexec/platinum-launch-app";

/// Каталог пользовательских юнитов systemd.
const UNITS_DIRECTORY: &str = "usr/lib/systemd/user";

/// Юнит слежения за файлом заявки.
const PATH_UNIT: &str = "platinum-launcher.path";

/// Юнит, запускающий приложение.
const SERVICE_UNIT: &str = "platinum-launcher.service";

/// Каталог включённых пользовательских юнитов.
const WANTS_DIRECTORY: &str = "etc/systemd/user/default.target.wants";

/// Ошибки установки лаунчера.
#[derive(Debug, Error)]
pub enum LauncherAgentError {
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

/// Ставит службу, запускающую приложения для оболочки.
#[derive(Debug, Clone, Default)]
pub struct LauncherAgent;

impl LauncherAgent {
    /// Создаёт установщик.
    pub fn new() -> Self {
        Self
    }

    /// Пишет скрипт и пользовательские юниты в rootfs.
    pub fn apply(&self, rootfs: &Path) -> Result<(), LauncherAgentError> {
        let agent = rootfs.join(AGENT_PATH);
        write(&agent, AGENT)?;
        fs::set_permissions(&agent, fs::Permissions::from_mode(0o755)).map_err(|source| {
            LauncherAgentError::Write {
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

        // Юнит включается симлинком: systemd целевой системы в chroot не
        // запущен, а `systemctl --user enable` требует живой сессии.
        let wants = rootfs.join(WANTS_DIRECTORY).join(PATH_UNIT);
        if let Some(parent) = wants.parent() {
            fs::create_dir_all(parent).map_err(|source| LauncherAgentError::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let _ = fs::remove_file(&wants);
        std::os::unix::fs::symlink(format!("/{UNITS_DIRECTORY}/{PATH_UNIT}"), &wants).map_err(
            |source| LauncherAgentError::Write {
                path: wants.clone(),
                source,
            },
        )?;

        info!("launcher agent installed");

        Ok(())
    }
}

/// Пишет файл, создавая родительские каталоги.
fn write(path: &Path, contents: &str) -> Result<(), LauncherAgentError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| LauncherAgentError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    fs::write(path, contents).map_err(|source| LauncherAgentError::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Скрипт, запускающий приложение для оболочки.
const AGENT: &str = r#"#!/bin/sh
# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.
#
# Запускает приложение, запрошенное оболочкой, от имени её пользователя.
# Права здесь не повышаются: служба пользовательская.
set -u

RUNTIME="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
REQUEST="$RUNTIME/platinum/launch.in"

[ -r "$REQUEST" ] || exit 0

# Команда приходит в base64: сырую строку пришлось бы вынимать из INI, который
# пишет Qt, и кавычки с процентами внутри команды ломали бы разбор.
encoded=$(sed -n 's/^command=//p' "$REQUEST" | tail -1 | tr -d '\r' | sed 's/^"//; s/"$//')
[ -n "$encoded" ] || exit 0

command=$(printf '%s' "$encoded" | base64 -d 2> /dev/null || true)
[ -n "$command" ] || exit 0

# Приложение живёт в собственном transient-юните, а не в этой службе: служба
# oneshot завершилась бы и утащила процесс за собой, а systemd-run отвязывает
# его и сам прибирает юнит после выхода (--collect).
#
# WAYLAND_DISPLAY указывает на сокет самой оболочки, а не cage: окно должно
# попасть в её сцену, а не лечь поверх оболочки во весь экран.
exec systemd-run --user --collect \
    --setenv=WAYLAND_DISPLAY=platinum-0 \
    --setenv=QT_QPA_PLATFORM=wayland \
    --setenv=GDK_BACKEND=wayland \
    -- sh -c "$command"
"#;

/// Юнит слежения за файлом заявки.
const PATH_UNIT_FILE: &str = "\
# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.
[Unit]
Description=Заявка оболочки Platinum на запуск приложения

[Path]
# %t — каталог времени выполнения пользователя, /run/user/<uid>.
PathChanged=%t/platinum/launch.in
Unit=platinum-launcher.service

[Install]
WantedBy=default.target
";

/// Юнит, запускающий приложение.
const SERVICE_UNIT_FILE: &str = "\
# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.
[Unit]
Description=Запуск приложения для оболочки Platinum

[Service]
Type=oneshot
ExecStart=/usr/libexec/platinum-launch-app
";

#[cfg(test)]
mod tests {
    use super::AGENT;

    /// Приложение обязано попадать в сцену оболочки, а не в cage: иначе окно
    /// ляжет поверх оболочки во весь экран, минуя её управление окнами.
    #[test]
    fn points_the_application_at_the_shell_socket() {
        assert!(AGENT.contains("WAYLAND_DISPLAY=platinum-0"));
    }

    /// Служба oneshot завершается сразу: без transient-юнита процесс
    /// приложения был бы убит вместе с ней.
    #[test]
    fn detaches_the_application_from_the_oneshot_service() {
        assert!(AGENT.contains("systemd-run --user --collect"));
    }

    /// Команда приходит в base64: сырую строку ломали бы кавычки внутри неё.
    #[test]
    fn decodes_the_command_from_base64() {
        assert!(AGENT.contains("base64 -d"));
    }
}
