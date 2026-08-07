# platinum-board

[← Модули](README.md)

## Назначение

Загрузка и валидация TOML конфигурации плат. Новая плата = новые данные, не
новый код engine.

## Публичные типы

`BoardConfig`, `RootfsConfig`, `ArmbianConfig`, bootloader methods,
`PackagesConfig`, `PartitionsConfig`, `SystemConfig` и связанные секции,
`FirmwareConfig`.

Неизвестные поля запрещены (`deny_unknown_fields`).

## Зависимости

`serde`, `thiserror`, `toml`

См. [Конфигурация плат](../boards.md).
