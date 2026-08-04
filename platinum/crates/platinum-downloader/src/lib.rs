//! Загрузка внешних артефактов с обязательной проверкой SHA-256.
//!
//! Артефакт описан отдельно от транспорта: конфигурацию можно проверить без
//! сети, а уже скачанный файл переиспользуется только после сверки хеша.

mod artifact;
mod hash;

use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use thiserror::Error;
use tracing::{info, warn};

pub use artifact::{Artifact, ArtifactError};
pub use hash::sha256_of_file;

/// Ошибки загрузки артефакта.
#[derive(Debug, Error)]
pub enum DownloadError {
    /// Каталог загрузок недоступен для записи.
    #[error("не удалось подготовить каталог загрузок `{path}`: {source}")]
    PrepareDirectory {
        /// Каталог, который не удалось создать.
        path: PathBuf,
        /// Исходная ошибка файловой системы.
        #[source]
        source: io::Error,
    },
    /// Транспорт не смог получить ответ.
    #[error("не удалось запросить `{url}`: {source}")]
    Transport {
        /// Запрошенный URL.
        url: String,
        /// Исходная ошибка HTTP-клиента.
        #[source]
        source: Box<ureq::Error>,
    },
    /// Сервер ответил кодом, при котором тело нельзя считать артефактом.
    #[error("сервер вернул статус {status} для `{url}`")]
    UnexpectedStatus {
        /// Запрошенный URL.
        url: String,
        /// HTTP-статус ответа.
        status: u16,
    },
    /// Ошибка чтения тела ответа или записи файла на диск.
    #[error("не удалось сохранить `{path}`: {source}")]
    Write {
        /// Файл, в который выполнялась запись.
        path: PathBuf,
        /// Исходная ошибка ввода-вывода.
        #[source]
        source: io::Error,
    },
    /// Контрольная сумма не совпала: артефакт нельзя использовать в сборке.
    #[error("SHA-256 `{url}` не совпал: ожидался `{expected}`, получен `{actual}`")]
    ChecksumMismatch {
        /// Запрошенный URL.
        url: String,
        /// Сумма из конфигурации.
        expected: String,
        /// Сумма фактически полученных данных.
        actual: String,
    },
    /// Не удалось прочитать существующий файл для проверки суммы.
    #[error("не удалось вычислить SHA-256 файла `{path}`: {source}")]
    Hash {
        /// Файл, который не удалось прочитать.
        path: PathBuf,
        /// Исходная ошибка ввода-вывода.
        #[source]
        source: io::Error,
    },
    /// Описание артефакта не позволяет определить имя файла на диске.
    #[error("не удалось определить имя файла артефакта: {source}")]
    Artifact {
        /// Исходная ошибка валидации артефакта.
        #[source]
        source: ArtifactError,
    },
}

/// Blocking HTTP-загрузчик артефактов сборки.
///
/// Синхронный транспорт выбран намеренно: pipeline выполняет stages
/// последовательно, поэтому async runtime добавил бы зависимости и сложность
/// диагностики, не ускорив сборку.
#[derive(Debug, Clone)]
pub struct Downloader {
    agent: ureq::Agent,
}

impl Downloader {
    /// Создаёт загрузчик с таймаутом на установку соединения.
    ///
    /// Ограничен только connect: rootfs-архивы занимают десятки мегабайт, и
    /// глобальный таймаут обрывал бы корректную загрузку на медленном канале.
    pub fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(30)))
            .user_agent(concat!("platinum-os-one/", env!("CARGO_PKG_VERSION")))
            .build();

        Self {
            agent: config.into(),
        }
    }

    /// Возвращает путь к проверенному файлу артефакта в каталоге загрузок.
    ///
    /// Существующий файл переиспользуется только после совпадения SHA-256.
    /// Доверие имени файла сделало бы прерванную загрузку незаметным источником
    /// повреждённого rootfs.
    pub fn fetch(
        &self,
        artifact: &Artifact,
        downloads_dir: &Path,
    ) -> Result<PathBuf, DownloadError> {
        let file_name = artifact
            .file_name()
            .map_err(|source| DownloadError::Artifact { source })?;
        let target = downloads_dir.join(file_name);

        fs::create_dir_all(downloads_dir).map_err(|source| DownloadError::PrepareDirectory {
            path: downloads_dir.to_path_buf(),
            source,
        })?;

        if target.is_file() {
            let actual = sha256_of_file(&target).map_err(|source| DownloadError::Hash {
                path: target.clone(),
                source,
            })?;

            if actual == artifact.sha256 {
                info!(path = %target.display(), "artifact reused from cache");
                return Ok(target);
            }

            warn!(
                path = %target.display(),
                expected = %artifact.sha256,
                actual = %actual,
                "cached artifact has a different checksum, re-downloading"
            );
        }

        let temporary = temporary_path(&target);
        let downloaded = self.download_to_temporary(artifact, &temporary)?;

        if downloaded != artifact.sha256 {
            // Повреждённый файл удаляется сразу: иначе следующий запуск нашёл бы
            // его как незавершённую загрузку и снова потратил бы время на разбор.
            let _ = fs::remove_file(&temporary);

            return Err(DownloadError::ChecksumMismatch {
                url: artifact.url.clone(),
                expected: artifact.sha256.clone(),
                actual: downloaded,
            });
        }

        fs::rename(&temporary, &target).map_err(|source| DownloadError::Write {
            path: target.clone(),
            source,
        })?;

        info!(path = %target.display(), "artifact downloaded");

        Ok(target)
    }

    /// Скачивает тело ответа во временный файл и возвращает его SHA-256.
    ///
    /// Хеш считается во время записи: второй проход по файлу в десятки
    /// мегабайт удвоил бы дисковый ввод-вывод без пользы.
    fn download_to_temporary(
        &self,
        artifact: &Artifact,
        temporary: &Path,
    ) -> Result<String, DownloadError> {
        let mut response = self
            .agent
            .get(&artifact.url)
            // Прозрачная распаковка изменила бы байты ответа, и SHA-256 архива
            // перестал бы совпадать с опубликованным в SHA256SUMS.
            .header("accept-encoding", "identity")
            .call()
            .map_err(|source| DownloadError::Transport {
                url: artifact.url.clone(),
                source: Box::new(source),
            })?;

        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(DownloadError::UnexpectedStatus {
                url: artifact.url.clone(),
                status,
            });
        }

        let mut file = fs::File::create(temporary).map_err(|source| DownloadError::Write {
            path: temporary.to_path_buf(),
            source,
        })?;
        let mut reader = response.body_mut().as_reader();

        let digest =
            copy_and_hash(&mut reader, &mut file).map_err(|source| DownloadError::Write {
                path: temporary.to_path_buf(),
                source,
            })?;

        file.flush().map_err(|source| DownloadError::Write {
            path: temporary.to_path_buf(),
            source,
        })?;

        Ok(digest)
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}

/// Возвращает имя частичной загрузки рядом с целевым файлом.
///
/// Файл остаётся в каталоге загрузок, а не в системном temp: переименование
/// между разными файловыми системами не атомарно и может завершиться ошибкой.
fn temporary_path(target: &Path) -> PathBuf {
    let mut name = target.as_os_str().to_os_string();
    name.push(".part");

    PathBuf::from(name)
}

/// Копирует поток в файл и возвращает SHA-256 записанных байт.
fn copy_and_hash(reader: &mut impl Read, writer: &mut impl Write) -> io::Result<String> {
    let mut hasher = hash::Hasher::new();
    let mut buffer = vec![0_u8; 64 * 1024];

    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }

        hasher.update(&buffer[..read]);
        writer.write_all(&buffer[..read])?;
    }

    Ok(hasher.finish())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{Artifact, Downloader, sha256_of_file};

    /// SHA-256 строки `abc` из RFC 6234: фиксированное значение, не зависящее
    /// от реализации хеша.
    const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn temporary_directory(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "platinum-downloader-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("системное время должно быть позже Unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&path).expect("временный каталог должен создаваться");

        path
    }

    #[test]
    fn hashes_a_file_with_a_known_digest() {
        let directory = temporary_directory("hash");
        let file = directory.join("abc.txt");
        fs::write(&file, b"abc").expect("тестовый файл должен записываться");

        let digest = sha256_of_file(&file).expect("файл должен читаться");

        assert_eq!(digest, ABC_SHA256);

        fs::remove_dir_all(directory).expect("временный каталог должен удаляться");
    }

    #[test]
    fn reuses_a_cached_artifact_without_network_access() {
        let directory = temporary_directory("cache");
        fs::write(directory.join("abc.txt"), b"abc").expect("тестовый файл должен записываться");

        // URL заведомо неразрешим: успех теста доказывает, что кеш проверяется
        // раньше любого сетевого запроса.
        let artifact = Artifact::new("https://invalid.test/abc.txt".into(), ABC_SHA256.to_owned())
            .expect("артефакт должен быть корректным");

        let path = Downloader::new()
            .fetch(&artifact, &directory)
            .expect("кешированный артефакт должен переиспользоваться");

        assert_eq!(path, directory.join("abc.txt"));

        fs::remove_dir_all(directory).expect("временный каталог должен удаляться");
    }
}
