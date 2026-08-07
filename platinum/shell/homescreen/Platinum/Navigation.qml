pragma Singleton

import QtQuick

// Маршруты оболочки.
//
// Перенос `navigation.rs`. Экран определяется одним значением, а не набором
// флагов видимости: два флага рано или поздно оказались бы одновременно
// истинными, и поверх домашнего экрана нарисовалось бы приложение.
QtObject {
    id: navigation

    // Поверхность оболочки: "home", "apps", "window" — окно приложения — или
    // "switcher" — карусель открытых приложений.
    property string surface: "home"

    // Открытое приложение; пусто — показана поверхность.
    property string app: ""

    // Данные запуска: контакт, переданный из списка в звонилку или переписку.
    //
    // Перенос `AppLaunchPayload`. Приложение читает их при открытии, а не
    // получает свойствами: экраны создаются загрузчиком по идентификатору, и
    // прокидывать параметры сквозь него пришлось бы для каждого экрана.
    property var payload: null

    readonly property bool inApp: app !== ""

    /// Открывает приложение по идентификатору с необязательными данными.
    function open(id, data) {
        // Нативное приложение: QML-экрана у него нет, окно придёт от
        // композитора — реестр помечает такие поле `exec`.
        const module = Apps.find(id);
        if (module !== undefined && module.exec !== undefined) {
            Windows.launch(module.exec);

            return;
        }

        navigation.payload = data === undefined ? null : data;
        navigation.app = id;
    }

    /// Возвращает на поверхность оболочки.
    function back() {
        if (navigation.app !== "") {
            navigation.app = "";
            navigation.payload = null;

            return;
        }

        navigation.surface = "home";
    }

    /// Переключает поверхность, закрывая приложение.
    function show(name) {
        navigation.app = "";
        navigation.payload = null;
        navigation.surface = name;
    }
}
