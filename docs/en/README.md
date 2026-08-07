# Platinum OS One — Documentation (English)

[English](README.md) | [Русский](../ru/README.md) | [中文](../zh-CN/README.md)

This is the technical documentation for Platinum OS One: a universal Linux
platform and a Rust build system for bootable images.

## Contents

1. [Project overview](overview.md)
2. [Architecture](architecture.md)
3. [Build pipeline](pipeline.md)
4. [CLI reference](cli.md)
5. [Board configuration](boards.md)
6. [Modules](modules/README.md)
7. [Shell / UI](shell.md)
8. [Development guide](development.md)

## Quick facts

| Item | Value |
| --- | --- |
| Product | Platinum OS One |
| Userspace | Ubuntu Base 26.04 LTS + Platinum packages |
| Language | Rust stable, edition 2024 |
| Workspace version | 0.2.0 |
| First board | Orange Pi Zero 3W (Allwinner A733, `sun60iw2`) |
| Other boards | Raspberry Pi 5, Parallels arm64 (UEFI) |
| Architecture | `CLI → BuildEngine → Pipeline → Stage` |

## Source of truth

When documents disagree, prefer:

1. `platinum/boards/*/board.toml` and related TOML files
2. Source code under `platinum/crates/`
3. `AGENTS.md` / `CLAUDE.md`
4. `dev-ai.md`
5. This documentation
