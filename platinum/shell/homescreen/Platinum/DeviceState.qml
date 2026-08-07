pragma Singleton

import QtQuick

// Состояние устройства: заряд, сеть, время.
//
// Перенос `device.rs` из GTK-оболочки. Значения читаются из sysfs, а не
// задаются разметкой: статус-бар с выдуманным зарядом выглядит рабочим и
// поэтому опаснее пустого.
//
// Чтение идёт через XMLHttpRequest по `file://`. Это единственный способ
// добраться до файловой системы из чистого QML; альтернатива — расширение на
// C++, а оно потребовало бы сборки под каждую плату.
QtObject {
    id: device

    // Заряд в долях единицы и признак зарядки.
    property real battery: 1.0
    property bool charging: false

    // Уровень сигнала 0..4 и тип подключения.
    property int signalLevel: 0
    property string network: "offline"   // wifi | ethernet | lte | offline
    readonly property bool online: network !== "offline"

    property date now: new Date()

    // Формат часов — настройка оболочки, а не системы, поэтому он берётся из
    // SystemSettings, а не из локали.
    readonly property string timeLabel: Qt.formatTime(now,
                                                      SystemSettings.clock24h ? "HH:mm"
                                                                              : "h:mm AP")
    readonly property string dateLabel: Qt.formatDate(now, "ddd, dd MMM")

    // Часы идут каждую секунду, состояние железа опрашивается реже: заряд и
    // сеть меняются медленно, а лишний опрос sysfs будит процессор.
    property Timer clock: Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: device.now = new Date()
    }

    property Timer hardware: Timer {
        interval: 10000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: device.refresh()
    }

    /// Перечитывает состояние железа.
    function refresh() {
        readBattery();
        readNetwork();
    }

    /// Читает файл sysfs; возвращает пустую строку, если его нет.
    function readFile(path) {
        const request = new XMLHttpRequest();
        try {
            request.open("GET", "file://" + path, false);
            request.send(null);
            // Локальный файл отдаётся со статусом 0, а не 200.
            if (request.status === 0 || request.status === 200) {
                return request.responseText.trim();
            }
        } catch (error) {
            // Отсутствие файла — обычное дело: у виртуальной машины нет
            // батареи, у платы без модема нет сотовой сети.
        }

        return "";
    }

    /// Заряд из первого источника питания типа Battery.
    function readBattery() {
        // Имена источников зависят от платы, поэтому проверяются известные.
        const candidates = ["BAT0", "BAT1", "battery", "axp20x-battery"];

        for (const name of candidates) {
            const base = "/sys/class/power_supply/" + name;
            const capacity = readFile(base + "/capacity");
            if (capacity === "") {
                continue;
            }

            device.battery = Math.max(0, Math.min(100, parseInt(capacity, 10))) / 100;
            device.charging = readFile(base + "/status") === "Charging";

            return;
        }

        // Машина без батареи: показывать разряд было бы враньём.
        device.battery = 1.0;
        device.charging = true;
    }

    /// Тип подключения и уровень сигнала.
    function readNetwork() {
        // Проводное соединение приоритетнее: если оно есть, беспроводное
        // состояние пользователю неинтересно.
        for (const name of ["eth0", "end0", "enp0s5", "enp0s1"]) {
            if (readFile("/sys/class/net/" + name + "/operstate") === "up") {
                device.network = "ethernet";
                device.signalLevel = 4;

                return;
            }
        }

        for (const name of ["wlan0", "wlp1s0"]) {
            if (readFile("/sys/class/net/" + name + "/operstate") === "up") {
                device.network = "wifi";
                device.signalLevel = wirelessSignal(name);

                return;
            }
        }

        device.network = "offline";
        device.signalLevel = 0;
    }

    /// Уровень сигнала Wi-Fi 0..4 по данным ядра.
    function wirelessSignal(name) {
        // /proc/net/wireless держит качество связи в третьей колонке строки
        // интерфейса. Формат стабилен десятилетиями и не требует утилит.
        const table = readFile("/proc/net/wireless");
        for (const line of table.split("\n")) {
            if (!line.startsWith(name + ":")) {
                continue;
            }

            const fields = line.split(/\s+/);
            const quality = parseFloat(fields[2]);
            if (isNaN(quality)) {
                break;
            }

            // Качество приходит в шкале 0..70.
            return Math.max(1, Math.min(4, Math.round(quality / 70 * 4)));
        }

        return 3;
    }
}
