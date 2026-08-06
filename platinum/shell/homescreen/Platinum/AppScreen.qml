import QtQuick

// Каркас экрана приложения: заголовок и прокручиваемая колонка.
//
// Перенос `app_surface_root` и `app_hero` из `apps/mod.rs`. Прокрутка здесь, а
// не в каждом экране: на панели 720x1280 длинный экран без неё обрезается
// молча — нижние карточки просто не показываются.
Flickable {
    id: screen

    property string title: ""
    property string subtitle: ""

    default property alias content: column.data

    contentWidth: width
    contentHeight: column.height + Theme.spacingLarge * 2
    clip: true
    boundsBehavior: Flickable.StopAtBounds

    Column {
        id: column

        x: Theme.screenMargin
        y: Theme.spacingMedium
        width: screen.width - Theme.screenMargin * 2
        spacing: Theme.spacingMedium

        Column {
            width: parent.width
            spacing: 6

            Text {
                width: parent.width
                visible: text !== ""
                text: screen.title
                font.pixelSize: 30
                font.weight: Font.DemiBold
                color: Theme.textPrimary
            }

            Text {
                width: parent.width
                visible: text !== ""
                text: screen.subtitle
                font.pixelSize: 14
                color: Theme.textSecondary
                wrapMode: Text.WordWrap
            }
        }
    }
}
