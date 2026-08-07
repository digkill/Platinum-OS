import QtQuick

// Значок приложения: цветная плитка и подпись под ней.
//
// Плитка — это сам значок, а не картинка внутри стеклянной подложки. Подложка
// приглушала цвет и съедала четверть площади: на экране устройства значки
// становились неразличимы издалека.
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
    height: Theme.iconSize + 28

    Item {
        id: tile

        width: Theme.iconSize
        height: Theme.iconSize
        anchors.horizontalCenter: parent.horizontalCenter

        scale: press.pressed ? 0.92 : 1.0
        Behavior on scale { NumberAnimation { duration: 90 } }

        // Мягкая тень под плиткой: без неё значки на светлом фоне сливаются с
        // ним. Рисуется прямоугольником со скруглением, а не размытием —
        // MultiEffect при программном рендере не отрабатывает.
        //
        // Поля повторяют отступ внутри самого значка: рисунок занимает не всю
        // клетку, и тень во всю ширину торчала бы из-под него серым квадратом.
        Rectangle {
            anchors.fill: parent
            anchors.leftMargin: parent.width * 0.075
            anchors.rightMargin: parent.width * 0.075
            anchors.topMargin: parent.height * 0.11
            anchors.bottomMargin: parent.height * 0.04
            radius: Theme.iconRadius
            color: Qt.rgba(0.35, 0.35, 0.55, 0.13)
        }

        // Запасная подложка: под значками без своего фона (эмодзи, чужая
        // картинка) иначе просвечивал бы экран.
        Rectangle {
            anchors.fill: parent
            visible: app.icon === ""
            radius: Theme.iconRadius
            color: Theme.glassFillStrong
            border.width: 1
            border.color: Theme.glassBorder
        }

        Loader {
            anchors.centerIn: parent
            active: app.artwork !== null
            sourceComponent: app.artwork
        }

        // Значок продукта рисуется во всю плитку: фон и скругление у него свои.
        Image {
            anchors.fill: parent
            visible: app.icon !== ""
            source: app.icon === "" ? "" : "../icons/" + app.icon + ".svg"
            sourceSize.width: Theme.iconSize * 2
            sourceSize.height: Theme.iconSize * 2
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
        font.pixelSize: 13
        color: Theme.textPrimary
        elide: Text.ElideRight
        width: parent.width + 18
        horizontalAlignment: Text.AlignHCenter
    }
}
