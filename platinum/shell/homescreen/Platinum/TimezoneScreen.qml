import QtQuick

// Выбор часового пояса: сначала область, потом город.
//
// Два шага, а не один список: поясов больше четырёхсот, и прокручивать их
// подряд на телефоне невозможно. Так же это устроено в смартфонах.
AppScreen {
    id: screen

    // Пустая строка — показан список областей.
    property string region: ""

    title: "Часовой пояс"
    subtitle: region === ""
              ? "Сейчас: " + SystemSettings.timezone
              : "Область: " + region

    ActionButton {
        visible: screen.region !== ""
        text: "К областям"
        onClicked: screen.region = ""
    }

    AppCard {
        width: parent.width
        visible: screen.region === ""
        title: "Область"
        subtitle: "Материк или океан, к которому относится пояс."

        Column {
            width: parent.width
            spacing: Theme.spacingSmall

            Text {
                width: parent.width
                visible: Timezones.regions.length === 0
                text: "База часовых поясов не найдена."
                font.pixelSize: 13
                color: Theme.textSecondary
            }

            Repeater {
                model: Timezones.regions

                AppRow {
                    width: parent.width
                    title: modelData
                    // Отметка на области, в которой стоит текущий пояс: иначе
                    // после входа непонятно, где искать выбранное.
                    trailing: SystemSettings.timezone.indexOf(modelData + "/") === 0
                              || SystemSettings.timezone === modelData
                              ? "✓" : ""
                    interactive: true
                    onActivated: screen.region = modelData
                }
            }
        }
    }

    AppCard {
        width: parent.width
        visible: screen.region !== ""
        title: "Город"
        subtitle: "Время берётся по правилам выбранного города."

        Column {
            width: parent.width
            spacing: Theme.spacingSmall

            Repeater {
                model: screen.region === "" ? [] : Timezones.cities(screen.region)

                AppRow {
                    width: parent.width
                    title: modelData.title
                    trailing: SystemSettings.timezone === modelData.id ? "✓" : ""
                    active: SystemSettings.timezone === modelData.id
                    interactive: true
                    onActivated: {
                        SystemSettings.setTimezone(modelData.id);
                        Navigation.open("settings");
                    }
                }
            }
        }
    }
}
