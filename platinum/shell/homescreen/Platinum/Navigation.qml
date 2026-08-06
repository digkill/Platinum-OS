pragma Singleton

import QtQuick

// Маршруты оболочки.
//
// Перенос `navigation.rs`. Экран определяется одним значением, а не набором
// флагов видимости: два флага рано или поздно оказались бы одновременно
// истинными, и поверх домашнего экрана нарисовалось бы приложение.
QtObject {
    id: navigation

    // Поверхность оболочки: "home" или "apps".
    property string surface: "home"

    // Открытое приложение; пусто — показана поверхность.
    property string app: ""

    readonly property bool inApp: app !== ""

    /// Открывает приложение по идентификатору.
    function open(id) {
        navigation.app = id;
    }

    /// Возвращает на поверхность оболочки.
    function back() {
        if (navigation.app !== "") {
            navigation.app = "";

            return;
        }

        navigation.surface = "home";
    }

    /// Переключает поверхность, закрывая приложение.
    function show(name) {
        navigation.app = "";
        navigation.surface = name;
    }
}
