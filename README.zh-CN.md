# Platinum OS One

[English](README.md) | [Русский](README.ru.md) | [中文](README.zh-CN.md)

Platinum OS One 是一套通用 Linux 平台，以及基于 Rust 的可引导镜像构建系统。
统一 userspace 基于 Ubuntu Base 26.04 LTS，并在其上叠加 Platinum 自有软件包。

首个支持的开发板是 Orange Pi Zero 3W。架构并不绑定这块板：目标是一套 OS，
可覆盖手机、平板、PC、机器人以及其他具备合适板级 BSP 的设备。

```text
CLI → BuildEngine → Pipeline → Stage
```

项目将通用构建逻辑与板级 BSP 数据分离：

- `platinum-core` 负责 pipeline 契约；
- `platinum-builder` 编排各个 stage；
- `platinum/boards/<board-id>/` 存放板级 TOML 数据；
- `BuildEngine` 中不包含 Orange Pi、Raspberry Pi 或其他板卡的硬编码分支。

每块板只提供数据和 BSP 产物：bootloader、kernel、Device Tree、firmware 与
configuration。这样可以在不同设备形态上保持统一的 Ubuntu userspace 与
Platinum 软件包集合。

## 快速开始

```bash
cd platinum
cargo build
cargo test
cargo run -p platinum-cli -- version
cargo run -p platinum-cli -- help
```

板级 BSP 辅助命令：

```bash
cargo run -p platinum-cli -- bsp-sync boards/orangepi-zero3w/board.toml /absolute/path/to/armbian-cache
cargo run -p platinum-cli -- bsp-build-kernel boards/orangepi-zero3w/board.toml /absolute/path/to/armbian-cache
```

当前阶段，`build` 命令会准备显式指定的构建目录：

```bash
cargo run -p platinum-cli -- build <work-dir> <downloads-dir> <cache-dir> <output-dir>
```

`bsp-sync` 只会将 Armbian Build 克隆到你指定的 cache 目录，并校验
`board.toml` 中锁定的 Git commit。

`bsp-build-kernel` 先执行同样的 checkout 校验，再运行 Armbian 的 `kernel`
目标以构建 kernel 与 DTB。该命令目前不会生成最终 Platinum 镜像，也不会用
Armbian 镜像替换 Ubuntu Base rootfs。

## 文档

完整技术文档（默认英文，另有俄文与中文）：

- [docs/](docs/README.md)
- [English](docs/en/README.md)
- [Русский](docs/ru/README.md)
- [中文](docs/zh-CN/README.md)

## 代理文档

- Cursor 规则：`AGENTS.md`、`.cursor/rules/`
- Claude 记忆：`CLAUDE.md`
- 当前状态：`dev-ai.md`
