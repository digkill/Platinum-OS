import QtQuick

// Крупные часы и дата под ними.
Column {
    id: clock
    spacing: 2

    property date now: new Date()

    Item {
        anchors.horizontalCenter: parent.horizontalCenter
        width: time.width + 120
        height: time.height

        // Свечение за цифрами: мягкое белое пятно, из-за которого часы
        // отделяются от фона. Рисуется Canvas — радиального градиента в
        // QtQuick нет, а MultiEffect при программном рендере не отрабатывает.
        Canvas {
            anchors.fill: parent
            onPaint: {
                const ctx = getContext("2d");
                ctx.reset();

                const glow = ctx.createRadialGradient(width / 2, height / 2, 0,
                                                      width / 2, height / 2,
                                                      Math.min(width, height) * 0.62);
                glow.addColorStop(0.0, "rgba(255,255,255,0.85)");
                glow.addColorStop(0.5, "rgba(255,255,255,0.35)");
                glow.addColorStop(1.0, "rgba(255,255,255,0.0)");
                ctx.fillStyle = glow;
                ctx.fillRect(0, 0, width, height);
            }
        }

        Text {
            id: time
            anchors.centerIn: parent
            text: DeviceState.timeLabel
            // Крупный кегль с плотным трекингом — главный акцент экрана.
            font.pixelSize: 124
            font.weight: Font.Black
            font.letterSpacing: -4
            color: Theme.textPrimary
        }
    }

    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        text: Qt.formatDate(clock.now, "ddd, dd MMM")
        font.pixelSize: 19
        font.weight: Font.Medium
        color: Theme.textSecondary
    }
}
