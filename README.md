# Platinum OS One

[English](README.md) | [Русский](README.ru.md) | [中文](README.zh-CN.md)

Platinum OS One is a universal Linux platform and a Rust build system for
bootable images. The shared userspace is Ubuntu Base 26.04 LTS, with Platinum
packages layered on top.

The first supported board is Orange Pi Zero 3W. The architecture is not tied to
that board: the goal is one OS for phones, tablets, PCs, robots, and other
devices with a suitable board-specific BSP.

```text
CLI → BuildEngine → Pipeline → Stage
```

The project keeps generic build logic separate from board BSP data:

- `platinum-core` owns the pipeline contracts;
- `platinum-builder` orchestrates stages;
- `platinum/boards/<board-id>/` stores board TOML data;
- `BuildEngine` has no Orange Pi, Raspberry Pi, or other board-specific
  branches.

Each board contributes only data and BSP artifacts: bootloader, kernel, Device
Tree, firmware, and configuration. That keeps one Ubuntu userspace and one
Platinum package set across device classes.

## Quick start

```bash
cd platinum
cargo build
cargo test
cargo run -p platinum-cli -- version
cargo run -p platinum-cli -- help
```

Board BSP helpers:

```bash
cargo run -p platinum-cli -- bsp-sync boards/orangepi-zero3w/board.toml /absolute/path/to/armbian-cache
cargo run -p platinum-cli -- bsp-build-kernel boards/orangepi-zero3w/board.toml /absolute/path/to/armbian-cache
```

The current `build` command prepares explicitly provided directories:

```bash
cargo run -p platinum-cli -- build <work-dir> <downloads-dir> <cache-dir> <output-dir>
```

`bsp-sync` clones Armbian Build into the given cache directory and verifies the
pinned Git commit from `board.toml`.

`bsp-build-kernel` performs the same checkout checks, then runs the Armbian
`kernel` target for kernel and DTB. It does not yet create the final Platinum
image and does not replace the Ubuntu Base rootfs with an Armbian image.

## Agent docs

- Cursor rules: `AGENTS.md`, `.cursor/rules/`
- Claude memory: `CLAUDE.md`
- Working status: `dev-ai.md`
