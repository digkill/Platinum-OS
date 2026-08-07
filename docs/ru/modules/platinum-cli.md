# platinum-cli

[← Модули](README.md)

## Назначение

Граница приложения Platinum OS One. Разбирает аргументы CLI, включает
логирование, загружает TOML, создаёт `BuildEngine` / Armbian helpers и печатает
результат.

## Бинарник

- Package: `platinum-cli`
- Имя бинарника: `platinum`
- Точка входа: `src/main.rs` (без lib target)

## Зависимости

`anyhow`, `clap`, `tracing`, `platinum-logger`, `platinum-board`,
`platinum-builder`, `platinum-core`, `platinum-armbian-bsp`

## Ответственность

- Владеть argv и пользовательскими ошибками (`anyhow`)
- Не кодировать board-specific ветвления
- Передавать в libraries уже проверенные конфиги

См. также: [CLI](../cli.md), [Архитектура](../architecture.md).
