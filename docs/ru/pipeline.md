# Конвейер сборки

[← Содержание](README.md)

## Контракт Stage

Каждый stage реализует `platinum_core::Stage`:

- `name() -> &'static str`
- `execute(&self, context: &mut BuildContext) -> Result<()>`

`Pipeline` выполняет stages по порядку, логирует duration и останавливается на
первой ошибке.

## Порядок stages по умолчанию

```text
prepare
→ download-rootfs
→ unpack-rootfs
→ [install-packages]
→ [install-firmware]
→ [bsp-sync → bsp-kernel → bsp-uboot → bsp-inventory
   → install-kernel → install-uboot]
→ [configure-system]
→ [configure-boot]
→ [build-image]
```

Firmware ставится **до** BSP-пакетов, чтобы initramfs при установке ядра видел
файлы firmware.

## Каталог stages

| Stage | Зона | Ответственность |
| --- | --- | --- |
| `prepare` | builder | Создание каталогов сборки |
| `download-rootfs` | builder + downloader | Ubuntu Base + SHA-256 |
| `unpack-rootfs` | builder + rootfs | Распаковка rootfs |
| `install-packages` | builder + rootfs | apt в chroot |
| `install-firmware` | builder + rootfs | Vendor firmware |
| `bsp-sync` | builder + armbian-bsp | Pin Armbian checkout |
| `bsp-kernel` | builder + armbian-bsp | `compile.sh kernel` |
| `bsp-uboot` | builder + armbian-bsp | `compile.sh uboot` |
| `bsp-inventory` | builder + armbian-bsp | Поиск `.deb` |
| `install-kernel` | builder + rootfs | dpkg kernel/DTB |
| `install-uboot` | builder + rootfs | dpkg U-Boot |
| `configure-system` | builder + rootfs | `/etc`, users, shell… |
| `configure-boot` | builder + rootfs | extlinux / boot.scr / Pi / UEFI |
| `build-image` | builder + image | MBR, mkfs, optional raw U-Boot |

## BuildOptions

| Поле | Смысл |
| --- | --- |
| `with_bsp` | Включить Armbian sync/build/install |
| `packages` | Содержимое packages.toml |
| `system` | Содержимое system.toml |
| `partitions` | Содержимое partitions.toml |
| `config_dir` | База для относительных путей |

## Outputs контекста

Stages публикуют пути под стабильными ключами (`outputs.rs`), например
`rootfs.dir`, `bsp.kernel`, `image`. После `platinum build` CLI печатает
`key = path`.
