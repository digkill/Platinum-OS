# platinum-utils

[← Modules](README.md)

## Purpose

Small pure helpers with no board, filesystem, or pipeline coupling.

## Public API

- `format_duration(Duration) -> String` — compact CLI-friendly formatting

Keep this crate free of business logic. If a helper needs I/O or board data, it
belongs elsewhere.
