//! Малые чистые утилиты, которые можно использовать без доступа к filesystem.
//!
//! Здесь живут только функции без привязки к board, pipeline или CLI. Такое
//! ограничение не позволяет превратить utils в неявный слой бизнес-логики.

use std::time::Duration;

/// Форматирует duration для компактного вывода в сообщениях CLI.
pub fn format_duration(duration: Duration) -> String {
    if duration.as_secs() > 0 {
        format!("{}.{:03}s", duration.as_secs(), duration.subsec_millis())
    } else {
        format!("{}ms", duration.as_millis())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::format_duration;

    #[test]
    fn formats_subsecond_duration_in_milliseconds() {
        assert_eq!(format_duration(Duration::from_millis(42)), "42ms");
    }

    #[test]
    fn formats_longer_duration_in_seconds() {
        assert_eq!(format_duration(Duration::from_millis(1_234)), "1.234s");
    }
}
