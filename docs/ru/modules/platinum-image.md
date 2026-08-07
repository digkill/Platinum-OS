# platinum-image

[← Модули](README.md)

## Назначение

Сборка bootable дискового образа: layout разделов, MBR в Rust, создание
filesystem, опциональная запись raw U-Boot.

## Публичный API

`ImageLayout`, `PartitionSpec`, `Filesystem`, `ImageBuilder`,
`render_boot_sector`, `write_uboot`, константы секторов, typed errors.

Layout проверяется как данные до destructive операций.

## Зависимости

`thiserror`, `tracing`
