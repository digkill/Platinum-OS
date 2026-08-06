import QtQuick

// Кнопка экрана приложения.
//
// Собрана из стеклянной панели, а не взята из QtQuick.Controls: в образ входит
// только `qml6-module-qtquick`, и импорт Controls уронил бы оболочку на
// устройстве — на macOS это не видно, потому что там установлен весь Qt.
GlassPanel {
    id: button

    property string text: ""
    /// Выбранное состояние: подсвечивает кнопку акцентом.
    property bool active: false

    signal clicked()

    // Своего `enabled` здесь нет: у Item он уже есть и сам перестаёт пускать
    // нажатия внутрь. Одноимённое свойство перекрыло бы его и разошлось бы с
    // тем, что видит движок.

    implicitWidth: label.implicitWidth + Theme.spacingLarge
    implicitHeight: 44
    width: implicitWidth
    height: implicitHeight
    radius: 14
    strong: active
    opacity: enabled ? 1.0 : 0.45

    scale: press.pressed ? 0.95 : 1.0
    Behavior on scale { NumberAnimation { duration: 90 } }

    Text {
        id: label
        anchors.centerIn: parent
        text: button.text
        font.pixelSize: 15
        color: button.active ? Theme.accent : Theme.textPrimary
    }

    MouseArea {
        id: press
        anchors.fill: parent
        onClicked: button.clicked()
    }
}
