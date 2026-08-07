# 架构

[← 目录](README.md)

## 控制流

```text
platinum CLI
    ↓
加载 BoardConfig / Packages / System / Partitions（TOML）
    ↓
BuildPaths + BuildContext
    ↓
BuildEngine::new(board, BuildOptions)
    ↓
Stage trait object 组成的 Pipeline
    ↓
每个 Stage::execute(&mut BuildContext)
    ↓
打印 context outputs（key = path）
```

独立 BSP 命令绕过完整镜像流水线，只与 `platinum-armbian-bsp` 交互：

```text
bsp-sync / bsp-build-kernel / bsp-build-uboot / bsp-artifacts
    → ArmbianCheckout + ArmbianBspRunner + BspInventory
```

## 分层职责

| 层 | 负责 | 不应负责 |
| --- | --- | --- |
| CLI | argv、日志初始化、加载 TOML | 板级分支 |
| BuildEngine | 按选项选择 stages | Orange Pi / Pi 名称 |
| Pipeline | 顺序执行与耗时日志 | 下载/打包逻辑 |
| Stage | 单一职责 | 知晓全部其他 stages |
| Board TOML | 板身份、BSP pin | Rust 代码 |
| Armbian adapter | pinned checkout + compile.sh | Platinum rootfs 内容 |

## Armbian 边界

```text
Platinum Builder
├── Ubuntu Base 26.04 + Platinum 软件包     ← OS / rootfs / image
├── platinum-armbian-bsp（仅当存在 [armbian]）
│   ├── pinned checkout（校验 origin + HEAD）
│   ├── compile.sh kernel  → kernel/DTB .deb
│   └── compile.sh uboot   → U-Boot .deb + install helper
└── 最终 Platinum .img
```

Armbian 被当作 **BSP 工厂**，不是产品 OS 镜像。禁止用完整 Armbian rootfs
替换 Ubuntu Base。

## 三类启动 / BSP

1. **Armbian + raw U-Boot** —— Zero 3W（`boot-script` / `boot.scr`）
2. **Raspberry Pi firmware** —— Raspberry Pi 5
3. **UEFI** —— Parallels arm64（ESP）

`BuildEngine` 根据 TOML 数据选择 stages，绝不使用 `if board.id == ...`。

## 共享构建状态

`BuildContext` 保存 `BuildPaths` 与命名 outputs 映射。Stages 通过它通信。
没有全局可变状态。

## 错误与日志

- 应用边界：`anyhow`
- 库：`thiserror`
- Stages 通过 tracing 记录 start / finish / duration
- 生产代码禁止 `unwrap`；workspace 禁止 `unsafe_code`

## Crate 依赖示意

```text
platinum-cli
  ├── platinum-logger
  ├── platinum-board
  ├── platinum-builder
  │     ├── platinum-core
  │     ├── platinum-board
  │     ├── platinum-downloader
  │     ├── platinum-rootfs
  │     ├── platinum-image
  │     └── platinum-armbian-bsp → platinum-board
  └── platinum-armbian-bsp / platinum-core
```
