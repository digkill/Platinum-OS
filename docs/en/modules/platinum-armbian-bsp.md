# platinum-armbian-bsp

[← Modules](README.md)

## Purpose

Adapter around pinned Armbian Build. Platinum does not copy Armbian shell
logic; it runs the official `compile.sh` interface against a verified checkout.

## Public API

| Type | Role |
| --- | --- |
| `ArmbianCheckout` | Clone/fetch/detach to pinned SHA; verify origin + HEAD |
| `ArmbianBspRunner` | Run `kernel` / `uboot` targets; re-check HEAD first |
| `BspInventory` | Locate built `.deb` / artifacts |
| `KernelArtifacts` | Kernel/DTB discovery result |
| `ArmbianBspError`, `InventoryError` | Typed failures |

## Safety rules

- Revision must be a 40-character hex SHA.
- Existing cache with wrong `origin` is rejected.
- HEAD must match the configured revision before compile.
- Never treat Armbian rootfs/image as Platinum userspace.

## Dependencies

`platinum-board`, `thiserror`, `tracing`
