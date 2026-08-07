# platinum-image

[← 模块](README.md)

## 目的

组装可引导磁盘镜像：分区布局、Rust 中的 MBR 写入、文件系统创建、可选 raw
U-Boot 安装。

## 公共 API

`ImageLayout`、`PartitionSpec`、`Filesystem`、`ImageBuilder`、
`render_boot_sector`、`write_uboot`、扇区常量、typed errors。

在破坏性文件系统操作前，先把 layout 当作数据校验。

## 依赖

`thiserror`、`tracing`
