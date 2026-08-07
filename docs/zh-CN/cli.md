# CLI 参考

[← 目录](README.md)

二进制名：**`platinum`**（crate `platinum-cli`）。

```bash
cd platinum
cargo run -p platinum-cli -- <command> ...
```

## 命令

### `version`

打印 `Platinum OS One <version>`。

### `build`

```bash
cargo run -p platinum-cli -- build boards/<id>/board.toml \
  --work-dir <path> \
  --downloads-dir <path> \
  --cache-dir <path> \
  --output-dir <path> \
  [--with-bsp] \
  [--with-packages] [--packages <path>] \
  [--with-system] [--system <path>] \
  [--with-image] [--partitions <path>]
```

| 标志 | 作用 |
| --- | --- |
| `--with-bsp` | Armbian checkout、kernel、U-Boot、inventory、安装进 rootfs |
| `--with-packages` | 安装 board.toml 旁的 `packages.toml` |
| `--packages <path>` | 显式 packages.toml（同时启用安装） |
| `--with-system` | 应用 `system.toml` |
| `--system <path>` | 显式 system.toml |
| `--with-image` | 根据 `partitions.toml` 构建 `.img` |
| `--partitions <path>` | 显式 partitions.toml |

说明：

- packages/system 需要 Linux + root（跨架构还需 qemu-user-static）。
- `--with-bsp` 需要网络与长时间编译；默认关闭。
- 没有 `--with-bsp` 时镜像可能缺少板级 bootloader，构建会警告。

### `bsp-sync` / `bsp-build-kernel` / `bsp-build-uboot` / `bsp-artifacts`

参数均为 `<board.toml> <armbian-checkout-dir>`：同步 pin、通过 `compile.sh`
构建 kernel/U-Boot、盘点已有产物。

## 日志

`platinum-logger` 初始化 tracing。可用 `RUST_LOG` 覆盖，默认 `info`。
