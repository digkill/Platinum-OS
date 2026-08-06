import QtQuick

// Сетка приложений 4 в ряд.
//
// Состав приходит моделью, а не зашит в разметку: список приложений — это
// данные устройства, и оболочка не должна знать их наперёд.
Grid {
    id: grid

    property alias model: repeater.model

    signal launch(string id)

    columns: Theme.gridColumns
    // Подпись занимает место под плиткой, поэтому рядам нужен свой интервал.
    rowSpacing: Theme.spacingMedium
    columnSpacing: 0

    Repeater {
        id: repeater

        AppIcon {
            width: grid.width / grid.columns
            label: modelData.title !== undefined ? modelData.title : modelData.label
            glyph: modelData.glyph !== undefined ? modelData.glyph : ""
            icon: modelData.icon !== undefined ? modelData.icon : ""
            glyphColor: modelData.color !== undefined ? modelData.color : Theme.accent
            artwork: modelData.artwork !== undefined ? modelData.artwork : null
            onActivated: console.log("запуск:", modelData.label)
        }
    }
}
