//! Проверки окружения хоста, общие для операций над rootfs.
//!
//! Функции держатся отдельно от stages: их результат влияет на решение
//! «работать или отказаться», и такая проверка не должна дублироваться в каждом
//! месте, где выполняется привилегированная операция.

use std::process::Command;

/// Сообщает, выполняется ли процесс с правами root.
///
/// Проверка идёт через `id -u`, а не через libc: workspace запрещает `unsafe`,
/// а стоимость запуска процесса здесь несопоставима со стоимостью chroot.
pub(crate) fn is_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim() == "0")
        .unwrap_or(false)
}

/// Возвращает архитектуру хоста в терминах qemu (`aarch64`, `x86_64`).
pub(crate) fn host_qemu_architecture() -> &'static str {
    std::env::consts::ARCH
}

/// Возвращает семейство ОС хоста.
pub(crate) fn host_os() -> &'static str {
    std::env::consts::OS
}
