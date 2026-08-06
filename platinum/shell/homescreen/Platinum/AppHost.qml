import QtQuick

// Оболочка открытого приложения: полоса возврата и его экран.
//
// Перенос `app_host.rs`. Экран приходит загрузчиком по имени файла из реестра,
// а не собирается здесь: иначе хост знал бы про каждое приложение поимённо и
// менялся бы при добавлении любого нового.
Item {
    id: host

    readonly property var module: Apps.find(Navigation.app)

    GlassPanel {
        id: bar

        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.screenMargin
        anchors.rightMargin: Theme.screenMargin
        height: 56
        radius: 18

        ActionButton {
            anchors.left: parent.left
            anchors.leftMargin: Theme.spacingSmall
            anchors.verticalCenter: parent.verticalCenter
            text: "Назад"
            onClicked: Navigation.back()
        }

        Text {
            anchors.centerIn: parent
            text: host.module !== undefined ? host.module.title : ""
            font.pixelSize: 16
            font.weight: Font.DemiBold
            color: Theme.textPrimary
        }
    }

    Loader {
        anchors.top: bar.bottom
        anchors.topMargin: Theme.spacingSmall
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom

        // Приложение без своего экрана получает заглушку с описанием из
        // реестра: пустое место выглядело бы поломкой оболочки.
        source: host.module !== undefined && host.module.screen !== undefined
                ? host.module.screen
                : "PlaceholderScreen.qml"
    }
}
