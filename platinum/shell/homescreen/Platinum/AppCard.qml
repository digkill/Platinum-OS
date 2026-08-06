import QtQuick

// Карточка раздела: заголовок, пояснение и произвольное содержимое.
//
// Перенос `app_card` из `apps/mod.rs`. В GTK это была функция, строившая
// коробку с двумя подписями; здесь — компонент, потому что содержимое кладётся
// внутрь разметкой, а не аргументом.
GlassPanel {
    id: card

    property string title: ""
    property string subtitle: ""

    default property alias content: body.data

    implicitHeight: body.height + Theme.spacingMedium * 2
    height: implicitHeight

    Column {
        id: body

        x: Theme.spacingMedium
        y: Theme.spacingMedium
        width: card.width - Theme.spacingMedium * 2
        spacing: Theme.spacingSmall

        Text {
            width: parent.width
            visible: text !== ""
            text: card.title
            font.pixelSize: 17
            font.weight: Font.DemiBold
            color: Theme.textPrimary
            wrapMode: Text.WordWrap
        }

        Text {
            width: parent.width
            visible: text !== ""
            text: card.subtitle
            font.pixelSize: 13
            color: Theme.textSecondary
            wrapMode: Text.WordWrap
        }
    }
}
