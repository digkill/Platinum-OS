import QtQuick

// Сообщения: список переписок, история и поле ввода.
//
// Перенос `apps/message.rs`. Активная переписка задаётся индексом в хранилище,
// а не копией её содержимого: копия разошлась бы с хранилищем после первого же
// отправленного сообщения.
AppScreen {
    id: screen

    readonly property var launched: Navigation.payload

    property int active: 0
    property string composerStatus: launched !== null
                                    ? "Пишем: " + launched.name
                                    : "Черновик готов"

    readonly property var thread: active >= 0 && active < Store.threads.length
                                  ? Store.threads[active]
                                  : null

    title: "Messages"
    subtitle: "Единый список переписок, ретрансляция и прямой обмен сообщениями."

    // Контакт, с которым открыли экран, получает свою переписку сразу: искать
    // её в списке руками после нажатия «Message» бессмысленно.
    Component.onCompleted: {
        if (launched !== null) {
            screen.active = Store.threadFor(launched.phone, launched.name);
        }
    }

    Row {
        spacing: Theme.spacingSmall

        ActionButton {
            text: "Contacts"
            onClicked: Navigation.open("contacts")
        }
    }

    AppCard {
        width: parent.width
        title: "Threads"
        subtitle: "Нажатие открывает переписку в поле ниже."

        Column {
            width: parent.width
            spacing: Theme.spacingSmall

            Repeater {
                model: Store.threads

                AppRow {
                    width: parent.width
                    title: modelData.title
                    subtitle: modelData.preview
                    trailing: modelData.status
                    interactive: true
                    active: index === screen.active
                    onActivated: {
                        screen.active = index;
                        screen.composerStatus = "Переписка выбрана";
                    }
                }
            }
        }
    }

    AppCard {
        width: parent.width
        title: "Conversation"
        subtitle: "Выбранная переписка, история и поле ввода."

        Column {
            width: parent.width
            spacing: Theme.spacingSmall

            Text {
                width: parent.width
                visible: screen.thread === null
                text: "Переписка не выбрана."
                font.pixelSize: 13
                color: Theme.textSecondary
            }

            Column {
                width: parent.width
                visible: screen.thread !== null
                spacing: 2

                Text {
                    width: parent.width
                    text: screen.thread !== null ? screen.thread.title : ""
                    font.pixelSize: 15
                    font.weight: Font.DemiBold
                    color: Theme.textPrimary
                }

                Text {
                    width: parent.width
                    text: screen.thread !== null ? screen.thread.status : ""
                    font.pixelSize: 12
                    color: Theme.textSecondary
                }
            }

            // История: сообщения устройства слева, свои справа.
            Column {
                width: parent.width
                spacing: 6

                Repeater {
                    model: screen.thread !== null ? screen.thread.messages : []

                    Item {
                        width: parent.width
                        height: bubble.height

                        GlassPanel {
                            id: bubble

                            // Пузырь обнимает текст и не занимает всю ширину:
                            // иначе исходящее и входящее не различить взглядом.
                            // Ширина считается по естественной ширине надписи,
                            // а не по ширине списка — короткое «привет» иначе
                            // растянулось бы на весь экран.
                            readonly property int maxWidth: parent.width * 0.82

                            width: body.width + Theme.spacingSmall * 2
                            height: body.height + Theme.spacingSmall * 2
                            radius: 16
                            strong: modelData.outgoing
                            anchors.right: modelData.outgoing ? parent.right : undefined
                            anchors.left: modelData.outgoing ? undefined : parent.left

                            Column {
                                id: body

                                x: Theme.spacingSmall
                                y: Theme.spacingSmall
                                width: Math.min(bubble.maxWidth - Theme.spacingSmall * 2,
                                                Math.max(author.implicitWidth, line.implicitWidth))
                                spacing: 2

                                Text {
                                    id: author
                                    width: parent.width
                                    visible: text !== ""
                                    text: modelData.author
                                    font.pixelSize: 11
                                    color: Theme.textSecondary
                                }

                                Text {
                                    id: line
                                    width: parent.width
                                    text: modelData.body
                                    font.pixelSize: 13
                                    color: Theme.textPrimary
                                    wrapMode: Text.WordWrap
                                }
                            }
                        }
                    }
                }
            }

            // Поле ввода: TextEdit из QtQuick, а не TextArea из Controls —
            // Controls в образ не входит.
            GlassPanel {
                width: parent.width
                height: Math.max(84, composer.contentHeight + Theme.spacingMedium * 2)
                radius: 16
                strong: composer.activeFocus

                TextEdit {
                    id: composer

                    x: Theme.spacingMedium
                    y: Theme.spacingSmall
                    width: parent.width - Theme.spacingMedium * 2
                    height: parent.height - Theme.spacingSmall * 2
                    font.pixelSize: 14
                    color: Theme.textPrimary
                    selectionColor: Theme.accent
                    wrapMode: TextEdit.Wrap
                    activeFocusOnPress: true
                }

                Text {
                    x: Theme.spacingMedium
                    y: Theme.spacingSmall
                    visible: composer.text === ""
                    text: "Сообщение"
                    font.pixelSize: 14
                    color: Theme.textSecondary
                }
            }

            Text {
                width: parent.width
                text: screen.composerStatus
                font.pixelSize: 12
                color: Theme.textSecondary
            }

            Row {
                anchors.right: parent.right
                spacing: Theme.spacingSmall

                ActionButton {
                    text: "Очистить"
                    onClicked: {
                        composer.text = "";
                        screen.composerStatus = "Черновик очищен";
                    }
                }

                ActionButton {
                    text: "Отправить"
                    active: true
                    onClicked: screen.send()
                }
            }
        }
    }

    /// Дописывает исходящее сообщение в выбранную переписку.
    function send() {
        const text = composer.text.trim();

        if (text === "") {
            screen.composerStatus = "Сначала напишите сообщение";

            return;
        }

        if (screen.thread === null) {
            screen.composerStatus = "Сначала выберите переписку";

            return;
        }

        Store.appendMessage(screen.active, text, true);
        composer.text = "";
        // Сообщение сохранено на устройстве, но никуда не отправлено:
        // транспорта пока нет, и обещать доставку было бы враньём.
        screen.composerStatus = "Сообщение сохранено на устройстве";
    }
}
