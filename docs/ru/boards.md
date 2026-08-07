# Конфигурация плат

[← Содержание](README.md)

Данные плат лежат только в `platinum/boards/<board-id>/`. Engine не хардкодит
имена плат.

## Файлы

```text
boards/<id>/
├── board.toml
├── packages.toml / packages-shell.toml
├── system.toml / system-shell.toml
└── partitions.toml / partitions-shell.toml
```

Схемы парсит `platinum-board` с `deny_unknown_fields`.

## Идеи `board.toml`

| Поле / секция | Роль |
| --- | --- |
| `id`, `name` | Машинный и человеческий идентификатор |
| `architecture` | Например `arm64` |
| `soc`, `bsp_family` | Железная идентичность |
| `memory_mib` | Объём RAM |
| `dtb` | Путь DTB (может быть пустым для UEFI/ACPI) |
| `modules` | Нужные kernel modules |
| `[rootfs]` | URL Ubuntu Base, SHA-256, release, arch |
| `[bootloader]` | `extlinux` / `boot-script` / `raspberry-pi` / `uefi` |
| `[firmware]` | Опциональный pin vendor firmware |
| `[armbian]` | Опциональный pinned Armbian Build |

Правила pin Armbian: только 40-символьный SHA, никогда `main`.

## Каталог плат

### `orangepi-zero3w`

A733 / `sun60iw2`, 12 GiB, Armbian `orangepizero3w` / `vendor`, pin
`a7f3a943…`, DTB `allwinner/sun60i-a733-orangepi-zero3w.dtb`, boot через
`boot-script`, firmware AIC8800.

### `raspberrypi-5`

Без Armbian; ядро `linux-image-raspi`; boot `raspberry-pi`.

### `parallels-arm64`

UEFI/ACPI; ядро `linux-image-generic`; ESP + root для разработки shell.

## Добавление новой платы

1. Создать TOML в `platinum/boards/<id>/`.
2. Выбрать bootloader method и нужен ли `[armbian]`.
3. Запинить все внешние артефакты.
4. Не добавлять `if board.id == ...` в BuildEngine.
5. Покрыть загрузку TOML тестами `platinum-board`.
