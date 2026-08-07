pragma Singleton

import QtQuick
import QtCore

// Выполнение команд для консоли оболочки.
//
// Оболочка процессы не запускает: команду выполняет служба уровня пользователя,
// а не root — консоль не должна повышать права. Всё, что через неё делается,
// пользователь и так может сделать своей учётной записью.
//
// Настоящего терминала здесь нет: псевдотерминала тоже, поэтому `vim`, `top` и
// всё, что перерисовывает экран, работать не будет. Команда выполняется, её
// вывод складывается в файл целиком, консоль его показывает.
QtObject {
    id: console_

    // Каталог времени выполнения пользователя, /run/user/<uid>.
    readonly property string runtime: StandardPaths.writableLocation(StandardPaths.RuntimeLocation)

    readonly property string resultPath: runtime.toString().replace("file://", "")
                                         + "/platinum/console.out"

    // Вывод последней команды.
    property string output: ""

    // Команда отправлена, ответа ещё нет.
    property bool running: false

    property Settings request: Settings {
        location: console_.runtime + "/platinum/console.in"
        category: "console"

        // Команда уходит в base64: сырую строку пришлось бы разбирать из INI,
        // и кавычки с процентами внутри команды ломали бы разбор.
        property string command: ""

        // Номер запроса. Без него повторный запуск той же команды не менял бы
        // файл, служба не заметила бы изменения и ничего не сделала.
        property int seq: 0
    }

    // Опрос идёт только пока ждём ответа: постоянное чтение файла будило бы
    // процессор на устройстве, где консоль открывают раз в неделю.
    property Timer poll: Timer {
        interval: 250
        repeat: true
        running: console_.running
        onTriggered: console_.collect()
    }

    /// Отправляет команду службе.
    function run(command) {
        const trimmed = command.trim();
        if (trimmed === "") {
            return;
        }

        console_.output = "";
        console_.running = true;
        console_.request.command = Qt.btoa(trimmed);
        console_.request.seq = console_.request.seq + 1;
    }

    /// Забирает вывод, если он готов.
    function collect() {
        const text = read(console_.resultPath);
        if (text === "") {
            return;
        }

        // Первая строка — номер запроса. В файле мог остаться ответ предыдущей
        // команды: без проверки он показался бы результатом новой.
        const newline = text.indexOf("\n");
        const header = newline === -1 ? text : text.substring(0, newline);
        if (header !== "#seq " + console_.request.seq) {
            return;
        }

        console_.output = newline === -1 ? "" : text.substring(newline + 1);
        console_.running = false;
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
            // Ответа ещё нет — это обычное состояние сразу после запуска.
        }

        return "";
    }
}
