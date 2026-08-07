pragma Singleton

import QtQuick
import QtCore

// Единственный источник размеров и цветов оболочки.
//
// Значения собраны здесь, а не разбросаны по компонентам: домашний экран,
// док и карточки обязаны выглядеть одним целым, а подгонять десяток файлов
// под смену акцента — верный способ их рассинхронизировать.
QtObject {
    id: theme

    // Режим оформления: "light" или "dark".
    //
    // Перенос `theme.rs`. Режима "auto" здесь нет намеренно: в GTK его решал
    // AdwStyleManager по настройке рабочего стола, а оболочка запускается под
    // cage — рабочего стола и портала настроек рядом нет, и "auto" всегда
    // означал бы светлую тему. Кнопка, которая ничего не делает, хуже её
    // отсутствия.
    property string mode: "light"

    readonly property bool dark: mode === "dark"

    property Settings settings: Settings {
        category: "theme"
        property alias mode: theme.mode
    }

    /// Переключает оформление; значение сохраняется само через Settings.
    function setMode(value) {
        theme.mode = value === "dark" ? "dark" : "light";
    }

    // Фон: мягкий градиент сиреневый → голубой, как на прототипе.
    readonly property color backgroundTop: dark ? "#20202e" : "#c9c6ee"
    readonly property color backgroundMiddle: dark ? "#1c2033" : "#c3cdf0"
    readonly property color backgroundBottom: dark ? "#191d2c" : "#bcc9ec"

    // Стекло. Заливка полупрозрачная, а не размытие фона: на статичном
    // градиенте разница неразличима, зато не нужен backdrop-blur, который на
    // плате без GPU считался бы каждый кадр процессором.
    readonly property color glassFill: dark ? Qt.rgba(1, 1, 1, 0.10)
                                            : Qt.rgba(1, 1, 1, 0.38)
    readonly property color glassFillStrong: dark ? Qt.rgba(1, 1, 1, 0.17)
                                                  : Qt.rgba(1, 1, 1, 0.52)
    readonly property color glassBorder: dark ? Qt.rgba(1, 1, 1, 0.22)
                                              : Qt.rgba(1, 1, 1, 0.65)
    readonly property color glassShadow: dark ? Qt.rgba(0, 0, 0, 0.45)
                                              : Qt.rgba(0.35, 0.35, 0.55, 0.18)

    // Блик по верхней кромке панели: на тёмном фоне белая полоса в три четверти
    // яркости выглядит царапиной, поэтому она заметно слабее.
    readonly property color glassHighlight: dark ? Qt.rgba(1, 1, 1, 0.16)
                                                 : Qt.rgba(1, 1, 1, 0.75)

    readonly property color textPrimary: dark ? "#f2f2f7" : "#1d1d2b"
    readonly property color textSecondary: dark ? Qt.rgba(1, 1, 1, 0.62)
                                                : Qt.rgba(0.29, 0.29, 0.40, 0.72)
    // Строка состояния белая на обеих темах: она лежит поверх самой светлой
    // части градиента, и тёмный текст там выглядит грязным пятном.
    readonly property color statusText: "#ffffff"
    readonly property color statusTextDim: Qt.rgba(1, 1, 1, 0.55)

    readonly property color accent: dark ? "#8f8ae8" : "#6c5ce7"
    readonly property color accentSoft: dark ? "#a6a1f0" : "#8f8ae8"

    // Радиусы: иконки — «квадрат со скруглением», панели мягче.
    readonly property int iconRadius: 30
    readonly property int panelRadius: 26
    readonly property int dockRadius: 32

    // Размер значка. Крупный намеренно: значок и есть плитка, а не картинка
    // внутри подложки, и на экране устройства он должен читаться издалека.
    readonly property int iconSize: 128
    readonly property int dockIconSize: 84
    readonly property int gridColumns: 4

    readonly property int spacingSmall: 8
    readonly property int spacingMedium: 16
    readonly property int spacingLarge: 24
    readonly property int screenMargin: 24
}
