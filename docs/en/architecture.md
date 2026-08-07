# Architecture

[← Index](README.md)

## Control flow

```text
platinum CLI
    ↓
load BoardConfig / Packages / System / Partitions (TOML)
    ↓
BuildPaths + BuildContext
    ↓
BuildEngine::new(board, BuildOptions)
    ↓
Pipeline of Stage trait objects
    ↓
each Stage::execute(&mut BuildContext)
    ↓
print context outputs (key = path)
```

Standalone BSP commands bypass the full image pipeline and talk only to
`platinum-armbian-bsp`:

```text
bsp-sync / bsp-build-kernel / bsp-build-uboot / bsp-artifacts
    → ArmbianCheckout + ArmbianBspRunner + BspInventory
```

## Layer responsibilities

| Layer | Owns | Must not own |
| --- | --- | --- |
| CLI | argv, logging init, loading TOML | board-specific branches |
| BuildEngine | which stages to add for options | Orange Pi / Pi names |
| Pipeline | ordered execution + timing logs | download or packaging logic |
| Stage | one responsibility | knowing all other stages |
| Board TOML | board identity, BSP pins, packages | Rust code |
| Armbian adapter | pinned checkout + compile.sh | Platinum rootfs contents |

## Armbian boundary

```text
Platinum Builder
├── Ubuntu Base 26.04 + Platinum packages     ← OS / rootfs / image
├── platinum-armbian-bsp (only if [armbian])
│   ├── pinned checkout (origin + HEAD checks)
│   ├── compile.sh kernel  → kernel/DTB .deb
│   └── compile.sh uboot   → U-Boot .deb + install helper
└── final Platinum .img
```

Armbian is treated as a **BSP factory**, not as the product OS image.
Replacing Ubuntu Base with a full Armbian rootfs is forbidden by design.

## Three boot / BSP classes

The engine is generic. Board TOML selects the boot method:

1. **Armbian + raw U-Boot** — Orange Pi Zero 3W (`boot-script` / `boot.scr`)
2. **Raspberry Pi firmware** — Raspberry Pi 5 (`raspberry-pi`, EEPROM firmware)
3. **UEFI** — Parallels arm64 (`uefi`, ESP + GRUB-style boot)

`BuildEngine` chooses stages from data (`board.armbian`, bootloader method,
partitions), never from string compares on board ids.

## Shared build state

`BuildContext` holds:

- `BuildPaths` — work, downloads, cache, output directories
- a map of named outputs written by stages (`rootfs.dir`, `bsp.kernel`,
  `image`, …)

Stages communicate through that map. They do not share global mutable state.

## Error and logging model

- Application boundary (`platinum-cli`): `anyhow`
- Libraries: `thiserror`
- Stages emit tracing events: start, finish, duration (and failure)
- Production code forbids `unwrap`; `expect` only in tests
- Workspace forbids `unsafe_code`

## Crate dependency sketch

```text
platinum-cli
  ├── platinum-logger
  ├── platinum-board
  ├── platinum-builder
  │     ├── platinum-core
  │     ├── platinum-board
  │     ├── platinum-downloader
  │     ├── platinum-rootfs
  │     ├── platinum-image
  │     └── platinum-armbian-bsp → platinum-board
  └── platinum-armbian-bsp / platinum-core (BSP commands)
```

`platinum-utils` stays free of board/filesystem/pipeline coupling.
