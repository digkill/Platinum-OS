# Обзор проекта

[← Содержание](README.md)

## Что такое Platinum OS One

Platinum OS One — это одновременно:

1. **Универсальная Linux-платформа** для смартфонов, планшетов, ПК, роботов и
   других устройств.
2. **Production-quality система сборки образов** на Rust.

Единый userspace — **Ubuntu Base 26.04 LTS**. Пакеты и конфигурация Platinum
устанавливаются поверх него. Поддержка железа приходит из board-specific BSP
(часто через pinned checkout Armbian Build) **без** подмены Platinum rootfs
полным образом Armbian.

## Цели дизайна

- Одна OS для разных классов устройств.
- Различия плат — в **TOML-данных**, не в ветвлениях engine.
- Воспроизводимость: pinned Git commit, SHA-256, явные пути.
- Чёткие границы crates, рассчитанные на рост кодовой базы.
- Safe Rust по умолчанию (`unsafe_code = forbid` на уровне workspace).

## Чем проект не является

- Не форк Ubuntu или Armbian целиком.
- Не vendor-образ Orange Pi, обновлённый через `do-release-upgrade`.
- Не набор скриптов с захардкоженными путями под одну плату.

## Стек верхнего уровня

```text
Приложения / Shell (QML homescreen, agents)
        ↓
Ubuntu Base 26.04 + пакеты Platinum
        ↓
Kernel / DTB / firmware / bootloader (board BSP)
        ↓
Дисковый образ (.img) с MBR / разделами
```

## Первая поддерживаемая плата

**Orange Pi Zero 3W** — это не Orange Pi Zero 3.

| Поле | Значение |
| --- | --- |
| Platinum id | `orangepi-zero3w` |
| SoC | Allwinner A733 |
| Family | `sun60iw2` |
| Arch | arm64 |
| RAM | 12 GiB |
| Armbian board | `orangepizero3w` |
| Kernel branch | `vendor` |
| DTB | `allwinner/sun60i-a733-orangepi-zero3w.dtb` |
| Armbian pin | `a7f3a943d30769d5657354e9660329171ca5c39d` |

Нельзя использовать идентификаторы Zero 3 (`orangepizero3`, H618,
`sun50i-h618-orangepi-zero3.dtb`).

## Другие профили плат

| Плата | Стиль BSP | Заметки |
| --- | --- | --- |
| `orangepi-zero3w` | Armbian vendor + raw U-Boot | Основная железная цель |
| `raspberrypi-5` | Ubuntu `linux-image-raspi` | Без Armbian; firmware boot Pi |
| `parallels-arm64` | UEFI + `linux-image-generic` | VM / разработка shell |

## Структура репозитория

```text
PlatinumOS-One/
├── README.md                 # английский по умолчанию
├── README.ru.md
├── README.zh-CN.md
├── AGENTS.md / CLAUDE.md
├── docs/
└── platinum/
    ├── Cargo.toml
    ├── boards/
    ├── crates/
    └── shell/
```

## Версионирование

Версия workspace — `0.2.0` (Apache-2.0). Имя бинарника — `platinum`
(crate `platinum-cli`).
