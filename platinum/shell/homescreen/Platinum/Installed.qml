pragma Singleton

import QtQuick
import Qt.labs.folderlistmodel

// Приложения, установленные в системе.
//
// Состав читается с диска при запуске оболочки, а не перечисляется в реестре:
// иначе установленное из репозитория приложение не появилось бы на устройстве
// без правки самой оболочки. Формат — стандартные `.desktop`, которые несут
// все пакеты Ubuntu, поэтому `apt install` любого приложения достаточно.
//
// Значка из темы иконок здесь нет намеренно: разбор `Icon=` требует обхода
// каталогов темы по спецификации, а свои значки у нас есть только для своих
// приложений. Остальные показываются буквой — это честнее пустой плитки.
QtObject {
    id: installed

    // Каталог по стандарту XDG. Пользовательский (`~/.local/share`) не
    // читается: приложения ставит пакетный менеджер, а не пользователь.
    readonly property string directory: "/usr/share/applications"

    // Найденные приложения: те же поля, что у записей реестра.
    property var list: []

    // Язык интерфейса: `.desktop` несут переводы имён, и показывать английское
    // имя рядом с русскими подписями оболочки незачем.
    readonly property string language: Qt.locale().name.split("_")[0]

    property FolderListModel folder: FolderListModel {
        folder: "file://" + installed.directory
        nameFilters: ["*.desktop"]
        showDirs: false
        showHidden: false
        sortField: FolderListModel.Name

        onStatusChanged: {
            if (status === FolderListModel.Ready) {
                installed.scan();
            }
        }
    }

    /// Перечитывает каталог приложений.
    function scan() {
        const found = [];

        for (let index = 0; index < installed.folder.count; index += 1) {
            const entry = parse(installed.folder.get(index, "fileURL").toString(),
                                installed.folder.get(index, "fileBaseName"));
            if (entry !== null) {
                found.push(entry);
            }
        }

        found.sort(function (left, right) { return left.title.localeCompare(right.title); });
        installed.list = found;
    }

    /// Читает `.desktop` и возвращает запись реестра либо null.
    function parse(url, baseName) {
        const text = read(url);
        if (text === "") {
            return null;
        }

        const fields = {};
        const lines = text.split("\n");
        let inside = false;

        for (let index = 0; index < lines.length; index += 1) {
            const line = lines[index].trim();

            // Читается только группа [Desktop Entry]: дальше идут действия
            // приложения со своими Name и Exec, и они не отдельные записи.
            if (line.startsWith("[")) {
                if (inside) {
                    break;
                }
                inside = line === "[Desktop Entry]";
                continue;
            }

            if (!inside || line === "" || line.startsWith("#")) {
                continue;
            }

            const split = line.indexOf("=");
            if (split > 0) {
                fields[line.substring(0, split).trim()] = line.substring(split + 1).trim();
            }
        }

        if (!accepts(fields)) {
            return null;
        }

        const title = localized(fields, "Name") || baseName;

        return {
            id: "desktop:" + baseName,
            title: title,
            description: localized(fields, "Comment") || "Приложение системы.",
            exec: command(fields["Exec"]),
            // Значок темы не разбирается: показывается первая буква имени.
            icon: "",
            glyph: title.substring(0, 1).toUpperCase()
        };
    }

    /// Решает, показывать ли запись пользователю.
    function accepts(fields) {
        if (fields["Type"] !== undefined && fields["Type"] !== "Application") {
            return false;
        }

        // NoDisplay — служебные записи вроде обработчиков типов файлов;
        // Hidden означает «удалено пользователем».
        if (fields["NoDisplay"] === "true" || fields["Hidden"] === "true") {
            return false;
        }

        // Terminal=true — программе нужен терминал, сама окна она не создаёт:
        // запуск дал бы бесконечное «Запуск…» вместо приложения.
        if (fields["Terminal"] === "true") {
            return false;
        }

        return command(fields["Exec"]) !== "";
    }

    /// Возвращает перевод поля под язык интерфейса.
    function localized(fields, key) {
        const translated = fields[key + "[" + installed.language + "]"];

        return translated !== undefined ? translated : (fields[key] !== undefined ? fields[key] : "");
    }

    /// Очищает `Exec` от подстановок спецификации.
    ///
    /// `%U`, `%f` и прочие подставляет запускающий: файлов и ссылок мы не
    /// передаём, а оставленные как есть, они попали бы в командную строку
    /// приложения буквально.
    function command(exec) {
        if (exec === undefined || exec === "") {
            return "";
        }

        return exec.replace(/%[a-zA-Z]/g, "").replace(/\s+/g, " ").trim();
    }

    /// Читает файл; пустая строка, если его нет.
    function read(url) {
        const request = new XMLHttpRequest();
        try {
            request.open("GET", url, false);
            request.send(null);
            if (request.status === 0 || request.status === 200) {
                return request.responseText;
            }
        } catch (error) {
            // Нечитаемый файл — не повод ронять весь список.
        }

        return "";
    }
}
