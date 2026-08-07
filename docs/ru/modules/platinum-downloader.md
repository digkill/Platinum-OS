# platinum-downloader

[← Модули](README.md)

## Назначение

Blocking HTTP-загрузка артефактов с обязательной проверкой SHA-256. Кеш
переиспользуется только после совпадения хеша.

## Публичный API

`Artifact` / `ArtifactError`, `Downloader`, `DownloadError`, `sha256_of_file`

## Зависимости

`sha2`, `thiserror`, `tracing`, `ureq`
