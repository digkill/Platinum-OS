# Сборка образа в Docker

[← Содержание](README.md)

Пошаговый ручной сценарий: от тестов Rust до образа, записанного на карту.
Каждый шаг выполняется отдельной командой, чтобы можно было остановиться,
посмотреть результат и продолжить.

## Зачем Docker

Стадии `install-packages`, `configure-system` и `build-image` требуют Linux и
root: они входят в chroot, монтируют `/proc` и `/dev`, ставят пакеты. На macOS
они завершаются явной ошибкой.

На Apple Silicon контейнер `arm64v8/ubuntu` даёт **нативный** arm64: chroot
целевой системы выполняется без qemu, и установка userspace занимает секунды,
а не полчаса. Для плат другой архитектуры вернётся qemu-user-static со всеми
его накладными расходами.

## Требования

- Docker Desktop; диск виртуальной машины **не меньше 32 ГБ**
  (Settings → Resources → Disk image size)
- ~10 ГБ на диске Mac под образ и кэш загрузок
- Сеть: архив Ubuntu, GitHub, кэш артефактов Armbian

Обычная сборка занимает около 8 ГБ внутри контейнера. Если Armbian не найдёт
ядро в своём кэше и начнёт компиляцию — счёт пойдёт на десятки гигабайт и часы.

## 1. Каталоги и том

```bash
mkdir -p ~/platinum-build/out ~/platinum-build/downloads

# Checkout Armbian живёт в томе Docker, а не в общей папке с macOS: та
# монтируется с noexec, и Armbian отказывается на ней работать.
docker volume create platinum-armbian
```

## 2. Контейнер

```bash
cd ~/Projects/OS/PlatinumOS-One

docker run -it --privileged --name platinum-build \
    -v "$PWD:/repo:ro" \
    -v "$HOME/platinum-build/out:/out" \
    -v "$HOME/platinum-build/downloads:/downloads" \
    -v platinum-armbian:/armbian \
    arm64v8/ubuntu:24.04 bash
```

Контейнер намеренно без `--rm`: выйдя из него, вернуться со всем установленным
можно командой

```bash
docker start -ai platinum-build
```

Репозиторий примонтирован только на чтение — сборка не может испортить рабочую
копию и не оставит в ней файлов, принадлежащих root.

## 3. Инструменты внутри контейнера

```bash
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y --no-install-recommends \
    ca-certificates curl git build-essential pkg-config libssl-dev \
    e2fsprogs dosfstools tar xz-utils openssl u-boot-tools bc \
    device-tree-compiler
```

Rust ставится через rustup: в Ubuntu 24.04 пакетный `cargo` версии 1.75, а
workspace требует edition 2024 (Rust ≥ 1.85).

```bash
curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
. "$HOME/.cargo/env"
```

Зачем отдельные пакеты: `git` — checkout Armbian **и** firmware Wi-Fi платы;
`u-boot-tools` — `mkimage` для `boot.scr` и `uInitrd`.

## 4. Исходники

```bash
cp -a /repo/platinum /src
cd /src
export CARGO_TARGET_DIR=/build/target
```

Копия, а не работа прямо в `/repo`: он смонтирован только на чтение. После
правок на Mac повторите `cp -a /repo/platinum /src` — или скопируйте
только изменившееся:

```bash
rsync -a --delete /repo/platinum/ /src/
```

## 5. Тесты и сборка CLI

```bash
cargo fmt --all --check
cargo test
cargo build -p platinum-cli --release
```

Исполняемый файл называется **`platinum`**, а не `platinum-cli`: имя пакета и
`[[bin]]` в этом workspace различаются.

```bash
/build/target/release/platinum version
```

## 6. Учётная запись

Хешей паролей в репозитории нет намеренно: попав в git, любой из них стал бы
общим credential всех устройств. Для графического образа учётная запись
обязательна — автовход в несуществующего пользователя оставит чёрный экран.

```bash
hash=$(openssl passwd -6 platinum)

sed 's|homescreen = "../../shell/homescreen"|homescreen = "/src/shell/homescreen"|' \
    /src/boards/orangepi-zero3w/system-shell.toml > /work-system.toml

cat >> /work-system.toml <<EOF

[[users]]
name = "platinum"
password_hash = "$hash"
groups = ["sudo", "adm", "dialout", "netdev", "video", "input", "render"]
force_password_change = false
EOF
```

Путь до `homescreen` заменяется на абсолютный: в файле он задан относительно
самого файла, а копия лежит в другом месте.

`force_password_change = false` — только для сборок разработки. На устройстве
без клавиатуры требование сменить пароль упирается в тупик прямо на автовходе.

## 7. Образ без BSP — быстрая проверка

Этот шаг проверяет то, что ломается чаще всего: имена пакетов, установку в
chroot, системную конфигурацию и создание файловой системы. Занимает минуты.

```bash
/build/target/release/platinum build /src/boards/orangepi-zero3w/board.toml \
    --work-dir /build/work \
    --downloads-dir /downloads \
    --cache-dir /armbian \
    --output-dir /out \
    --with-packages --packages /src/boards/orangepi-zero3w/packages-shell.toml \
    --with-system --system /work-system.toml \
    --with-image --partitions /src/boards/orangepi-zero3w/partitions-shell.toml
```

Образ получится **без ядра и загрузчика** — сборка честно предупредит об этом
строкой «образ собран без загрузчика: BSP не участвовал в сборке». Загрузить
его нельзя, он нужен только для проверки userspace.

## 8. Образ с BSP

Добавляется `--with-bsp`: появляются стадии `bsp-sync`, `bsp-kernel`,
`bsp-uboot`, `bsp-inventory`, `install-kernel`, `install-uboot`, а следом
становится возможной `configure-boot`.

```bash
/build/target/release/platinum build /src/boards/orangepi-zero3w/board.toml \
    --work-dir /build/work \
    --downloads-dir /downloads \
    --cache-dir /armbian \
    --output-dir /out \
    --with-bsp \
    --with-packages --packages /src/boards/orangepi-zero3w/packages-shell.toml \
    --with-system --system /work-system.toml \
    --with-image --partitions /src/boards/orangepi-zero3w/partitions-shell.toml
```

Ключевые строки в выводе:

```text
[💖] artifact [ obtained from remote cache: kernel-sun60iw2-vendor 6.6.98-… ]
[💖] artifact [ obtained from remote cache: uboot-orangepizero3w-vendor 2018.07-… ]
```

Это значит, что ядро и U-Boot скачаны готовыми — минуты. Если вместо них пойдёт
компиляция, приготовьтесь к часам и десяткам гигабайт.

## 9. Проверка готового образа

Прямо в том же контейнере, без копирования файла:

```bash
IMG=/out/orangepi-zero3w.img

# Загрузчик в сырых секторах: eGON.BT0 на 8 КиБ, sunxi-package на 16400 КиБ.
dd if=$IMG bs=1k skip=8     count=1 2>/dev/null | od -c | head -2
dd if=$IMG bs=1k skip=16400 count=1 2>/dev/null | od -c | head -2

start=$(fdisk -l -o Device,Start $IMG | awk '/img[0-9]/ {print $2}' | head -1)
mkdir -p /mnt/img
mount -o ro,noload,loop,offset=$((start * 512)) $IMG /mnt/img

ls /mnt/img/boot/                       # Image, dtb, boot.scr, armbianEnv.txt, uInitrd
grep User= /mnt/img/etc/sddm.conf.d/10-platinum.conf
umount /mnt/img
```

Копировать образ внутрь контейнера **нельзя**: виртуальная машина Docker одна
на все контейнеры, и лишние гигабайты уронят параллельно идущую сборку.

## 10. Запись на карту

Выполняется на macOS, не в контейнере. Образ лежит в
`~/platinum-build/out/`.

```bash
diskutil list                      # найдите свою карту
diskutil unmountDisk /dev/diskN
sudo dd if=~/platinum-build/out/orangepi-zero3w.img of=/dev/rdiskN bs=4m
diskutil eject /dev/diskN
```

`rdiskN` вместо `diskN` — «сырое» устройство, запись в разы быстрее.
`status=progress` у macOS не поддерживается: прогресс показывает **Ctrl+T**.
После записи macOS предложит инициализировать «нечитаемый» диск — это ext4,
жмите **«Извлечь»**.

## Грабли

| Симптом | Причина |
| --- | --- |
| `Directory .tmp is mounted with 'noexec'` | Checkout Armbian оказался в общей папке с macOS. Нужен том Docker |
| `platinum-cli: not found` | Бинарь называется `platinum` |
| `error: edition 2024 is unstable` | Пакетный cargo 1.75; нужен rustup |
| `apt-get` код 100 посреди сборки | Кончился диск виртуальной машины Docker |
| `Error waiting for container` | Docker Desktop перезапустился во время сборки |
| Сборка требует `git`, хотя BSP выключен | Firmware Wi-Fi платы тоже приходит из репозитория |
