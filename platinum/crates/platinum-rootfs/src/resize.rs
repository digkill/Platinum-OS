//! Расширение корневой файловой системы при первом запуске.
//!
//! Образ выпускается фиксированного размера, рассчитанного на самую маленькую
//! поддерживаемую карту. Без расширения устройство с картой на 64 ГБ
//! использовало бы 3 ГиБ, а остальное осталось бы недоступным.
//!
//! Расширение делается на устройстве, а не при сборке: размер носителя
//! известен только там. Служба одноразовая и снимает себя сама — повторные
//! запуски ничего не изменили бы, но каждый стоил бы времени загрузки.

use std::{
    fs, io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use thiserror::Error;
use tracing::info;

/// Каталог unit-файлов, которые ставит не пакетный менеджер, а сборка.
const UNIT_DIRECTORY: &str = "etc/systemd/system";

/// Каталог, из которого systemd запускает службы уровня multi-user.
const WANTS_DIRECTORY: &str = "etc/systemd/system/multi-user.target.wants";

/// Каталог вспомогательных скриптов Platinum.
const SCRIPT_DIRECTORY: &str = "usr/lib/platinum";

/// Имя службы расширения.
const UNIT_FILE: &str = "platinum-resize-rootfs.service";

/// Имя скрипта расширения.
const SCRIPT_FILE: &str = "resize-rootfs.sh";

/// Ошибки подготовки расширения rootfs.
#[derive(Debug, Error)]
pub enum ResizeError {
    /// Файл или каталог не удалось создать.
    #[error("не удалось записать `{path}`: {source}")]
    Write {
        /// Проблемный путь.
        path: PathBuf,
        /// Исходная ошибка файловой системы.
        #[source]
        source: io::Error,
    },
}

/// Устанавливает одноразовую службу расширения корня.
#[derive(Debug, Clone, Copy, Default)]
pub struct RootfsExpander;

impl RootfsExpander {
    /// Создаёт установщик службы.
    pub fn new() -> Self {
        Self
    }

    /// Пишет скрипт, unit и симлинк автозапуска в rootfs.
    pub fn install(&self, rootfs: &Path) -> Result<(), ResizeError> {
        let script = rootfs.join(SCRIPT_DIRECTORY).join(SCRIPT_FILE);
        write(&script, SCRIPT)?;
        // Скрипт запускает systemd, поэтому бит исполнения обязателен: без него
        // служба упала бы с `Exec format error` уже на устройстве.
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).map_err(|source| {
            ResizeError::Write {
                path: script.clone(),
                source,
            }
        })?;

        write(&rootfs.join(UNIT_DIRECTORY).join(UNIT_FILE), UNIT)?;

        // Служба включается симлинком, а не `systemctl enable`: включение
        // делается на хосте сборки, где systemd целевой системы не запущен.
        let link = rootfs.join(WANTS_DIRECTORY).join(UNIT_FILE);
        create_directory(link.parent().unwrap_or(&link))?;
        if fs::symlink_metadata(&link).is_ok() {
            fs::remove_file(&link).map_err(|source| ResizeError::Write {
                path: link.clone(),
                source,
            })?;
        }
        std::os::unix::fs::symlink(format!("/{UNIT_DIRECTORY}/{UNIT_FILE}"), &link).map_err(
            |source| ResizeError::Write {
                path: link.clone(),
                source,
            },
        )?;

        info!(unit = UNIT_FILE, "rootfs expansion enabled");

        Ok(())
    }
}

/// Записывает файл, создавая родительские каталоги.
fn write(path: &Path, contents: &str) -> Result<(), ResizeError> {
    if let Some(parent) = path.parent() {
        create_directory(parent)?;
    }

    fs::write(path, contents).map_err(|source| ResizeError::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Создаёт каталог вместе с родителями.
fn create_directory(path: &Path) -> Result<(), ResizeError> {
    fs::create_dir_all(path).map_err(|source| ResizeError::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Unit одноразового расширения корня.
///
/// `Before=` не указывается: расширение online, файловая система остаётся
/// смонтированной и доступной, поэтому задерживать из-за него загрузку незачем.
const UNIT: &str = "\
# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.
[Unit]
Description=Platinum OS: расширение корневой файловой системы на весь носитель
Documentation=man:resize2fs(8)
ConditionPathExists=/usr/lib/platinum/resize-rootfs.sh
After=local-fs.target
Wants=local-fs.target

[Service]
Type=oneshot
ExecStart=/usr/lib/platinum/resize-rootfs.sh
RemainAfterExit=yes
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
";

/// Скрипт расширения раздела и файловой системы.
///
/// Используются `parted` и `resize2fs` из `packages.toml`; отдельный
/// `cloud-guest-utils` ради `growpart` не добавляется.
const SCRIPT: &str = r#"#!/bin/sh
# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.
#
# Расширяет последний раздел на весь носитель и растит ext4 по месту.
# Служба снимает себя после успеха: повторный запуск ничего не изменит.
set -eu

unit="platinum-resize-rootfs.service"

fail() {
    echo "resize-rootfs: $1" >&2
    exit 1
}

part="$(findmnt -no SOURCE /)" || fail "не удалось определить устройство корня"
case "$part" in
    /dev/*) ;;
    *) fail "корень смонтирован не с блочного устройства: $part" ;;
esac

# Имя диска и номер раздела: mmcblk0p1 -> mmcblk0 + 1, sda1 -> sda + 1.
name="${part#/dev/}"
sys="/sys/class/block/$name"
[ -d "$sys" ] || fail "нет $sys"
number="$(cat "$sys/partition")" || fail "$name не является разделом"
disk="/dev/$(basename "$(readlink -f "$sys/..")")"

echo "resize-rootfs: диск $disk, раздел $number ($part)"

# parted печатает предупреждение о смонтированной ФС и растит раздел на месте;
# ядро узнаёт новый размер через BLKPG, перезагрузка не нужна.
parted -s -m "$disk" resizepart "$number" 100% || fail "parted не смог расширить раздел"
resize2fs "$part" || fail "resize2fs не смог расширить файловую систему"

echo "resize-rootfs: готово"

# Служба выключается только после успеха: иначе ошибка осталась бы незамеченной
# и носитель навсегда остался бы недоиспользованным.
rm -f "/etc/systemd/system/multi-user.target.wants/$unit"

exit 0
"#;

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{RootfsExpander, SCRIPT, UNIT};

    fn temporary(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "platinum-resize-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("системное время должно быть позже Unix epoch")
                .as_nanos()
        ))
    }

    #[test]
    fn installs_an_enabled_oneshot_service() {
        let root = temporary("install");
        fs::create_dir_all(&root).expect("временный каталог должен создаваться");

        RootfsExpander::new()
            .install(&root)
            .expect("служба должна устанавливаться");

        let script = root.join("usr/lib/platinum/resize-rootfs.sh");
        assert!(script.is_file());

        // Без бита исполнения systemd не запустит скрипт.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mode = fs::metadata(&script)
                .expect("метаданные скрипта должны читаться")
                .permissions()
                .mode();
            assert_eq!(mode & 0o111, 0o111, "скрипт обязан быть исполняемым");
        }

        // Симлинк в multi-user.target.wants — это и есть «служба включена».
        let link =
            root.join("etc/systemd/system/multi-user.target.wants/platinum-resize-rootfs.service");
        assert!(
            fs::symlink_metadata(&link).is_ok(),
            "служба обязана быть включена"
        );

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }

    /// Повторная установка не должна падать на существующем симлинке.
    #[test]
    fn reinstalls_over_a_previous_run() {
        let root = temporary("repeat");
        fs::create_dir_all(&root).expect("временный каталог должен создаваться");

        let expander = RootfsExpander::new();
        expander.install(&root).expect("первая установка");
        expander
            .install(&root)
            .expect("повторная установка не должна падать");

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }

    /// Скрипт снимает службу только после успешного расширения.
    #[test]
    fn disables_itself_only_after_success() {
        let disable = SCRIPT
            .find("rm -f \"/etc/systemd/system/multi-user.target.wants/$unit\"")
            .expect("скрипт обязан снимать службу");
        let resize = SCRIPT
            .find("resize2fs \"$part\"")
            .expect("скрипт обязан вызывать resize2fs");

        assert!(
            resize < disable,
            "служба снимается после расширения, а не до него"
        );
        assert!(
            SCRIPT.contains("set -eu"),
            "ошибки обязаны прерывать скрипт"
        );
        assert!(UNIT.contains("Type=oneshot"));
    }
}
