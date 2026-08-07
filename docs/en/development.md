# Development guide

[← Index](README.md)

## Prerequisites

- Rust stable ≥ 1.85
- From `platinum/`: standard Cargo toolchain
- Full image/package builds: Linux host with root (or suitable container),
  `qemu-user-static` for foreign arch when needed
- Armbian BSP builds: Linux + network + large disk/time budget
- Shell preview tools: see `platinum/shell/tools/`

## Everyday commands

```bash
cd platinum
cargo fmt --all
cargo build
cargo test
cargo run -p platinum-cli -- version
cargo run -p platinum-cli -- help
```

## Coding standards

- Explain architecture before large changes
- Comments document **why**
- Public items have rustdoc
- `anyhow` at CLI boundary; `thiserror` in libraries
- No `unwrap` in production; no undocumented `unsafe`
- No placeholder crates
- Prefer small compiling increments
- Never invent BSP pins, URLs, or SHA-256 values

## Agent memory files

| File | Audience |
| --- | --- |
| `AGENTS.md` | Cursor agents |
| `CLAUDE.md` | Claude Code / Claude |
| `dev-ai.md` | Working status / known issues |
| `.cursor/rules/*.mdc` | Always-on / scoped Cursor rules |

## Documentation languages

| Path | Language |
| --- | --- |
| `docs/en/` | English (default) |
| `docs/ru/` | Russian |
| `docs/zh-CN/` | Simplified Chinese |

Keep the three trees in sync when behavior changes.

## Current status snapshot

Implemented: full BuildEngine pipeline for rootfs/packages/firmware/BSP/
system/boot/image; CLI build + bsp-* commands; board TOML for Zero 3W,
Raspberry Pi 5, Parallels arm64; QML homescreen; hardware boot of Zero 3W
headless path and shell work on Parallels.

Still open (see `dev-ai.md` for details): device-side uInitrd update hook,
dedicated `/boot` partition, first-boot Wi-Fi provisioning, Platinum apt repo,
and related bring-up items.
