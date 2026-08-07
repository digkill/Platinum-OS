# platinum-cli

[← Modules](README.md)

## Purpose

Application boundary for Platinum OS One. Parses CLI arguments, initializes
logging, loads TOML configs, constructs `BuildEngine` / Armbian helpers, and
prints results.

## Binary

- Package: `platinum-cli`
- Binary name: `platinum`
- Entry: `src/main.rs` (no library target)

## Dependencies

`anyhow`, `clap`, `tracing`, `platinum-logger`, `platinum-board`,
`platinum-builder`, `platinum-core`, `platinum-armbian-bsp`

## Responsibilities

- Own process argv and user-facing errors (`anyhow`)
- Never encode board-specific branches
- Pass already-validated configs into libraries

## Related docs

- [CLI reference](../cli.md)
- [Architecture](../architecture.md)
