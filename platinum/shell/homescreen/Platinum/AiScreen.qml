import QtQuick

// Помощник: контекст устройства и заготовки действий.
//
// Перенос `apps/ai.rs`. Действия пока ничего не запускают — модели на
// устройстве нет. Каждое помечено состоянием, чтобы это было видно, а не
// выяснялось нажатием.
AppScreen {
    id: screen

    title: "AI Assistant"
    subtitle: "Сводки по оболочке, извлечение задач и переход к локальному выводу."

    AppCard {
        width: parent.width
        title: "Context Window"
        subtitle: "Состояние устройства, которое получит помощник."

        Row {
            spacing: Theme.spacingSmall

            Pill { text: "Время " + DeviceState.timeLabel }
            Pill { text: DeviceState.dateLabel }
            Pill { text: DeviceState.online ? "Online" : "Offline" }
        }
    }

    AppCard {
        width: parent.width
        title: "Quick Actions"
        subtitle: "Частые задачи помощника, рассчитанные на работу одной рукой."

        Grid {
            width: parent.width
            columns: 2
            spacing: Theme.spacingSmall

            Repeater {
                model: ["Сводка сессии", "Собрать бриф", "Извлечь задачи", "Черновик ответа"]

                ActionButton {
                    width: (parent.width - Theme.spacingSmall) / 2
                    text: modelData
                    // Модели на устройстве нет: нажатие сообщает об этом прямо,
                    // а не открывает пустой экран.
                    onClicked: notice.text = "Локальная модель ещё не установлена"
                }
            }
        }

        Text {
            id: notice
            width: parent.width
            visible: text !== ""
            font.pixelSize: 12
            color: Theme.textSecondary
        }
    }

    AppRow {
        width: parent.width
        title: "Session Summary"
        subtitle: "Свести недавние действия оболочки и заметки оператора в краткий отчёт."
        trailing: "Ready"
    }

    AppRow {
        width: parent.width
        title: "Field Digest"
        subtitle: "Обзор состояния устройства: питание, связь, активность контактов."
        trailing: "Queued"
    }

    AppRow {
        width: parent.width
        title: "Notes Pipeline"
        subtitle: "Извлечение следующих шагов из контактов и переписки."
        trailing: "Standby"
    }
}
