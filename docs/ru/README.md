# Platinum OS One — Документация (Русский)

[English](../en/README.md) | [Русский](README.md) | [中文](../zh-CN/README.md)

Техническая документация Platinum OS One: универсальная Linux-платформа и
build-система на Rust для bootable images.

## Содержание

1. [Обзор проекта](overview.md)
2. [Архитектура](architecture.md)
3. [Конвейер сборки](pipeline.md)
4. [Справочник CLI](cli.md)
5. [Конфигурация плат](boards.md)
6. [Модули](modules/README.md)
7. [Shell / UI](shell.md)
8. [Руководство разработчика](development.md)

## Краткие факты

| Параметр | Значение |
| --- | --- |
| Продукт | Platinum OS One |
| Userspace | Ubuntu Base 26.04 LTS + пакеты Platinum |
| Язык | Rust stable, edition 2024 |
| Версия workspace | 0.2.0 |
| Первая плата | Orange Pi Zero 3W (Allwinner A733, `sun60iw2`) |
| Другие платы | Raspberry Pi 5, Parallels arm64 (UEFI) |
| Архитектура | `CLI → BuildEngine → Pipeline → Stage` |

## Источник истины

При расхождениях приоритет:

1. `platinum/boards/*/board.toml` и связанные TOML
2. Исходники в `platinum/crates/`
3. `AGENTS.md` / `CLAUDE.md`
4. `dev-ai.md`
5. Эта документация
