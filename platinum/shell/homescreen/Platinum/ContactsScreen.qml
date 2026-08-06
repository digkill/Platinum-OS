import QtQuick

// Контакты: люди и переход в звонок или переписку.
//
// Перенос `apps/contacts.rs`. Кнопки открывают другое приложение с уже
// подставленным контактом — тот же обмен данными, что `AppLaunchPayload` в
// GTK-оболочке.
AppScreen {
    id: screen

    title: "Contacts"
    subtitle: "Избранные люди, быстрые действия и передача контакта в звонок или сообщение."

    AppCard {
        width: parent.width
        title: "Favorites"
        subtitle: "Call и Message открывают приложение с подставленным контактом."

        Column {
            width: parent.width
            spacing: Theme.spacingSmall

            Repeater {
                model: Store.sorted

                GlassPanel {
                    width: parent.width
                    height: entry.height + Theme.spacingMedium * 2
                    radius: 18

                    Column {
                        id: entry

                        x: Theme.spacingMedium
                        y: Theme.spacingMedium
                        width: parent.width - Theme.spacingMedium * 2
                        spacing: Theme.spacingSmall

                        Item {
                            width: parent.width
                            height: copy.height

                            Column {
                                id: copy
                                width: parent.width - note.width - Theme.spacingSmall
                                spacing: 2

                                Text {
                                    width: parent.width
                                    text: modelData.name
                                    font.pixelSize: 15
                                    font.weight: Font.DemiBold
                                    color: Theme.textPrimary
                                    elide: Text.ElideRight
                                }

                                Text {
                                    width: parent.width
                                    text: modelData.phone
                                    font.pixelSize: 13
                                    color: Theme.textSecondary
                                }
                            }

                            Text {
                                id: note
                                anchors.right: parent.right
                                anchors.top: parent.top
                                text: modelData.note
                                font.pixelSize: 12
                                color: Theme.textSecondary
                            }
                        }

                        Row {
                            spacing: Theme.spacingSmall

                            ActionButton {
                                text: "Call"
                                onClicked: Navigation.open("call", {
                                    name: modelData.name,
                                    phone: modelData.phone
                                })
                            }

                            ActionButton {
                                text: "Message"
                                onClicked: Navigation.open("message", {
                                    name: modelData.name,
                                    phone: modelData.phone
                                })
                            }
                        }
                    }
                }
            }
        }
    }
}
