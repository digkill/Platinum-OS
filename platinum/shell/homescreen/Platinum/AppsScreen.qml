import QtQuick

// Список всех приложений.
//
// Перенос `components/screens/apps_screen.rs`. Состав берётся из реестра, а не
// повторяет разметку домашнего экрана: два списка разошлись бы при первом же
// добавлении приложения.
AppScreen {
    id: screen

    title: "Приложения"
    subtitle: "Всё, что установлено на устройстве."

    AppGrid {
        width: parent.width
        model: Apps.modules
        onLaunch: function (id) { Navigation.open(id); }
    }
}
