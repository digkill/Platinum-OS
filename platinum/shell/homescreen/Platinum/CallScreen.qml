import QtQuick

// Звонилка: набор номера и недавние вызовы.
//
// Перенос `apps/call.rs`. Номер живёт в свойстве экрана, а не в поле ввода:
// его пишут и клавиатура, и список недавних, и контакт, переданный при
// открытии, — читать состояние из виджета значило бы иметь три источника
// правды.
AppScreen {
    id: screen

    // Контакт, с которым открыли приложение.
    readonly property var launched: Navigation.payload

    property string number: launched !== null ? launched.phone : ""
    property string status: launched !== null
                            ? "Загружен " + launched.name
                            : (DeviceState.online ? "Готов к вызову" : "Нет сети")

    title: "Call"
    subtitle: "Быстрый набор, недавние вызовы и голосовые действия одной рукой."

    Row {
        spacing: Theme.spacingSmall

        ActionButton {
            text: "Contacts"
            onClicked: Navigation.open("contacts")
        }
    }

    AppCard {
        width: parent.width
        title: "Dialer"
        subtitle: "Номеронабиратель и быстрый возврат недавнего вызова."

        Column {
            width: parent.width
            spacing: Theme.spacingSmall

            Text {
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                text: screen.number === "" ? "—" : screen.number
                font.pixelSize: 30
                font.weight: Font.Light
                color: Theme.textPrimary
                elide: Text.ElideLeft
            }

            Text {
                id: statusLabel
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                text: screen.status
                font.pixelSize: 13
                color: Theme.textSecondary

                // Смена подписи заметна глазу только через вспышку: в GTK ту же
                // роль играл Revealer с crossfade. Анимируется прозрачность, а
                // не сам текст — Behavior на строке подменяет значение в
                // середине перехода и показывает старую подпись поверх новой.
                onTextChanged: blink.restart()

                SequentialAnimation {
                    id: blink
                    NumberAnimation {
                        target: statusLabel; property: "opacity"; to: 0.25; duration: 70
                    }
                    NumberAnimation {
                        target: statusLabel; property: "opacity"; to: 1.0; duration: 140
                    }
                }
            }

            Grid {
                anchors.horizontalCenter: parent.horizontalCenter
                columns: 3
                spacing: Theme.spacingSmall

                Repeater {
                    model: ["1", "2", "3", "4", "5", "6", "7", "8", "9", "*", "0", "#"]

                    ActionButton {
                        width: 72
                        height: 52
                        text: modelData
                        onClicked: {
                            screen.number += modelData;
                            screen.status = "Номер изменён";
                        }
                    }
                }
            }

            Row {
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: Theme.spacingSmall

                ActionButton {
                    text: "Стереть"
                    onClicked: {
                        screen.number = screen.number.slice(0, -1);
                        screen.status = screen.number === ""
                                        ? "Готов к вызову"
                                        : "Удалена последняя цифра";
                    }
                }

                ActionButton {
                    text: "Вызов"
                    active: true
                    // Кнопка без сети не гаснет молча: неактивной её делает
                    // именно отсутствие сети, и это видно по подписи статуса.
                    enabled: DeviceState.online
                    onClicked: screen.call()
                }

                ActionButton {
                    text: "Сброс"
                    onClicked: {
                        screen.number = "";
                        screen.status = "Набор очищен";
                    }
                }
            }
        }
    }

    AppCard {
        width: parent.width
        title: "Recent Calls"
        subtitle: "Нажатие возвращает номер в набор."

        Column {
            width: parent.width
            spacing: Theme.spacingSmall

            Text {
                width: parent.width
                visible: Store.recents.length === 0
                text: "Вызовов ещё не было."
                font.pixelSize: 13
                color: Theme.textSecondary
            }

            Repeater {
                model: Store.recents

                AppRow {
                    width: parent.width
                    title: modelData.name
                    subtitle: modelData.phone
                    trailing: modelData.note
                    interactive: true
                    onActivated: {
                        screen.number = modelData.phone;
                        screen.status = "Загружен " + modelData.name;
                    }
                }
            }
        }
    }

    /// Отмечает вызов в хранилище: список недавних строится по этой метке.
    function call() {
        if (screen.number === "") {
            screen.status = "Сначала наберите номер";

            return;
        }

        Store.markCalled(screen.number);
        screen.status = "Вызов " + screen.number;
    }
}
