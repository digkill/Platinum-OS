//! Подготовка загрузки для плат Raspberry Pi.
//!
//! Загрузчик Raspberry Pi живёт в SPI EEPROM платы, а не в образе: сборке
//! нечего писать в сырые сектора. Прошивка сама читает FAT-раздел, находит там
//! `config.txt`, и уже по нему загружает ядро, initramfs и device tree.
//!
//! Поэтому ядро, initramfs и DTB **копируются** на FAT-раздел, а не остаются
//! симлинками в `/boot`, как у U-Boot: FAT симлинков не знает, а прошивка не
//! умеет читать ext4.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use thiserror::Error;
use tracing::info;

use crate::boot::{BOOT_DIRECTORY, BootError, discover_kernel};

/// Каталог DTB, который раскладывает пакет `linux-modules-*-raspi`.
const DEVICE_TREE_DIRECTORY: &str = "usr/lib/firmware";

/// Подкаталог device tree внутри каталога версии ядра.
const DEVICE_TREE_SUBDIRECTORY: &str = "device-tree";

/// Каталог DT overlay внутри дерева device-tree.
const OVERLAYS_DIRECTORY: &str = "overlays";

/// Имя ядра на FAT-разделе.
const KERNEL_FILE: &str = "vmlinuz";

/// Имя initramfs на FAT-разделе.
const INITRAMFS_FILE: &str = "initramfs";

/// Файл параметров прошивки.
const CONFIG_FILE: &str = "config.txt";

/// Файл командной строки ядра.
const CMDLINE_FILE: &str = "cmdline.txt";

/// Ошибки подготовки загрузки Raspberry Pi.
#[derive(Debug, Error)]
pub enum RaspberryPiError {
    /// Загрузочные файлы ядра не найдены или несогласованны.
    #[error(transparent)]
    Boot(#[from] BootError),
    /// DTB платы отсутствует в дереве, разложенном пакетом модулей.
    #[error("DTB платы `{dtb}` не найден: нет `{path}`; проверьте linux-modules-*-raspi")]
    MissingDeviceTree {
        /// Имя DTB из board-конфигурации.
        dtb: String,
        /// Ожидавшийся путь внутри rootfs.
        path: PathBuf,
    },
    /// Каталога DT overlay нет: без него `dtoverlay=` в config.txt не работает.
    #[error("каталог DT overlay отсутствует: {path}; проверьте linux-modules-*-raspi")]
    MissingOverlays {
        /// Ожидавшийся каталог.
        path: PathBuf,
    },
    /// Файловая операция не удалась.
    #[error("не удалось записать `{path}`: {source}")]
    Write {
        /// Проблемный путь.
        path: PathBuf,
        /// Исходная ошибка файловой системы.
        #[source]
        source: io::Error,
    },
}

/// Параметры загрузки платы Raspberry Pi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaspberryPiSpec {
    /// Источник корневой файловой системы, например `LABEL=platinum-root`.
    pub root_source: String,
    /// Тип корневой файловой системы.
    pub root_filesystem: String,
    /// Дополнительные аргументы командной строки ядра.
    pub extra_arguments: Vec<String>,
    /// Точка монтирования FAT-раздела внутри rootfs, например `/boot/firmware`.
    pub firmware_mount_point: String,
    /// Строки `config.txt` помимо выведенных из данных платы.
    pub config: Vec<String>,
}

/// Раскладывает загрузочные файлы на FAT-раздел Raspberry Pi.
#[derive(Debug, Clone)]
pub struct RaspberryPiConfigurator {
    spec: RaspberryPiSpec,
}

impl RaspberryPiConfigurator {
    /// Создаёт конфигуратор для параметров загрузки платы.
    pub fn new(spec: RaspberryPiSpec) -> Self {
        Self { spec }
    }

    /// Готовит загрузочный раздел и возвращает путь к `config.txt`.
    ///
    /// `dtb` — имя файла device tree платы, например `bcm2712-rpi-5-b.dtb`.
    pub fn apply(&self, rootfs: &Path, dtb: &str) -> Result<PathBuf, RaspberryPiError> {
        let boot = rootfs.join(BOOT_DIRECTORY);
        let (version, kernel, initrd) = discover_kernel(&boot)?;
        let artifacts_version = version.clone();

        let firmware = rootfs.join(self.spec.firmware_mount_point.trim_start_matches('/'));
        fs::create_dir_all(&firmware).map_err(|source| RaspberryPiError::Write {
            path: firmware.clone(),
            source,
        })?;

        // Ядро и initramfs копируются: прошивка читает FAT, где симлинков нет.
        copy(&boot.join(&kernel), &firmware.join(KERNEL_FILE))?;
        copy(&boot.join(&initrd), &firmware.join(INITRAMFS_FILE))?;

        let device_tree = self.device_tree_path(rootfs, &version, dtb);
        if !device_tree.is_file() {
            return Err(RaspberryPiError::MissingDeviceTree {
                dtb: dtb.to_owned(),
                path: device_tree,
            });
        }
        copy(&device_tree, &firmware.join(dtb))?;

        // Overlay обязательны: без них прошивка не применит `dtoverlay=` из
        // config.txt. В частности не включится vc4-kms-v3d, а значит не
        // появится DRM-устройство — HDMI останется чёрным при полностью
        // загруженной системе. Поймано на живой плате.
        self.copy_overlays(rootfs, &artifacts_version, &firmware)?;

        write(&firmware.join(CONFIG_FILE), &render_config(&self.spec, dtb))?;
        write(&firmware.join(CMDLINE_FILE), &render_cmdline(&self.spec))?;

        let path = firmware.join(CONFIG_FILE);

        info!(
            version = %version,
            path = %path.display(),
            "raspberry pi boot files prepared"
        );

        Ok(path)
    }

    /// Копирует каталог DT overlay на загрузочный раздел.
    fn copy_overlays(
        &self,
        rootfs: &Path,
        version: &str,
        firmware: &Path,
    ) -> Result<(), RaspberryPiError> {
        let source = rootfs
            .join(DEVICE_TREE_DIRECTORY)
            .join(version)
            .join(DEVICE_TREE_SUBDIRECTORY)
            .join(OVERLAYS_DIRECTORY);

        if !source.is_dir() {
            return Err(RaspberryPiError::MissingOverlays { path: source });
        }

        let target = firmware.join(OVERLAYS_DIRECTORY);
        fs::create_dir_all(&target).map_err(|error| RaspberryPiError::Write {
            path: target.clone(),
            source: error,
        })?;

        let entries = fs::read_dir(&source).map_err(|error| RaspberryPiError::Write {
            path: source.clone(),
            source: error,
        })?;

        for entry in entries {
            let entry = entry.map_err(|error| RaspberryPiError::Write {
                path: source.clone(),
                source: error,
            })?;

            let from = entry.path();
            if from.is_file() {
                copy(&from, &target.join(entry.file_name()))?;
            }
        }

        Ok(())
    }

    /// Возвращает путь DTB внутри дерева, разложенного пакетом модулей.
    fn device_tree_path(&self, rootfs: &Path, version: &str, dtb: &str) -> PathBuf {
        rootfs
            .join(DEVICE_TREE_DIRECTORY)
            .join(version)
            .join(DEVICE_TREE_SUBDIRECTORY)
            .join("broadcom")
            .join(dtb)
    }
}

/// Формирует `config.txt`.
///
/// Имена файлов задаются явно: прошивка иначе искала бы `kernel8.img` и
/// `bcm2712-rpi-5-b.dtb` по собственным правилам, а сборка кладёт свои имена.
fn render_config(spec: &RaspberryPiSpec, dtb: &str) -> String {
    let mut lines = vec![
        "# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.".to_owned(),
        format!("kernel={KERNEL_FILE}"),
        format!("device_tree={dtb}"),
        // followkernel кладёт initramfs сразу за ядром: без него прошивка
        // выбрала бы адрес, который ядро уже заняло.
        format!("initramfs {INITRAMFS_FILE} followkernel"),
        format!("cmdline={CMDLINE_FILE}"),
    ];
    lines.extend(spec.config.iter().cloned());

    format!("{}\n", lines.join("\n"))
}

/// Формирует `cmdline.txt` одной строкой, как требует прошивка.
fn render_cmdline(spec: &RaspberryPiSpec) -> String {
    let mut arguments = vec![
        format!("root={}", spec.root_source),
        format!("rootfstype={}", spec.root_filesystem),
        // rootwait обязателен: контроллер носителя может появиться позже
        // попытки монтирования корня.
        "rootwait".to_owned(),
    ];
    arguments.extend(spec.extra_arguments.iter().cloned());

    format!("{}\n", arguments.join(" "))
}

/// Копирует файл, создавая родительский каталог.
fn copy(source: &Path, target: &Path) -> Result<(), RaspberryPiError> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| RaspberryPiError::Write {
            path: parent.to_path_buf(),
            source: error,
        })?;
    }

    fs::copy(source, target)
        .map(|_| ())
        .map_err(|error| RaspberryPiError::Write {
            path: target.to_path_buf(),
            source: error,
        })
}

/// Записывает текстовый файл.
fn write(path: &Path, contents: &str) -> Result<(), RaspberryPiError> {
    fs::write(path, contents).map_err(|source| RaspberryPiError::Write {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::{RaspberryPiSpec, render_cmdline, render_config};

    fn spec() -> RaspberryPiSpec {
        RaspberryPiSpec {
            root_source: "LABEL=platinum-root".into(),
            root_filesystem: "ext4".into(),
            extra_arguments: vec!["console=serial0,115200".into()],
            firmware_mount_point: "/boot/firmware".into(),
            config: vec!["arm_64bit=1".into()],
        }
    }

    #[test]
    fn writes_the_names_the_build_actually_uses() {
        let config = render_config(&spec(), "bcm2712-rpi-5-b.dtb");

        assert!(config.contains("kernel=vmlinuz\n"));
        assert!(config.contains("device_tree=bcm2712-rpi-5-b.dtb\n"));
        assert!(config.contains("initramfs initramfs followkernel\n"));
        assert!(config.contains("cmdline=cmdline.txt\n"));
        assert!(config.contains("arm_64bit=1\n"));
    }

    /// Прошивка читает `cmdline.txt` одной строкой: перевод строки внутри
    /// оборвал бы всё, что идёт после него.
    #[test]
    fn keeps_the_kernel_command_line_on_a_single_line() {
        let cmdline = render_cmdline(&spec());

        assert_eq!(cmdline.lines().count(), 1);
        assert!(cmdline.starts_with("root=LABEL=platinum-root rootfstype=ext4 rootwait "));
        assert!(cmdline.contains("console=serial0,115200"));
    }
}
