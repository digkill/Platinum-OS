import QtQuick

// Экран приложения, которое ещё не написано.
//
// Перенос `placeholder_root` из `apps/mod.rs`. Показывает описание из реестра,
// чтобы значок вёл хоть куда-то: молча ничего не делающая плитка выглядит
// сломанной оболочкой, а не незаконченным приложением.
AppScreen {
    id: screen

    readonly property var module: Apps.find(Navigation.app)

    title: module !== undefined ? module.title : ""
    subtitle: module !== undefined ? module.description : ""

    AppCard {
        width: parent.width
        title: "Скоро"
        subtitle: "Экран приложения ещё не написан. Остальная оболочка работает."

        Pill {
            text: "В разработке"
        }
    }
}
