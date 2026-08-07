# platinum-armbian-bsp

[← 模块](README.md)

## 目的

Pinned Armbian Build 适配器。Platinum 不复制 Armbian shell 逻辑，而是对已校验
checkout 调用官方 `compile.sh`。

## 公共 API

| 类型 | 作用 |
| --- | --- |
| `ArmbianCheckout` | clone/fetch/detach，校验 origin + HEAD |
| `ArmbianBspRunner` | 运行 `kernel` / `uboot`，编译前复查 HEAD |
| `BspInventory` | 定位已构建 `.deb` |
| `KernelArtifacts` | kernel/DTB 发现结果 |
| 错误类型 | `ArmbianBspError`、`InventoryError` |

## 安全规则

- revision 必须是 40 位十六进制 SHA。
- origin 不符的 cache 会被拒绝。
- 编译前 HEAD 必须等于 pin。
- 禁止把 Armbian rootfs/image 当作 Platinum userspace。

## 依赖

`platinum-board`、`thiserror`、`tracing`
