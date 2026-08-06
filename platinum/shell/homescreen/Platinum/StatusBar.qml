import QtQuick

// Верхняя строка: время, вырез камеры, индикаторы сети и заряда.
Item {
    id: bar
    height: 44

    // Значения приходят из состояния устройства, а не задаются разметкой.
    property string time: DeviceState.timeLabel
    property int signalBars: DeviceState.signalLevel
    property real battery: DeviceState.battery
    property bool charging: DeviceState.charging
    property bool online: DeviceState.online

    Text {
        anchors.verticalCenter: parent.verticalCenter
        anchors.left: parent.left
        anchors.leftMargin: Theme.screenMargin
        text: bar.time
        font.pixelSize: 17
        font.weight: Font.DemiBold
        color: Theme.textPrimary
    }

    // Вырез фронтальной камеры.
    Rectangle {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.verticalCenter: parent.verticalCenter
        width: 92; height: 9; radius: height / 2
        color: Qt.rgba(0.42, 0.42, 0.52, 0.55)
    }

    Row {
        anchors.verticalCenter: parent.verticalCenter
        anchors.right: parent.right
        anchors.rightMargin: Theme.screenMargin
        spacing: 7

        // Уровень сигнала: четыре растущих штриха.
        Row {
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2
            Repeater {
                model: 4
                Rectangle {
                    width: 3
                    height: 4 + index * 3
                    radius: 1
                    anchors.bottom: parent.bottom
                    color: index < bar.signalBars ? Theme.textPrimary
                                                  : Qt.rgba(0, 0, 0, 0.22)
                }
            }
        }

        // Wi-Fi: три дуги и точка.
        Canvas {
            width: 17; height: 13
            anchors.verticalCenter: parent.verticalCenter
            onPaint: {
                const ctx = getContext("2d");
                ctx.reset();
                ctx.strokeStyle = bar.online ? Theme.textPrimary
                                             : Qt.rgba(0, 0, 0, 0.25);
                ctx.lineCap = "round";
                for (let i = 0; i < 3; ++i) {
                    ctx.lineWidth = 1.8;
                    ctx.beginPath();
                    ctx.arc(width / 2, height, 3.5 + i * 4, Math.PI * 1.25, Math.PI * 1.75);
                    ctx.stroke();
                }
                ctx.fillStyle = bar.online ? Theme.textPrimary
                                           : Qt.rgba(0, 0, 0, 0.25);
                ctx.beginPath();
                ctx.arc(width / 2, height - 1, 1.4, 0, Math.PI * 2);
                ctx.fill();
            }
        }

        // Заряд.
        Item {
            width: 26; height: 13
            anchors.verticalCenter: parent.verticalCenter

            Rectangle {
                anchors.fill: parent
                anchors.rightMargin: 3
                radius: 3.5
                color: "transparent"
                border.width: 1.4
                border.color: Theme.textPrimary

                Rectangle {
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.leftMargin: 2
                    width: (parent.width - 4) * bar.battery
                    height: parent.height - 4
                    radius: 2
                    // Зелёный при зарядке — единственный цветной элемент
                    // строки, поэтому читается сразу.
                    color: bar.charging ? "#34c759" : Theme.textPrimary
                }
            }

            Rectangle {
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                width: 2; height: 5; radius: 1
                color: Theme.textPrimary
            }
        }
    }
}
