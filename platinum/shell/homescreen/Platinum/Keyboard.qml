pragma Singleton

import QtQuick
import QtCore

// Режим экранной клавиатуры.
//
// Три режима, а не переключатель «вкл/выкл»: устройство одно, а сценариев два.
// С сенсорным экраном клавиатура нужна по требованию поля, с подключённой USB-
// клавиатурой она только отнимает место, а при отладке бывает нужна постоянно
// открытой — например, когда фокус ловит окно приложения, а не поле оболочки.
QtObject {
    id: keyboard

    // "auto" — по запросу поля, "always" — всегда на экране, "off" — никогда.
    property string mode: "auto"

    property Settings settings: Settings {
        category: "keyboard"
        property alias mode: keyboard.mode
    }

    readonly property bool enabled: keyboard.mode !== "off"
    readonly property bool pinned: keyboard.mode === "always"

    /// Порядок режимов для переключателя; он же задаёт порядок обхода.
    readonly property var modes: [
        { id: "auto",   title: "Авто" },
        { id: "always", title: "Всегда" },
        { id: "off",    title: "Выкл" }
    ]

    /// Устанавливает режим; неизвестное значение откатывается к "auto",
    /// иначе опечатка в сохранённой настройке оставила бы устройство без
    /// клавиатуры и без способа её вернуть.
    function setMode(value) {
        const known = keyboard.modes.some(function (entry) { return entry.id === value; });
        keyboard.mode = known ? value : "auto";
    }
}
