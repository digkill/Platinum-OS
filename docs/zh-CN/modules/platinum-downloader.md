# platinum-downloader

[← 模块](README.md)

## 目的

阻塞式 HTTP 下载外部产物，并强制校验 SHA-256。仅当校验通过时才复用缓存文件。

## 公共 API

`Artifact` / `ArtifactError`、`Downloader`、`DownloadError`、`sha256_of_file`

## 依赖

`sha2`、`thiserror`、`tracing`、`ureq`
