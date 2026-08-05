import QtQuick

// Виджет-карточка «Focus» с кнопкой запуска сессии.
GlassPanel {
    id: card
    height: 148

    property string title: "Focus"
    property string subtitle: "Stay focused, achieve more."
    property string action: "Start session"

    signal started()

    // Декоративный горный силуэт справа, как на прототипе.
    // Отступы равны радиусу панели: clip у Item прямоугольный, и без них
    // силуэт вылезает за скруглённый угол карточки.
    Item {
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.top: parent.top
        anchors.rightMargin: parent.radius * 0.55
        anchors.bottomMargin: 2
        anchors.topMargin: 2
        width: parent.width * 0.44
        clip: true

        Canvas {
            anchors.fill: parent
            onPaint: {
                const ctx = getContext("2d");
                ctx.reset();

                const gradient = ctx.createLinearGradient(0, 0, 0, height);
                gradient.addColorStop(0, Qt.rgba(0.62, 0.60, 0.86, 0.55));
                gradient.addColorStop(1, Qt.rgba(0.52, 0.58, 0.84, 0.20));
                ctx.fillStyle = gradient;

                ctx.beginPath();
                ctx.moveTo(0, height);
                ctx.lineTo(width * 0.26, height * 0.42);
                ctx.lineTo(width * 0.45, height * 0.70);
                ctx.lineTo(width * 0.66, height * 0.24);
                ctx.lineTo(width, height * 0.78);
                ctx.lineTo(width, height);
                ctx.closePath();
                ctx.fill();
            }
        }

        // Светящееся кольцо — тот же мотив, что в логотипе.
        Rectangle {
            width: 74; height: 74; radius: 37
            anchors.centerIn: parent
            anchors.verticalCenterOffset: 8
            color: "transparent"
            border.width: 3
            border.color: Qt.rgba(1, 1, 1, 0.85)
            opacity: 0.9
        }
    }

    Column {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.margins: 20
        spacing: 8

        Row {
            spacing: 10
            Rectangle {
                width: 22; height: 22; radius: 11
                anchors.verticalCenter: parent.verticalCenter
                color: "transparent"
                border.width: 2.4
                border.color: Theme.accent
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: card.title
                font.pixelSize: 21
                font.weight: Font.DemiBold
                color: Theme.textPrimary
            }
        }

        Text {
            text: card.subtitle
            font.pixelSize: 13
            color: Theme.textSecondary
        }

        GlassPanel {
            width: actionText.implicitWidth + 34
            height: 34
            radius: 17
            strong: true

            Text {
                id: actionText
                anchors.centerIn: parent
                text: card.action
                font.pixelSize: 13
                font.weight: Font.Medium
                color: Theme.textPrimary
            }

            MouseArea {
                anchors.fill: parent
                onClicked: card.started()
            }
        }
    }
}
