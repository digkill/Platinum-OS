use std::path::{Path, PathBuf};

use thiserror::Error;

/// Ошибки в наборе директорий конкретной сборки.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BuildPathsError {
    /// Пустой путь нельзя передавать файловым операциям: его смысл зависит от
    /// текущей директории процесса, что сделало бы сборку непредсказуемой.
    #[error("путь `{name}` не должен быть пустым")]
    EmptyPath {
        /// Логическое имя неверного поля для сообщения об ошибке.
        name: &'static str,
    },
}

/// Явно заданные директории, используемые одной сборкой.
///
/// Пути передаются извне, а не конструируются из скрытых строк внутри stages.
/// Это делает BuildEngine одинаковым для локального запуска, CI и будущего
/// remote builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildPaths {
    /// Промежуточные результаты текущей сборки.
    pub work_dir: PathBuf,
    /// Каталог загруженных артефактов.
    pub downloads_dir: PathBuf,
    /// Повторно используемые артефакты между сборками.
    pub cache_dir: PathBuf,
    /// Каталог, в который попадёт готовый образ.
    pub output_dir: PathBuf,
}

impl BuildPaths {
    /// Создаёт проверенный набор директорий сборки.
    pub fn new(
        work_dir: PathBuf,
        downloads_dir: PathBuf,
        cache_dir: PathBuf,
        output_dir: PathBuf,
    ) -> Result<Self, BuildPathsError> {
        Self::validate("work_dir", &work_dir)?;
        Self::validate("downloads_dir", &downloads_dir)?;
        Self::validate("cache_dir", &cache_dir)?;
        Self::validate("output_dir", &output_dir)?;

        Ok(Self {
            work_dir,
            downloads_dir,
            cache_dir,
            output_dir,
        })
    }

    fn validate(name: &'static str, path: &Path) -> Result<(), BuildPathsError> {
        if path.as_os_str().is_empty() {
            return Err(BuildPathsError::EmptyPath { name });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{BuildPaths, BuildPathsError};
    use std::path::PathBuf;

    #[test]
    fn rejects_an_empty_work_directory() {
        let error = BuildPaths::new(
            PathBuf::new(),
            PathBuf::from("downloads"),
            PathBuf::from("cache"),
            PathBuf::from("output"),
        )
        .expect_err("пустой путь должен быть отклонён");

        assert_eq!(error, BuildPathsError::EmptyPath { name: "work_dir" });
    }
}
