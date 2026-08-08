#!/bin/sh
# Готовит значки оболочки из исходников.
#
# Исходник — один квадратный PNG на приложение, 1024x1024, с готовым фоном и
# скруглением: плитка и есть значок, подложку оболочка под него не рисует.
#
# На выходе три размера рядом друг с другом:
#
#   icons/<имя>.png      128 px   обычный экран
#   icons/<имя>@2x.png   256 px   плотность 2
#   icons/<имя>@3x.png   384 px   плотность 3
#
# Суффиксы `@2x`/`@3x` — не наша выдумка: Qt подставляет их сам по плотности
# экрана, ровно как iOS. В QML остаётся один путь без суффикса.
#
# Используется `sips` из macOS: отдельный ImageMagick ради изменения размера
# ставить незачем.
#
#   make-icons.sh                 — все исходники
#   make-icons.sh calendar clock  — только названные
set -eu

HERE=$(cd "$(dirname "$0")/.." && pwd)
SRC="${SRC:-$HERE/icons-src}"
OUT="${OUT:-$HERE/homescreen/icons}"

# 128 — размер плитки Theme.iconSize. Остальное кратно ему.
BASE=128

command -v sips > /dev/null 2>&1 || {
    echo "make-icons: нужен sips (входит в macOS)" >&2
    exit 1
}

[ -d "$SRC" ] || {
    echo "make-icons: нет каталога исходников $SRC" >&2
    echo "положите туда <имя>.png размером 1024x1024" >&2
    exit 1
}

mkdir -p "$OUT"

if [ "$#" -gt 0 ]; then
    names="$*"
else
    names=$(ls "$SRC"/*.png 2> /dev/null | while read -r f; do basename "$f" .png; done)
fi

[ -n "$names" ] || { echo "make-icons: исходников не найдено в $SRC" >&2; exit 1; }

for name in $names; do
    source_file="$SRC/$name.png"
    [ -f "$source_file" ] || { echo "make-icons: нет $source_file" >&2; continue; }

    # Квадрат обязателен: sips растянул бы прямоугольник, и значок поехал бы.
    width=$(sips -g pixelWidth "$source_file" | awk '/pixelWidth/ {print $2}')
    height=$(sips -g pixelHeight "$source_file" | awk '/pixelHeight/ {print $2}')
    if [ "$width" != "$height" ]; then
        echo "make-icons: $name.png не квадратный (${width}x${height}), пропущен" >&2
        continue
    fi

    sips -Z "$BASE"            "$source_file" --out "$OUT/$name.png"     > /dev/null
    sips -Z $((BASE * 2))      "$source_file" --out "$OUT/$name@2x.png"  > /dev/null
    sips -Z $((BASE * 3))      "$source_file" --out "$OUT/$name@3x.png"  > /dev/null

    echo "$name: ${BASE}, $((BASE * 2)), $((BASE * 3)) px"
done

echo "Готово. Значки лежат в $OUT"
