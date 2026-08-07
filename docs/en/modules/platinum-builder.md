# platinum-builder

[← Modules](README.md)

## Purpose

Orchestrates the standard Platinum build pipeline. Selects stages from board
data and `BuildOptions` without board-name conditionals.

## Public API (high level)

- `BuildEngine`, `BuildOptions`
- Stages: `PrepareStage`, `DownloadRootfsStage`, `UnpackRootfsStage`,
  `InstallPackagesStage`, `InstallFirmwareStage`, `BspSyncStage`,
  `BspKernelStage`, `BspUbootStage`, `BspInventoryStage`,
  `InstallKernelStage`, `InstallUbootStage`, `ConfigureSystemStage`,
  `ConfigureBootStage`, `BuildImageStage`
- Helpers for specs/layouts derived from TOML
- `outputs` keys for context publication

## Source layout

```text
src/
├── lib.rs
├── engine.rs
├── prepare.rs
├── rootfs.rs
├── bsp.rs
├── firmware.rs
├── system.rs
├── boot.rs
├── image.rs
└── outputs.rs
```

## Dependencies

`anyhow`, `tracing`, `platinum-core`, `platinum-board`, `platinum-downloader`,
`platinum-rootfs`, `platinum-image`, `platinum-armbian-bsp`

## Related docs

- [Pipeline](../pipeline.md)
- [Architecture](../architecture.md)
