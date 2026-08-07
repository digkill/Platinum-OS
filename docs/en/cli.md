# CLI reference

[← Index](README.md)

Binary name: **`platinum`** (crate `platinum-cli`).

```bash
cd platinum
cargo run -p platinum-cli -- <command> ...
```

## Commands

### `version`

Prints `Platinum OS One <version>`.

### `build`

Builds a Platinum image/rootfs for a board.

```bash
cargo run -p platinum-cli -- build boards/<id>/board.toml \
  --work-dir <path> \
  --downloads-dir <path> \
  --cache-dir <path> \
  --output-dir <path> \
  [--with-bsp] \
  [--with-packages] [--packages <path>] \
  [--with-system] [--system <path>] \
  [--with-image] [--partitions <path>]
```

| Flag | Effect |
| --- | --- |
| `--with-bsp` | Armbian checkout, kernel, U-Boot, inventory, install into rootfs |
| `--with-packages` | Install packages from `packages.toml` next to board.toml |
| `--packages <path>` | Explicit packages.toml (also enables package install) |
| `--with-system` | Apply `system.toml` next to board.toml |
| `--system <path>` | Explicit system.toml |
| `--with-image` | Build `.img` from `partitions.toml` next to board.toml |
| `--partitions <path>` | Explicit partitions.toml |

Notes:

- Package/system install needs Linux + root (and qemu-user-static for foreign
  arch). On macOS those paths fail with an explicit error.
- `--with-bsp` needs network and a long Armbian compile. Off by default.
- Without `--with-bsp`, an image may lack a board bootloader; the build warns.

### `bsp-sync`

```bash
cargo run -p platinum-cli -- bsp-sync <board.toml> <armbian-checkout-dir>
```

Clones or updates Armbian Build at the pinned commit from board TOML. Verifies
`origin` and detached `HEAD`.

### `bsp-build-kernel`

```bash
cargo run -p platinum-cli -- bsp-build-kernel <board.toml> <armbian-checkout-dir>
```

Runs `bsp-sync`, verifies HEAD again, then:

```text
./compile.sh kernel BOARD=<armbian_board> BRANCH=<kernel_branch> KERNEL_CONFIGURE=no
```

Prints discovered kernel/DTB artifacts.

### `bsp-build-uboot`

Same pattern for U-Boot via Armbian `compile.sh uboot`.

### `bsp-artifacts`

Inventories already built `.deb` / artifacts in an existing checkout without
recompiling.

## Logging

`platinum-logger` initializes `tracing-subscriber`. Override with `RUST_LOG`.
Default level is `info`.
