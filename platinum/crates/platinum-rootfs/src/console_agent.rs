//! Выполнение команд, запрошенных консолью оболочки.
//!
//! Оболочка на чистом QML процессы запускать не умеет, поэтому команду для неё
//! выполняет служба. В отличие от [`crate::settings_agent`], эта служба
//! работает **от пользователя оболочки**, а не от root: консоль не должна
//! повышать права. Всё, что через неё можно сделать, пользователь и так может
//! сделать своей учётной записью.
//!
//! Настоящего терминала здесь нет. Псевдотерминала нет тоже, поэтому `vim`,
//! `top` и всё, что перерисовывает экран, работать не будет: команда
//! запускается, её вывод целиком складывается в файл, оболочка его показывает.
//! Это осознанный размен — интерактивный терминал требует эмулятора VT и
//! отдельного окна Wayland, а до них ещё далеко.
//!
//! Команда передаётся в base64. Иначе её пришлось бы разбирать из INI, который
//! пишет Qt, и кавычки с процентами внутри команды ломали бы разбор.

use std::{
    fs, io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use thiserror::Error;
use tracing::info;

/// Скрипт, выполняющий команду.
const AGENT_PATH: &str = "usr/libexec/platinum-run-console";

/// Каталог пользовательских юнитов systemd.
const UNITS_DIRECTORY: &str = "usr/lib/systemd/user";

/// Юнит слежения за файлом команды.
const PATH_UNIT: &str = "platinum-console.path";

/// Юнит, выполняющий команду.
const SERVICE_UNIT: &str = "platinum-console.service";

/// Каталог включённых пользовательских юнитов.
const WANTS_DIRECTORY: &str = "etc/systemd/user/default.target.wants";

/// Ошибки установки консоли.
#[derive(Debug, Error)]
pub enum ConsoleAgentError {
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

/// Ставит службу, выполняющую команды консоли.
#[derive(Debug, Clone, Default)]
pub struct ConsoleAgent;

impl ConsoleAgent {
    /// Создаёт установщик.
    pub fn new() -> Self {
        Self
    }

    /// Пишет скрипт и пользовательские юниты в rootfs.
    pub fn apply(&self, rootfs: &Path) -> Result<(), ConsoleAgentError> {
        let agent = rootfs.join(AGENT_PATH);
        write(&agent, AGENT)?;
        fs::set_permissions(&agent, fs::Permissions::from_mode(0o755)).map_err(|source| {
            ConsoleAgentError::Write {
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
            fs::create_dir_all(parent).map_err(|source| ConsoleAgentError::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let _ = fs::remove_file(&wants);
        std::os::unix::fs::symlink(format!("/{UNITS_DIRECTORY}/{PATH_UNIT}"), &wants).map_err(
            |source| ConsoleAgentError::Write {
                path: wants.clone(),
                source,
            },
        )?;

        info!("console agent installed");

        Ok(())
    }
}

/// Пишет файл, создавая родительские каталоги.
fn write(path: &Path, contents: &str) -> Result<(), ConsoleAgentError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ConsoleAgentError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    fs::write(path, contents).map_err(|source| ConsoleAgentError::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Скрипт, выполняющий команду консоли.
const AGENT: &str = r#"#!/bin/sh
# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.
#
# Выполняет команду, запрошенную консолью оболочки, от имени пользователя
# оболочки. Права здесь не повышаются: служба пользовательская.
set -u

RUNTIME="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
DIRECTORY="$RUNTIME/platinum"
REQUEST="$DIRECTORY/console.in"
RESULT="$DIRECTORY/console.out"

[ -r "$REQUEST" ] || exit 0

mkdir -p "$DIRECTORY"

# Команда приходит в base64: сырую строку пришлось бы вынимать из INI, который
# пишет Qt, и кавычки с процентами внутри команды ломали бы разбор.
encoded=$(sed -n 's/^command=//p' "$REQUEST" | tail -1 | tr -d '\r' | sed 's/^"//; s/"$//')
[ -n "$encoded" ] || exit 0

command=$(printf '%s' "$encoded" | base64 -d 2> /dev/null || true)
[ -n "$command" ] || exit 0

# Номер запроса возвращается в первой строке вывода. Без него оболочка не
# отличила бы свежий ответ от предыдущего, который лежит в том же файле, и
# показала бы результат прошлой команды как результат новой.
seq=$(sed -n 's/^seq=//p' "$REQUEST" | tail -1 | tr -d '\r' | sed 's/^"//; s/"$//')

{
    printf '#seq %s\n' "$seq"
    printf '$ %s\n' "$command"

    # Ограничение по времени обязательно: команда без него подвесила бы
    # консоль навсегда, а прервать её с экрана нечем.
    timeout 20 sh -c "$command" 2>&1
    status=$?

    if [ "$status" -eq 124 ]; then
        printf '\n[прервано по времени: 20 с]\n'
    elif [ "$status" -ne 0 ]; then
        printf '\n[код возврата %s]\n' "$status"
    fi
} > "$RESULT.part" 2>&1

# Готовый вывод появляется целиком: оболочка иначе прочитала бы половину.
mv "$RESULT.part" "$RESULT"
"#;

/// Юнит слежения за файлом команды.
const PATH_UNIT_FILE: &str = "\
# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.
[Unit]
Description=Команда консоли Platinum

[Path]
# %t — каталог времени выполнения пользователя, /run/user/<uid>.
PathChanged=%t/platinum/console.in
Unit=platinum-console.service

[Install]
WantedBy=default.target
";

/// Юнит, выполняющий команду.
const SERVICE_UNIT_FILE: &str = "\
# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.
[Unit]
Description=Выполнение команды консоли Platinum

[Service]
Type=oneshot
ExecStart=/usr/libexec/platinum-run-console
";

#[cfg(test)]
mod tests {
    use super::AGENT;

    /// Команда без ограничения по времени подвесила бы консоль навсегда:
    /// прервать её с экрана нечем.
    #[test]
    fn limits_how_long_a_command_may_run() {
        assert!(AGENT.contains("timeout 20 sh -c"));
    }

    /// Вывод должен появляться целиком, иначе оболочка прочитает половину.
    #[test]
    fn publishes_the_output_atomically() {
        assert!(AGENT.contains(".part"));
        assert!(AGENT.contains("mv \"$RESULT.part\" \"$RESULT\""));
    }

    /// Команда приходит в base64: сырую строку ломали бы кавычки внутри неё.
    #[test]
    fn decodes_the_command_from_base64() {
        assert!(AGENT.contains("base64 -d"));
    }

    /// Номер запроса возвращается с ответом: иначе оболочка приняла бы за
    /// свежий результат вывод предыдущей команды, лежащий в том же файле.
    #[test]
    fn echoes_the_request_number() {
        assert!(AGENT.contains("printf '#seq %s"));
    }
}
