# 在 Docker 中构建镜像

[← 目录](README.md)

从 Rust 测试到写入存储卡的逐步手动流程。每一步都是独立命令，可以随时停下来
检查结果再继续。

## 为什么用 Docker

`install-packages`、`configure-system` 和 `build-image` 阶段需要 Linux 和
root：它们进入 chroot、挂载 `/proc` 与 `/dev` 并安装软件包。在 macOS 上这些
阶段会直接报错退出。

在 Apple Silicon 上，`arm64v8/ubuntu` 容器是**原生** arm64：目标系统的 chroot
无需 qemu，安装 userspace 只需几秒而不是半小时。其他架构的板卡则会重新用到
qemu-user-static，并承担它的全部开销。

## 环境要求

- Docker Desktop；虚拟机磁盘**不少于 32 GB**
  （Settings → Resources → Disk image size）
- Mac 磁盘上约 10 GB，用于镜像和下载缓存
- 网络：Ubuntu 归档、GitHub、Armbian 构件缓存

一次常规构建在容器内约占 8 GB。如果 Armbian 在缓存中找不到内核并开始编译，
则需要数十 GB 和数小时。

## 1. 目录与卷

```bash
mkdir -p ~/platinum-build/out ~/platinum-build/downloads

# Armbian checkout 放在 Docker 卷中，而不是 macOS 共享目录：后者以 noexec
# 挂载，Armbian 会拒绝在其上工作。
docker volume create platinum-armbian
```

## 2. 容器

```bash
cd ~/Projects/OS/PlatinumOS-One

docker run -it --privileged --name platinum-build \
    -v "$PWD:/repo:ro" \
    -v "$HOME/platinum-build/out:/out" \
    -v "$HOME/platinum-build/downloads:/downloads" \
    -v platinum-armbian:/armbian \
    arm64v8/ubuntu:24.04 bash
```

容器刻意不加 `--rm`：退出后可以带着已安装的一切回来：

```bash
docker start -ai platinum-build
```

仓库以只读方式挂载，因此构建既不会破坏工作副本，也不会在其中留下属于 root
的文件。

## 3. 容器内的工具

```bash
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y --no-install-recommends \
    ca-certificates curl git build-essential pkg-config libssl-dev \
    e2fsprogs dosfstools tar xz-utils openssl u-boot-tools bc \
    device-tree-compiler
```

Rust 通过 rustup 安装：Ubuntu 24.04 自带的 `cargo` 是 1.75，而本 workspace
需要 edition 2024（Rust ≥ 1.85）。

```bash
curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
. "$HOME/.cargo/env"
```

这些包的用途：`git` 用于获取 Armbian checkout **以及**板卡 Wi-Fi 固件；
`u-boot-tools` 提供生成 `boot.scr` 和 `uInitrd` 所需的 `mkimage`。

## 4. 源码

```bash
cp -a /repo/platinum /src
cd /src
export CARGO_TARGET_DIR=/build/target
```

之所以复制而不是直接在 `/repo` 中工作，是因为该挂载为只读。在 Mac 上修改后
重复执行 `cp -a /repo/platinum /src`，或只同步变化的部分：

```bash
rsync -a --delete /repo/platinum/ /src/
```

## 5. 测试与 CLI

```bash
cargo fmt --all --check
cargo test
cargo build -p platinum-cli --release
```

可执行文件叫 **`platinum`**，不是 `platinum-cli`：本 workspace 中包名与
`[[bin]]` 名称不同。

```bash
/build/target/release/platinum version
```

## 6. 用户账户

仓库中刻意不存放密码哈希：一旦进入 git，任何哈希都会变成所有设备共用的凭据。
但图形镜像必须有账户——自动登录到不存在的用户只会得到黑屏。

```bash
hash=$(openssl passwd -6 platinum)

sed -e 's|homescreen = "../../shell/homescreen"|homescreen = "/src/shell/homescreen"|' \
    -e 's|image = "../../../assets/|image = "/repo/assets/|' \
    /src/boards/orangepi-zero3w/system-shell.toml > /work-system.toml

cat >> /work-system.toml <<EOF

[[users]]
name = "platinum"
password_hash = "$hash"
groups = ["sudo", "adm", "dialout", "netdev", "video", "input", "render"]
force_password_change = false
EOF
```

路径改为绝对路径：原文件中它们相对于该文件本身，而副本位于别处。启动画面图片
同样如此——它位于 `platinum/` 之外的 `assets/`，因此直接从 `/repo` 取用。

`force_password_change = false` 仅用于开发构建。在没有键盘的设备上，强制改密
会在自动登录时直接变成死路。

## 7. 不含 BSP 的镜像——快速验证

这一步验证最容易出错的部分：软件包名称、chroot 安装、系统配置和文件系统创建，
耗时几分钟。

```bash
/build/target/release/platinum build /src/boards/orangepi-zero3w/board.toml \
    --work-dir /build/work \
    --downloads-dir /downloads \
    --cache-dir /armbian \
    --output-dir /out \
    --with-packages --packages /src/boards/orangepi-zero3w/packages-shell.toml \
    --with-system --system /work-system.toml \
    --with-image --partitions /src/boards/orangepi-zero3w/partitions-shell.toml
```

得到的镜像**没有内核和引导器**，构建会明确提示：「образ собран без загрузчика:
BSP не участвовал в сборке」。它无法启动，只用于验证 userspace。

## 8. 含 BSP 的镜像

加上 `--with-bsp` 会引入 `bsp-sync`、`bsp-kernel`、`bsp-uboot`、
`bsp-inventory`、`install-kernel`、`install-uboot` 阶段，随后 `configure-boot`
才成为可能。

```bash
/build/target/release/platinum build /src/boards/orangepi-zero3w/board.toml \
    --work-dir /build/work \
    --downloads-dir /downloads \
    --cache-dir /armbian \
    --output-dir /out \
    --with-bsp \
    --with-packages --packages /src/boards/orangepi-zero3w/packages-shell.toml \
    --with-system --system /work-system.toml \
    --with-image --partitions /src/boards/orangepi-zero3w/partitions-shell.toml
```

关键输出：

```text
[💖] artifact [ obtained from remote cache: kernel-sun60iw2-vendor 6.6.98-… ]
[💖] artifact [ obtained from remote cache: uboot-orangepizero3w-vendor 2018.07-… ]
```

这表示内核与 U-Boot 是直接下载的成品——只需几分钟。如果转而开始编译，就要做好
数小时和数十 GB 的准备。

## 9. 检查成品镜像

在同一容器内完成，无需复制文件：

```bash
IMG=/out/orangepi-zero3w.img

# 原始扇区中的引导器：8 KiB 处为 eGON.BT0，16400 KiB 处为 sunxi-package。
dd if=$IMG bs=1k skip=8     count=1 2>/dev/null | od -c | head -2
dd if=$IMG bs=1k skip=16400 count=1 2>/dev/null | od -c | head -2

start=$(fdisk -l -o Device,Start $IMG | awk '/img[0-9]/ {print $2}' | head -1)
mkdir -p /mnt/img
mount -o ro,noload,loop,offset=$((start * 512)) $IMG /mnt/img

ls /mnt/img/boot/                       # Image、dtb、boot.scr、armbianEnv.txt、uInitrd
grep User= /mnt/img/etc/sddm.conf.d/10-platinum.conf
umount /mnt/img
```

**不要**把镜像复制到容器内部：所有容器共用一个虚拟机，多出的几 GB 会拖垮同时
运行的构建。

## 10. 写入存储卡

在 macOS 上执行，而非容器内。镜像位于 `~/platinum-build/out/`。

```bash
diskutil list                      # 找到你的存储卡
diskutil unmountDisk /dev/diskN
sudo dd if=~/platinum-build/out/orangepi-zero3w.img of=/dev/rdiskN bs=4m
diskutil eject /dev/diskN
```

用 `rdiskN` 而非 `diskN`：这是原始设备，写入快得多。macOS 的 `dd` 不支持
`status=progress`，按 **Ctrl+T** 查看进度。写完后 macOS 会提示初始化「无法读取」
的磁盘——那是 ext4，请选择**推出**。

## 常见陷阱

| 现象 | 原因 |
| --- | --- |
| `Directory .tmp is mounted with 'noexec'` | Armbian checkout 落在了 macOS 共享目录，应改用 Docker 卷 |
| `platinum-cli: not found` | 可执行文件名为 `platinum` |
| `error: edition 2024 is unstable` | 系统自带 cargo 1.75，需要 rustup |
| 构建中途 `apt-get` 返回 100 | Docker 虚拟机磁盘被占满 |
| `Error waiting for container` | 构建期间 Docker Desktop 重启 |
| 关闭 BSP 仍需要 `git` | 板卡 Wi-Fi 固件同样来自仓库 |
