# platinum-builder

[← 模块](README.md)

## 目的

编排标准 Platinum 构建流水线。根据板数据与 `BuildOptions` 选择 stages，
不使用板名条件分支。

## 公共 API（高层）

- `BuildEngine`、`BuildOptions`
- Stages：prepare、download/unpack rootfs、packages、firmware、BSP*、
  configure-system/boot、build-image
- specs/layouts helpers
- `outputs` 键

## 源码布局

`engine.rs`、`prepare.rs`、`rootfs.rs`、`bsp.rs`、`firmware.rs`、`system.rs`、
`boot.rs`、`image.rs`、`outputs.rs`

## 依赖

`anyhow`、`tracing`、`platinum-core`、`platinum-board`、`platinum-downloader`、
`platinum-rootfs`、`platinum-image`、`platinum-armbian-bsp`

另见：[流水线](../pipeline.md)。
