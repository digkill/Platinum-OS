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

                Image {
                    anchors.centerIn: parent
                    visible: modelData.icon !== undefined
                    source: modelData.icon === undefined
                            ? "" : "../icons/" + modelData.icon + ".svg"
                    sourceSize.width: Theme.iconSize * 0.52
                    sourceSize.height: Theme.iconSize * 0.52
                    smooth: true
                }

                Text {
                    anchors.centerIn: parent
                    visible: modelData.icon === undefined
                    text: modelData.glyph
                    font.pixelSize: Theme.iconSize * 0.44
                    color: modelData.color !== undefined ? modelData.color : Theme.accent
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: modelData.id === "apps"
                               ? Navigation.show("apps")
                               : Navigation.open(modelData.id)
                }
            }
        }
    }
}
