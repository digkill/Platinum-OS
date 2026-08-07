# Build pipeline

[← Index](README.md)

## Stage contract

Every stage implements `platinum_core::Stage`:

- `name() -> &'static str` — stable identifier for logs / future resume
- `execute(&self, context: &mut BuildContext) -> Result<()>`

`Pipeline` runs stages in order, logs duration, and stops on the first error.

## Default stage order

Built by `BuildEngine::new(board, options)`:

```text
prepare
→ download-rootfs
→ unpack-rootfs
→ [install-packages]                 # if packages config present
→ [install-firmware]                 # if board.firmware present
→ [bsp-sync                          # if --with-bsp and board.armbian
   → bsp-kernel
   → bsp-uboot
   → bsp-inventory
   → install-kernel
   → install-uboot]
→ [configure-system]                 # if system config present
→ [configure-boot]                   # boot method from board TOML
→ [build-image]                      # if partitions config present
```

Firmware is installed **before** BSP packages so initramfs rebuild during kernel
install can see firmware files.

## Stage catalog

| Stage | Crate area | Responsibility |
| --- | --- | --- |
| `prepare` | builder | Create work/downloads/cache/output dirs |
| `download-rootfs` | builder + downloader | Fetch Ubuntu Base with SHA-256 |
| `unpack-rootfs` | builder + rootfs | Extract rootfs tarball |
| `install-packages` | builder + rootfs | Chroot apt install |
| `install-firmware` | builder + rootfs | Vendor firmware checkout/links |
| `bsp-sync` | builder + armbian-bsp | Pin Armbian checkout |
| `bsp-kernel` | builder + armbian-bsp | `compile.sh kernel` |
| `bsp-uboot` | builder + armbian-bsp | `compile.sh uboot` |
| `bsp-inventory` | builder + armbian-bsp | Discover `.deb` paths |
| `install-kernel` | builder + rootfs | dpkg kernel/DTB into rootfs |
| `install-uboot` | builder + rootfs | dpkg U-Boot into rootfs |
| `configure-system` | builder + rootfs | hostname, fstab, users, shell, … |
| `configure-boot` | builder + rootfs | extlinux / boot.scr / Pi / UEFI |
| `build-image` | builder + image | MBR layout, mkfs, optional raw U-Boot |

## BuildOptions

| Field | Meaning |
| --- | --- |
| `with_bsp` | Enable Armbian sync/build/install stages |
| `packages` | Optional packages.toml contents |
| `system` | Optional system.toml contents |
| `partitions` | Optional partitions.toml contents |
| `config_dir` | Base directory for relative paths in system config |

Options are chosen by CLI flags; the engine never invents board-specific
defaults beyond what TOML already declares.

## Context outputs

Stages publish paths under stable keys (see
`platinum-builder/src/outputs.rs`), for example:

- `rootfs.archive`, `rootfs.dir`
- `bsp.checkout`, `bsp.kernel`, `bsp.dtb`, `bsp.uboot`
- `boot.*`
- `image`

After `platinum build`, the CLI prints `key = path` for every output.
