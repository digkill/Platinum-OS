# platinum-board

[← 模块](README.md)

## 目的

加载并校验板级 TOML。新增板卡 = 新增数据，而不是改 engine 代码。

## 公共类型

`BoardConfig`、`RootfsConfig`、`ArmbianConfig`、bootloader methods、
`PackagesConfig`、`PartitionsConfig`、`SystemConfig` 及相关段、
`FirmwareConfig`。

未知字段会被拒绝（`deny_unknown_fields`）。

## 依赖

`serde`、`thiserror`、`toml`

另见：[板级配置](../boards.md)。
