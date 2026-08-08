import QtQuick

// Шторка быстрых настроек: тянется вниз от строки состояния.
//
// Настройки, которые меняют по ходу дела, не должны требовать захода в
// приложение: экранная клавиатура мешает ровно тогда, когда ты уже что-то
// делаешь, и путь до неё обязан быть в один жест.
Item {
    id: shade

    // Открыта ли шторка. Смена значения анимирует выезд.
    property bool open: false

    // Высота содержимого: панель выезжает на неё и не больше.
    readonly property real panelHeight: content.height + Theme.spacingLarge

    // Затемнение под шторкой: без него панель сливается с домашним экраном.
    Rectangle {
        anchors.fill: parent
        visible: shade.open
        color: Qt.rgba(0, 0, 0, 0.35)

        MouseArea {
            anchors.fill: parent
            onClicked: shade.open = false
        }
    }

    GlassPanel {
        id: panel

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.spacingSmall
        anchors.rightMargin: Theme.spacingSmall
        height: shade.panelHeight
        strong: true

        // Панель висит над верхней кромкой, пока закрыта: так она не занимает
        // места и не перехватывает нажатия домашнего экрана.
        y: shade.open ? 0 : -height - Theme.spacingSmall
        Behavior on y { NumberAnimation { duration: 160; easing.type: Easing.OutCubic } }

        Column {
            id: content

            x: Theme.spacingMedium
            y: Theme.spacingMedium
            width: panel.width - Theme.spacingMedium * 2
            spacing: Theme.spacingSmall

            Text {
                text: "Экранная клавиатура"
                font.pixelSize: 15
                font.weight: Font.DemiBold
                color: Theme.textPrimary
            }

            // Переключатель режимов. Три состояния подписаны словами, а не
            // значками: «всегда» и «авто» значком не различить.
            Row {
                spacing: 6

                Repeater {
                    model: Keyboard.modes

                    Rectangle {
                        readonly property bool current: modelData.id === Keyboard.mode

                        width: (content.width - 12) / 3
                        height: 40
                        radius: 14
                        color: current ? Theme.accent : Theme.glassFill
                        border.width: 1
                        border.color: current ? Theme.accent : Theme.glassBorder

                        Text {
                            anchors.centerIn: parent
                            text: modelData.title
                            font.pixelSize: 14
                            font.weight: current ? Font.DemiBold : Font.Normal
                            color: current ? "#ffffff" : Theme.textPrimary
                        }

                        MouseArea {
                            anchors.fill: parent
                            onClicked: Keyboard.setMode(modelData.id)
                        }
                    }
                }
            }

            Text {
                width: content.width
                wrapMode: Text.WordWrap
                font.pixelSize: 12
                color: Theme.textSecondary
                text: Keyboard.mode === "auto"
                      ? "Появляется, когда поле просит ввод."
                      : (Keyboard.mode === "always"
                         ? "Всегда на экране: удобно с окнами приложений."
                         : "Отключена: нужна внешняя клавиатура.")
            }
        }

        // Язычок внизу панели: за него шторку закрывают, и он же показывает,
        // что панель тянется, а не появилась сама по себе.
        Rectangle {
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 6
            width: 44
            height: 4
            radius: 2
            color: Theme.textSecondary

            MouseArea {
                anchors.fill: parent
                anchors.margins: -14
                onClicked: shade.open = false
            }
        }
    }
}
