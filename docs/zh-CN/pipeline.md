# 构建流水线

[← 目录](README.md)

## Stage 契约

每个 stage 实现 `platinum_core::Stage`：

- `name() -> &'static str`
- `execute(&self, context: &mut BuildContext) -> Result<()>`

`Pipeline` 按序执行，记录耗时，并在首个错误处停止。

## 默认 stage 顺序

```text
prepare
→ download-rootfs
→ unpack-rootfs
→ [install-packages]
→ [install-firmware]
→ [bsp-sync → bsp-kernel → bsp-uboot → bsp-inventory
   → install-kernel → install-uboot]
→ [configure-system]
→ [configure-boot]
→ [build-image]
```

Firmware 在 BSP 软件包**之前**安装，以便内核安装时重建的 initramfs 能看到固件文件。

## Stage 一览

| Stage | 区域 | 职责 |
| --- | --- | --- |
| `prepare` | builder | 创建构建目录 |
| `download-rootfs` | builder + downloader | Ubuntu Base + SHA-256 |
| `unpack-rootfs` | builder + rootfs | 解压 rootfs |
| `install-packages` | builder + rootfs | chroot 中 apt |
| `install-firmware` | builder + rootfs | 厂商 firmware |
| `bsp-sync` | builder + armbian-bsp | 固定 Armbian checkout |
| `bsp-kernel` | builder + armbian-bsp | `compile.sh kernel` |
| `bsp-uboot` | builder + armbian-bsp | `compile.sh uboot` |
| `bsp-inventory` | builder + armbian-bsp | 发现 `.deb` |
| `install-kernel` | builder + rootfs | dpkg 安装 kernel/DTB |
| `install-uboot` | builder + rootfs | dpkg 安装 U-Boot |
| `configure-system` | builder + rootfs | `/etc`、用户、shell… |
| `configure-boot` | builder + rootfs | extlinux / boot.scr / Pi / UEFI |
| `build-image` | builder + image | MBR、mkfs、可选 raw U-Boot |

## BuildOptions

| 字段 | 含义 |
| --- | --- |
| `with_bsp` | 启用 Armbian sync/build/install |
| `packages` | packages.toml 内容 |
| `system` | system.toml 内容 |
| `partitions` | partitions.toml 内容 |
| `config_dir` | 相对路径解析基准 |

## Context outputs

Stages 使用稳定键发布路径（见 `outputs.rs`），例如 `rootfs.dir`、`bsp.kernel`、
`image`。`platinum build` 结束后 CLI 打印 `key = path`。
