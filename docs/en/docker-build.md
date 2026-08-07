# Building an image in Docker

[← Contents](README.md)

A step-by-step manual walkthrough: from Rust tests to an image written to a
card. Every step is its own command, so you can stop, inspect the result and
continue.

## Why Docker

The `install-packages`, `configure-system` and `build-image` stages need Linux
and root: they enter a chroot, mount `/proc` and `/dev`, and install packages.
On macOS they fail with an explicit error.

On Apple Silicon the `arm64v8/ubuntu` container is **native** arm64: the target
chroot runs without qemu, so installing the userspace takes seconds instead of
half an hour. Boards of a different architecture bring qemu-user-static back,
with all of its overhead.

## Requirements

- Docker Desktop; virtual machine disk of **at least 32 GB**
  (Settings → Resources → Disk image size)
- ~10 GB on the Mac disk for the image and the download cache
- Network: Ubuntu archive, GitHub, Armbian artifact cache

A normal build uses about 8 GB inside the container. If Armbian does not find
the kernel in its cache and starts compiling, expect tens of gigabytes and
hours.

## 1. Directories and volume

```bash
mkdir -p ~/platinum-build/out ~/platinum-build/downloads

# The Armbian checkout lives in a Docker volume rather than a macOS bind mount:
# the latter is mounted noexec, and Armbian refuses to work on it.
docker volume create platinum-armbian
```

## 2. Container

```bash
cd ~/Projects/OS/PlatinumOS-One

docker run -it --privileged --name platinum-build \
    -v "$PWD:/repo:ro" \
    -v "$HOME/platinum-build/out:/out" \
    -v "$HOME/platinum-build/downloads:/downloads" \
    -v platinum-armbian:/armbian \
    arm64v8/ubuntu:24.04 bash
```

The container deliberately has no `--rm`: after leaving it, come back with
everything already installed via

```bash
docker start -ai platinum-build
```

The repository is mounted read-only, so the build cannot damage your working
copy or leave root-owned files in it.

## 3. Tools inside the container

```bash
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y --no-install-recommends \
    ca-certificates curl git build-essential pkg-config libssl-dev \
    e2fsprogs dosfstools tar xz-utils openssl u-boot-tools bc \
    device-tree-compiler
```

Rust comes from rustup: Ubuntu 24.04 ships `cargo` 1.75, while the workspace
requires edition 2024 (Rust ≥ 1.85).

```bash
curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
. "$HOME/.cargo/env"
```

Why those packages: `git` fetches the Armbian checkout **and** the board Wi-Fi
firmware; `u-boot-tools` provides `mkimage` for `boot.scr` and `uInitrd`.

## 4. Sources

```bash
cp -a /repo/platinum /src
cd /src
export CARGO_TARGET_DIR=/build/target
```

A copy rather than working in `/repo` directly, because that mount is
read-only. After editing on the Mac, repeat `cp -a /repo/platinum /src` — or
copy only what changed:

```bash
rsync -a --delete /repo/platinum/ /src/
```

## 5. Tests and the CLI

```bash
cargo fmt --all --check
cargo test
cargo build -p platinum-cli --release
```

The binary is called **`platinum`**, not `platinum-cli`: the package name and
`[[bin]]` differ in this workspace.

```bash
/build/target/release/platinum version
```

## 6. User account

Password hashes are deliberately absent from the repository: once in git, any
of them would become a shared credential for every device. A graphical image
still needs an account — autologin into a user that does not exist leaves a
black screen.

```bash
hash=$(openssl passwd -6 platinum)

sed 's|homescreen = "../../shell/homescreen"|homescreen = "/src/shell/homescreen"|' \
    /src/boards/orangepi-zero3w/system-shell.toml > /work-system.toml

cat >> /work-system.toml <<EOF

[[users]]
name = "platinum"
password_hash = "$hash"
groups = ["sudo", "adm", "dialout", "netdev", "video", "input", "render"]
force_password_change = false
EOF
```

The `homescreen` path becomes absolute: in the original file it is relative to
that file, and the copy lives elsewhere.

`force_password_change = false` is for development builds only. On a device
without a keyboard, a forced password change is a dead end right at autologin.

## 7. Image without BSP — a quick check

This step validates what breaks most often: package names, the chroot install,
system configuration and filesystem creation. It takes minutes.

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

The image comes out **without a kernel or bootloader**, and the build says so:
"образ собран без загрузчика: BSP не участвовал в сборке". It cannot boot; it
only proves the userspace.

## 8. Image with BSP

Adding `--with-bsp` brings in the `bsp-sync`, `bsp-kernel`, `bsp-uboot`,
`bsp-inventory`, `install-kernel` and `install-uboot` stages, which in turn make
`configure-boot` possible.

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

The lines that matter:

```text
[💖] artifact [ obtained from remote cache: kernel-sun60iw2-vendor 6.6.98-… ]
[💖] artifact [ obtained from remote cache: uboot-orangepizero3w-vendor 2018.07-… ]
```

They mean the kernel and U-Boot were downloaded ready-made — minutes. If a
compile starts instead, expect hours and tens of gigabytes.

## 9. Inspecting the finished image

In the same container, without copying the file:

```bash
IMG=/out/orangepi-zero3w.img

# Bootloader in raw sectors: eGON.BT0 at 8 KiB, sunxi-package at 16400 KiB.
dd if=$IMG bs=1k skip=8     count=1 2>/dev/null | od -c | head -2
dd if=$IMG bs=1k skip=16400 count=1 2>/dev/null | od -c | head -2

start=$(fdisk -l -o Device,Start $IMG | awk '/img[0-9]/ {print $2}' | head -1)
mkdir -p /mnt/img
mount -o ro,noload,loop,offset=$((start * 512)) $IMG /mnt/img

ls /mnt/img/boot/                       # Image, dtb, boot.scr, armbianEnv.txt, uInitrd
grep User= /mnt/img/etc/sddm.conf.d/10-platinum.conf
umount /mnt/img
```

Do **not** copy the image inside the container: one virtual machine backs every
container, and the extra gigabytes will kill a build running in parallel.

## 10. Writing to a card

Done on macOS, not in the container. The image is in `~/platinum-build/out/`.

```bash
diskutil list                      # find your card
diskutil unmountDisk /dev/diskN
sudo dd if=~/platinum-build/out/orangepi-zero3w.img of=/dev/rdiskN bs=4m
diskutil eject /dev/diskN
```

`rdiskN` instead of `diskN` is the raw device and writes far faster. macOS `dd`
has no `status=progress`; press **Ctrl+T** for progress. Afterwards macOS offers
to initialise the "unreadable" disk — that is ext4, choose **Eject**.

## Pitfalls

| Symptom | Cause |
| --- | --- |
| `Directory .tmp is mounted with 'noexec'` | The Armbian checkout landed on a macOS bind mount. Use a Docker volume |
| `platinum-cli: not found` | The binary is called `platinum` |
| `error: edition 2024 is unstable` | Packaged cargo 1.75; rustup is required |
| `apt-get` exit 100 mid-build | The Docker virtual machine disk filled up |
| `Error waiting for container` | Docker Desktop restarted during the build |
| The build needs `git` even with BSP off | Board Wi-Fi firmware also comes from a repository |
