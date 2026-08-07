# platinum-downloader

[← Modules](README.md)

## Purpose

Blocking HTTP download of external artifacts with mandatory SHA-256 verification.
Cached files are reused only after the checksum matches.

## Public API

- `Artifact` / `ArtifactError` — URL + 64-hex SHA-256
- `Downloader` — fetch into destination path
- `DownloadError`
- `sha256_of_file`

## Design notes

Network transport is separated from artifact description so tests can validate
hashes without performing downloads.

## Dependencies

`sha2`, `thiserror`, `tracing`, `ureq`
