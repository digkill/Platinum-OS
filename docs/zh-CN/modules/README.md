# 模块

[← 目录](../README.md)

| Crate | 职责 |
| --- | --- |
| [platinum-cli](platinum-cli.md) | 二进制入口 `platinum` |
| [platinum-core](platinum-core.md) | Pipeline 契约 |
| [platinum-builder](platinum-builder.md) | BuildEngine 与 stages |
| [platinum-board](platinum-board.md) | TOML schema |
| [platinum-armbian-bsp](platinum-armbian-bsp.md) | Pinned Armbian 适配器 |
| [platinum-downloader](platinum-downloader.md) | HTTP + SHA-256 |
| [platinum-rootfs](platinum-rootfs.md) | 解压、chroot、配置 |
| [platinum-image](platinum-image.md) | 磁盘镜像组装 |
| [platinum-logger](platinum-logger.md) | Tracing subscriber |
| [platinum-utils](platinum-utils.md) | 纯工具函数 |

Workspace 根：`platinum/Cargo.toml`（版本 `0.2.0`，edition `2024`，
`rust-version = "1.85"`，Apache-2.0）。
