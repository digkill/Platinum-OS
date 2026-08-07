import QtQuick

// Точки страниц домашнего экрана.
//
// Показывают, что экранов больше одного. Пока страница одна, но место под
// вторую занято сразу: появившийся из ниоткуда индикатор сдвинул бы док.
Row {
    id: dots

    property int count: 2
    property int current: 0

    spacing: 8

    Repeater {
        model: dots.count

        Rectangle {
            width: 8
            height: 8
            radius: 4
            color: index === dots.current ? "#ffffff" : Qt.rgba(1, 1, 1, 0.45)
            border.width: index === dots.current ? 0 : 1
            border.color: Qt.rgba(0.35, 0.35, 0.55, 0.18)
            antialiasing: true
        }
    }
}
