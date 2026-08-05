import QtQuick

// Крупные часы и дата под ними.
Column {
    id: clock
    spacing: 2

    property date now: new Date()

    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        text: Qt.formatTime(clock.now, "HH:mm")
        // Крупный кегль с плотным трекингом — главный акцент экрана.
        font.pixelSize: 96
        font.weight: Font.Black
        font.letterSpacing: -3
        color: Theme.textPrimary
    }

    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        text: Qt.formatDate(clock.now, "ddd, dd MMM")
        font.pixelSize: 17
        font.weight: Font.Medium
        color: Theme.textSecondary
    }
}
