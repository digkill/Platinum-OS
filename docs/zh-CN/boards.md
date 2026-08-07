# 板级配置

[← 目录](README.md)

板数据只存在于 `platinum/boards/<board-id>/`。Engine 从不硬编码板名。

## 文件

```text
boards/<id>/
├── board.toml
├── packages.toml / packages-shell.toml
├── system.toml / system-shell.toml
└── partitions.toml / partitions-shell.toml
```

由 `platinum-board` 解析，并启用 `deny_unknown_fields`。

## `board.toml` 概念

| 字段 / 段 | 作用 |
| --- | --- |
| `id`, `name` | 机器可读与人类可读标识 |
| `architecture` | 如 `arm64` |
| `soc`, `bsp_family` | 硬件身份 |
| `memory_mib` | 内存容量 |
| `dtb` | DTB 路径（UEFI/ACPI 可为空） |
| `modules` | 需要的内核模块 |
| `[rootfs]` | Ubuntu Base URL、SHA-256、发行版、架构 |
| `[bootloader]` | `extlinux` / `boot-script` / `raspberry-pi` / `uefi` |
| `[firmware]` | 可选厂商 firmware pin |
| `[armbian]` | 可选 pinned Armbian Build |

Armbian pin 规则：只能是 40 位十六进制 SHA，绝不用 `main`。

## 板目录

### `orangepi-zero3w`

A733 / `sun60iw2`，12 GiB，Armbian `orangepizero3w` / `vendor`，pin
`a7f3a943…`，DTB `allwinner/sun60i-a733-orangepi-zero3w.dtb`，`boot-script`
启动，AIC8800 firmware。

### `raspberrypi-5`

无 Armbian；内核 `linux-image-raspi`；启动方式 `raspberry-pi`。

### `parallels-arm64`

UEFI/ACPI；内核 `linux-image-generic`；ESP + root，用于 Shell 开发。

## 添加新板

1. 在 `platinum/boards/<id>/` 创建已校验 TOML。
2. 选择 bootloader method 以及是否需要 `[armbian]`。
3. 固定所有外部产物。
4. 不要在 BuildEngine 中写 `if board.id == ...`。
5. 用 `platinum-board` 测试加载 TOML。
