# platinum-logger

[← Modules](README.md)

## Purpose

Initialize the global `tracing` subscriber at the application boundary.
Libraries only emit events; they do not install subscribers.

## Public API

- `init() -> Result<(), LoggerError>`
- Uses `RUST_LOG` when set, otherwise `info`
- Returns an error if a subscriber is already installed (no panic)

## Dependencies

`thiserror`, `tracing-subscriber` (with `env-filter`)
