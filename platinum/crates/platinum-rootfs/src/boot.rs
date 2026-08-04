//! Формирование `extlinux.conf` по содержимому готового rootfs.
//!
//! Имена ядра, initramfs и каталога DTB не задаются конфигурацией: их выбирает
//! пакет ядра при установке. Сборка читает их из `/boot`, поэтому смена версии
//! ядра не требует правки данных платы, а расхождение имён обнаруживается на
//! сборке, а не на устройстве, которое не загрузилось.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use thiserror::Error;
use tracing::info;

/// Каталог загрузочных файлов внутри rootfs.
pub(crate) const BOOT_DIRECTORY: &str = "boot";

/// Каталог конфигурации extlinux относительно `/boot`.
const EXTLINUX_DIRECTORY: &str = "extlinux";

/// Имя файла конфигурации, который ищет U-Boot.
const EXTLINUX_FILE: &str = "extlinux.conf";

/// Префикс файла ядра.
const KERNEL_PREFIX: &str = "vmlinuz-";

/// Префикс файла initramfs.
const INITRD_PREFIX: &str = "initrd.img-";

/// Префикс каталога Device Tree blobs.
const DTB_PREFIX: &str = "dtb-";

/// Ошибки подготовки загрузки.
#[derive(Debug, Error)]
pub enum BootError {
    /// В rootfs нет каталога `/boot`.
    #[error("в rootfs нет каталога `{path}`: пакет ядра не установлен")]
    MissingBootDirectory {
        /// Ожидавшийся каталог.
        path: PathBuf,
    },
    /// Каталог `/boot` не читается.
    #[error("не удалось прочитать `{path}`: {source}")]
    ReadBootDirectory {
        /// Проблемный каталог.
        path: PathBuf,
        /// Исходная ошибка файловой системы.
        #[source]
        source: io::Error,
    },
    /// Нужного файла нет: установка пакета ядра не завершилась.
    #[error("в `{path}` нет `{prefix}*`; проверьте установку пакетов ядра и initramfs-tools")]
    NotFound {
        /// Каталог, в котором шёл поиск.
        path: PathBuf,
        /// Префикс, который искали.
        prefix: &'static str,
    },
    /// Несколько кандидатов: выбор наугад дал бы незагружаемую систему.
    #[error("в `{path}` несколько файлов `{prefix}*`: {matches}")]
    Ambiguous {
        /// Каталог, в котором шёл поиск.
        path: PathBuf,
        /// Префикс, который искали.
        prefix: &'static str,
        /// Найденные кандидаты.
        matches: String,
    },
    /// Версии ядра, initramfs и DTB разошлись.
    #[error(
        "версии загрузочных файлов не совпадают: ядро `{kernel}`, initramfs `{initrd}`, dtb `{dtb}`"
    )]
    VersionMismatch {
        /// Версия ядра.
        kernel: String,
        /// Версия initramfs.
        initrd: String,
        /// Версия каталога DTB.
        dtb: String,
    },
    /// DTB платы отсутствует в каталоге пакета.
    #[error("DTB платы `{dtb}` не найден: нет `{path}`")]
    MissingDeviceTree {
        /// Путь DTB из board-конфигурации.
        dtb: String,
        /// Ожидавшийся путь внутри rootfs.
        path: PathBuf,
    },
    /// Конфигурацию не удалось записать.
    #[error("не удалось записать `{path}`: {source}")]
    Write {
        /// Проблемный путь.
        path: PathBuf,
        /// Исходная ошибка файловой системы.
        #[source]
        source: io::Error,
    },
}

/// Параметры загрузки, задаваемые конфигурацией.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootSpec {
    /// Источник корневой файловой системы, например `LABEL=platinum-root`.
    pub root_source: String,
    /// Тип корневой файловой системы.
    pub root_filesystem: String,
    /// Дополнительные аргументы командной строки ядра.
    pub extra_arguments: Vec<String>,
    /// Задержка меню в десятых долях секунды.
    pub timeout_deciseconds: u32,
}

/// Загрузочные файлы, найденные в rootfs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootArtifacts {
    /// Версия ядра, общая для всех файлов.
    pub version: String,
    /// Имя файла ядра в `/boot`.
    pub kernel: String,
    /// Имя файла initramfs в `/boot`.
    pub initrd: String,
    /// Имя каталога DTB в `/boot`.
    pub dtb_directory: String,
}

/// Создаёт `extlinux.conf` по содержимому rootfs.
#[derive(Debug, Clone)]
pub struct BootConfigurator {
    spec: BootSpec,
}

impl BootConfigurator {
    /// Создаёт конфигуратор для параметров загрузки.
    pub fn new(spec: BootSpec) -> Self {
        Self { spec }
    }

    /// Записывает `extlinux.conf` и возвращает путь к нему.
    ///
    /// `dtb` — путь DTB платы внутри каталога пакета, например
    /// `allwinner/sun60i-a733-orangepi-zero3w.dtb`.
    pub fn apply(&self, rootfs: &Path, dtb: &str) -> Result<PathBuf, BootError> {
        let boot = rootfs.join(BOOT_DIRECTORY);
        let artifacts = discover(&boot)?;

        let device_tree = boot.join(&artifacts.dtb_directory).join(dtb);
        if !device_tree.is_file() {
            return Err(BootError::MissingDeviceTree {
                dtb: dtb.to_owned(),
                path: device_tree,
            });
        }

        let directory = boot.join(EXTLINUX_DIRECTORY);
        fs::create_dir_all(&directory).map_err(|source| BootError::Write {
            path: directory.clone(),
            source,
        })?;

        let path = directory.join(EXTLINUX_FILE);
        fs::write(&path, render(&self.spec, &artifacts, dtb)).map_err(|source| {
            BootError::Write {
                path: path.clone(),
                source,
            }
        })?;

        info!(
            version = %artifacts.version,
            path = %path.display(),
            "extlinux configuration written"
        );

        Ok(path)
    }
}

/// Находит ядро и initramfs в `/boot`.
///
/// Каталог DTB здесь не ищется: его раскладывают по-разному. Пакеты Armbian
/// кладут `/boot/dtb-<версия>`, а ядро Ubuntu для Raspberry Pi —
/// `/usr/lib/firmware/<версия>/device-tree`. Общая часть — только ядро и
/// initramfs, и версии у них обязаны совпадать.
pub(crate) fn discover_kernel(boot: &Path) -> Result<(String, String, String), BootError> {
    let names = boot_entries(boot)?;

    let kernel = find_single(boot, &names, KERNEL_PREFIX)?;
    let initrd = find_single(boot, &names, INITRD_PREFIX)?;

    let version = kernel
        .strip_prefix(KERNEL_PREFIX)
        .unwrap_or_default()
        .to_owned();
    let initrd_version = initrd.strip_prefix(INITRD_PREFIX).unwrap_or_default();

    // Расхождение версий означает остатки предыдущего ядра: система собралась
    // бы, но загрузила ядро с чужими модулями.
    if version != initrd_version {
        return Err(BootError::VersionMismatch {
            kernel: version.clone(),
            initrd: initrd_version.to_owned(),
            dtb: version,
        });
    }

    Ok((version, kernel, initrd))
}

/// Читает отсортированный список имён в `/boot`.
fn boot_entries(boot: &Path) -> Result<Vec<String>, BootError> {
    if !boot.is_dir() {
        return Err(BootError::MissingBootDirectory {
            path: boot.to_path_buf(),
        });
    }

    let entries = fs::read_dir(boot).map_err(|source| BootError::ReadBootDirectory {
        path: boot.to_path_buf(),
        source,
    })?;

    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| BootError::ReadBootDirectory {
            path: boot.to_path_buf(),
            source,
        })?;

        names.push(entry.file_name().to_string_lossy().into_owned());
    }

    // Порядок `read_dir` зависит от файловой системы: без сортировки сообщения
    // об ошибках были бы невоспроизводимыми.
    names.sort();

    Ok(names)
}

/// Находит ядро, initramfs и каталог DTB в `/boot`.
pub(crate) fn discover(boot: &Path) -> Result<BootArtifacts, BootError> {
    if !boot.is_dir() {
        return Err(BootError::MissingBootDirectory {
            path: boot.to_path_buf(),
        });
    }

    let entries = fs::read_dir(boot).map_err(|source| BootError::ReadBootDirectory {
        path: boot.to_path_buf(),
        source,
    })?;

    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| BootError::ReadBootDirectory {
            path: boot.to_path_buf(),
            source,
        })?;

        names.push(entry.file_name().to_string_lossy().into_owned());
    }

    // Порядок `read_dir` зависит от файловой системы: без сортировки сообщения
    // об ошибках были бы невоспроизводимыми.
    names.sort();

    let kernel = find_single(boot, &names, KERNEL_PREFIX)?;
    let initrd = find_single(boot, &names, INITRD_PREFIX)?;
    let dtb_directory = find_single(boot, &names, DTB_PREFIX)?;

    let version = kernel
        .strip_prefix(KERNEL_PREFIX)
        .unwrap_or_default()
        .to_owned();
    let initrd_version = initrd.strip_prefix(INITRD_PREFIX).unwrap_or_default();
    let dtb_version = dtb_directory.strip_prefix(DTB_PREFIX).unwrap_or_default();

    // Расхождение версий означает остатки предыдущего ядра: система собралась
    // бы, но загрузила ядро с чужими модулями.
    if version != initrd_version || version != dtb_version {
        return Err(BootError::VersionMismatch {
            kernel: version,
            initrd: initrd_version.to_owned(),
            dtb: dtb_version.to_owned(),
        });
    }

    Ok(BootArtifacts {
        version,
        kernel,
        initrd,
        dtb_directory,
    })
}

/// Возвращает единственное имя с заданным префиксом.
fn find_single(boot: &Path, names: &[String], prefix: &'static str) -> Result<String, BootError> {
    let matches: Vec<&String> = names
        .iter()
        .filter(|name| name.starts_with(prefix))
        .collect();

    match matches.as_slice() {
        [] => Err(BootError::NotFound {
            path: boot.to_path_buf(),
            prefix,
        }),
        [single] => Ok((*single).clone()),
        _ => Err(BootError::Ambiguous {
            path: boot.to_path_buf(),
            prefix,
            matches: matches
                .iter()
                .map(|name| name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        }),
    }
}

/// Формирует содержимое `extlinux.conf`.
///
/// Пути указываются относительно `/boot`: U-Boot ищет конфигурацию по этому
/// префиксу и разрешает остальные пути относительно него же.
fn render(spec: &BootSpec, artifacts: &BootArtifacts, dtb: &str) -> String {
    let mut append = vec![
        format!("root={}", spec.root_source),
        format!("rootfstype={}", spec.root_filesystem),
        // rootwait обязателен для SD-карт: контроллер может появиться позже
        // попытки монтирования корня.
        "rootwait".to_owned(),
    ];
    append.extend(spec.extra_arguments.iter().cloned());

    format!(
        "# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.\n\
         DEFAULT platinum\n\
         TIMEOUT {timeout}\n\
         MENU TITLE Platinum OS\n\
         \n\
         LABEL platinum\n\
         \x20 MENU LABEL Platinum OS {version}\n\
         \x20 LINUX /{kernel}\n\
         \x20 INITRD /{initrd}\n\
         \x20 FDT /{dtb_directory}/{dtb}\n\
         \x20 APPEND {append}\n",
        timeout = spec.timeout_deciseconds,
        version = artifacts.version,
        kernel = artifacts.kernel,
        initrd = artifacts.initrd,
        dtb_directory = artifacts.dtb_directory,
        append = append.join(" ")
    )
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{BootConfigurator, BootError, BootSpec, discover};

    fn boot_directory(label: &str, kernel_version: &str, initrd_version: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "platinum-boot-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("системное время должно быть позже Unix epoch")
                .as_nanos()
        ));
        let boot = root.join("boot");
        fs::create_dir_all(boot.join(format!("dtb-{kernel_version}/allwinner")))
            .expect("каталог dtb должен создаваться");
        fs::write(boot.join(format!("vmlinuz-{kernel_version}")), b"kernel")
            .expect("ядро должно записываться");
        fs::write(boot.join(format!("initrd.img-{initrd_version}")), b"initrd")
            .expect("initramfs должен записываться");
        fs::write(
            boot.join(format!(
                "dtb-{kernel_version}/allwinner/sun60i-a733-orangepi-zero3w.dtb"
            )),
            b"dtb",
        )
        .expect("DTB должен записываться");

        root
    }

    fn spec() -> BootSpec {
        BootSpec {
            root_source: "LABEL=platinum-root".into(),
            root_filesystem: "ext4".into(),
            extra_arguments: vec!["quiet".into()],
            timeout_deciseconds: 30,
        }
    }

    #[test]
    fn writes_paths_relative_to_the_boot_directory() {
        let root = boot_directory("render", "6.12.0-vendor", "6.12.0-vendor");

        let path = BootConfigurator::new(spec())
            .apply(&root, "allwinner/sun60i-a733-orangepi-zero3w.dtb")
            .expect("конфигурация загрузки должна записываться");

        let contents = fs::read_to_string(&path).expect("конфигурация должна читаться");
        assert!(contents.contains("LINUX /vmlinuz-6.12.0-vendor\n"));
        assert!(contents.contains("INITRD /initrd.img-6.12.0-vendor\n"));
        assert!(
            contents.contains("FDT /dtb-6.12.0-vendor/allwinner/sun60i-a733-orangepi-zero3w.dtb\n")
        );
        assert!(
            contents.contains("APPEND root=LABEL=platinum-root rootfstype=ext4 rootwait quiet\n")
        );
        assert!(contents.contains("TIMEOUT 30\n"));

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }

    #[test]
    fn rejects_a_leftover_initramfs_of_another_kernel() {
        let root = boot_directory("mismatch", "6.12.0-vendor", "6.1.0-old");

        let error =
            discover(&root.join("boot")).expect_err("несовпадение версий должно отклоняться");

        assert!(matches!(error, BootError::VersionMismatch { .. }));

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }

    #[test]
    fn reports_a_rootfs_without_an_installed_kernel() {
        let root = std::env::temp_dir().join(format!(
            "platinum-boot-empty-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("системное время должно быть позже Unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(root.join("boot")).expect("каталог boot должен создаваться");

        let error = discover(&root.join("boot")).expect_err("пустой /boot должен быть ошибкой");

        assert!(matches!(error, BootError::NotFound { .. }));

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }

    #[test]
    fn reports_a_missing_device_tree_of_the_board() {
        let root = boot_directory("dtb", "6.12.0-vendor", "6.12.0-vendor");

        let error = BootConfigurator::new(spec())
            .apply(&root, "allwinner/sun50i-h618-orangepi-zero3.dtb")
            .expect_err("чужой DTB не должен приниматься");

        assert!(matches!(error, BootError::MissingDeviceTree { .. }));

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }
}
