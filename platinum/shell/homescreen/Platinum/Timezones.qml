pragma Singleton

import QtQuick

// Список часовых поясов системы.
//
// Читается из `/usr/share/zoneinfo/zone1970.tab` — той же таблицы, которой
// пользуется сама система. Свой список пришлось бы обновлять вслед за
// изменениями границ и переводов часов, а он и так лежит в образе.
QtObject {
    id: zones

    // Все пояса, отсортированные. Читаются один раз при первом обращении.
    property var all: []

    // Области верхнего уровня: Europe, Asia, America и прочие.
    readonly property var regions: {
        const seen = [];
        for (const zone of zones.all) {
            const region = zone.split("/")[0];
            if (seen.indexOf(region) === -1) {
                seen.push(region);
            }
        }

        return seen.sort();
    }

    Component.onCompleted: load()

    /// Города выбранной области, без её имени в подписи.
    function cities(region) {
        return zones.all
            .filter(function (zone) { return zone.indexOf(region + "/") === 0; })
            .map(function (zone) {
                return { id: zone, title: zone.substring(region.length + 1).replace(/_/g, " ") };
            });
    }

    /// Читает таблицу поясов.
    function load() {
        // zone1970.tab — современная таблица; zone.tab остаётся на системах,
        // где её ещё нет.
        let text = read("/usr/share/zoneinfo/zone1970.tab");
        if (text === "") {
            text = read("/usr/share/zoneinfo/zone.tab");
        }

        const found = [];
        for (const line of text.split("\n")) {
            if (line === "" || line.startsWith("#")) {
                continue;
            }

            // Колонки: коды стран, координаты, имя пояса, комментарий.
            const columns = line.split("\t");
            if (columns.length < 3) {
                continue;
            }

            const zone = columns[2].trim();
            if (zone !== "" && found.indexOf(zone) === -1) {
                found.push(zone);
            }
        }

        // UTC в таблице нет, но выбрать его нужно уметь: это состояние образа
        // до первой настройки.
        if (found.indexOf("UTC") === -1) {
            found.push("UTC");
        }

        zones.all = found.sort();
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
            // На системе без базы поясов список останется пустым, и экран
            // честно покажет, что выбирать не из чего.
        }

        return "";
    }
}
