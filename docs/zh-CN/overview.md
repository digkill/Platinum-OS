# 项目概览

[← 目录](README.md)

## Platinum OS One 是什么

Platinum OS One 同时是：

1. 面向手机、平板、PC、机器人及其他设备的**通用 Linux 平台**；
2. 用 Rust 编写的**生产级镜像构建系统**。

统一 userspace 为 **Ubuntu Base 26.04 LTS**。Platinum 软件包与配置叠加其上。
硬件支持来自板级 BSP 数据（常见来源是 pinned 的 Armbian Build checkout），
**不会**用完整 Armbian 镜像替换 Platinum rootfs。

## 设计目标

- 跨设备形态保持一套 OS。
- 板差异用 **TOML 数据**表达，而不是 engine 分支。
- 可复现：pinned Git commit、SHA-256、显式路径。
- 清晰 crate 边界，可扩展到更大代码库。
- 默认安全 Rust（workspace 级 `unsafe_code = forbid`）。

## 它不是什么

- 不是完整 fork Ubuntu 或 Armbian。
- 不是通过 `do-release-upgrade` 升级的 Orange Pi 厂商镜像。
- 不是为一块板硬编码路径的脚本集合。

## 上层栈

```text
应用 / Shell（QML homescreen、agents）
        ↓
Ubuntu Base 26.04 + Platinum 软件包
        ↓
Kernel / DTB / firmware / bootloader（板级 BSP）
        ↓
磁盘镜像（.img，MBR / 分区）
```

## 首块支持板

**Orange Pi Zero 3W** —— 不是 Orange Pi Zero 3。

| 字段 | 值 |
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

禁止使用 Zero 3 标识（`orangepizero3`、H618、`sun50i-h618-orangepi-zero3.dtb`）。

## 其他板配置

| 板 | BSP 风格 | 说明 |
| --- | --- | --- |
| `orangepi-zero3w` | Armbian vendor + raw U-Boot | 主硬件目标 |
| `raspberrypi-5` | Ubuntu `linux-image-raspi` | 无 Armbian；Pi firmware 启动 |
| `parallels-arm64` | UEFI + `linux-image-generic` | VM / Shell 开发 |

## 仓库结构

```text
PlatinumOS-One/
├── README.md                 # 默认英文
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

## 版本

Workspace 版本 `0.2.0`（Apache-2.0）。二进制名为 `platinum`（crate `platinum-cli`）。
