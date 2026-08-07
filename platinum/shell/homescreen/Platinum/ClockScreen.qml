import QtQuick

// Часы: время и дата устройства.
//
// Перенос `apps/clock.rs`. Значения берутся из состояния устройства, а не из
// своего таймера: два независимых таймера разошлись бы между этим экраном и
// строкой состояния на секунду, и это было бы заметно.
AppScreen {
    id: screen

    title: "Clock"
    subtitle: "Время устройства, дата и часовой пояс системы."

    AppCard {
        width: parent.width
        title: "Сейчас"

        Column {
            spacing: 2

            Text {
                text: DeviceState.timeLabel
                font.pixelSize: 64
                font.weight: Font.Light
                color: Theme.textPrimary
            }

            Text {
                text: DeviceState.dateLabel
                font.pixelSize: 16
                color: Theme.textSecondary
            }
        }
    }

    AppRow {
        width: parent.width
        title: "Часовой пояс"
        subtitle: "Нажмите, чтобы выбрать другой."
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
        onActivated: Navigation.open("settings")
    }
}
