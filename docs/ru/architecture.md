# Архитектура

[← Содержание](README.md)

## Поток управления

```text
platinum CLI
    ↓
загрузка BoardConfig / Packages / System / Partitions (TOML)
    ↓
BuildPaths + BuildContext
    ↓
BuildEngine::new(board, BuildOptions)
    ↓
Pipeline из trait objects Stage
    ↓
каждый Stage::execute(&mut BuildContext)
    ↓
печать outputs контекста (key = path)
```

Отдельные BSP-команды обходят полный image pipeline и работают только с
`platinum-armbian-bsp`:

```text
bsp-sync / bsp-build-kernel / bsp-build-uboot / bsp-artifacts
    → ArmbianCheckout + ArmbianBspRunner + BspInventory
```

## Ответственность слоёв

| Слой | Владеет | Не должен владеть |
| --- | --- | --- |
| CLI | argv, логирование, загрузка TOML | board-specific ветвления |
| BuildEngine | состав stages по опциям | имена Orange Pi / Pi |
| Pipeline | порядок и timing-логи | download/packaging логика |
| Stage | одна ответственность | знание всех остальных stages |
| Board TOML | идентичность платы, pin BSP | Rust-код |
| Armbian adapter | pinned checkout + compile.sh | содержимое Platinum rootfs |

## Граница Armbian

```text
Platinum Builder
├── Ubuntu Base 26.04 + пакеты Platinum     ← OS / rootfs / image
├── platinum-armbian-bsp (только если [armbian])
│   ├── pinned checkout (проверки origin + HEAD)
│   ├── compile.sh kernel  → .deb kernel/DTB
│   └── compile.sh uboot   → .deb U-Boot + install helper
└── финальный Platinum .img
```

Armbian — **фабрика BSP**, а не продукт-OS. Подмена Ubuntu Base полным
Armbian rootfs запрещена дизайном.

## Три класса загрузки / BSP

1. **Armbian + raw U-Boot** — Zero 3W (`boot-script` / `boot.scr`)
2. **Raspberry Pi firmware** — Raspberry Pi 5
3. **UEFI** — Parallels arm64 (ESP)

`BuildEngine` выбирает stages по данным TOML, никогда по `if board.id == ...`.

## Общее состояние сборки

`BuildContext` хранит `BuildPaths` и карту named outputs. Stages общаются через
неё. Глобального mutable state нет.

## Ошибки и логирование

- Граница приложения: `anyhow`
- Библиотеки: `thiserror`
- Stages пишут tracing: start / finish / duration
- В production нет `unwrap`; workspace forbid `unsafe_code`

## Зависимости crates

```text
platinum-cli
  ├── platinum-logger
  ├── platinum-board
  ├── platinum-builder
  │     ├── platinum-core
  │     ├── platinum-board
  │     ├── platinum-downloader
  │     ├── platinum-rootfs
  │     ├── platinum-image
  │     └── platinum-armbian-bsp → platinum-board
  └── platinum-armbian-bsp / platinum-core
```
