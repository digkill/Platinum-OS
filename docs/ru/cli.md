# Справочник CLI

[← Содержание](README.md)

Бинарник: **`platinum`** (crate `platinum-cli`).

```bash
cd platinum
cargo run -p platinum-cli -- <команда> ...
```

## Команды

### `version`

Печатает `Platinum OS One <version>`.

### `build`

```bash
cargo run -p platinum-cli -- build boards/<id>/board.toml \
  --work-dir <path> \
  --downloads-dir <path> \
  --cache-dir <path> \
  --output-dir <path> \
  [--with-bsp] \
  [--with-packages] [--packages <path>] \
  [--with-system] [--system <path>] \
  [--with-image] [--partitions <path>]
```

| Флаг | Эффект |
| --- | --- |
| `--with-bsp` | Armbian checkout, kernel, U-Boot, inventory, install |
| `--with-packages` | Пакеты из `packages.toml` рядом с board.toml |
| `--packages <path>` | Явный packages.toml (включает установку) |
| `--with-system` | Применить `system.toml` |
| `--system <path>` | Явный system.toml |
| `--with-image` | Собрать `.img` из `partitions.toml` |
| `--partitions <path>` | Явный partitions.toml |

Замечания:

- packages/system требуют Linux + root (+ qemu-user-static для чужой arch).
- `--with-bsp` тянет сеть и долгую компиляцию; по умолчанию выключен.
- Без `--with-bsp` образ может остаться без board bootloader — будет warning.

### `bsp-sync` / `bsp-build-kernel` / `bsp-build-uboot` / `bsp-artifacts`

Работают с `<board.toml> <armbian-checkout-dir>`: синхронизация pin, сборка
kernel/U-Boot через `compile.sh`, inventory уже собранных артефактов.

## Логирование

`platinum-logger` инициализирует tracing. Уровень задаётся `RUST_LOG`, по
умолчанию `info`.
