# platinum-logger

[← Модули](README.md)

## Назначение

Инициализация глобального tracing subscriber на границе приложения.
Libraries только эмитят события.

## Публичный API

`init() -> Result<(), LoggerError>` — использует `RUST_LOG` или `info`;
ошибка, если subscriber уже установлен (без panic).

## Зависимости

`thiserror`, `tracing-subscriber` (`env-filter`)
