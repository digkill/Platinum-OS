import QtQuick

// Нижний док: постоянные приложения поверх стеклянной подложки.
GlassPanel {
    id: dock

    property alias model: repeater.model

    height: 96
    radius: Theme.dockRadius

    Row {
        anchors.centerIn: parent
        spacing: (dock.width - Theme.iconSize * repeater.count - 32) / Math.max(1, repeater.count - 1)

        Repeater {
            id: repeater

            GlassPanel {
                width: Theme.iconSize
                height: Theme.iconSize
                radius: Theme.iconRadius
                strong: true

                Text {
                    anchors.centerIn: parent
                    text: modelData.glyph
                    font.pixelSize: Theme.iconSize * 0.44
                    color: modelData.color !== undefined ? modelData.color : Theme.accent
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: console.log("док:", modelData.label)
                }
            }
        }
    }
}
