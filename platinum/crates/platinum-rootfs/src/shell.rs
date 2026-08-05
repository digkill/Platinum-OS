//! Запуск графической оболочки при старте системы.
//!
//! Оболочка — это две вещи, которых нет в пакетах: цель systemd по умолчанию и
//! автоматический вход. Устройство с сенсорным экраном и без клавиатуры иначе
//! останавливается на приглашении логина, которого пользователь не ждёт.
//!
//! Настройка пишется файлами с хоста, а не командами в chroot: результат виден
//! в diff сборки и не зависит от того, запущен ли systemd целевой системы.

use std::{
    fs, io,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
};

use thiserror::Error;
use tracing::info;

/// Каталог drop-in конфигурации display manager.
const SDDM_DIRECTORY: &str = "etc/sddm.conf.d";

/// Файл конфигурации, создаваемый сборкой.
const SDDM_FILE: &str = "10-platinum.conf";

/// Каталог, куда ставится собственный домашний экран.
const HOMESCREEN_DIRECTORY: &str = "usr/share/platinum/homescreen";

/// Каталог сессий Wayland, из которого их читает display manager.
const SESSIONS_DIRECTORY: &str = "usr/share/wayland-sessions";

/// Запускающий скрипт собственной оболочки.
const LAUNCHER_PATH: &str = "usr/bin/platinum-shell";

/// Имя сессии собственной оболочки.
pub const OWN_SESSION: &str = "platinum-shell";

/// Симлинк цели systemd по умолчанию.
const DEFAULT_TARGET_LINK: &str = "etc/systemd/system/default.target";

/// Цель systemd, поднимающая графическую сессию.
const GRAPHICAL_TARGET: &str = "/usr/lib/systemd/system/graphical.target";

/// Ошибки настройки оболочки.
#[derive(Debug, Error)]
pub enum ShellError {
    /// Имя сессии попадает в конфигурацию display manager.
    #[error("недопустимое имя сессии `{session}`")]
    InvalidSession {
        /// Отклонённое значение.
        session: String,
    },
    /// Имя пользователя автовхода по политике Debian.
    #[error("недопустимое имя пользователя автовхода `{user}`")]
    InvalidUser {
        /// Отклонённое значение.
        user: String,
    },
    /// Каталога с QML оболочки нет по указанному пути.
    #[error("каталог домашнего экрана отсутствует: {path}")]
    MissingHomescreen {
        /// Ожидавшийся каталог.
        path: PathBuf,
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

/// Параметры графической оболочки.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellSpec {
    /// Имя `.desktop`-сессии Wayland, например `plasma-mobile`.
    pub session: String,
    /// Пользователь автоматического входа, если он включён.
    pub autologin_user: Option<String>,
    /// Каталог QML собственного домашнего экрана, если он есть.
    pub homescreen: Option<PathBuf>,
}

impl ShellSpec {
    /// Создаёт проверенное описание оболочки.
    pub fn new(
        session: String,
        autologin_user: Option<String>,
        homescreen: Option<PathBuf>,
    ) -> Result<Self, ShellError> {
        // Имя сессии подставляется в ini-файл: перевод строки в нём разорвал бы
        // секцию, и display manager молча взял бы настройки по умолчанию.
        if session.is_empty()
            || session.contains(['\n', '\r', '='])
            || session.contains(char::is_whitespace)
        {
            return Err(ShellError::InvalidSession { session });
        }

        if let Some(user) = &autologin_user
            && (user.is_empty()
                || !user
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'))
        {
            return Err(ShellError::InvalidUser { user: user.clone() });
        }

        Ok(Self {
            session,
            autologin_user,
            homescreen,
        })
    }
}

/// Включает графическую сессию в готовом rootfs.
#[derive(Debug, Clone)]
pub struct ShellConfigurator {
    spec: ShellSpec,
}

impl ShellConfigurator {
    /// Создаёт конфигуратор для параметров оболочки.
    pub fn new(spec: ShellSpec) -> Self {
        Self { spec }
    }

    /// Пишет конфигурацию display manager и переключает цель по умолчанию.
    pub fn apply(&self, rootfs: &Path) -> Result<(), ShellError> {
        let directory = rootfs.join(SDDM_DIRECTORY);
        fs::create_dir_all(&directory).map_err(|source| ShellError::Write {
            path: directory.clone(),
            source,
        })?;

        let path = directory.join(SDDM_FILE);
        fs::write(&path, render(&self.spec)).map_err(|source| ShellError::Write {
            path: path.clone(),
            source,
        })?;

        self.install_homescreen(rootfs)?;
        self.select_graphical_target(rootfs)?;

        info!(
            session = %self.spec.session,
            autologin = self.spec.autologin_user.is_some(),
            "graphical shell enabled"
        );

        Ok(())
    }

    /// Ставит собственный домашний экран и сессию для него.
    ///
    /// Оболочка запускается композитором-киоском `cage`: он даёт Wayland и
    /// полноэкранное окно, но не тащит менеджер окон. Для домашнего экрана
    /// этого достаточно, а запуск приложений появится вместе с плазмоидом.
    fn install_homescreen(&self, rootfs: &Path) -> Result<(), ShellError> {
        let Some(source) = &self.spec.homescreen else {
            return Ok(());
        };

        if !source.is_dir() {
            return Err(ShellError::MissingHomescreen {
                path: source.clone(),
            });
        }

        let target = rootfs.join(HOMESCREEN_DIRECTORY);
        remove_tree(&target)?;
        copy_tree(source, &target)?;

        let launcher = rootfs.join(LAUNCHER_PATH);
        write_file(&launcher, LAUNCHER)?;
        fs::set_permissions(&launcher, fs::Permissions::from_mode(0o755)).map_err(|source| {
            ShellError::Write {
                path: launcher.clone(),
                source,
            }
        })?;

        write_file(
            &rootfs
                .join(SESSIONS_DIRECTORY)
                .join(format!("{OWN_SESSION}.desktop")),
            SESSION,
        )
    }

    /// Делает `graphical.target` целью systemd по умолчанию.
    ///
    /// Симлинк ставится вручную, а не через `systemctl set-default`: команда
    /// обращается к запущенному systemd, которого в chroot нет.
    fn select_graphical_target(&self, rootfs: &Path) -> Result<(), ShellError> {
        let link = rootfs.join(DEFAULT_TARGET_LINK);
        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent).map_err(|source| ShellError::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        if fs::symlink_metadata(&link).is_ok() {
            fs::remove_file(&link).map_err(|source| ShellError::Write {
                path: link.clone(),
                source,
            })?;
        }

        symlink(GRAPHICAL_TARGET, &link).map_err(|source| ShellError::Write { path: link, source })
    }
}

/// Записывает файл, создавая родительские каталоги.
fn write_file(path: &Path, contents: &str) -> Result<(), ShellError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ShellError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    fs::write(path, contents).map_err(|source| ShellError::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Удаляет дерево, если оно есть: сборка переиспользует work-dir.
fn remove_tree(path: &Path) -> Result<(), ShellError> {
    let _ = fs::remove_dir_all(path);

    Ok(())
}

/// Рекурсивно копирует каталог QML в rootfs.
fn copy_tree(source: &Path, target: &Path) -> Result<(), ShellError> {
    fs::create_dir_all(target).map_err(|error| ShellError::Write {
        path: target.to_path_buf(),
        source: error,
    })?;

    let entries = fs::read_dir(source).map_err(|error| ShellError::Write {
        path: source.to_path_buf(),
        source: error,
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| ShellError::Write {
            path: source.to_path_buf(),
            source: error,
        })?;

        let from = entry.path();
        let to = target.join(entry.file_name());

        if from.is_dir() {
            copy_tree(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|error| ShellError::Write {
                path: to.clone(),
                source: error,
            })?;
        }
    }

    Ok(())
}

/// Скрипт запуска собственной оболочки.
///
/// Имя бинаря QML между выпусками Qt менялось, поэтому кандидаты проверяются:
/// жёстко зашитый путь дал бы пустой экран без единого сообщения.
const LAUNCHER: &str = r#"#!/bin/sh
# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.
set -eu

HOME_QML=/usr/share/platinum/homescreen/Home.qml

for candidate in qml6 /usr/lib/qt6/bin/qml qml; do
    if command -v "$candidate" > /dev/null 2>&1; then
        QML="$candidate"
        break
    fi
done

[ -n "${QML:-}" ] || { echo "platinum-shell: не найден исполняемый файл qml" >&2; exit 1; }

# cage -d: композитор-киоск на всё окно, без панелей и декораций.
exec cage -d -- "$QML" -I /usr/share/platinum/homescreen "$HOME_QML"
"#;

/// Файл сессии Wayland для display manager.
const SESSION: &str = "\
[Desktop Entry]
Name=Platinum Shell
Comment=Домашний экран Platinum OS
Exec=/usr/bin/platinum-shell
Type=Application
DesktopNames=Platinum
";

/// Формирует drop-in конфигурацию display manager.
fn render(spec: &ShellSpec) -> String {
    let mut contents = String::from(
        "# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.\n\
         [General]\n\
         DisplayServer=wayland\n\
         \n",
    );

    if let Some(user) = &spec.autologin_user {
        contents.push_str(&format!(
            "[Autologin]\n\
             User={user}\n\
             Session={}\n\
             # Relogin возвращает сессию после выхода: на устройстве без\n\
             # клавиатуры экран логина — тупик.\n\
             Relogin=true\n",
            spec.session
        ));
    }

    contents
}

#[cfg(test)]
mod tests {
    use super::{ShellError, ShellSpec, render};

    #[test]
    fn renders_wayland_autologin() {
        let spec = ShellSpec::new("plasma-mobile".into(), Some("platinum".into()), None)
            .expect("описание оболочки должно быть корректным");

        let contents = render(&spec);

        assert!(contents.contains("DisplayServer=wayland\n"));
        assert!(contents.contains("User=platinum\n"));
        assert!(contents.contains("Session=plasma-mobile\n"));
        assert!(contents.contains("Relogin=true\n"));
    }

    /// Без автовхода секции быть не должно: пустой `User=` заставил бы sddm
    /// пытаться войти под несуществующей учётной записью.
    #[test]
    fn omits_autologin_when_it_is_not_requested() {
        let spec =
            ShellSpec::new("plasma-mobile".into(), None, None).expect("сессия без автовхода");

        assert!(!render(&spec).contains("[Autologin]"));
    }

    /// Имя бинаря QML между выпусками Qt менялось, поэтому запуск обязан
    /// перебирать кандидатов, а не полагаться на один путь.
    #[test]
    fn launcher_probes_several_qml_binaries() {
        assert!(super::LAUNCHER.contains("for candidate in qml6"));
        assert!(super::LAUNCHER.contains("cage -d --"));
        assert!(
            super::LAUNCHER.contains("не найден исполняемый файл qml"),
            "отсутствие qml обязано быть сообщением, а не пустым экраном"
        );
    }

    #[test]
    fn rejects_a_session_name_that_breaks_the_ini_file() {
        for session in ["", "plasma mobile", "a\nb", "x=y"] {
            let error = ShellSpec::new(session.into(), None, None)
                .expect_err("некорректное имя сессии должно отклоняться");

            assert!(matches!(error, ShellError::InvalidSession { .. }));
        }
    }

    #[test]
    fn rejects_an_invalid_autologin_user() {
        let error = ShellSpec::new("plasma-mobile".into(), Some("root; rm -rf".into()), None)
            .expect_err("некорректное имя пользователя должно отклоняться");

        assert!(matches!(error, ShellError::InvalidUser { .. }));
    }
}
