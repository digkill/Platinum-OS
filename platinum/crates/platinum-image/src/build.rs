//! Сборка дискового образа из готового rootfs.
//!
//! Образ создаётся без loop-устройств и монтирования: файловая система строится
//! в отдельном файле через `mkfs.ext4 -d` и переносится в образ по смещению
//! раздела. Так сборке не нужен ни `losetup`, ни привилегии сверх тех, что уже
//! требуются для владения файлами rootfs.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::Command,
};

use thiserror::Error;
use tracing::info;

use crate::{
    layout::{Filesystem, ImageLayout, LayoutError, PartitionSpec},
    mbr::render_boot_sector,
};

/// Размер буфера переноса файловой системы в образ.
const COPY_BUFFER: usize = 4 * 1024 * 1024;

/// Ошибки сборки образа.
#[derive(Debug, Error)]
pub enum ImageError {
    /// Разметка оказалась некорректной уже во время сборки.
    #[error(transparent)]
    Layout(#[from] LayoutError),
    /// Каталог-источник раздела отсутствует в rootfs.
    #[error("каталог `{path}` раздела `{name}` отсутствует в rootfs")]
    MissingSource {
        /// Раздел, для которого искали содержимое.
        name: String,
        /// Ожидавшийся каталог.
        path: PathBuf,
    },
    /// Не удалось подготовить staging-каталог раздела.
    #[error("не удалось подготовить содержимое раздела в `{path}`: {stderr}")]
    Staging {
        /// Каталог staging.
        path: PathBuf,
        /// Диагностика утилиты.
        stderr: String,
    },
    /// Утилиту не удалось запустить.
    #[error("не удалось запустить `{command}`: {source}; установите {package}")]
    StartTool {
        /// Имя утилиты.
        command: &'static str,
        /// Пакет, который её приносит.
        package: &'static str,
        /// Исходная ошибка запуска процесса.
        #[source]
        source: io::Error,
    },
    /// Копирование содержимого в FAT завершилось ошибкой.
    #[error("`mcopy` завершился с кодом {code}: {stderr}")]
    McopyFailed {
        /// Код завершения или -1, если ОС не предоставила его.
        code: i32,
        /// Диагностика утилиты.
        stderr: String,
    },
    /// Файловую операцию над образом не удалось выполнить.
    #[error("не удалось работать с файлом `{path}`: {source}")]
    Io {
        /// Проблемный путь.
        path: PathBuf,
        /// Исходная ошибка ввода-вывода.
        #[source]
        source: io::Error,
    },
    /// Утилиту создания файловой системы не удалось запустить.
    #[error("не удалось запустить `{command}`: {source}; установите e2fsprogs")]
    StartMkfs {
        /// Имя утилиты.
        command: &'static str,
        /// Исходная ошибка запуска процесса.
        #[source]
        source: io::Error,
    },
    /// Утилита создания файловой системы завершилась с ошибкой.
    #[error("`{command}` завершился с кодом {code}: {stderr}")]
    MkfsFailed {
        /// Имя утилиты.
        command: &'static str,
        /// Код завершения или -1, если ОС не предоставила его.
        code: i32,
        /// Диагностика утилиты.
        stderr: String,
    },
}

/// Сборщик дискового образа по проверенной разметке.
#[derive(Debug, Clone)]
pub struct ImageBuilder {
    layout: ImageLayout,
}

impl ImageBuilder {
    /// Создаёт сборщик для разметки платы.
    pub fn new(layout: ImageLayout) -> Self {
        Self { layout }
    }

    /// Собирает образ из rootfs и возвращает путь к готовому файлу.
    pub fn build(&self, rootfs: &Path, image_path: &Path) -> Result<(), ImageError> {
        let image = self.create_image(image_path)?;
        self.write_boot_sector(&image, image_path)?;

        for partition in self.layout.partitions() {
            let filesystem = self.make_filesystem(partition, rootfs, image_path)?;
            let copied = copy_into_image(&filesystem, image_path, partition.offset_bytes());

            // Промежуточный файл убирается в любом случае: он занимает столько
            // же, сколько раздел, и повторная сборка иначе упёрлась бы в диск.
            remove_quietly(&filesystem);
            copied?;

            info!(
                partition = %partition.name,
                label = %partition.label,
                size_mib = partition.size_mib,
                "partition written into image"
            );
        }

        info!(
            image = %image_path.display(),
            size_mib = self.layout.size_mib(),
            "image built"
        );

        Ok(())
    }

    /// Создаёт разреженный файл образа нужного размера.
    fn create_image(&self, image_path: &Path) -> Result<File, ImageError> {
        if let Some(parent) = image_path.parent() {
            fs::create_dir_all(parent).map_err(|source| ImageError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let image = File::create(image_path).map_err(|source| ImageError::Io {
            path: image_path.to_path_buf(),
            source,
        })?;

        // set_len создаёт разреженный файл: нули не занимают места на диске, а
        // ext4 всё равно перезапишет только используемые блоки.
        image
            .set_len(self.layout.size_bytes())
            .map_err(|source| ImageError::Io {
                path: image_path.to_path_buf(),
                source,
            })?;

        Ok(image)
    }

    /// Записывает таблицу разделов в начало образа.
    fn write_boot_sector(&self, image: &File, image_path: &Path) -> Result<(), ImageError> {
        let sector = render_boot_sector(&self.layout)?;

        let mut image = image.try_clone().map_err(|source| ImageError::Io {
            path: image_path.to_path_buf(),
            source,
        })?;

        image
            .seek(SeekFrom::Start(0))
            .and_then(|_| image.write_all(&sector))
            .map_err(|source| ImageError::Io {
                path: image_path.to_path_buf(),
                source,
            })
    }

    /// Создаёт файловую систему раздела в отдельном файле.
    fn make_filesystem(
        &self,
        partition: &PartitionSpec,
        rootfs: &Path,
        image_path: &Path,
    ) -> Result<PathBuf, ImageError> {
        let path = image_path.with_extension(format!("{}.fs", partition.name));

        let file = File::create(&path).map_err(|source| ImageError::Io {
            path: path.clone(),
            source,
        })?;
        file.set_len(partition.size_bytes())
            .map_err(|source| ImageError::Io {
                path: path.clone(),
                source,
            })?;
        drop(file);

        let command = partition.filesystem.mkfs_command();
        let mut mkfs = Command::new(command);

        match partition.filesystem {
            Filesystem::Ext4 => {
                mkfs.arg("-q")
                    .arg("-F")
                    .arg("-L")
                    .arg(&partition.label)
                    // root_owner фиксирует владельца корня раздела: иначе им
                    // стал бы пользователь сборки, и система не смонтировала бы
                    // `/` корректно.
                    .arg("-E")
                    .arg("root_owner=0:0");
            }
            Filesystem::Fat32 => {
                // `-F 32` у mkfs.vfat — разрядность FAT, а не «force» как у
                // mkfs.ext4. Прошивка Raspberry Pi читает только FAT32.
                mkfs.arg("-F").arg("32").arg("-n").arg(&partition.label);
            }
        }

        let staged = self.stage_source(partition, rootfs, image_path)?;

        if partition.filesystem.populated_by_mkfs()
            && let Some(staged) = &staged
        {
            mkfs.arg("-d").arg(&staged.path);
        }

        let output = mkfs
            .arg(&path)
            .output()
            .map_err(|source| ImageError::StartMkfs { command, source });

        let result = output.and_then(|output| {
            if output.status.success() {
                Ok(())
            } else {
                Err(ImageError::MkfsFailed {
                    command,
                    code: output.status.code().unwrap_or(-1),
                    stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
                })
            }
        });

        // FAT заполняется после создания: `mkfs.vfat` не умеет `-d`, а
        // монтировать раздел ради копирования значило бы вернуть
        // loop-устройства, от которых сборка намеренно ушла.
        let result = result.and_then(|()| {
            if partition.filesystem.populated_by_mkfs() {
                return Ok(());
            }

            match &staged {
                Some(staged) => copy_into_fat(&staged.path, &path),
                None => Ok(()),
            }
        });

        if let Some(staged) = staged {
            staged.cleanup();
        }

        if let Err(error) = result {
            remove_quietly(&path);

            return Err(error);
        }

        Ok(path)
    }

    /// Готовит каталог, содержимое которого попадёт в раздел.
    ///
    /// Если внутрь раздела монтируются другие, их содержимое обязано быть
    /// исключено: иначе оно попало бы и в этот раздел, и в собственный, заняв
    /// место дважды и разойдясь при первой же правке на устройстве.
    ///
    /// Исключение делается через staging-каталог из жёстких ссылок: полная
    /// копия rootfs стоила бы гигабайты записи, а перенос каталога из rootfs
    /// портил бы исходное дерево, если сборка прервётся.
    fn stage_source(
        &self,
        partition: &PartitionSpec,
        rootfs: &Path,
        image_path: &Path,
    ) -> Result<Option<StagedSource>, ImageError> {
        let Some(mount_point) = &partition.mount_point else {
            return Ok(None);
        };

        let source = if mount_point == "/" {
            rootfs.to_path_buf()
        } else {
            rootfs.join(mount_point.trim_start_matches('/'))
        };

        if !source.is_dir() {
            return Err(ImageError::MissingSource {
                name: partition.name.clone(),
                path: source,
            });
        }

        let nested = self.nested_mount_points(mount_point);
        if nested.is_empty() {
            return Ok(Some(StagedSource {
                path: source,
                temporary: false,
            }));
        }

        let staging = image_path.with_extension(format!("{}.staging", partition.name));
        remove_tree_quietly(&staging);

        link_tree(&source, &staging)?;

        for point in nested {
            let relative = point
                .trim_start_matches('/')
                .strip_prefix(mount_point.trim_start_matches('/'))
                .unwrap_or_else(|| point.trim_start_matches('/'))
                .trim_start_matches('/');

            // Каталог точки монтирования остаётся: ядру нужно, куда монтировать.
            let target = staging.join(relative);
            remove_tree_quietly(&target);
            fs::create_dir_all(&target).map_err(|source| ImageError::Io {
                path: target.clone(),
                source,
            })?;
        }

        Ok(Some(StagedSource {
            path: staging,
            temporary: true,
        }))
    }

    /// Возвращает точки монтирования, лежащие внутри указанной.
    fn nested_mount_points(&self, mount_point: &str) -> Vec<String> {
        let prefix = if mount_point == "/" {
            "/".to_owned()
        } else {
            format!("{mount_point}/")
        };

        self.layout
            .partitions()
            .iter()
            .filter_map(|partition| partition.mount_point.clone())
            .filter(|point| point != mount_point && point.starts_with(&prefix))
            .collect()
    }
}

/// Переносит готовую файловую систему в образ по смещению раздела.
fn copy_into_image(filesystem: &Path, image_path: &Path, offset: u64) -> Result<(), ImageError> {
    let mut source = File::open(filesystem).map_err(|source| ImageError::Io {
        path: filesystem.to_path_buf(),
        source,
    })?;

    let mut image = OpenOptions::new()
        .write(true)
        .open(image_path)
        .map_err(|source| ImageError::Io {
            path: image_path.to_path_buf(),
            source,
        })?;

    image
        .seek(SeekFrom::Start(offset))
        .map_err(|source| ImageError::Io {
            path: image_path.to_path_buf(),
            source,
        })?;

    let mut buffer = vec![0_u8; COPY_BUFFER];
    loop {
        let read = io::Read::read(&mut source, &mut buffer).map_err(|source| ImageError::Io {
            path: filesystem.to_path_buf(),
            source,
        })?;

        if read == 0 {
            break;
        }

        image
            .write_all(&buffer[..read])
            .map_err(|source| ImageError::Io {
                path: image_path.to_path_buf(),
                source,
            })?;
    }

    // sync_all до записи загрузчика: иначе ошибка записи проявилась бы уже на
    // устройстве, а не в сборке.
    image.sync_all().map_err(|source| ImageError::Io {
        path: image_path.to_path_buf(),
        source,
    })
}

/// Удаляет промежуточный файл, не прерывая сборку из-за уборки.
fn remove_quietly(path: &Path) {
    if let Err(error) = fs::remove_file(path) {
        tracing::warn!(path = %path.display(), %error, "не удалось удалить промежуточный файл");
    }
}

/// Каталог, содержимое которого попадёт в раздел.
///
/// Временный staging удаляется после создания файловой системы: он состоит из
/// жёстких ссылок, но каталоги и inode всё равно занимают место.
#[derive(Debug)]
struct StagedSource {
    path: PathBuf,
    temporary: bool,
}

impl StagedSource {
    /// Удаляет staging, если он создавался сборкой.
    fn cleanup(self) {
        if self.temporary {
            remove_tree_quietly(&self.path);
        }
    }
}

/// Создаёт дерево жёстких ссылок на исходный каталог.
///
/// `cp -al` вместо копирования: rootfs весит гигабайты, а разделу нужно лишь
/// дерево имён, из которого можно удалить вложенные точки монтирования, не
/// трогая исходный rootfs.
fn link_tree(source: &Path, target: &Path) -> Result<(), ImageError> {
    fs::create_dir_all(target).map_err(|error| ImageError::Io {
        path: target.to_path_buf(),
        source: error,
    })?;

    let output = Command::new("cp")
        .arg("-al")
        .arg(format!("{}/.", source.display()))
        .arg(target)
        .output()
        .map_err(|error| ImageError::StartTool {
            command: "cp",
            package: "coreutils",
            source: error,
        })?;

    if output.status.success() {
        return Ok(());
    }

    Err(ImageError::Staging {
        path: target.to_path_buf(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

/// Копирует содержимое каталога в образ FAT через mtools.
///
/// `mcopy` работает с файлом файловой системы напрямую, поэтому монтирование и
/// loop-устройства по-прежнему не нужны.
fn copy_into_fat(source: &Path, filesystem: &Path) -> Result<(), ImageError> {
    let entries = fs::read_dir(source).map_err(|error| ImageError::Io {
        path: source.to_path_buf(),
        source: error,
    })?;

    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| ImageError::Io {
            path: source.to_path_buf(),
            source: error,
        })?;

        paths.push(entry.path());
    }

    // Пустой загрузочный раздел — не ошибка сборки образа: содержимое кладёт
    // конфигуратор загрузки, и его отсутствие диагностируется там.
    if paths.is_empty() {
        return Ok(());
    }

    // Порядок фиксируется: без сортировки образ зависел бы от порядка readdir и
    // перестал быть побайтово воспроизводимым.
    paths.sort();

    let output = Command::new("mcopy")
        .arg("-i")
        .arg(filesystem)
        .arg("-s")
        .arg("-Q")
        .arg("-o")
        .args(&paths)
        .arg("::")
        .output()
        .map_err(|error| ImageError::StartTool {
            command: "mcopy",
            package: "mtools",
            source: error,
        })?;

    if output.status.success() {
        return Ok(());
    }

    Err(ImageError::McopyFailed {
        code: output.status.code().unwrap_or(-1),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

/// Удаляет дерево, игнорируя отсутствие.
fn remove_tree_quietly(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::layout::{Filesystem, ImageLayout, PartitionSpec};

    use super::{ImageBuilder, ImageError};

    fn temporary_directory(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "platinum-image-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("системное время должно быть позже Unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&path).expect("временный каталог должен создаваться");

        path
    }

    fn partition(name: &str, mount_point: Option<&str>, start_mib: u64) -> PartitionSpec {
        PartitionSpec::new(
            name.into(),
            format!("platinum-{name}"),
            Filesystem::Ext4,
            start_mib,
            16,
            mount_point.map(str::to_owned),
        )
        .expect("описание раздела должно быть корректным")
        .bootable(false)
        .esp(false)
    }

    #[test]
    fn creates_a_sparse_image_of_the_declared_size() {
        let root = temporary_directory("sparse");
        let image = root.join("platinum.img");
        let layout = ImageLayout::new(vec![partition("root", None, 16)], 1)
            .expect("разметка должна быть корректной");

        ImageBuilder::new(layout)
            .create_image(&image)
            .expect("файл образа должен создаваться");

        let size = fs::metadata(&image)
            .expect("метаданные образа должны читаться")
            .len();
        assert_eq!(size, 32 * 1024 * 1024);

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }

    /// Вложенный раздел обязан быть исключён из содержимого внешнего.
    ///
    /// Иначе `/boot/firmware` попал бы и в корень, и в собственный раздел:
    /// место занято дважды, а правка на устройстве меняет только одну копию.
    #[test]
    fn excludes_a_nested_partition_from_the_outer_one() {
        let root = temporary_directory("nested");
        let rootfs = root.join("rootfs");
        fs::create_dir_all(rootfs.join("boot/firmware")).expect("дерево должно создаваться");
        fs::create_dir_all(rootfs.join("etc")).expect("дерево должно создаваться");
        fs::write(rootfs.join("etc/os-release"), b"ID=platinum\n").expect("файл должен писаться");
        fs::write(rootfs.join("boot/firmware/config.txt"), b"arm_64bit=1\n")
            .expect("файл должен писаться");

        let layout = ImageLayout::new(
            vec![
                partition("root", Some("/"), 16),
                partition("boot", Some("/boot/firmware"), 64),
            ],
            1,
        )
        .expect("разметка должна быть корректной");

        let builder = ImageBuilder::new(layout);
        let staged = builder
            .stage_source(
                &partition("root", Some("/"), 16),
                &rootfs,
                &root.join("platinum.img"),
            )
            .expect("staging должен готовиться")
            .expect("у корня есть точка монтирования");

        assert!(
            staged.path.join("etc/os-release").is_file(),
            "корень сохранён"
        );
        assert!(
            staged.path.join("boot/firmware").is_dir(),
            "каталог точки монтирования обязан остаться"
        );
        assert!(
            !staged.path.join("boot/firmware/config.txt").exists(),
            "содержимое вложенного раздела не должно попадать в корневой"
        );
        assert!(
            rootfs.join("boot/firmware/config.txt").is_file(),
            "исходный rootfs не должен изменяться"
        );

        staged.cleanup();
        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }

    #[test]
    fn reports_a_missing_source_directory() {
        let root = temporary_directory("source");
        let layout = ImageLayout::new(vec![partition("root", Some("/"), 16)], 1)
            .expect("разметка должна быть корректной");

        let error = ImageBuilder::new(layout)
            .stage_source(
                &partition("root", Some("/boot"), 16),
                &root,
                &root.join("i.img"),
            )
            .expect_err("отсутствующий каталог должен быть ошибкой");

        assert!(matches!(error, ImageError::MissingSource { .. }));

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }
}
