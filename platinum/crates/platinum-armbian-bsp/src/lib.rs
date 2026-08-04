//! Адаптер Armbian Build для получения board-specific BSP артефактов.
//!
//! Platinum не копирует shell-логику Armbian. Вместо этого crate запускает
//! зафиксированный checkout Armbian через его публичный `compile.sh` interface
//! и сохраняет границу между universal OS builder и BSP конкретной платы.

mod inventory;

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use platinum_board::ArmbianConfig;
use thiserror::Error;

pub use inventory::{BspInventory, InventoryError, KernelArtifacts};

/// Ошибки подготовки или запуска Armbian Build.
#[derive(Debug, Error)]
pub enum ArmbianBspError {
    /// Плавающая Git-ссылка не обеспечивает повторяемый BSP build.
    #[error("revision Armbian должен быть 40-символьным SHA-1 commit")]
    InvalidRevision,
    /// Cache-путь уже занят каталогом, который не является Git checkout.
    #[error("Armbian cache-путь не является Git checkout: {path}")]
    NotGitCheckout {
        /// Каталог, который должен содержать `.git`.
        path: PathBuf,
    },
    /// Нельзя доверять существующему cache, если его origin указывает на иной
    /// repository, даже когда в нём случайно найден нужный commit.
    #[error("Armbian checkout использует другой origin: ожидался `{expected}`, получен `{actual}`")]
    UnexpectedRemote {
        /// Repository из board-конфигурации.
        expected: String,
        /// Repository, записанный в существующем checkout.
        actual: String,
    },
    /// Git выполнился, но checkout не оказался на запрошенном immutable commit.
    #[error("Armbian checkout имеет commit `{actual}`, ожидался `{expected}`")]
    UnexpectedHead {
        /// Commit из board-конфигурации.
        expected: String,
        /// Фактический HEAD checkout.
        actual: String,
    },
    /// Файловая система не позволила подготовить родительский cache-каталог.
    #[error("не удалось создать каталог Armbian cache `{path}`: {source}")]
    CreateCacheDirectory {
        /// Каталог, который не удалось создать.
        path: PathBuf,
        /// Исходная ошибка файловой системы.
        #[source]
        source: std::io::Error,
    },
    /// Git не удалось запустить как процесс.
    #[error("не удалось запустить git для операции `{operation}`: {source}")]
    StartGit {
        /// Логическое имя git-операции.
        operation: &'static str,
        /// Исходная ошибка запуска процесса.
        #[source]
        source: std::io::Error,
    },
    /// Git завершился с ошибкой, которую нельзя безопасно игнорировать.
    #[error("git-операция `{operation}` завершилась с кодом {code}: {stderr}")]
    GitFailed {
        /// Логическое имя git-операции.
        operation: &'static str,
        /// Код завершения или -1, если ОС не предоставила код.
        code: i32,
        /// Диагностика git.
        stderr: String,
    },
    /// Runner не запускает shell-скрипт, пока checkout не содержит compile.sh.
    #[error("в Armbian checkout отсутствует compile.sh: {path}")]
    MissingCompileScript {
        /// Ожидаемый путь к скрипту.
        path: PathBuf,
    },
    /// Операционная система не смогла запустить compile.sh.
    #[error("не удалось запустить Armbian Build: {source}")]
    Start {
        /// Исходная ошибка запуска процесса.
        #[source]
        source: std::io::Error,
    },
    /// Armbian Build завершился с ненулевым кодом.
    #[error("Armbian Build завершился с кодом {code}")]
    Failed {
        /// Код завершения или -1, если ОС не предоставила его.
        code: i32,
    },
}

/// Запуск pinned Armbian Build для одного board BSP.
///
/// Адаптер получает путь к уже подготовленному checkout. Клонирование и
/// верификация commit будут отдельным stage: разделение предотвращает сетевые
/// операции внутри кода, который отвечает только за исполнение BSP build.
#[derive(Debug, Clone)]
pub struct ArmbianBspRunner {
    source_dir: PathBuf,
    config: ArmbianConfig,
}

impl ArmbianBspRunner {
    /// Создаёт runner и проверяет, что configuration использует immutable commit.
    pub fn new(source_dir: PathBuf, config: ArmbianConfig) -> Result<Self, ArmbianBspError> {
        validate_revision(&config)?;

        Ok(Self { source_dir, config })
    }

    /// Запускает официальный Armbian target `kernel` для kernel и DTB.
    ///
    /// Метод не выдаёт kernel stage за полный bootable image: загрузчик
    /// собирается отдельным target.
    pub fn build_kernel(&self) -> Result<(), ArmbianBspError> {
        self.run_target("kernel")
    }

    /// Запускает официальный Armbian target `uboot` для загрузчика платы.
    ///
    /// Собранный пакет несёт `platform_install.sh` со смещениями записи SPL и
    /// U-Boot для family платы. Поэтому Platinum не хранит эти смещения у себя
    /// и не расходится с upstream при их изменении.
    pub fn build_uboot(&self) -> Result<(), ArmbianBspError> {
        self.run_target("uboot")
    }

    /// Выполняет один официальный target `compile.sh` для платы.
    fn run_target(&self, target: &'static str) -> Result<(), ArmbianBspError> {
        let compile_script = self.source_dir.join("compile.sh");

        self.verify_pinned_head()?;

        if !compile_script.is_file() {
            return Err(ArmbianBspError::MissingCompileScript {
                path: compile_script,
            });
        }

        let status = Command::new("./compile.sh")
            .current_dir(&self.source_dir)
            .arg(target)
            .arg(format!("BOARD={}", self.config.board))
            .arg(format!("BRANCH={}", self.config.kernel_branch))
            .arg("KERNEL_CONFIGURE=no")
            .status()
            .map_err(|source| ArmbianBspError::Start { source })?;

        if status.success() {
            Ok(())
        } else {
            Err(ArmbianBspError::Failed {
                code: status.code().unwrap_or(-1),
            })
        }
    }

    /// Возвращает путь к pinned checkout, не передавая владение вызывающему коду.
    pub fn source_dir(&self) -> &Path {
        &self.source_dir
    }

    /// Проверяет, что существующий checkout остаётся на board-specific pin.
    ///
    /// Runner делает проверку самостоятельно, даже когда CLI уже выполнил
    /// `bsp-sync`. Так библиотечный API нельзя безопасно вызвать с checkout,
    /// который был изменён между синхронизацией и compilation.
    fn verify_pinned_head(&self) -> Result<(), ArmbianBspError> {
        if !self.source_dir.join(".git").exists() {
            return Err(ArmbianBspError::NotGitCheckout {
                path: self.source_dir.clone(),
            });
        }

        let head = run_git(
            Some(&self.source_dir),
            "read HEAD before bsp build",
            &["rev-parse".into(), "HEAD".into()],
        )?;
        let head = head.trim();

        if head != self.config.revision {
            return Err(ArmbianBspError::UnexpectedHead {
                expected: self.config.revision.clone(),
                actual: head.to_owned(),
            });
        }

        Ok(())
    }
}

/// Управляет воспроизводимым cache checkout Armbian Build.
///
/// Этот тип отвечает только за Git lifecycle. Разделение с ArmbianBspRunner
/// важно: подготовка входного source и запуск BSP build имеют разные failure
/// modes и будут отдельными stages pipeline.
#[derive(Debug, Clone)]
pub struct ArmbianCheckout {
    checkout_dir: PathBuf,
    config: ArmbianConfig,
}

impl ArmbianCheckout {
    /// Создаёт checkout manager для заданного cache-каталога.
    pub fn new(checkout_dir: PathBuf, config: ArmbianConfig) -> Result<Self, ArmbianBspError> {
        validate_revision(&config)?;

        Ok(Self {
            checkout_dir,
            config,
        })
    }

    /// Клонирует или обновляет checkout и проверяет pinned Git commit.
    ///
    /// Fetch выполняется по SHA, а не по ветке. Поэтому будущие изменения
    /// Armbian `main` не способны незаметно изменить результат Platinum build.
    pub fn sync(&self) -> Result<(), ArmbianBspError> {
        if !self.checkout_dir.exists() {
            let parent =
                self.checkout_dir
                    .parent()
                    .ok_or_else(|| ArmbianBspError::NotGitCheckout {
                        path: self.checkout_dir.clone(),
                    })?;

            fs::create_dir_all(parent).map_err(|source| ArmbianBspError::CreateCacheDirectory {
                path: parent.to_path_buf(),
                source,
            })?;

            run_git(
                None,
                "clone",
                &[
                    "clone".into(),
                    "--no-checkout".into(),
                    self.config.repository.clone(),
                    self.checkout_dir.display().to_string(),
                ],
            )?;
        }

        if !self.checkout_dir.join(".git").exists() {
            return Err(ArmbianBspError::NotGitCheckout {
                path: self.checkout_dir.clone(),
            });
        }

        let remote = run_git(
            Some(&self.checkout_dir),
            "read origin",
            &["remote".into(), "get-url".into(), "origin".into()],
        )?;
        let remote = remote.trim();

        if remote != self.config.repository {
            return Err(ArmbianBspError::UnexpectedRemote {
                expected: self.config.repository.clone(),
                actual: remote.to_owned(),
            });
        }

        run_git(
            Some(&self.checkout_dir),
            "fetch pinned commit",
            &[
                "fetch".into(),
                "--depth".into(),
                "1".into(),
                "origin".into(),
                self.config.revision.clone(),
            ],
        )?;
        run_git(
            Some(&self.checkout_dir),
            "checkout pinned commit",
            &["checkout".into(), "--detach".into(), "FETCH_HEAD".into()],
        )?;

        let head = run_git(
            Some(&self.checkout_dir),
            "read HEAD",
            &["rev-parse".into(), "HEAD".into()],
        )?;
        let head = head.trim();

        if head != self.config.revision {
            return Err(ArmbianBspError::UnexpectedHead {
                expected: self.config.revision.clone(),
                actual: head.to_owned(),
            });
        }

        Ok(())
    }

    /// Возвращает путь к checkout без передачи владения вызывающему коду.
    pub fn checkout_dir(&self) -> &Path {
        &self.checkout_dir
    }
}

fn validate_revision(config: &ArmbianConfig) -> Result<(), ArmbianBspError> {
    if !is_commit_sha(&config.revision) {
        return Err(ArmbianBspError::InvalidRevision);
    }

    Ok(())
}

fn run_git(
    current_dir: Option<&Path>,
    operation: &'static str,
    arguments: &[String],
) -> Result<String, ArmbianBspError> {
    let mut command = Command::new("git");
    command.args(arguments);

    if let Some(current_dir) = current_dir {
        command.current_dir(current_dir);
    }

    let output = command
        .output()
        .map_err(|source| ArmbianBspError::StartGit { operation, source })?;

    if !output.status.success() {
        return Err(ArmbianBspError::GitFailed {
            operation,
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn is_commit_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use platinum_board::ArmbianConfig;

    use super::{ArmbianBspError, ArmbianBspRunner};

    #[test]
    fn rejects_a_floating_armbian_revision() {
        let config = ArmbianConfig {
            repository: "https://example.test/armbian.git".into(),
            revision: "main".into(),
            board: "orangepizero3w".into(),
            kernel_branch: "vendor".into(),
        };

        let error = ArmbianBspRunner::new(PathBuf::from("/armbian"), config)
            .expect_err("плавающая ссылка не должна приниматься");

        assert!(matches!(error, ArmbianBspError::InvalidRevision));
    }
}
