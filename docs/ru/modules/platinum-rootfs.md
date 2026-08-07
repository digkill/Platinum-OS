# platinum-rootfs

[← Модули](README.md)

## Назначение

Подготовка Ubuntu Base rootfs: unpack, apt/dpkg в chroot, системные файлы,
boot-конфигурация, установка shell/agents.

## Основные компоненты

`RootfsSpec` / unpacker, `Chroot`, `AptInstaller`, `DpkgInstaller`,
`FirmwareInstaller`, `SystemConfigurator`, boot configurators (extlinux /
boot.scr / Pi / UEFI), shell/splash/cloud-init helpers, agents
(`ConsoleAgent`, `LauncherAgent`, `SettingsAgent`).

Внешние утилиты (`tar`, `chroot`, `apt`, `dpkg`) вызываются явно; бизнес-
валидация остаётся в Rust-типах.

## Зависимости

`thiserror`, `tracing`
