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
            id: "platinum", icon: "platinum-one-home", title: "Platinum OS",
            description: "Сведения о системе, сборке образа и устройстве."
        },

        {
            id: "ai", icon: "ai-home", title: "AI Assistant",
            description: "Помощник: сводки, извлечение задач, черновики ответов.",
            screen: "AiScreen.qml"
        },
        {
            id: "message", icon: "message", title: "Messages",
            description: "Переписки, ретрансляция и синхронизация сообщений.",
            screen: "MessageScreen.qml"
        },
        {
            id: "files", icon: "apps", title: "Files",
            description: "Файлы устройства и подключённых носителей."
        },
        {
            id: "notes", icon: "apps", title: "Notes",
            description: "Заметки и быстрые записи."
        },

        {
            id: "gallery", icon: "apps", title: "Gallery",
            description: "Снимки и записи с камеры устройства."
        },
        {
            id: "settings", icon: "settings-home", title: "Settings",
            description: "Оформление оболочки, состояние железа и профиль устройства.",
            screen: "SettingsScreen.qml"
        },
        {
            id: "security", icon: "relay", title: "Security",
            description: "Доступ, ключи и ретрансляция между устройствами."
        },
        {
            id: "store", icon: "apps", title: "Store",
            description: "Каталог приложений Platinum."
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

    // Звонилка в сетке приложений не показана: её место — в доке. Но открыть её
    // можно и из списка контактов, поэтому описание нужно и ей.
    readonly property var hidden: [
        {
            id: "call", icon: "dock-call", title: "Call",
            description: "Набор номера и недавние вызовы.",
            screen: "CallScreen.qml"
        }
    ]

    /// Возвращает описание приложения по идентификатору.
    function find(id) {
        const all = apps.modules.concat(apps.hidden);

        return all.find(function (module) { return module.id === id; });
    }
}
