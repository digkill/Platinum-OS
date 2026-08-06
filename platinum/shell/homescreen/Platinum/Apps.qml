pragma Singleton

import QtQuick

// Реестр приложений устройства.
//
// Перенос `shell_state.rs` и реестра из GTK-оболочки. Состав вынесен из
// разметки в данные: домашний экран, список приложений и док показывают один и
// тот же набор, а три копии списка разошлись бы при первом же добавлении.
QtObject {
    id: apps

    // Каждая запись: id для маршрутизации, значок, подпись.
    readonly property var modules: [
        { id: "calendar", icon: "calendar-home", title: "Calendar" },
        { id: "clock",    icon: "clock-home",    title: "Clock" },
        { id: "contacts", icon: "contacts-home", title: "Contacts" },
        { id: "platinum", icon: "platinum-one-home", title: "Platinum OS" },

        { id: "ai",       icon: "ai-home",       title: "AI Assistant" },
        { id: "messages", icon: "message",       title: "Messages" },
        { id: "files",    icon: "apps",          title: "Files" },
        { id: "notes",    icon: "apps",          title: "Notes" },

        { id: "gallery",  icon: "apps",          title: "Gallery" },
        { id: "settings", icon: "settings-home", title: "Settings" },
        { id: "security", icon: "relay",         title: "Security" },
        { id: "store",    icon: "apps",          title: "Store" }
    ]

    // Постоянные приложения дока.
    readonly property var dock: [
        { id: "home",     icon: "dock-home",    title: "Home" },
        { id: "apps",     icon: "dock-apps",    title: "Apps" },
        { id: "phone",    icon: "dock-call",    title: "Phone" },
        { id: "messages", icon: "dock-message", title: "Messages" }
    ]

    /// Возвращает описание приложения по идентификатору.
    function find(id) {
        return apps.modules.find(function (module) { return module.id === id; });
    }
}
