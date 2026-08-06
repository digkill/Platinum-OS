#!/bin/sh
# Превращает образ Platinum OS в диск Parallels.
#
# `prl_convert` сырые образы не принимает — он рассчитан на диски других
# гипервизоров. Но диск Parallels типа `Plain` и есть сырой образ: в бандле
# `.hdd` лежит файл `.hds`, побайтово равный содержимому диска. Поэтому диск
# нужного размера создаётся штатной утилитой, а данные записываются в него.
#
# Образ подключается как ЖЁСТКИЙ ДИСК виртуальной машины, а не как установочный
# носитель: `.img` — это диск, а не ISO, и как CD он не загрузится.
set -eu

IMAGE="${1:-$HOME/Downloads/platinum-arm64-uefi.img.xz}"
TARGET="${2:-$HOME/Documents/Parallels/PlatinumOS.hdd}"

fail() {
    echo "to-parallels: $1" >&2
    exit 1
}

[ -f "$IMAGE" ] || fail "нет образа: $IMAGE"
command -v prl_disk_tool > /dev/null 2>&1 || fail "не найден prl_disk_tool — установите Parallels Desktop"

work=$(dirname "$TARGET")
mkdir -p "$work"

# Распаковка во временный файл рядом с целью: на том же томе, чтобы не упереться
# в место при копировании.
raw="$work/.platinum-raw.img"
case "$IMAGE" in
    *.xz)
        echo "Распаковка образа..."
        xz -dc "$IMAGE" > "$raw"
        ;;
    *)
        raw="$IMAGE"
        ;;
esac

bytes=$(wc -c < "$raw" | tr -d ' ')
mib=$((bytes / 1048576))
[ $((mib * 1048576)) -eq "$bytes" ] || fail "размер образа не кратен мебибайту: $bytes"

echo "Образ: $bytes байт ($mib MiB)"

rm -rf "$TARGET"
prl_disk_tool create --hdd "$TARGET" --size "${mib}M" > /dev/null

hds=$(ls "$TARGET"/*.hds 2>/dev/null | head -1)
[ -n "$hds" ] || fail "в бандле нет файла .hds — формат Parallels изменился"

hds_bytes=$(wc -c < "$hds" | tr -d ' ')
# Размеры обязаны совпасть: иначе таблица разделов укажет за пределы диска.
[ "$hds_bytes" -eq "$bytes" ] ||
    fail "размер диска ($hds_bytes) не совпал с образом ($bytes)"

echo "Запись данных в диск..."
dd if="$raw" of="$hds" bs=4m conv=notrunc 2>/dev/null

[ "$raw" = "$IMAGE" ] || rm -f "$raw"

echo
echo "Готово: $TARGET"
echo
echo "Подключить в Parallels:"
echo "  1. Configure -> Hardware -> Hard Disk -> Source: Existing image"
echo "  2. Выбрать $TARGET"
echo "  3. Boot order: этот диск первым; тип BIOS — UEFI, не Legacy"
