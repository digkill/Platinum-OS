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

    AppCard {
        width: parent.width
        title: "Дата и время"
        subtitle: "Часовой пояс, синхронизация по сети и формат часов."

        Column {
            width: parent.width
            spacing: Theme.spacingSmall

            AppRow {
                width: parent.width
                title: "Часовой пояс"
                subtitle: "По его правилам считается время устройства."
                trailing: SystemSettings.timezone
                interactive: true
                onActivated: Navigation.open("timezone")
            }

            AppRow {
                width: parent.width
                title: "Автоматическое время"
                subtitle: SystemSettings.ntpEnabled
                          ? (SystemSettings.ntpSynchronized
                             ? "Синхронизировано по сети"
                             : "Синхронизация ещё не прошла")
                          : "Время задаётся вручную"
                trailing: SystemSettings.ntpEnabled ? "Вкл" : "Выкл"
                interactive: true
                onActivated: SystemSettings.setNtp(!SystemSettings.ntpEnabled)
            }

            AppRow {
                width: parent.width
                title: "24-часовой формат"
                subtitle: "Иначе часы показываются с AM и PM."
                trailing: SystemSettings.clock24h ? "Вкл" : "Выкл"
                interactive: true
                onActivated: SystemSettings.clock24h = !SystemSettings.clock24h
            }

            // Ручная установка появляется только при выключенной
            // синхронизации: иначе выставленное время уехало бы обратно через
            // секунду, и это выглядело бы поломкой.
            Column {
                id: manual

                width: parent.width
                visible: !SystemSettings.ntpEnabled
                spacing: Theme.spacingSmall

                // Правка начинается с текущего времени и дальше живёт сама:
                // иначе тикающие часы сбрасывали бы набранное значение.
                property date moment: DeviceState.now

                function shift(seconds) {
                    manual.moment = new Date(manual.moment.getTime() + seconds * 1000);
                }

                Text {
                    width: parent.width
                    text: Qt.formatDateTime(manual.moment, "dd MMMM yyyy, HH:mm")
                    font.pixelSize: 15
                    font.weight: Font.DemiBold
                    color: Theme.textPrimary
                }

                Row {
                    spacing: Theme.spacingSmall

                    ActionButton { text: "−1 день"; onClicked: manual.shift(-86400) }
                    ActionButton { text: "+1 день"; onClicked: manual.shift(86400) }
                    ActionButton { text: "−1 ч"; onClicked: manual.shift(-3600) }
                    ActionButton { text: "+1 ч"; onClicked: manual.shift(3600) }
                }

                Row {
                    spacing: Theme.spacingSmall

                    ActionButton { text: "−5 мин"; onClicked: manual.shift(-300) }
                    ActionButton { text: "+5 мин"; onClicked: manual.shift(300) }
                    ActionButton { text: "−1 мин"; onClicked: manual.shift(-60) }
                    ActionButton { text: "+1 мин"; onClicked: manual.shift(60) }
                }

                ActionButton {
                    text: "Применить"
                    active: true
                    onClicked: SystemSettings.setTime(manual.moment)
                }
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
