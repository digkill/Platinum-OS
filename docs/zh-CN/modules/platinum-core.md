# platinum-core

[← 模块](README.md)

## 目的

共享构建契约，**不**了解板、镜像或 Armbian。

## 公共 API

| 类型 | 作用 |
| --- | --- |
| `BuildPaths` | 已校验的构建目录 |
| `BuildPathsError` | 空路径错误 |
| `BuildContext` | 路径 + 命名 outputs |
| `MissingOutput` | 缺少输出键 |
| `Pipeline` | 带耗时日志的顺序执行器 |
| `Stage` | 独立步骤 trait |

## 设计说明

- Stages 借用 `&mut BuildContext`，无需整体 clone。
- 空路径会被立即拒绝。
- Pipeline 通过 `tracing` 记录 `stage` 与 `duration_ms`。

## 依赖

`anyhow`、`thiserror`、`tracing`
