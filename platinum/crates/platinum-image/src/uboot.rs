//! Запись загрузчика в собранный образ.
//!
//! Смещения записи SPL и U-Boot Platinum у себя не хранит. Пакет Armbian
//! приносит в rootfs `platform_install.sh` с функцией `write_uboot_platform`,
//! которая знает их для family платы. Сборка вызывает именно её: иначе числа
//! пришлось бы дублировать и они разошлись бы с upstream при первом же
//! изменении.

use std::{
    io,
    path::{Path, PathBuf},
    process::Command,
};

use thiserror::Error;
use tracing::info;

/// Скрипт Armbian со смещениями записи загрузчика.
const PLATFORM_INSTALL: &str = "usr/lib/u-boot/platform_install.sh";

/// Ошибки записи загрузчика.
#[derive(Debug, Error)]
pub enum UbootError {
    /// В rootfs нет скрипта: пакет U-Boot не установлен.
    #[error("в rootfs нет `{path}`; сначала установите пакет U-Boot (stage install-uboot)")]
    MissingPlatformScript {
        /// Ожидавшийся путь.
        path: PathBuf,
    },
    /// Оболочку не удалось запустить.
    #[error("не удалось запустить bash для записи загрузчика: {source}")]
    StartShell {
        /// Исходная ошибка запуска процесса.
        #[source]
        source: io::Error,
    },
    /// Скрипт Armbian завершился с ошибкой.
    #[error("запись загрузчика завершилась с кодом {code}: {stderr}")]
    Failed {
        /// Код завершения или -1, если ОС не предоставила его.
        code: i32,
        /// Диагностика скрипта.
        stderr: String,
    },
}

/// Записывает загрузчик из rootfs в файл образа.
pub fn write_uboot(rootfs: &Path, image: &Path) -> Result<(), UbootError> {
    let script = rootfs.join(PLATFORM_INSTALL);
    if !script.is_file() {
        return Err(UbootError::MissingPlatformScript { path: script });
    }

    // Каталог бинарей объявляет сам скрипт: Armbian кладёт их в
    // `/usr/lib/linux-u-boot-<branch>-<board>`, а не рядом со скриптом, и имя
    // зависит от платы. Путь абсолютный для целевой системы, поэтому на хосте
    // к нему добавляется префикс rootfs.
    //
    // Скрипт исполняется на хосте, а не в chroot: цель записи — файл образа,
    // который внутри chroot недоступен.
    let output = Command::new("bash")
        .arg("-c")
        .arg(
            "set -euo pipefail\n\
             source \"$2\"\n\
             : \"${DIR:?platform_install.sh не объявил каталог загрузчика}\"\n\
             write_uboot_platform \"$1${DIR}\" \"$3\"",
        )
        .arg("platinum-write-uboot")
        .arg(rootfs)
        .arg(&script)
        .arg(image)
        .output()
        .map_err(|source| UbootError::StartShell { source })?;

    if !output.status.success() {
        return Err(UbootError::Failed {
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }

    info!(image = %image.display(), "bootloader written into image");

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{UbootError, write_uboot};

    #[test]
    fn reports_a_rootfs_without_the_platform_script() {
        let root: PathBuf = std::env::temp_dir().join(format!(
            "platinum-uboot-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("системное время должно быть позже Unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("временный каталог должен создаваться");

        let error = write_uboot(&root, &root.join("platinum.img"))
            .expect_err("rootfs без пакета U-Boot должен быть ошибкой");

        assert!(matches!(error, UbootError::MissingPlatformScript { .. }));

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }
}
