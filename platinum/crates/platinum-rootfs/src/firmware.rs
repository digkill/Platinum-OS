//! Установка vendor-firmware в rootfs.
//!
//! Firmware берётся из репозитория, зафиксированного commit, и только теми
//! каталогами, которые объявила плата. Полный пакет Armbian содержит firmware
//! всех поддерживаемых устройств: для платы с одним Wi-Fi-чипом это сотни
//! мебибайт ради нескольких файлов.
//!
//! Выборка делается sparse checkout на глубину 1. Полный clone репозитория
//! firmware стоил бы гигабайты истории blob-ов, которые сборке не нужны.

use std::{
    collections::BTreeMap,
    fs, io,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::Command,
};

use thiserror::Error;
use tracing::info;

/// Ошибки установки firmware.
#[derive(Debug, Error)]
pub enum FirmwareError {
    /// Плавающая Git-ссылка не даёт воспроизводимый образ.
    #[error("revision firmware должен быть 40-символьным SHA-1 commit, получен `{revision}`")]
    InvalidRevision {
        /// Отклонённое значение.
        revision: String,
    },
    /// Список каталогов пуст: установка не перенесла бы ни одного файла.
    #[error("в описании firmware не указано ни одного каталога")]
    NoDirectories,
    /// Путь выходит за пределы репозитория или каталога firmware.
    #[error("путь firmware `{path}` должен быть относительным и без `..`")]
    UnsafePath {
        /// Отклонённый путь.
        path: String,
    },
    /// Git не удалось запустить.
    #[error("не удалось запустить git для операции `{operation}`: {source}")]
    StartGit {
        /// Логическое имя операции.
        operation: &'static str,
        /// Исходная ошибка запуска процесса.
        #[source]
        source: io::Error,
    },
    /// Git завершился с ошибкой.
    #[error("git-операция `{operation}` завершилась с кодом {code}: {stderr}")]
    GitFailed {
        /// Логическое имя операции.
        operation: &'static str,
        /// Код завершения или -1, если ОС не предоставила его.
        code: i32,
        /// Диагностика git.
        stderr: String,
    },
    /// Checkout оказался не на запрошенном commit.
    #[error("checkout firmware имеет commit `{actual}`, ожидался `{expected}`")]
    UnexpectedHead {
        /// Ожидаемый commit.
        expected: String,
        /// Фактический HEAD.
        actual: String,
    },
    /// Объявленного каталога нет в репозитории на этом commit.
    #[error("в репозитории firmware нет каталога `{directory}` на commit `{revision}`")]
    MissingDirectory {
        /// Каталог из board-конфигурации.
        directory: String,
        /// Commit, на котором шёл поиск.
        revision: String,
    },
    /// Цель симлинка не появилась после копирования.
    #[error("симлинк `{name}` указывает на `{target}`, которого нет в `/lib/firmware`")]
    MissingLinkTarget {
        /// Имя симлинка.
        name: String,
        /// Цель симлинка.
        target: String,
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

/// Каталог firmware внутри rootfs.
const FIRMWARE_DIRECTORY: &str = "lib/firmware";

/// Что и откуда ставить в `/lib/firmware`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareSpec {
    /// Git URL репозитория firmware.
    pub repository: String,
    /// Неизменяемый commit репозитория.
    pub revision: String,
    /// Каталоги репозитория, копируемые тем же относительным путём.
    pub directories: Vec<String>,
    /// Симлинки внутри `/lib/firmware`: имя — цель.
    pub links: BTreeMap<String, String>,
}

impl FirmwareSpec {
    /// Создаёт проверенное описание firmware.
    pub fn new(
        repository: String,
        revision: String,
        directories: Vec<String>,
        links: BTreeMap<String, String>,
    ) -> Result<Self, FirmwareError> {
        if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(FirmwareError::InvalidRevision { revision });
        }

        if directories.is_empty() {
            return Err(FirmwareError::NoDirectories);
        }

        for path in directories.iter().chain(links.keys()).chain(links.values()) {
            if !is_safe_relative(path) {
                return Err(FirmwareError::UnsafePath { path: path.clone() });
            }
        }

        Ok(Self {
            repository,
            revision,
            directories,
            links,
        })
    }
}

/// Переносит firmware из pinned репозитория в rootfs.
#[derive(Debug, Clone)]
pub struct FirmwareInstaller {
    spec: FirmwareSpec,
}

impl FirmwareInstaller {
    /// Создаёт установщик для описания firmware.
    pub fn new(spec: FirmwareSpec) -> Self {
        Self { spec }
    }

    /// Синхронизирует checkout и переносит объявленные каталоги в rootfs.
    ///
    /// `checkout` переиспользуется между сборками: репозиторий firmware меняется
    /// редко, а его выборка стоит сети.
    pub fn install(&self, rootfs: &Path, checkout: &Path) -> Result<usize, FirmwareError> {
        self.sync(checkout)?;

        let target_root = rootfs.join(FIRMWARE_DIRECTORY);
        let mut files = 0;

        for directory in &self.spec.directories {
            let source = checkout.join(directory);
            if !source.is_dir() {
                return Err(FirmwareError::MissingDirectory {
                    directory: directory.clone(),
                    revision: self.spec.revision.clone(),
                });
            }

            let target = target_root.join(directory);
            files += copy_tree(&source, &target)?;
        }

        for (name, target) in &self.spec.links {
            // Цель проверяется после копирования: симлинк на отсутствующий
            // каталог драйвер отработает как «firmware нет», и Wi-Fi молча не
            // поднимется уже на устройстве.
            if !target_root.join(target).exists() {
                return Err(FirmwareError::MissingLinkTarget {
                    name: name.clone(),
                    target: target.clone(),
                });
            }

            let link = target_root.join(name);
            remove_existing(&link)?;
            symlink(target, &link).map_err(|source| FirmwareError::Write {
                path: link.clone(),
                source,
            })?;
        }

        info!(
            files,
            links = self.spec.links.len(),
            revision = %self.spec.revision,
            "firmware installed"
        );

        Ok(files)
    }

    /// Готовит sparse checkout репозитория на зафиксированном commit.
    fn sync(&self, checkout: &Path) -> Result<(), FirmwareError> {
        fs::create_dir_all(checkout).map_err(|source| FirmwareError::Write {
            path: checkout.to_path_buf(),
            source,
        })?;

        run_git(checkout, "init", &["init", "-q"])?;

        // Origin переустанавливается каждый раз: checkout мог остаться от платы
        // с другим репозиторием firmware, и тогда fetch по SHA молча взял бы
        // объект из чужой истории.
        run_git(checkout, "set origin", &["remote", "remove", "origin"]).ok();
        run_git(
            checkout,
            "add origin",
            &["remote", "add", "origin", &self.spec.repository],
        )?;

        run_git(
            checkout,
            "enable sparse checkout",
            &["config", "core.sparseCheckout", "true"],
        )?;

        let sparse = checkout.join(".git/info/sparse-checkout");
        if let Some(parent) = sparse.parent() {
            fs::create_dir_all(parent).map_err(|source| FirmwareError::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        fs::write(
            &sparse,
            self.spec
                .directories
                .iter()
                .map(|directory| format!("{directory}/*"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .map_err(|source| FirmwareError::Write {
            path: sparse.clone(),
            source,
        })?;

        run_git(
            checkout,
            "fetch pinned commit",
            &["fetch", "-q", "--depth", "1", "origin", &self.spec.revision],
        )?;
        run_git(checkout, "checkout", &["checkout", "-q", "FETCH_HEAD"])?;

        let head = run_git(checkout, "read HEAD", &["rev-parse", "HEAD"])?;
        let head = head.trim();
        if head != self.spec.revision {
            return Err(FirmwareError::UnexpectedHead {
                expected: self.spec.revision.clone(),
                actual: head.to_owned(),
            });
        }

        Ok(())
    }
}

/// Сообщает, безопасен ли путь как относительный внутри дерева.
fn is_safe_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.split('/').any(|part| part == ".." || part.is_empty())
}

/// Рекурсивно копирует каталог, возвращая количество скопированных файлов.
fn copy_tree(source: &Path, target: &Path) -> Result<usize, FirmwareError> {
    fs::create_dir_all(target).map_err(|error| FirmwareError::Write {
        path: target.to_path_buf(),
        source: error,
    })?;

    let entries = fs::read_dir(source).map_err(|error| FirmwareError::Write {
        path: source.to_path_buf(),
        source: error,
    })?;

    let mut files = 0;
    for entry in entries {
        let entry = entry.map_err(|error| FirmwareError::Write {
            path: source.to_path_buf(),
            source: error,
        })?;

        let from = entry.path();
        let to = target.join(entry.file_name());

        if from.is_dir() {
            files += copy_tree(&from, &to)?;
        } else {
            remove_existing(&to)?;
            fs::copy(&from, &to).map_err(|error| FirmwareError::Write {
                path: to.clone(),
                source: error,
            })?;
            files += 1;
        }
    }

    Ok(files)
}

/// Удаляет файл или симлинк, если он есть.
///
/// `symlink_metadata` вместо `exists`: битый симлинк не «существует», но
/// занимает имя, и создание нового завершилось бы `AlreadyExists`.
fn remove_existing(path: &Path) -> Result<(), FirmwareError> {
    if fs::symlink_metadata(path).is_ok() {
        fs::remove_file(path).map_err(|source| FirmwareError::Write {
            path: path.to_path_buf(),
            source,
        })?;
    }

    Ok(())
}

/// Запускает git в каталоге checkout.
fn run_git(
    checkout: &Path,
    operation: &'static str,
    arguments: &[&str],
) -> Result<String, FirmwareError> {
    let output = Command::new("git")
        .current_dir(checkout)
        .args(arguments)
        .output()
        .map_err(|source| FirmwareError::StartGit { operation, source })?;

    if !output.status.success() {
        return Err(FirmwareError::GitFailed {
            operation,
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{FirmwareError, FirmwareSpec};

    fn links() -> BTreeMap<String, String> {
        BTreeMap::from([(
            "aic8800d80".to_owned(),
            "aic8800/SDIO/aic8800D80".to_owned(),
        )])
    }

    #[test]
    fn rejects_a_floating_revision() {
        let error = FirmwareSpec::new(
            "https://example.test/firmware".into(),
            "master".into(),
            vec!["aic8800".into()],
            BTreeMap::new(),
        )
        .expect_err("плавающая ссылка не должна приниматься");

        assert!(matches!(error, FirmwareError::InvalidRevision { .. }));
    }

    #[test]
    fn rejects_an_empty_directory_list() {
        let error = FirmwareSpec::new(
            "https://example.test/firmware".into(),
            "0123456789abcdef0123456789abcdef01234567".into(),
            Vec::new(),
            BTreeMap::new(),
        )
        .expect_err("пустой список каталогов не должен приниматься");

        assert!(matches!(error, FirmwareError::NoDirectories));
    }

    /// Путь с `..` вынес бы копирование за пределы `/lib/firmware`.
    #[test]
    fn rejects_a_path_escaping_the_firmware_directory() {
        for path in ["../etc", "/etc/shadow", "aic8800/../../etc"] {
            let error = FirmwareSpec::new(
                "https://example.test/firmware".into(),
                "0123456789abcdef0123456789abcdef01234567".into(),
                vec![path.into()],
                BTreeMap::new(),
            )
            .expect_err("небезопасный путь не должен приниматься");

            assert!(matches!(error, FirmwareError::UnsafePath { .. }));
        }
    }

    #[test]
    fn accepts_a_pinned_specification() {
        let spec = FirmwareSpec::new(
            "https://github.com/armbian/firmware".into(),
            "d9846710f54da5e4383e2d67311819659ac2cf5c".into(),
            vec!["aic8800/SDIO/aic8800D80".into()],
            links(),
        )
        .expect("корректное описание должно приниматься");

        assert_eq!(spec.directories.len(), 1);
        assert_eq!(spec.links.len(), 1);
    }
}
