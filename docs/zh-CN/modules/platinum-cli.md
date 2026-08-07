# platinum-cli

[← 模块](README.md)

## 目的

Platinum OS One 的应用边界。解析 CLI 参数、初始化日志、加载 TOML、构造
`BuildEngine` / Armbian helpers，并打印结果。

## 二进制

- Package：`platinum-cli`
- 二进制名：`platinum`
- 入口：`src/main.rs`（无 lib target）

## 依赖

`anyhow`、`clap`、`tracing`、`platinum-logger`、`platinum-board`、
`platinum-builder`、`platinum-core`、`platinum-armbian-bsp`

## 职责

- 拥有 argv 与面向用户的错误（`anyhow`）
- 不编码板级分支
- 向库传入已校验配置

另见：[CLI](../cli.md)、[架构](../architecture.md)。
