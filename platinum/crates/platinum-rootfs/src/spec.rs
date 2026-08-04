use thiserror::Error;

/// Ошибки конфигурации rootfs.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RootfsError {
    /// Выпуск Ubuntu определяет URL и набор пакетов base archive.
    #[error("release Ubuntu Base не должен быть пустым")]
    EmptyRelease,
    /// Архитектура нужна, чтобы не смешать arm64 rootfs с образом другой платы.
    #[error("архитектура rootfs не должна быть пустой")]
    EmptyArchitecture,
}

/// Проверенная спецификация базовой Ubuntu filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootfsSpec {
    /// Выпуск Ubuntu, например `26.04`.
    pub release: String,
    /// Архитектура rootfs, например `arm64`.
    pub architecture: String,
}

impl RootfsSpec {
    /// Создаёт спецификацию rootfs с обязательными release и architecture.
    pub fn new(release: String, architecture: String) -> Result<Self, RootfsError> {
        if release.trim().is_empty() {
            return Err(RootfsError::EmptyRelease);
        }

        if architecture.trim().is_empty() {
            return Err(RootfsError::EmptyArchitecture);
        }

        Ok(Self {
            release,
            architecture,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{RootfsError, RootfsSpec};

    #[test]
    fn rejects_a_missing_release() {
        let error = RootfsSpec::new(String::new(), "arm64".into())
            .expect_err("release должен быть обязательным");

        assert_eq!(error, RootfsError::EmptyRelease);
    }
}
