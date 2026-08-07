import QtQuick
import QtWayland.Compositor

// Поверхность окон приложений: содержимое активного окна и переключатель.
//
// Каждое открытое окно — живой QML-элемент; неактивные не уничтожаются, а
// прячутся, поэтому переключение мгновенно и не перезапускает приложение.
Item {
    id: host

    // Переключатель: заголовки открытых окон и закрытие активного. Полоса
    // видна всегда, пока есть окна — без неё из приложения без своей кнопки
    // выхода можно было бы выйти только жест-баром, не закрыв его.
    GlassPanel {
        id: switcher

        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.screenMargin
        anchors.rightMargin: Theme.screenMargin
        height: 56
        radius: 18
        visible: Windows.list.length > 0

        // Переключение живёт в карусели, а не здесь: список названий в строку
        // не помещался уже на третьем приложении и ничего не показывал о том,
        // что в окне происходит.
        ActionButton {
            anchors.left: parent.left
            anchors.leftMargin: Theme.spacingSmall
            anchors.verticalCenter: parent.verticalCenter
            text: "Окна"
            onClicked: Navigation.show("switcher")
        }

        Text {
            anchors.centerIn: parent
            width: parent.width * 0.45
            horizontalAlignment: Text.AlignHCenter
            elide: Text.ElideRight
            visible: Windows.active >= 0
            text: Windows.active >= 0 && Windows.list[Windows.active] !== undefined
                  ? (Windows.list[Windows.active].toplevel.title !== ""
                     ? Windows.list[Windows.active].toplevel.title
                     : Windows.list[Windows.active].toplevel.appId)
                  : ""
            font.pixelSize: 15
            font.weight: Font.DemiBold
            color: Theme.textPrimary
        }

        ActionButton {
            anchors.right: parent.right
            anchors.rightMargin: Theme.spacingSmall
            anchors.verticalCenter: parent.verticalCenter
            text: "Закрыть"
            onClicked: {
                if (Windows.active >= 0) {
                    Windows.list[Windows.active].toplevel.sendClose();
                }
            }
        }
    }

    Item {
        id: area

        anchors.top: switcher.visible ? switcher.bottom : parent.top
        anchors.topMargin: switcher.visible ? Theme.spacingSmall : 0
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom

        Repeater {
            model: Windows.list

            ShellSurfaceItem {
                anchors.fill: parent
                shellSurface: modelData.surface
                visible: index === Windows.active

                // Окно не таскается пальцем: оно занимает сцену целиком.
                moveItem: Item {}

                onSurfaceDestroyed: Windows.remove(modelData.surface)

                // Клиенту сообщается размер сцены, а не предпочтительный его
                // собственный: окно в четверть экрана посреди пустоты выглядит
                // как рабочий стол, а не как устройство.
                onWidthChanged: host.fit(modelData.toplevel, width, height)
                onHeightChanged: host.fit(modelData.toplevel, width, height)
                Component.onCompleted: host.fit(modelData.toplevel, width, height)
            }
        }

        // Заявка отправлена, окна ещё нет: молчание выглядело бы как зависание.
        Text {
            anchors.centerIn: parent
            visible: Windows.launching && Windows.list.length === 0
            text: "Запуск…"
            font.pixelSize: 18
            color: Theme.textSecondary
        }
    }

    /// Просит клиента занять размер сцены.
    function fit(toplevel, width, height) {
        if (width > 0 && height > 0) {
            toplevel.sendFullscreen(Qt.size(Math.round(width), Math.round(height)));
        }
    }
}
