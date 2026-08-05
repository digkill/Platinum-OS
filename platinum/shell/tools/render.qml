import QtQuick

// Служебный рендер в PNG для проверки внешнего вида без интерактивного окна.
Item {
    width: 720
    height: 1280

    Loader {
        id: loader
        anchors.fill: parent
        source: "../homescreen/Home.qml"
        onLoaded: grab.start()
    }

    Timer {
        id: grab
        interval: 900
        onTriggered: loader.item.grabToImage(function (result) {
            result.saveToFile("/tmp/platinum-home.png");
            Qt.quit();
        }, Qt.size(720, 1280))
    }
}
