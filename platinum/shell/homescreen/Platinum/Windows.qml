pragma Singleton

import QtQuick
import QtCore

// Реестр открытых окон приложений и запуск новых.
//
// Окна приходят из композитора (Shell.qml): каждый xdg-toplevel регистрируется
// здесь, а WindowHost показывает их содержимое. Запускать процессы оболочка не
// умеет — заявку выполняет служба пользователя (launcher_agent.rs), тем же
// файловым протоколом, что и консоль.
QtObject {
    id: windows

    // Каталог времени выполнения пользователя, /run/user/<uid>.
    readonly property string runtime: StandardPaths.writableLocation(StandardPaths.RuntimeLocation)

    // Открытые окна: { toplevel, surface }. Массив пересоздаётся при каждом
    // изменении: мутаций внутри var-свойства QML не замечает.
    property var list: []

    // Индекс активного окна; −1 — окон нет.
    property int active: -1

    // Заявка отправлена, окна ещё нет.
    property bool launching: false

    // Ограничение ожидания: если приложение не открыло окно — упало или его
    // нет в системе — «запуск» не должен крутиться вечно, а пустая поверхность
    // окон без единого окна выглядит зависшей оболочкой. Поэтому по тайм-ауту
    // пользователь возвращается домой.
    property Timer launchTimeout: Timer {
        interval: 15000
        onTriggered: {
            windows.launching = false;
            if (windows.list.length === 0 && Navigation.surface === "window") {
                Navigation.show("home");
            }
        }
    }

    property Settings request: Settings {
        location: windows.runtime + "/platinum/launch.in"
        category: "launch"

        // Команда уходит в base64: сырую строку пришлось бы разбирать из INI,
        // и кавычки с процентами внутри команды ломали бы разбор.
        property string command: ""

        // Номер заявки. Без него повторный запуск того же приложения не менял
        // бы файл, служба не заметила бы изменения и ничего не сделала.
        property int seq: 0
    }

    /// Отправляет заявку на запуск; окно придёт от композитора.
    function launch(command) {
        windows.launching = true;
        windows.launchTimeout.restart();
        windows.request.command = Qt.btoa(command);
        windows.request.seq = windows.request.seq + 1;
        Navigation.show("window");
    }

    /// Регистрирует новое окно и показывает его.
    function add(toplevel, surface) {
        windows.list = windows.list.concat([{ toplevel: toplevel, surface: surface }]);
        windows.active = windows.list.length - 1;
        windows.launching = false;
        windows.launchTimeout.stop();
        Navigation.show("window");
    }

    /// Убирает окно, когда клиент его закрыл.
    function remove(surface) {
        const rest = windows.list.filter(function (entry) { return entry.surface !== surface; });
        windows.list = rest;
        if (windows.active >= rest.length) {
            windows.active = rest.length - 1;
        }

        // Последнее окно закрылось — возвращаемся домой: и пустая поверхность
        // окон, и пустая карусель выглядят зависшей оболочкой.
        if (rest.length === 0
            && (Navigation.surface === "window" || Navigation.surface === "switcher")) {
            Navigation.show("home");
        }
    }

    /// Делает окно активным и показывает поверхность окон.
    function activate(index) {
        windows.active = index;
        Navigation.show("window");
    }
}
