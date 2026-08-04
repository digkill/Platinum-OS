//! Проверенный набор пакетов, устанавливаемых поверх Ubuntu Base.

use std::collections::BTreeSet;

use thiserror::Error;

/// Ошибки описания набора пакетов.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PackageError {
    /// Пустой список означал бы Ubuntu Base без Platinum userspace.
    #[error("список устанавливаемых пакетов не должен быть пустым")]
    Empty,
    /// Имя не соответствует политике Debian.
    #[error("недопустимое имя пакета `{name}`")]
    InvalidName {
        /// Отклонённое имя.
        name: String,
    },
    /// Дубликат почти всегда означает ошибку слияния конфигураций.
    #[error("пакет `{name}` указан в списке несколько раз")]
    Duplicate {
        /// Повторяющееся имя.
        name: String,
    },
}

/// Список пакетов, пригодный для передачи в apt.
///
/// Валидация имён здесь — не косметика: непроверенная строка вида `--force-yes`
/// попала бы в командную строку apt как опция, а не как имя пакета.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageSet {
    packages: Vec<String>,
}

impl PackageSet {
    /// Создаёт набор из списка имён, сохраняя порядок конфигурации.
    pub fn new(packages: Vec<String>) -> Result<Self, PackageError> {
        if packages.is_empty() {
            return Err(PackageError::Empty);
        }

        let mut seen = BTreeSet::new();
        for name in &packages {
            if !is_valid_package_name(name) {
                return Err(PackageError::InvalidName { name: name.clone() });
            }

            if !seen.insert(name.as_str()) {
                return Err(PackageError::Duplicate { name: name.clone() });
            }
        }

        Ok(Self { packages })
    }

    /// Возвращает имена пакетов в порядке объявления.
    pub fn names(&self) -> &[String] {
        &self.packages
    }
}

/// Проверяет имя по политике Debian: `[a-z0-9][a-z0-9+-.]+`.
fn is_valid_package_name(name: &str) -> bool {
    let mut characters = name.chars();

    let starts_correctly = matches!(
        characters.next(),
        Some(first) if first.is_ascii_lowercase() || first.is_ascii_digit()
    );

    starts_correctly && name.len() >= 2 && name.chars().all(is_allowed_package_character)
}

/// Сообщает, допустим ли символ внутри имени пакета.
fn is_allowed_package_character(character: char) -> bool {
    character.is_ascii_lowercase()
        || character.is_ascii_digit()
        || matches!(character, '+' | '-' | '.')
}

#[cfg(test)]
mod tests {
    use super::{PackageError, PackageSet};

    #[test]
    fn keeps_the_declared_order_of_packages() {
        let set = PackageSet::new(vec!["systemd".into(), "sudo".into()])
            .expect("корректный список должен приниматься");

        assert_eq!(set.names(), ["systemd".to_owned(), "sudo".to_owned()]);
    }

    #[test]
    fn rejects_an_option_disguised_as_a_package() {
        let error = PackageSet::new(vec!["--force-yes".into()])
            .expect_err("опция apt не должна проходить как имя пакета");

        assert_eq!(
            error,
            PackageError::InvalidName {
                name: "--force-yes".into()
            }
        );
    }

    #[test]
    fn rejects_a_duplicated_package() {
        let error = PackageSet::new(vec!["sudo".into(), "sudo".into()])
            .expect_err("дубликат должен отклоняться");

        assert_eq!(
            error,
            PackageError::Duplicate {
                name: "sudo".into()
            }
        );
    }

    #[test]
    fn rejects_an_empty_list() {
        let error = PackageSet::new(Vec::new()).expect_err("пустой список должен отклоняться");

        assert_eq!(error, PackageError::Empty);
    }
}
