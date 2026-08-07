import QtQuick

// Список всех приложений.
//
// Перенос `components/screens/apps_screen.rs`. Состав берётся из реестра, а не
// повторяет разметку домашнего экрана: два списка разошлись бы при первом же
// добавлении приложения.
AppScreen {
    id: screen

    title: "Приложения"
    subtitle: Apps.discovered.length > 0
              ? "Всё, что установлено на устройстве: " + Apps.listed.length
                + ", из них найдено в системе " + Apps.discovered.length + "."
              : "Всё, что установлено на устройстве."

    AppGrid {
        width: parent.width
        model: Apps.listed
        onLaunch: function (id) { Navigation.open(id); }
    }
}
