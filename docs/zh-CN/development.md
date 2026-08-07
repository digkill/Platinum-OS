# 开发指南

[← 目录](README.md)

## 前置条件

- Rust stable ≥ 1.85
- 完整 image/package 构建：Linux + root（必要时 qemu-user-static）
- Armbian BSP：Linux + 网络 + 充足磁盘/时间
- Shell 预览：见 `platinum/shell/tools/`

## 日常命令

```bash
cd platinum
cargo fmt --all
cargo build
cargo test
cargo run -p platinum-cli -- version
cargo run -p platinum-cli -- help
```

## 编码标准

- 大改前先说明架构
- 注释写 **为什么**
- 公共项写 rustdoc
- CLI 边界用 `anyhow`；库用 `thiserror`
- 生产代码禁止 `unwrap`；禁止未说明的 `unsafe`
- 禁止 placeholder crates
- 小步提交，每步可编译
- 不要臆造 BSP pin、URL 或 SHA-256

## Agent 记忆文件

| 文件 | 受众 |
| --- | --- |
| `AGENTS.md` | Cursor |
| `CLAUDE.md` | Claude |
| `dev-ai.md` | 工作状态 |
| `.cursor/rules/*.mdc` | Cursor rules |

## 文档语言

| 路径 | 语言 |
| --- | --- |
| `docs/en/` | English（默认） |
| `docs/ru/` | 俄语 |
| `docs/zh-CN/` | 简体中文 |

行为变更时请同步三套文档树。

## 状态快照

已实现：完整 BuildEngine 流水线；CLI build + bsp-*；Zero 3W / Raspberry Pi 5 /
Parallels 的板级 TOML；QML homescreen；Zero 3W headless 启动与 Parallels 上的
Shell 工作。

仍开放（详见 `dev-ai.md`）：设备侧 uInitrd 更新钩子、独立 `/boot` 分区、
首次启动 Wi-Fi 配置、Platinum apt 仓库及相关 bring-up 事项。
