# Platinum OS One — 文档（中文）

[English](../en/README.md) | [Русский](../ru/README.md) | [中文](README.md)

这是 Platinum OS One 的技术文档：一套通用 Linux 平台，以及基于 Rust 的可引导镜像构建系统。

## 目录

1. [项目概览](overview.md)
2. [架构](architecture.md)
3. [构建流水线](pipeline.md)
4. [CLI 参考](cli.md)
5. [板级配置](boards.md)
6. [模块](modules/README.md)
7. [Shell / UI](shell.md)
8. [开发指南](development.md)
9. [在 Docker 中构建镜像](docker-build.md)

## 速览

| 项目 | 值 |
| --- | --- |
| 产品 | Platinum OS One |
| Userspace | Ubuntu Base 26.04 LTS + Platinum 软件包 |
| 语言 | Rust stable，edition 2024 |
| Workspace 版本 | 0.2.0 |
| 首块板 | Orange Pi Zero 3W（Allwinner A733，`sun60iw2`） |
| 其他板 | Raspberry Pi 5、Parallels arm64（UEFI） |
| 架构 | `CLI → BuildEngine → Pipeline → Stage` |

## 真相来源

出现冲突时优先顺序：

1. `platinum/boards/*/board.toml` 及相关 TOML
2. `platinum/crates/` 源码
3. `AGENTS.md` / `CLAUDE.md`
4. `dev-ai.md`
5. 本文档
