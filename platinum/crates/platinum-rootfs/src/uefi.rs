//! Подготовка загрузки через UEFI и GRUB.
//!
//! Способ для машин с прошивкой UEFI: виртуальных (Parallels, QEMU с EDK2,
//! UTM) и обычных arm64-компьютеров. Ни vendor U-Boot, ни прошивки платы здесь
//! нет — firmware само находит `EFI/BOOT/BOOTAA64.EFI` на разделе ESP.
//!
//! Ядро, initramfs, модули GRUB и его конфигурация кладутся **на сам ESP**.
//! Так GRUB обходится встроенной поддержкой FAT и ему не нужно уметь читать
//! корневую файловую систему, чтобы найти собственные файлы.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use thiserror::Error;
use tracing::info;

use crate::{
    boot::{BOOT_DIRECTORY, BootError, discover_kernel},
    chroot::ChrootSession,
};

/// Имя ядра на ESP.
const KERNEL_FILE: &str = "vmlinuz";

/// Имя initramfs на ESP.
const INITRAMFS_FILE: &str = "initrd.img";

/// Каталог GRUB на ESP: туда же `grub-install` кладёт модули.
const GRUB_DIRECTORY: &str = "grub";

/// Имя конфигурации, которую GRUB читает из своего каталога.
const GRUB_CONFIG: &str = "grub.cfg";

/// Модули, встроенные в двоичный файл загрузчика.
///
/// Ровно те, без которых GRUB не сможет прочитать таблицу разделов, найти ESP
/// и разобрать собственную конфигурацию. Остальное подгружается из каталога на
/// самом разделе.
const EMBEDDED_MODULES: [&str; 10] = [
    "part_msdos",
    "part_gpt",
    "fat",
    "ext2",
    "normal",
    "linux",
    "search",
    "search_label",
    "configfile",
    "all_video",
];

/// Ошибки подготовки загрузки UEFI.
#[derive(Debug, Error)]
pub enum UefiError {
    /// Загрузочные файлы ядра не найдены или несогласованны.
    #[error(transparent)]
    Boot(#[from] BootError),
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

/// Параметры загрузки через UEFI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UefiSpec {
    /// Источник корневой файловой системы, например `LABEL=platinum-root`.
    pub root_source: String,
    /// Тип корневой файловой системы.
    pub root_filesystem: String,
    /// Дополнительные аргументы командной строки ядра.
    pub extra_arguments: Vec<String>,
    /// Точка монтирования ESP внутри rootfs, например `/boot/efi`.
    pub esp_mount_point: String,
    /// Задержка меню в секундах.
    pub timeout_seconds: u32,
}

/// Раскладывает загрузочные файлы на ESP и ставит GRUB.
#[derive(Debug, Clone)]
pub struct UefiConfigurator {
    spec: UefiSpec,
}

impl UefiConfigurator {
    /// Создаёт конфигуратор для параметров загрузки.
    pub fn new(spec: UefiSpec) -> Self {
        Self { spec }
    }

    /// Копирует ядро с initramfs на ESP и пишет конфигурацию GRUB.
    pub fn apply(&self, rootfs: &Path) -> Result<PathBuf, UefiError> {
        let boot = rootfs.join(BOOT_DIRECTORY);
        let (version, kernel, initrd) = discover_kernel(&boot)?;

        let esp = rootfs.join(self.spec.esp_mount_point.trim_start_matches('/'));
        fs::create_dir_all(&esp).map_err(|source| UefiError::Write {
            path: esp.clone(),
            source,
        })?;

        // Копируются, а не связываются: FAT симлинков не знает, а GRUB читает
        // эти файлы до того, как смонтирован корень.
        copy(&boot.join(&kernel), &esp.join(KERNEL_FILE))?;
        copy(&boot.join(&initrd), &esp.join(INITRAMFS_FILE))?;

        let config = esp.join(GRUB_DIRECTORY).join(GRUB_CONFIG);
        write(&config, &render_config(&self.spec, &version))?;

        info!(version = %version, path = %config.display(), "uefi boot files prepared");

        Ok(config)
    }

    /// Собирает загрузчик в `EFI/BOOT/BOOTAA64.EFI` на ESP.
    ///
    /// Используется `grub-mkimage`, а не `grub-install`: последний опрашивает
    /// блочное устройство под каталогом ESP и отказывается работать словами
    /// «doesn't look like an EFI partition». В chroot раздела ещё нет — там
    /// обычный каталог на ext4, и обойти проверку флагом нельзя.
    ///
    /// Путь `EFI/BOOT/BOOTAA64.EFI` прошивка ищет сама, без записи в NVRAM:
    /// образ обязан загружаться на любой машине, а не только там, где собран.
    pub fn install(&self, session: &ChrootSession<'_>) -> Result<(), crate::ChrootError> {
        let esp = &self.spec.esp_mount_point;
        let binary = format!("{esp}/EFI/BOOT/BOOTAA64.EFI");

        session.run("mkdir", &["-p", &format!("{esp}/EFI/BOOT")])?;

        let mut arguments = vec![
            "--format=arm64-efi".to_owned(),
            // Префикс относительный: GRUB ищет свои файлы на том же разделе,
            // с которого его загрузила прошивка.
            format!("--prefix=/{GRUB_DIRECTORY}"),
            format!("--output={binary}"),
        ];
        arguments.extend(EMBEDDED_MODULES.iter().map(|module| (*module).to_owned()));

        let borrowed: Vec<&str> = arguments.iter().map(String::as_str).collect();
        session.run("grub-mkimage", &borrowed)?;

        // Остальные модули кладутся рядом: встроены только те, без которых
        // GRUB не доберётся до собственного каталога.
        session.run(
            "sh",
            &[
                "-c",
                &format!("cp -r /usr/lib/grub/arm64-efi {esp}/{GRUB_DIRECTORY}/"),
            ],
        )
    }
}

/// Формирует `grub.cfg`.
///
/// Конфигурация пишется сборкой, а не `grub-mkconfig`: тот опрашивает
/// смонтированные файловые системы работающей машины, а в chroot они чужие —
/// получился бы конфиг, указывающий на диск хоста сборки.
fn render_config(spec: &UefiSpec, version: &str) -> String {
    let mut arguments = vec![
        format!("root={}", spec.root_source),
        format!("rootfstype={}", spec.root_filesystem),
        "rootwait".to_owned(),
    ];
    arguments.extend(spec.extra_arguments.iter().cloned());

    format!(
        "# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.\n\
         set timeout={timeout}\n\
         set default=0\n\
         \n\
         menuentry 'Platinum OS {version}' {{\n\
         \x20   # Файлы лежат на этом же разделе, поэтому путь от его корня.\n\
         \x20   linux /{KERNEL_FILE} {arguments}\n\
         \x20   initrd /{INITRAMFS_FILE}\n\
         }}\n",
        timeout = spec.timeout_seconds,
        arguments = arguments.join(" ")
    )
}

/// Копирует файл, создавая родительский каталог.
fn copy(source: &Path, target: &Path) -> Result<(), UefiError> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| UefiError::Write {
            path: parent.to_path_buf(),
            source: error,
        })?;
    }

    fs::copy(source, target)
        .map(|_| ())
        .map_err(|error| UefiError::Write {
            path: target.to_path_buf(),
            source: error,
        })
}

/// Записывает текстовый файл, создавая родительские каталоги.
fn write(path: &Path, contents: &str) -> Result<(), UefiError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| UefiError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    fs::write(path, contents).map_err(|source| UefiError::Write {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::{UefiSpec, render_config};

    fn spec() -> UefiSpec {
        UefiSpec {
            root_source: "LABEL=platinum-root".into(),
            root_filesystem: "ext4".into(),
            extra_arguments: vec!["console=ttyAMA0".into()],
            esp_mount_point: "/boot/efi".into(),
            timeout_seconds: 3,
        }
    }

    /// Пути к ядру считаются от корня ESP: GRUB читает их до монтирования
    /// корневой файловой системы и о ней ничего не знает.
    #[test]
    fn refers_to_files_on_the_boot_partition() {
        let config = render_config(&spec(), "7.0.0-14-generic");

        assert!(config.contains("linux /vmlinuz "));
        assert!(config.contains("initrd /initrd.img\n"));
        assert!(config.contains("root=LABEL=platinum-root"));
        assert!(config.contains("console=ttyAMA0"));
        assert!(config.contains("set timeout=3\n"));
    }

    #[test]
    fn names_the_entry_after_the_kernel_version() {
        assert!(
            render_config(&spec(), "7.0.0-14-generic")
                .contains("menuentry 'Platinum OS 7.0.0-14-generic'")
        );
    }
}
