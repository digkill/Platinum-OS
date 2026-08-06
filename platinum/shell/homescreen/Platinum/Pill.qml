import QtQuick

// Короткая метка-«пилюля»: состояние одним словом рядом с заголовком.
Rectangle {
    id: pill

    property string text: ""

    implicitWidth: label.implicitWidth + Theme.spacingMedium
    implicitHeight: 28
    width: implicitWidth
    height: implicitHeight
    radius: height / 2
    color: Theme.glassFill
    border.width: 1
    border.color: Theme.glassBorder
    antialiasing: true

    Text {
        id: label
        anchors.centerIn: parent
        text: pill.text
        font.pixelSize: 12
        color: Theme.textSecondary
    }
}
