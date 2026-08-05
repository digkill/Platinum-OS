pragma Singleton

import QtQuick

// Единственный источник размеров и цветов оболочки.
//
// Значения собраны здесь, а не разбросаны по компонентам: домашний экран,
// док и карточки обязаны выглядеть одним целым, а подгонять десяток файлов
// под смену акцента — верный способ их рассинхронизировать.
QtObject {
    id: theme

    // Фон: мягкий градиент сиреневый → голубой, как на прототипе.
    readonly property color backgroundTop: "#c9c6ee"
    readonly property color backgroundMiddle: "#c3cdf0"
    readonly property color backgroundBottom: "#bcc9ec"

    // Стекло. Заливка полупрозрачная, а не размытие фона: на статичном
    // градиенте разница неразличима, зато не нужен backdrop-blur, который на
    // плате без GPU считался бы каждый кадр процессором.
    readonly property color glassFill: Qt.rgba(1, 1, 1, 0.38)
    readonly property color glassFillStrong: Qt.rgba(1, 1, 1, 0.52)
    readonly property color glassBorder: Qt.rgba(1, 1, 1, 0.65)
    readonly property color glassShadow: Qt.rgba(0.35, 0.35, 0.55, 0.18)

    readonly property color textPrimary: "#1d1d2b"
    readonly property color textSecondary: Qt.rgba(0.29, 0.29, 0.40, 0.72)
    readonly property color accent: "#6c5ce7"
    readonly property color accentSoft: "#8f8ae8"

    // Радиусы: иконки — «квадрат со скруглением», панели мягче.
    readonly property int iconRadius: 22
    readonly property int panelRadius: 26
    readonly property int dockRadius: 32

    readonly property int iconSize: 72
    readonly property int gridColumns: 4

    readonly property int spacingSmall: 8
    readonly property int spacingMedium: 16
    readonly property int spacingLarge: 24
    readonly property int screenMargin: 24
}
