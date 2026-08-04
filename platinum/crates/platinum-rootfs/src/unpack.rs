use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

use thiserror::Error;
use tracing::{info, warn};

use crate::sys;

/// Ошибки распаковки base-архива в каталог rootfs.
#[derive(Debug, Error)]
pub enum UnpackError {
    /// Архив не найден: stage загрузки не выполнялся или дал другой путь.
    #[error("архив rootfs не найден: {path}")]
    MissingArchive {
        /// Ожидавшийся путь к архиву.
        path: PathBuf,
    },
    /// Каталог назначения занят: смешение двух rootfs даст неработоспособный образ.
    #[error("каталог rootfs `{path}` не пуст")]
    TargetNotEmpty {
        /// Каталог, который должен быть пустым или отсутствовать.
        path: PathBuf,
    },
    /// Каталог назначения не удалось создать или прочитать.
    #[error("не удалось подготовить каталог rootfs `{path}`: {source}")]
    PrepareTarget {
        /// Проблемный каталог.
        path: PathBuf,
        /// Исходная ошибка файловой системы.
        #[source]
        source: io::Error,
    },
    /// Внешний `tar` не удалось запустить.
    #[error("не удалось запустить tar: {source}")]
    StartTar {
        /// Исходная ошибка запуска процесса.
        #[source]
        source: io::Error,
    },
    /// `tar` завершился с ошибкой, которую нельзя игнорировать.
    #[error("tar завершился с кодом {code}: {stderr}")]
    TarFailed {
        /// Код завершения или -1, если ОС не предоставила его.
        code: i32,
        /// Диагностика tar.
        stderr: String,
    },
}

/// Распаковка Ubuntu Base архива в каталог будущего rootfs.
///
/// Платформенный `tar` используется вместо чистой Rust-реализации намеренно:
/// base-архив содержит symlinks, специальные файлы и владельцев, а собственная
/// распаковка молча потеряла бы их и дала бы незагружаемый образ.
#[derive(Debug, Clone)]
pub struct RootfsUnpacker {
    archive: PathBuf,
    target: PathBuf,
}

impl RootfsUnpacker {
    /// Создаёт распаковщик для конкретной пары «архив — каталог rootfs».
    pub fn new(archive: PathBuf, target: PathBuf) -> Self {
        Self { archive, target }
    }

    /// Распаковывает архив и возвращает путь к готовому каталогу rootfs.
    ///
    /// Права и владельцы сохраняются только при запуске от root. Под обычным
    /// пользователем результат пригоден для инспекции, но не для загрузки, о чём
    /// stage сообщает предупреждением, а не молчаливым успехом.
    pub fn unpack(&self) -> Result<&Path, UnpackError> {
        if !self.archive.is_file() {
            return Err(UnpackError::MissingArchive {
                path: self.archive.clone(),
            });
        }

        self.prepare_target()?;

        if !sys::is_root() {
            warn!(
                target = %self.target.display(),
                "rootfs распакован не от root: владельцы файлов и device nodes не будут восстановлены"
            );
        }

        let output = Command::new("tar")
            .arg("--numeric-owner")
            .arg("-xpf")
            .arg(&self.archive)
            .arg("-C")
            .arg(&self.target)
            .output()
            .map_err(|source| UnpackError::StartTar { source })?;

        if !output.status.success() {
            return Err(UnpackError::TarFailed {
                code: output.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            });
        }

        info!(
            archive = %self.archive.display(),
            target = %self.target.display(),
            "rootfs unpacked"
        );

        Ok(&self.target)
    }

    /// Возвращает каталог, в который распаковывается rootfs.
    pub fn target(&self) -> &Path {
        &self.target
    }

    /// Создаёт пустой каталог назначения либо подтверждает, что он пуст.
    fn prepare_target(&self) -> Result<(), UnpackError> {
        if self.target.exists() {
            let mut entries =
                fs::read_dir(&self.target).map_err(|source| UnpackError::PrepareTarget {
                    path: self.target.clone(),
                    source,
                })?;

            if entries.next().is_some() {
                return Err(UnpackError::TargetNotEmpty {
                    path: self.target.clone(),
                });
            }

            return Ok(());
        }

        fs::create_dir_all(&self.target).map_err(|source| UnpackError::PrepareTarget {
            path: self.target.clone(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{RootfsUnpacker, UnpackError};

    fn temporary_directory(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "platinum-rootfs-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("системное время должно быть позже Unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&path).expect("временный каталог должен создаваться");

        path
    }

    #[test]
    fn unpacks_a_tarball_into_an_empty_directory() {
        let root = temporary_directory("unpack");
        let source = root.join("source");
        fs::create_dir_all(source.join("etc")).expect("исходный каталог должен создаваться");
        fs::write(source.join("etc/os-release"), b"ID=ubuntu\n")
            .expect("тестовый файл должен записываться");

        let archive = root.join("base.tar.gz");
        let status = Command::new("tar")
            .arg("-czf")
            .arg(&archive)
            .arg("-C")
            .arg(&source)
            .arg(".")
            .status()
            .expect("tar должен быть доступен в системе");
        assert!(
            status.success(),
            "подготовка архива должна завершаться успешно"
        );

        let target = root.join("rootfs");
        RootfsUnpacker::new(archive, target.clone())
            .unpack()
            .expect("архив должен распаковываться");

        assert_eq!(
            fs::read_to_string(target.join("etc/os-release"))
                .expect("распакованный файл должен читаться"),
            "ID=ubuntu\n"
        );

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }

    #[test]
    fn refuses_to_unpack_into_a_used_directory() {
        let root = temporary_directory("busy");
        let archive = root.join("base.tar.gz");
        fs::write(&archive, b"not really an archive").expect("файл должен записываться");

        let target = root.join("rootfs");
        fs::create_dir_all(&target).expect("каталог должен создаваться");
        fs::write(target.join("existing"), b"x").expect("файл должен записываться");

        let error = RootfsUnpacker::new(archive, target)
            .unpack()
            .expect_err("непустой каталог должен быть отклонён");

        assert!(matches!(error, UnpackError::TargetNotEmpty { .. }));

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }
}
