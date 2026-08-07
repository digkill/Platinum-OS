import QtQuick

// Нижний док: постоянные приложения поверх стеклянной подложки.
GlassPanel {
    id: dock

    property alias model: repeater.model

    height: 112
    radius: Theme.dockRadius

    Row {
        anchors.centerIn: parent
        spacing: (dock.width - Theme.dockIconSize * repeater.count - 32) / Math.max(1, repeater.count - 1)

        Repeater {
            id: repeater

            GlassPanel {
                width: Theme.dockIconSize
                height: Theme.dockIconSize
                radius: Theme.iconRadius
                strong: true

                Image {
                    // Значки дока — рисунки без своего фона, поэтому они
                    // ставятся внутрь стеклянной плитки, а не во всю её
                    // площадь: растянутый глиф обрезался бы по краям.
                    anchors.centerIn: parent
                    width: parent.width * 0.62
                    height: parent.height * 0.62
                    visible: modelData.icon !== undefined
                    source: modelData.icon === undefined
                            ? "" : "../icons/" + modelData.icon + ".svg"
                    sourceSize.width: Theme.dockIconSize * 2
                    sourceSize.height: Theme.dockIconSize * 2
                    smooth: true
                }

                Text {
                    anchors.centerIn: parent
                    visible: modelData.icon === undefined
                    // Запись дока может не иметь эмодзи: у всех есть свой
                    // значок, и подставлять undefined в текст незачем.
                    text: modelData.glyph !== undefined ? modelData.glyph : ""
                    font.pixelSize: Theme.dockIconSize * 0.44
                    color: modelData.color !== undefined ? modelData.color : Theme.accent
                }

                MouseArea {
                    anchors.fill: parent
                    // "home" и "apps" — поверхности самой оболочки, остальное
                    // приложения. Открывать поверхность как приложение значило
                    // бы искать её в реестре, где её нет.
                    onClicked: modelData.id === "home" || modelData.id === "apps"
                               ? Navigation.show(modelData.id)
                               : Navigation.open(modelData.id)
                }
            }
        }
    }
}
