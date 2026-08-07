# platinum-core

[← Модули](README.md)

## Назначение

Общие контракты сборки **без** знания плат, образов и Armbian.

## Публичный API

| Тип | Роль |
| --- | --- |
| `BuildPaths` | Проверенные каталоги сборки |
| `BuildPathsError` | Ошибки пустых путей |
| `BuildContext` | Пути + карта outputs |
| `MissingOutput` | Запрошенный ключ отсутствует |
| `Pipeline` | Последовательный runner stages |
| `Stage` | Trait независимого этапа |

## Заметки

- Stages получают `&mut BuildContext`, чтобы записывать результаты без clone.
- Пустые пути отклоняются сразу.
- Логи pipeline идут через `tracing` (`stage`, `duration_ms`).

## Зависимости

`anyhow`, `thiserror`, `tracing`
