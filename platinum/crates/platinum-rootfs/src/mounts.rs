//! Проверка, что внутри каталога ничего не смонтировано.
//!
//! Нужна перед любым удалением rootfs. Сборка монтирует внутрь него `/proc`,
//! `/sys` и **bind `/dev` хоста**; штатно всё снимается при завершении сессии
//! chroot, но прерванная сборка оставляет монтирования висеть.
//!
//! Удаление такого каталога проходит по bind-монтированию и стирает `/dev`
//! самого хоста. Это не гипотеза: 2026-08-06 так был выведен из строя сервер
//! сборки — sshd перестал стартовать, помогла только перезагрузка.

use std::{fs, io, path::Path};

use thiserror::Error;

/// Таблица монтирований ядра.
const MOUNTINFO: &str = "/proc/self/mountinfo";

/// Ошибки проверки монтирований.
#[derive(Debug, Error)]
pub enum MountError {
    /// Таблицу монтирований не удалось прочитать.
    #[error("не удалось прочитать `{MOUNTINFO}`: {source}")]
    Read {
        /// Исходная ошибка ввода-вывода.
        #[source]
        source: io::Error,
    },
    /// Внутри каталога остались монтирования.
    #[error(
        "внутри `{path}` осталось смонтировано: {mounts}; \
         удаление прошло бы по bind-монтированию и повредило хост — \
         снимите их и повторите"
    )]
    StillMounted {
        /// Проверявшийся каталог.
        path: String,
        /// Перечень точек монтирования.
        mounts: String,
    },
}

/// Возвращает точки монтирования внутри каталога.
///
/// Сравнение идёт по строкам пути, а не по устройствам: важно ровно то, что
/// увидит рекурсивное удаление.
pub fn mounts_under(path: &Path) -> Result<Vec<String>, MountError> {
    let table = match fs::read_to_string(MOUNTINFO) {
        Ok(table) => table,
        // На системах без procfs проверять нечего: chroot там всё равно
        // невозможен, а отказ ломал бы сборку на macOS.
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(MountError::Read { source }),
    };

    let prefix = format!("{}/", path.display());
    let mut found = Vec::new();

    for line in table.lines() {
        // Поле 5 mountinfo — точка монтирования.
        let Some(point) = line.split_whitespace().nth(4) else {
            continue;
        };

        if point.starts_with(&prefix) || point == path.to_string_lossy() {
            found.push(point.to_owned());
        }
    }

    found.sort();
    found.dedup();

    Ok(found)
}

/// Отказывает, если внутри каталога что-то смонтировано.
pub fn ensure_nothing_mounted(path: &Path) -> Result<(), MountError> {
    let mounts = mounts_under(path)?;
    if mounts.is_empty() {
        return Ok(());
    }

    Err(MountError::StillMounted {
        path: path.display().to_string(),
        mounts: mounts.join(", "),
    })
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use std::path::Path;

    use super::{ensure_nothing_mounted, mounts_under};

    /// Корень смонтирован всегда, поэтому проверка обязана его видеть.
    #[test]
    #[cfg(target_os = "linux")]
    fn sees_the_root_mount() {
        assert!(ensure_nothing_mounted(Path::new("/")).is_err());
    }

    /// Каталог, которого нет, ничего не содержит.
    #[test]
    fn reports_nothing_for_an_unused_path() {
        let path = std::env::temp_dir().join("platinum-mounts-absent-directory");

        assert!(
            mounts_under(&path)
                .expect("проверка должна выполняться")
                .is_empty()
        );
        assert!(ensure_nothing_mounted(&path).is_ok());
    }
}
