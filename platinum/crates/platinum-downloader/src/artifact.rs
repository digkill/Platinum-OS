use thiserror::Error;

/// Ошибки при создании описания артефакта.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ArtifactError {
    /// Без URL stage не сможет определить источник данных.
    #[error("URL артефакта не должен быть пустым")]
    EmptyUrl,
    /// Проверка целостности нужна до использования внешнего архива.
    #[error("SHA-256 артефакта должен содержать 64 шестнадцатеричных символа")]
    InvalidSha256,
    /// URL без имени файла не позволяет детерминированно назвать загрузку.
    #[error("URL `{url}` не содержит имени файла")]
    MissingFileName {
        /// URL, из которого не удалось извлечь имя.
        url: String,
    },
}

/// Внешний артефакт и ожидаемая контрольная сумма SHA-256.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    /// URL, по которому доступен артефакт.
    pub url: String,
    /// SHA-256 в шестнадцатеричном виде.
    pub sha256: String,
}

impl Artifact {
    /// Создаёт артефакт после локальной проверки обязательных полей.
    pub fn new(url: String, sha256: String) -> Result<Self, ArtifactError> {
        if url.trim().is_empty() {
            return Err(ArtifactError::EmptyUrl);
        }

        if sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ArtifactError::InvalidSha256);
        }

        Ok(Self {
            url,
            // Регистр не влияет на значение хеша, но сравнение строк идёт
            // побайтово, поэтому форма нормализуется один раз при создании.
            sha256: sha256.to_ascii_lowercase(),
        })
    }

    /// Возвращает имя файла, под которым артефакт сохраняется в кеше загрузок.
    ///
    /// Имя берётся из URL, а не задаётся отдельным полем конфигурации: так
    /// файл в каталоге загрузок всегда соответствует своему источнику.
    pub fn file_name(&self) -> Result<&str, ArtifactError> {
        let without_query = self.url.split(['?', '#']).next().unwrap_or(&self.url);

        // Authority отбрасывается явно: иначе URL вида `https://host/` вернул бы
        // имя хоста и сборка сохранила бы артефакт под бессмысленным именем.
        let after_scheme = without_query
            .split_once("://")
            .map_or(without_query, |(_, rest)| rest);
        let path = after_scheme.split_once('/').map_or("", |(_, path)| path);

        let candidate = path.trim_end_matches('/').rsplit('/').next().unwrap_or("");

        if candidate.is_empty() || candidate == "." || candidate == ".." {
            return Err(ArtifactError::MissingFileName {
                url: self.url.clone(),
            });
        }

        Ok(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::{Artifact, ArtifactError};

    const VALID_SHA256: &str = "b2b46a37324ea1954e93f293fe6d7c2241daf2fc298c4022e6e4caceeed74cab";

    #[test]
    fn rejects_an_invalid_checksum() {
        let error = Artifact::new(
            "https://example.test/rootfs.tar.gz".into(),
            "invalid".into(),
        )
        .expect_err("некорректная контрольная сумма должна быть отклонена");

        assert_eq!(error, ArtifactError::InvalidSha256);
    }

    #[test]
    fn extracts_a_file_name_from_the_url() {
        let artifact = Artifact::new(
            "https://cdimage.ubuntu.com/ubuntu-base/releases/26.04/release/ubuntu-base-26.04-base-arm64.tar.gz".into(),
            VALID_SHA256.into(),
        )
        .expect("артефакт должен быть корректным");

        assert_eq!(
            artifact.file_name().expect("имя файла должно определяться"),
            "ubuntu-base-26.04-base-arm64.tar.gz"
        );
    }

    #[test]
    fn rejects_a_url_without_a_file_name() {
        let artifact = Artifact::new("https://example.test/".into(), VALID_SHA256.into())
            .expect("артефакт должен быть корректным");

        let error = artifact
            .file_name()
            .expect_err("URL без имени файла должен быть отклонён");

        assert!(matches!(error, ArtifactError::MissingFileName { .. }));
    }
}
