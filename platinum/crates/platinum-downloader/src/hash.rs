use std::{
    fmt::Write as _,
    fs,
    io::{self, Read},
    path::Path,
};

use sha2::{Digest, Sha256};

/// Инкрементальный SHA-256 для потоковой проверки артефактов.
///
/// Обёртка скрывает конкретный crate хеширования: stages сравнивают только
/// шестнадцатеричные строки и не зависят от типов sha2.
pub(crate) struct Hasher {
    inner: Sha256,
}

impl Hasher {
    pub(crate) fn new() -> Self {
        Self {
            inner: Sha256::new(),
        }
    }

    pub(crate) fn update(&mut self, chunk: &[u8]) {
        self.inner.update(chunk);
    }

    pub(crate) fn finish(self) -> String {
        let digest = self.inner.finalize();
        let mut hex = String::with_capacity(digest.len() * 2);

        for byte in digest {
            // Форматирование в String не может завершиться ошибкой ввода-вывода,
            // поэтому результат намеренно игнорируется.
            let _ = write!(hex, "{byte:02x}");
        }

        hex
    }
}

/// Возвращает SHA-256 файла в шестнадцатеричном виде.
///
/// Файл читается блоками: rootfs-архив не помещается в память целиком на
/// небольших build-агентах.
pub fn sha256_of_file(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Hasher::new();
    let mut buffer = vec![0_u8; 64 * 1024];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }

        hasher.update(&buffer[..read]);
    }

    Ok(hasher.finish())
}
