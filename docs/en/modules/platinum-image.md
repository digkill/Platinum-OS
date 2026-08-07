# platinum-image

[← Modules](README.md)

## Purpose

Assemble a bootable disk image: partition layout, MBR writing in Rust,
filesystem creation, optional raw U-Boot install.

## Public API

- `ImageLayout`, `PartitionSpec`, `Filesystem`, `LayoutError`
- `ImageBuilder`, `ImageError`
- `SECTOR_SIZE`, `SECTORS_PER_MIB`
- `render_boot_sector`
- `write_uboot`, `UbootError`

## Design notes

Layout is validated as data before destructive filesystem operations.
MBR rendering is implemented in Rust (`mbr.rs`) for testability.

## Dependencies

`thiserror`, `tracing`
