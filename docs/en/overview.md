# Project overview

[← Index](README.md)

## What Platinum OS One is

Platinum OS One is both:

1. A **universal Linux platform** aimed at phones, tablets, PCs, robots, and
   other devices.
2. A **production-quality image build system** written in Rust.

The shared userspace is **Ubuntu Base 26.04 LTS**. Platinum packages and
configuration are layered on top. Hardware support comes from board-specific
BSP data — often produced through a pinned Armbian Build checkout — without
replacing the Platinum rootfs with a full Armbian image.

## Design goals

- One OS across device classes.
- Board differences expressed as **TOML data**, not engine branches.
- Reproducible builds: pinned Git commits, SHA-256 artifacts, explicit paths.
- Clear crate boundaries that can scale past 100k lines of code.
- Safe Rust by default (`unsafe_code = forbid` at workspace level).

## What it is not

- Not a fork of Ubuntu or Armbian as a whole.
- Not a vendor Orange Pi image upgraded with `do-release-upgrade`.
- Not a board-specific script with hardcoded paths for one SBC.

## High-level stack

```text
Applications / Shell (QML homescreen, agents)
        ↓
Ubuntu Base 26.04 userspace + Platinum packages
        ↓
Kernel / DTB / firmware / bootloader (board BSP)
        ↓
Disk image (.img) with MBR / partitions
```

## First supported board

**Orange Pi Zero 3W** — not Orange Pi Zero 3.

| Field | Value |
| --- | --- |
| Platinum id | `orangepi-zero3w` |
| SoC | Allwinner A733 |
| Family | `sun60iw2` |
| Architecture | arm64 |
| RAM | 12 GiB |
| Armbian board | `orangepizero3w` |
| Kernel branch | `vendor` |
| DTB | `allwinner/sun60i-a733-orangepi-zero3w.dtb` |
| Armbian pin | `a7f3a943d30769d5657354e9660329171ca5c39d` |

Do **not** use Zero 3 identifiers (`orangepizero3`, H618,
`sun50i-h618-orangepi-zero3.dtb`).

## Other board profiles

| Board | BSP style | Notes |
| --- | --- | --- |
| `orangepi-zero3w` | Armbian vendor + raw U-Boot | Primary hardware target |
| `raspberrypi-5` | Ubuntu `linux-image-raspi` | No Armbian; Pi firmware boot |
| `parallels-arm64` | UEFI + `linux-image-generic` | VM / desktop shell development |

## Repository layout

```text
PlatinumOS-One/
├── README.md                 # English default
├── README.ru.md
├── README.zh-CN.md
├── AGENTS.md / CLAUDE.md     # Agent memory
├── docs/                     # This documentation
└── platinum/
    ├── Cargo.toml            # Rust workspace
    ├── boards/               # Board TOML data only
    ├── crates/               # Build system crates
    └── shell/                # QML homescreen + tools
```

## Versioning

Workspace package version is `0.2.0` (Apache-2.0). Binary name is `platinum`
from crate `platinum-cli`.
