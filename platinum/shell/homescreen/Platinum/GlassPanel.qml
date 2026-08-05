import QtQuick

// Матовая «стеклянная» панель — базовый элемент всей оболочки.
//
// Эффект собран из полупрозрачной заливки, светлой рамки и мягкого внутреннего
// блика, без размытия фона и без layer-эффектов.
//
// Так сделано намеренно. Backdrop-blur пересчитывает гауссово размытие каждый
// кадр, а фон домашнего экрана статичен — на градиенте результат визуально тот
// же. MultiEffect в layer.effect тоже отпал: при программном рендере он не
// отрабатывает и элемент пропадает целиком (поймано на первом же рендере).
Rectangle {
    id: panel

    // Более плотное стекло для элементов поверх других панелей.
    property bool strong: false

    color: strong ? Theme.glassFillStrong : Theme.glassFill
    radius: Theme.panelRadius
    border.width: 1
    border.color: Theme.glassBorder
    antialiasing: true

    // Блик по верхней кромке: даёт объём там, где его дала бы тень.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: parent.radius * 0.5
        anchors.topMargin: 1
        height: 1
        radius: 0.5
        color: Qt.rgba(1, 1, 1, 0.75)
    }
}
