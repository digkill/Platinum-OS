# platinum-board

[← Modules](README.md)

## Purpose

Load and validate board configuration TOML. Adding a board changes data, not
engine code.

## Public types

- `BoardConfig`, `RootfsConfig`, `ArmbianConfig`, `BoardError`
- Bootloader: `BootloaderConfig` and method-specific configs
  (`Extlinux`, `BootScript`, `RaspberryPi`, `Uefi`)
- `PackagesConfig`, `PartitionsConfig`, `PartitionConfig`
- System: `SystemConfig`, `BootConfig`, `ShellConfig`, `SplashConfig`,
  `CloudInitConfig`, `UserConfig`, `NetworkConfig`, `WifiConfig`,
  `FilesystemConfig`
- `FirmwareConfig`

Unknown fields are denied so typos fail loudly.

## Dependencies

`serde`, `thiserror`, `toml`

## Related docs

- [Board configuration](../boards.md)
