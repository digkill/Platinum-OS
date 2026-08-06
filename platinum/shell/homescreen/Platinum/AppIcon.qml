import QtQuick

// Иконка приложения: стеклянная плитка и подпись под ней.
Item {
    id: app

    property string label: ""
    property string glyph: ""
    /// Имя файла значка в каталоге icons без расширения.
    property string icon: ""
    property color glyphColor: Theme.accent
    property Component artwork: null

    signal activated()

    width: Theme.iconSize
    height: Theme.iconSize + 26

    GlassPanel {
        id: tile
        width: Theme.iconSize
        height: Theme.iconSize
        radius: Theme.iconRadius
        anchors.horizontalCenter: parent.horizontalCenter

        scale: press.pressed ? 0.93 : 1.0
        Behavior on scale { NumberAnimation { duration: 90 } }

        Loader {
            anchors.centerIn: parent
            active: app.artwork !== null
            sourceComponent: app.artwork
        }

        // Значок продукта имеет приоритет: эмодзи остаются запасным вариантом
        // для приложений, которым свой значок ещё не нарисован.
        Image {
            anchors.centerIn: parent
            visible: app.icon !== ""
            source: app.icon === "" ? "" : "../icons/" + app.icon + ".svg"
            sourceSize.width: Theme.iconSize * 0.58
            sourceSize.height: Theme.iconSize * 0.58
            smooth: true
        }

        Text {
            anchors.centerIn: parent
            visible: app.artwork === null && app.icon === ""
            text: app.glyph
            font.pixelSize: Theme.iconSize * 0.46
            color: app.glyphColor
        }

        MouseArea {
            id: press
            anchors.fill: parent
            onClicked: app.activated()
        }
    }

    Text {
        anchors.top: tile.bottom
        anchors.topMargin: 7
        anchors.horizontalCenter: parent.horizontalCenter
        text: app.label
        font.pixelSize: 12
        color: Theme.textPrimary
        elide: Text.ElideRight
        width: parent.width + 16
        horizontalAlignment: Text.AlignHCenter
    }
}
