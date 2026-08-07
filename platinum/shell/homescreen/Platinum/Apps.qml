pragma Singleton

import QtQuick

// Реестр приложений устройства.
//
// Перенос `shell_state.rs` и реестра из GTK-оболочки. Состав вынесен из
// разметки в данные: домашний экран, список приложений и док показывают один и
// тот же набор, а три копии списка разошлись бы при первом же добавлении.
//
// Поле `screen` — файл экрана. Приложение без него получает заглушку с
// описанием: так в реестре видно, что ещё не написано, и добавление экрана не
// требует правки ни оболочки, ни загрузчика.
QtObject {
    id: apps

    // Каждая запись: id для маршрутизации, значок, подпись, описание, экран.
    readonly property var modules: [
        {
            id: "calendar", icon: "calendar-home", title: "Calendar",
            description: "Расписание, напоминания и события устройства."
        },
        {
            id: "clock", icon: "clock-home", title: "Clock",
            description: "Мировое время, будильники и таймеры.",
            screen: "ClockScreen.qml"
        },
        {
            id: "contacts", icon: "contacts-home", title: "Contacts",
            description: "Люди, избранное и переход в звонок или переписку.",
            screen: "ContactsScreen.qml"
        },
        {
            id: "platinum", icon: "app-platinum", title: "Platinum OS",
            description: "Сведения о системе, сборке образа и устройстве."
        },

        {
            id: "ai", icon: "app-ai", title: "AI",
            description: "Помощник: сводки, извлечение задач, черновики ответов.",
            screen: "AiScreen.qml"
        },
        {
            id: "settings", icon: "app-settings", title: "Settings",
            description: "Оформление оболочки, дата и время, состояние железа.",
            screen: "SettingsScreen.qml"
        },
        {
            id: "files", icon: "files", title: "Files",
            description: "Файлы устройства и подключённых носителей."
        },
        {
            id: "gallery", icon: "gallery", title: "Gallery",
            description: "Снимки и записи с камеры устройства."
        },

        {
            id: "notes", icon: "notes", title: "Notes",
            description: "Заметки и быстрые записи."
        },
        {
            id: "music", icon: "music", title: "Music",
            description: "Музыка на устройстве и в сети."
        },
        {
            id: "browser", icon: "browser", title: "Browser",
            description: "Просмотр веб-страниц."
        },
        {
            id: "camera", icon: "camera", title: "Camera",
            description: "Съёмка фото и видео."
        }
    ]

    // Постоянные приложения дока.
    //
    // Идентификаторы совпадают с реестром: док открывает те же приложения, а не
    // свои копии. Исключение — "home" и "apps": это поверхности оболочки.
    readonly property var dock: [
        { id: "home",    icon: "dock-home",    title: "Home" },
        { id: "apps",    icon: "dock-apps",    title: "Apps" },
        { id: "call",    icon: "dock-call",    title: "Phone" },
        { id: "message", icon: "dock-message", title: "Messages" }
    ]

    // Приложения, которых нет на домашнем экране: жители дока и служебные
    // инструменты. В общем списке приложений они всё равно показываются — это
    // полноценные приложения, а не разделы.
    // Поле `exec` — команда нативного приложения: вместо QML-экрана оно
    // открывает настоящее окно Wayland в сцене оболочки.
    readonly property var extra: [
        {
            id: "console", icon: "console", title: "Console",
            description: "Команды системы от имени пользователя оболочки.",
            screen: "ConsoleScreen.qml"
        },
        {
            id: "terminal", icon: "terminal", title: "Terminal",
            // Именно Qt-приложение: foot требует wl_seat v5, а композитор Qt
            // отдаёт v4 — проверено живым запуском, foot выходит с ошибкой.
            description: "Настоящий терминал — qterminal.",
            exec: "qterminal"
        },
        {
            id: "call", icon: "dock-call", title: "Call",
            description: "Набор номера и недавние вызовы.",
            screen: "CallScreen.qml"
        },
        {
            id: "message", icon: "dock-message", title: "Messages",
            description: "Переписки, ретрансляция и синхронизация сообщений.",
            screen: "MessageScreen.qml"
        }
    ]

    // Разделы: открываются изнутри приложений и в списках не показываются.
    readonly property var internal: [
        {
            id: "timezone", icon: "clock-home", title: "Часовой пояс",
            description: "Выбор часового пояса устройства.",
            screen: "TimezoneScreen.qml"
        }
    ]

    /// Всё, что показывается в общем списке приложений.
    readonly property var listed: apps.modules.concat(apps.extra)

    /// Возвращает описание приложения по идентификатору.
    function find(id) {
        const all = apps.listed.concat(apps.internal);

        return all.find(function (module) { return module.id === id; });
    }
}
