//! Инициализация структурированного логирования для приложений Platinum OS.
//!
//! Библиотеки только создают события через tracing. Подписчик настраивается на
//! границе приложения, чтобы CLI, тесты и будущий daemon могли выбирать свой
//! формат и уровень вывода независимо.

use thiserror::Error;
use tracing_subscriber::{EnvFilter, fmt};

/// Ошибки установки глобального tracing subscriber.
#[derive(Debug, Error)]
pub enum LoggerError {
    /// Subscriber уже мог быть установлен интеграционным тестом или вызывающим
    /// приложением, поэтому библиотека возвращает ошибку вместо panic.
    #[error("не удалось инициализировать tracing subscriber: {source}")]
    Initialization {
        /// Исходная ошибка tracing-subscriber.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// Инициализирует текстовый tracing subscriber для CLI.
///
/// Значение `RUST_LOG` имеет приоритет над уровнем `info`, чтобы CI и
/// разработчик могли увеличить детализацию без изменения конфигурации проекта.
pub fn init() -> Result<(), LoggerError> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init()
        .map_err(|source| LoggerError::Initialization { source })
}
