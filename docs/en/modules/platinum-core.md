# platinum-core

[← Modules](README.md)

## Purpose

Shared build contracts with **no** knowledge of boards, images, or Armbian.

## Public API

| Type | Role |
| --- | --- |
| `BuildPaths` | Validated work/downloads/cache/output directories |
| `BuildPathsError` | Empty-path validation errors |
| `BuildContext` | Paths + named stage outputs map |
| `MissingOutput` | Requested output key not present |
| `Pipeline` | Ordered stage runner with timing logs |
| `Stage` | Trait for independent pipeline steps |

## Design notes

- Stages borrow `&mut BuildContext` so later stages can record results without
  cloning the whole context.
- Paths must be non-empty; relative meaning depends on process CWD, so empty
  paths are rejected up front.
- Pipeline logging uses `tracing` fields `stage` and `duration_ms`.

## Dependencies

`anyhow`, `thiserror`, `tracing`
