# platinum-builder

[← Модули](README.md)

## Назначение

Оркестрация стандартного pipeline Platinum. Состав stages выбирается из данных
платы и `BuildOptions` без условий по имени платы.

## Публичный API (верхний уровень)

- `BuildEngine`, `BuildOptions`
- Stages: prepare, download/unpack rootfs, packages, firmware, BSP*,
  configure-system/boot, build-image
- Helpers для specs/layouts
- Ключи `outputs`

## Структура исходников

`engine.rs`, `prepare.rs`, `rootfs.rs`, `bsp.rs`, `firmware.rs`, `system.rs`,
`boot.rs`, `image.rs`, `outputs.rs`

## Зависимости

`anyhow`, `tracing`, `platinum-core`, `platinum-board`, `platinum-downloader`,
`platinum-rootfs`, `platinum-image`, `platinum-armbian-bsp`

См. [Конвейер](../pipeline.md).
