#!/bin/sh
# Локальная правка диска Parallels: секунды вместо пересборки образа.
#
# Полный цикл через сервер — сборка, сжатие, скачивание полутора гигабайт,
# конвертация — занимает десятки минут. Между тем правка оболочки или темы
# загрузки меняет несколько килобайт файлов, которые уже лежат внутри диска.
#
# Диск Parallels типа `Plain` — сырой образ, поэтому его разделы можно
# смонтировать и обновить на месте. macOS не читает ext4, поэтому работа идёт
# в контейнере Linux: на Apple Silicon он arm64 и запускает chroot целевой
# системы нативно, без эмуляции.
#
# Виртуальная машина должна быть выключена: запись в диск работающей машины
# повредит файловую систему.
#
#   dev-disk.sh shell    — обновить домашний экран
#   dev-disk.sh splash   — обновить заставку из tools/splash/{logo.png,theme.script}
#                          и пересобрать initramfs
set -eu

HERE=$(cd "$(dirname "$0")/.." && pwd)
HDD="${HDD:-$HOME/Documents/Parallels/PlatinumOS.hdd}"
WHAT="${1:-shell}"

# Кандидаты темы заставки для действия `splash`. Каталог передаётся внутрь
# целиком, чтобы правку скрипта и картинки можно было проверить одной
# загрузкой, не пересобирая образ.
SPLASH="${SPLASH:-$HERE/tools/splash}"

fail() {
    echo "dev-disk: $1" >&2
    exit 1
}

command -v docker > /dev/null 2>&1 || fail "нужен Docker: он даёт Linux с loop-устройствами"
docker info > /dev/null 2>&1 || fail "Docker не запущен"

hds=$(ls "$HDD"/*.hds 2>/dev/null | head -1)
[ -n "$hds" ] || fail "не найден диск: $HDD"

# Работающая машина держит файловую систему смонтированной, и запись снаружи
# разошлась бы с её кешем.
pgrep -f "prl_vm_app" > /dev/null 2>&1 &&
    echo "ВНИМАНИЕ: Parallels запущен. Выключите машину, иначе файловая система повредится." >&2

echo "Диск: $hds"

# Всё, что нужно передать внутрь, монтируется по путям контейнера.
mkdir -p "$SPLASH"

docker run --rm --privileged \
    -v "$hds:/disk.img" \
    -v "$HERE:/src:ro" \
    -v "$SPLASH:/splash:ro" \
    -e WHAT="$WHAT" \
    arm64v8/ubuntu:24.04 sh -c '
set -eu
apt-get update -qq > /dev/null 2>&1
apt-get install -y -qq rsync fdisk zstd > /dev/null 2>&1

# Разделы монтируются по смещению, а не через `losetup -P`.
#
# В виртуальной машине Docker Desktop нет udev, поэтому `losetup -P` создаёт
# только само loop-устройство: файлов `/dev/loopNpM` не появляется и монтировать
# нечего. Смещения читаются из таблицы разделов образа.
table=$(fdisk -l -o Device,Start,Sectors /disk.img | awk "/^\/disk\.img[0-9]/ {print \$2, \$3}")
esp_start=$(echo "$table" | sed -n 1p | cut -d" " -f1)
esp_size=$(echo "$table" | sed -n 1p | cut -d" " -f2)
root_start=$(echo "$table" | sed -n 2p | cut -d" " -f1)
root_size=$(echo "$table" | sed -n 2p | cut -d" " -f2)

[ -n "$root_start" ] || { echo "не разобрана таблица разделов" >&2; exit 1; }

# Прерванный запуск оставляет loop-устройство: оно живёт в виртуальной машине
# Docker, а не в контейнере, и переживает его завершение. Второй запуск иначе
# падает на «overlapping loop device exists».
losetup -j /disk.img | cut -d: -f1 | while read -r stale; do
    umount "$stale" 2> /dev/null || true
    losetup -d "$stale" || true
done

mkdir -p /mnt/root /mnt/esp
mount -o loop,offset=$((root_start * 512)),sizelimit=$((root_size * 512)) /disk.img /mnt/root
mount -o loop,offset=$((esp_start * 512)),sizelimit=$((esp_size * 512)) /disk.img /mnt/esp

case "$WHAT" in
    shell)
        rsync -a --delete /src/homescreen/ /mnt/root/usr/share/platinum/homescreen/
        echo "оболочка обновлена"
        ;;
    splash)
        theme=/mnt/root/usr/share/plymouth/themes/platinum
        [ -f /splash/logo.png ] && cp /splash/logo.png "$theme/logo.png"
        [ -f /splash/theme.script ] && cp /splash/theme.script "$theme/platinum.script"

        # Тема лежит и в корне, и в initramfs: без пересборки второго заставка
        # на раннем этапе останется прежней. Загрузчик читает initrd с ESP,
        # поэтому туда кладётся копия — иначе правка не доедет до загрузки.
        mount --bind /dev /mnt/root/dev
        mount -t proc proc /mnt/root/proc
        mount -t sysfs sys /mnt/root/sys
        chroot /mnt/root update-initramfs -u
        umount /mnt/root/sys /mnt/root/proc /mnt/root/dev
        cp /mnt/root/boot/initrd.img /mnt/esp/initrd.img
        echo "заставка обновлена, initramfs пересобран"
        ;;
    *)
        echo "неизвестное действие: $WHAT" >&2
        ;;
esac

sync
umount /mnt/esp /mnt/root
' || fail "обновление не выполнено"

echo "Готово. Запустите машину в Parallels."
