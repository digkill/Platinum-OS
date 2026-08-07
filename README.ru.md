# Platinum OS One

[English](README.md) | [Русский](README.ru.md) | [中文](README.zh-CN.md)

Platinum OS One — универсальная Linux-платформа и build-система на Rust для
создания bootable images. Базовый userspace — Ubuntu Base 26.04 LTS, поверх
которого устанавливаются собственные пакеты Platinum.

Первая поддерживаемая плата — Orange Pi Zero 3W. Архитектура не привязана к
ней: цель проекта — одна OS для смартфонов, планшетов, ПК, роботов и других
устройств с подходящим board-specific BSP.

```text
CLI → BuildEngine → Pipeline → Stage
```

Проект отделяет универсальную логику сборки от BSP конкретной платы:

- `platinum-core` содержит contracts pipeline;
- `platinum-builder` оркестрирует stages;
- `platinum/boards/<board-id>/` хранит TOML-данные платы;
- `BuildEngine` не содержит условий для Orange Pi, Raspberry Pi или иных плат.

Каждая плата поставляет только данные и BSP: загрузчик, kernel, Device Tree,
firmware и configuration. Это позволяет сохранять единый Ubuntu userspace и
набор пакетов Platinum на разных классах устройств.

## Быстрый запуск

```bash
cd platinum
cargo build
cargo test
cargo run -p platinum-cli -- version
cargo run -p platinum-cli -- help
```

Вспомогательные команды BSP платы:

```bash
cargo run -p platinum-cli -- bsp-sync boards/orangepi-zero3w/board.toml /absolute/path/to/armbian-cache
cargo run -p platinum-cli -- bsp-build-kernel boards/orangepi-zero3w/board.toml /absolute/path/to/armbian-cache
```

На текущем этапе команда `build` подготавливает явно заданные директории
сборки:

```bash
cargo run -p platinum-cli -- build <work-dir> <downloads-dir> <cache-dir> <output-dir>
```

`bsp-sync` клонирует Armbian Build только в явно переданный каталог cache и
проверяет pinned Git commit из `board.toml`.

`bsp-build-kernel` сначала выполняет такую же проверку checkout, затем запускает
Armbian target `kernel` для kernel и DTB. Команда пока не создаёт финальный
Platinum image и не подменяет Ubuntu Base rootfs Armbian-образом.

## Документация для агентов

- Правила Cursor: `AGENTS.md`, `.cursor/rules/`
- Память Claude: `CLAUDE.md`
- Оперативный статус: `dev-ai.md`
