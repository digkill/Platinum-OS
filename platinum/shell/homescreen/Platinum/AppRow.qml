import QtQuick

// Строка списка: название, пояснение и значение справа.
//
// Перенос `app_row` из `apps/mod.rs`. Нажатие необязательно: строка состояния
// устройства ведёт себя как надпись, строка контакта — как кнопка.
GlassPanel {
    id: row

    property string title: ""
    property string subtitle: ""
    property string trailing: ""
    /// Нажимаемая строка подсвечивается и шлёт `activated`.
    property bool interactive: false
    property bool active: false

    signal activated()

    implicitHeight: Math.max(copy.height, trailingLabel.height) + Theme.spacingMedium * 2
    height: implicitHeight
    radius: 18
    strong: active

    scale: press.pressed ? 0.98 : 1.0
    Behavior on scale { NumberAnimation { duration: 90 } }

    Column {
        id: copy

        x: Theme.spacingMedium
        y: Theme.spacingMedium
        // Значение справа не должно наезжать на пояснение, поэтому ширина
        // текста считается по фактическому месту, оставшемуся от него.
        width: row.width - Theme.spacingMedium * 3 - trailingLabel.width
        spacing: 2

        Text {
            width: parent.width
            visible: text !== ""
            text: row.title
            font.pixelSize: 15
            font.weight: Font.DemiBold
            color: Theme.textPrimary
            elide: Text.ElideRight
        }

        Text {
            width: parent.width
            visible: text !== ""
            text: row.subtitle
            font.pixelSize: 13
            color: Theme.textSecondary
            wrapMode: Text.WordWrap
        }
    }

    Text {
        id: trailingLabel

        anchors.right: parent.right
        anchors.rightMargin: Theme.spacingMedium
        anchors.verticalCenter: parent.verticalCenter
        text: row.trailing
        font.pixelSize: 12
        color: Theme.textSecondary
    }

    MouseArea {
        id: press
        anchors.fill: parent
        enabled: row.interactive
        onClicked: row.activated()
    }
}
