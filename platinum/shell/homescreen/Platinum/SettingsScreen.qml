import QtQuick

// Настройки: оформление оболочки и состояние устройства.
//
// Перенос `apps/settings.rs`. Состояние железа читается из DeviceState, а не
// пишется в разметке: экран настроек с выдуманным зарядом выглядит рабочим и
// потому опаснее пустого.
AppScreen {
    id: screen

    title: "Settings"
    subtitle: "Оформление оболочки, состояние железа и профиль устройства."

    AppCard {
        width: parent.width
        title: "Appearance"
        subtitle: "Режим оформления сохраняется и переживает перезапуск оболочки."

        Row {
            spacing: Theme.spacingSmall

            Pill { text: Theme.dark ? "Тема тёмная" : "Тема светлая" }
            Pill { text: "Portrait 720x1280" }
        }

        Row {
            spacing: Theme.spacingSmall

            ActionButton {
                text: "Светлая"
                active: !Theme.dark
                onClicked: Theme.setMode("light")
            }

            ActionButton {
                text: "Тёмная"
                active: Theme.dark
                onClicked: Theme.setMode("dark")
            }
        }
    }

    AppRow {
        width: parent.width
        title: "Battery"
        subtitle: "Текущее питание устройства."
        trailing: Math.round(DeviceState.battery * 100) + "%"
            + (DeviceState.charging ? " · заряд" : "")
    }

    AppRow {
        width: parent.width
        title: "Network"
        subtitle: "Связь, которую используют помощник и сообщения."
        trailing: DeviceState.online ? DeviceState.network : "offline"
    }

    AppRow {
        width: parent.width
        title: "Signal"
        subtitle: "Уровень сигнала активного подключения."
        trailing: DeviceState.signalLevel + "/4"
    }

    AppCard {
        width: parent.width
        title: "Device"
        subtitle: "Сведения о системе, собранной образом Platinum."

        Column {
            width: parent.width
            spacing: 2

            Text {
                width: parent.width
                text: "Оболочка: QML под cage"
                font.pixelSize: 13
                color: Theme.textSecondary
            }

            Text {
                width: parent.width
                text: "Время: " + DeviceState.timeLabel + " · " + DeviceState.dateLabel
                font.pixelSize: 13
                color: Theme.textSecondary
            }
        }
    }
}
