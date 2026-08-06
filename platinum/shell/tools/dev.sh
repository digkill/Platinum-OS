#!/bin/sh
# Живая правка домашнего экрана на macOS.
#
# Открывает окно с оболочкой и следит за файлами: сохранили QML — окно
# перерисовалось. Пересобирать образ и перезапускать QEMU не нужно.
#
# Так проверяется внешний вид и поведение самого экрана. Всё, что зависит от
# системы (сессия, композитор, запуск приложений), проверяется только в QEMU
# или на плате.
set -eu

HERE=$(cd "$(dirname "$0")/.." && pwd)
SCREEN="$HERE/homescreen"
QT_BIN="${QT_BIN:-/opt/homebrew/opt/qt/bin}"

# Размер окна: по умолчанию экран устройства. PORTRAIT=0 даёт ландшафтное окно,
# чтобы проверить поворот холста, как на HDMI-мониторе.
if [ "${PORTRAIT:-1}" = "1" ]; then
    WIDTH=720; HEIGHT=1280
else
    WIDTH=1280; HEIGHT=720
fi

[ -x "$QT_BIN/qml" ] || {
    echo "dev: не найден qml в $QT_BIN — установите Qt: brew install qt" >&2
    exit 1
}

cat > /tmp/platinum-dev.qml <<EOF
import QtQuick

// Обёртка для разработки: задаёт размер окна и грузит экран из файла.
Item {
    width: $WIDTH
    height: $HEIGHT

    Loader {
        anchors.fill: parent
        source: "file://$SCREEN/Home.qml"
    }
}
EOF

echo "Окно: ${WIDTH}x${HEIGHT}. Правьте файлы в $SCREEN — окно обновится само."
echo "Закрыть: Ctrl-C."

# qmlpreview запускает приложение сам и следит за деревом QML: изменённый файл
# перезагружается в уже открытом окне, без перезапуска.
exec "$QT_BIN/qmlpreview" "$QT_BIN/qml" -I "$SCREEN" /tmp/platinum-dev.qml
