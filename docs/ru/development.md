# Руководство разработчика

[← Содержание](README.md)

## Требования

- Rust stable ≥ 1.85
- Полные image/package builds: Linux + root (+ qemu-user-static)
- Armbian BSP: Linux + сеть + много места/времени
- Preview shell: `platinum/shell/tools/`

## Ежедневные команды

```bash
cd platinum
cargo fmt --all
cargo build
cargo test
cargo run -p platinum-cli -- version
cargo run -p platinum-cli -- help
```

## Стандарты кода

- Сначала архитектура, потом код
- Комментарии про **почему**
- rustdoc на public API
- `anyhow` на CLI; `thiserror` в libraries
- Нет `unwrap` в production; нет undocumented `unsafe`
- Нет placeholder crates
- Маленькие компилируемые шаги
- Не угадывать pin/URL/SHA

## Память агентов

| Файл | Аудитория |
| --- | --- |
| `AGENTS.md` | Cursor |
| `CLAUDE.md` | Claude |
| `dev-ai.md` | Оперативный статус |
| `.cursor/rules/*.mdc` | Cursor rules |

## Языки документации

| Путь | Язык |
| --- | --- |
| `docs/en/` | English (по умолчанию) |
| `docs/ru/` | Русский |
| `docs/zh-CN/` | Китайский (упрощённый) |

Держите три дерева синхронными при изменении поведения.

## Снимок статуса

Реализовано: полный BuildEngine pipeline; CLI build + bsp-*; TOML для Zero 3W,
Raspberry Pi 5, Parallels; QML homescreen; загрузка Zero 3W headless и работа
shell на Parallels.

Открыто (см. `dev-ai.md`): хук uInitrd на устройстве, отдельный `/boot`,
first-boot Wi-Fi, apt-репозиторий Platinum и связанные задачи bring-up.
