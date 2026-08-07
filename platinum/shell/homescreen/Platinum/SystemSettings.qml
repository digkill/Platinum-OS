pragma Singleton

import QtQuick
import QtCore

// Системные настройки устройства: часовой пояс, синхронизация, время.
//
// Оболочка не может выполнять команды: в образ входит только QtQuick, а запуск
// процессов есть лишь в расширениях на C++. Поэтому она пишет заявку в файл, а
// применяет её служба `platinum-settings` от root.
//
// Показывается при этом не заявка, а состояние, которое опубликовала служба:
// заявку могли отклонить, и выдавать её за факт значило бы врать пользователю.
QtObject {
    id: system

    // Куда служба публикует то, что получилось.
    readonly property string statePath: "/run/platinum/state.conf"

    // Часовой пояс, действующий сейчас.
    property string timezone: "UTC"

    // Синхронизация времени по сети включена.
    property bool ntpEnabled: false

    // Время уже синхронизировано с источником.
    property bool ntpSynchronized: false

    // Формат часов. Живёт только в оболочке: системе он безразличен.
    property bool clock24h: true

    property Settings display: Settings {
        category: "display"
        property alias clock24h: system.clock24h
    }

    // Заявка. Файл лежит в tmpfs: это запрос, а не хранилище настроек, и
    // переживать перезагрузку ему незачем — применённый пояс живёт в системе.
    property Settings request: Settings {
        location: "file:///run/platinum/system.conf"
        category: "request"

        property string timezone: ""
        property bool ntp: true
        property string time: ""
    }

    // Состояние перечитывается редко: меняется оно только по нашей же заявке
    // либо раз в сутки при синхронизации.
    property Timer poll: Timer {
        interval: 5000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: system.reload()
    }

    /// Читает опубликованное службой состояние.
    function reload() {
        const text = read(system.statePath);
        if (text === "") {
            return;
        }

        const zone = field(text, "timezone");
        if (zone !== "") {
            system.timezone = zone;
        }

        system.ntpEnabled = field(text, "ntp_enabled") === "yes";
        system.ntpSynchronized = field(text, "ntp") === "yes";
    }

    /// Просит систему сменить часовой пояс.
    function setTimezone(zone) {
        system.request.timezone = zone;
        // Показываем новое значение сразу: служба подтвердит его через
        // секунду, а до тех пор список без отметки выглядит сломанным.
        system.timezone = zone;
    }

    /// Включает или выключает синхронизацию времени по сети.
    function setNtp(enabled) {
        system.request.ntp = enabled;
        system.ntpEnabled = enabled;
    }

    /// Ставит время вручную; принимается только при выключенной синхронизации.
    function setTime(moment) {
        system.request.time = Qt.formatDateTime(moment, "yyyy-MM-dd hh:mm:ss");
    }

    /// Значение ключа из INI-файла.
    function field(text, key) {
        for (const line of text.split("\n")) {
            if (line.startsWith(key + "=")) {
                return line.substring(key.length + 1).trim();
            }
        }

        return "";
    }

    /// Читает файл; пустая строка, если его нет.
    function read(path) {
        const request = new XMLHttpRequest();
        try {
            request.open("GET", "file://" + path, false);
            request.send(null);
            if (request.status === 0 || request.status === 200) {
                return request.responseText;
            }
        } catch (error) {
            // Службы может не быть — например, в отладочном запуске на macOS.
        }

        return "";
    }
}
