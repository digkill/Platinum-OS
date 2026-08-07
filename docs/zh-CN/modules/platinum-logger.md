# platinum-logger

[← 模块](README.md)

## 目的

在应用边界初始化全局 `tracing` subscriber。库只发事件，不安装 subscriber。

## 公共 API

`init() -> Result<(), LoggerError>` —— 优先 `RUST_LOG`，否则 `info`；若已安装
subscriber 则返回错误（不 panic）。

## 依赖

`thiserror`、`tracing-subscriber`（`env-filter`）
