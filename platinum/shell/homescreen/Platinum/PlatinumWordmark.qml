import QtQuick

// Название продукта под логотипом.
//
// Набирается текстом, а не картинкой: подпись должна оставаться резкой на любом
// экране, а разреженный трекинг задаётся здесь же и не требует правки макета.
Column {
    id: wordmark

    property real scale_: 1.0

    spacing: 2 * scale_

    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        text: "PLATINUM OS"
        font.pixelSize: 34 * wordmark.scale_
        font.weight: Font.Light
        font.letterSpacing: 5 * wordmark.scale_
        color: Qt.rgba(0.29, 0.31, 0.40, 0.88)
    }

    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        text: "ONE"
        font.pixelSize: 19 * wordmark.scale_
        font.weight: Font.Light
        font.letterSpacing: 11 * wordmark.scale_
        color: Qt.rgba(0.29, 0.31, 0.40, 0.62)
    }
}
