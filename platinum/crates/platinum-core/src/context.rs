use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::BuildPaths;

/// Ошибка обращения к результату, который не получен предыдущими stages.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("stage не нашёл обязательный результат `{key}` в контексте сборки")]
pub struct MissingOutput {
    /// Ключ, который ожидал текущий stage.
    key: String,
}

impl MissingOutput {
    /// Возвращает ключ отсутствующего результата.
    pub fn key(&self) -> &str {
        &self.key
    }
}

/// Состояние, общее для всех stages одной сборки.
///
/// Контекст владеет путями и реестром результатов, поэтому stages обмениваются
/// данными через явные ключи, а не через общий типизированный объект. Так
/// platinum-core остаётся независимым от board, BSP и формата образа.
#[derive(Debug)]
pub struct BuildContext {
    paths: BuildPaths,
    outputs: BTreeMap<String, PathBuf>,
}

impl BuildContext {
    /// Создаёт контекст для одной сборки из уже проверенных путей.
    pub fn new(paths: BuildPaths) -> Self {
        Self {
            paths,
            outputs: BTreeMap::new(),
        }
    }

    /// Возвращает пути, принадлежащие текущей сборке.
    pub fn paths(&self) -> &BuildPaths {
        &self.paths
    }

    /// Записывает путь к результату stage под стабильным ключом.
    pub fn record(&mut self, key: impl Into<String>, path: PathBuf) {
        self.outputs.insert(key.into(), path);
    }

    /// Возвращает результат предыдущего stage, если он был записан.
    pub fn output(&self, key: &str) -> Option<&Path> {
        self.outputs.get(key).map(PathBuf::as_path)
    }

    /// Возвращает обязательный результат предыдущего stage.
    ///
    /// Отсутствие ключа — ошибка состава pipeline, а не окружения, поэтому она
    /// отделена от ошибок ввода-вывода и называет недостающий ключ.
    pub fn require_output(&self, key: &str) -> Result<&Path, MissingOutput> {
        self.output(key).ok_or_else(|| MissingOutput {
            key: key.to_owned(),
        })
    }

    /// Перечисляет записанные результаты в детерминированном порядке.
    pub fn outputs(&self) -> impl Iterator<Item = (&str, &Path)> {
        self.outputs
            .iter()
            .map(|(key, path)| (key.as_str(), path.as_path()))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{BuildContext, BuildPaths};

    fn context() -> BuildContext {
        let paths = BuildPaths::new(
            PathBuf::from("work"),
            PathBuf::from("downloads"),
            PathBuf::from("cache"),
            PathBuf::from("output"),
        )
        .expect("тестовые пути должны быть корректными");

        BuildContext::new(paths)
    }

    #[test]
    fn returns_a_recorded_output() {
        let mut context = context();
        context.record("rootfs.archive", PathBuf::from("downloads/base.tar.gz"));

        assert_eq!(
            context
                .require_output("rootfs.archive")
                .expect("записанный результат должен находиться"),
            PathBuf::from("downloads/base.tar.gz")
        );
    }

    #[test]
    fn reports_a_missing_output_by_key() {
        let error = context()
            .require_output("rootfs.archive")
            .expect_err("отсутствующий результат должен быть ошибкой");

        assert_eq!(error.key(), "rootfs.archive");
    }
}
