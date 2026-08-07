# Board configuration

[← Index](README.md)

Board data lives only under `platinum/boards/<board-id>/`. The engine never
hardcodes board names.

## Files

Typical layout:

```text
boards/<id>/
├── board.toml              # identity, rootfs, bootloader, armbian, firmware
├── packages.toml           # headless userspace packages
├── packages-shell.toml     # graphical shell package set (optional)
├── system.toml             # hostname, users, network, boot, shell…
├── system-shell.toml       # shell-oriented system config (optional)
├── partitions.toml         # disk layout
└── partitions-shell.toml   # larger root for shell images (optional)
```

Schemas are parsed by `platinum-board` with `deny_unknown_fields`.

## `board.toml` concepts

| Section / field | Role |
| --- | --- |
| `id`, `name` | Stable machine id + human name |
| `architecture` | e.g. `arm64` |
| `soc`, `bsp_family` | Hardware identity |
| `memory_mib` | Installed RAM |
| `dtb` | Device Tree path inside BSP artifacts (may be empty for UEFI/ACPI) |
| `modules` | Kernel modules to ensure |
| `[rootfs]` | Ubuntu Base URL, SHA-256, release, architecture |
| `[bootloader]` | Method: `extlinux`, `boot-script`, `raspberry-pi`, `uefi` |
| `[firmware]` | Optional vendor firmware pin/directory |
| `[armbian]` | Optional pinned Armbian Build source |

### Armbian pin rules

When `[armbian]` is present:

- `repository` — Git URL
- `revision` — **40-character commit SHA only** (never `main`)
- `board` — Armbian board id (`orangepizero3w`, …)
- `kernel_branch` — e.g. `vendor`

### Bootloader methods

| Method | Typical board | Behavior |
| --- | --- | --- |
| `boot-script` | Zero 3W | Vendor U-Boot + `boot.scr` |
| `extlinux` | Mainline-style boards | `extlinux.conf` |
| `raspberry-pi` | Raspberry Pi 5 | Firmware partition `/boot/firmware` |
| `uefi` | Parallels arm64 | ESP + UEFI boot |

## Shared rootfs artifact

All current boards use Ubuntu Base 26.04 arm64 from Canonical with a pinned
SHA-256 in board TOML. Do not invent alternate URLs without updating both URL
and checksum together.

## Board catalog

### `orangepi-zero3w`

- SoC A733 / `sun60iw2`, 12 GiB RAM
- Armbian board `orangepizero3w`, branch `vendor`
- Pin `a7f3a943d30769d5657354e9660329171ca5c39d`
- DTB `allwinner/sun60i-a733-orangepi-zero3w.dtb`
- Boot via `boot-script` (vendor U-Boot 2018.05; extlinux not relied on)
- Firmware pin for AIC8800 Wi-Fi
- Shell variant uses larger root partition and QML homescreen packages

### `raspberrypi-5`

- No Armbian section
- Kernel from apt: `linux-image-raspi`
- Boot method `raspberry-pi`
- FAT32 firmware partition + ext4 root

### `parallels-arm64`

- UEFI / ACPI (empty DTB)
- Kernel from apt: `linux-image-generic`
- ESP + large root for shell development on macOS via Parallels

## Adding a new board

1. Create `platinum/boards/<id>/` with validated TOML.
2. Choose bootloader method and whether `[armbian]` is needed.
3. Pin every external artifact (rootfs SHA, Armbian commit, firmware commit).
4. Do **not** add `if board.id == ...` to BuildEngine.
5. Add tests that load the TOML through `platinum-board`.
