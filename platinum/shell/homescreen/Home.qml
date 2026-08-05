import QtQuick
import Platinum

// Домашний экран Platinum OS.
//
// Разметка вертикальная и привязана к краям: оболочка обязана одинаково
// смотреться на 720x1280 панели устройства и в окне QEMU другого размера,
// поэтому фиксированных координат здесь нет.
Rectangle {
    id: home

    // Размер окна: под панелью устройства это её разрешение, под HDMI —
    // разрешение монитора. Компоновка от него не зависит, см. canvas.
    width: 720
    height: 1280

    // Холст фиксированной пропорции, вписанный в экран.
    //
    // Без него на ландшафтном мониторе сетка растягивается по всей ширине, а
    // карточка Focus и док наезжают друг на друга — проверено запуском в QEMU
    // на 1920x1080. Экран устройства портретный, поэтому компоновка считается
    // в его координатах, а на чужом разрешении холст масштабируется целиком.
    readonly property int designWidth: 720
    readonly property int designHeight: 1280

    // Оболочка портретная по своей природе. На ландшафтном экране — а это
    // любой обычный HDMI-монитор — холст разворачивается, а не растягивается:
    // растянутая раскладка ломается, проверено в QEMU на 1920x1080.
    //
    // Монитор при этом ставится вертикально; на портретной панели устройства
    // поворот не нужен и не применяется.
    readonly property bool rotated: width > height

    // Фон: мягкий градиент, поверх которого «стекло» читается без размытия.
    gradient: Gradient {
        GradientStop { position: 0.0; color: Theme.backgroundTop }
        GradientStop { position: 0.55; color: Theme.backgroundMiddle }
        GradientStop { position: 1.0; color: Theme.backgroundBottom }
    }

    property date now: new Date()

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: home.now = new Date()
    }

    Item {
        id: canvas

        width: home.designWidth
        height: home.designHeight
        anchors.centerIn: parent
        rotation: home.rotated ? 90 : 0

        // Вписываем по меньшей стороне: пропорция сохраняется, поля остаются
        // залитыми фоном родителя. При повороте стороны экрана меняются
        // местами, иначе холст вылез бы за края.
        scale: home.rotated
               ? Math.min(home.width / height, home.height / width)
               : Math.min(home.width / width, home.height / height)

        StatusBar {
            id: status
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            time: Qt.formatTime(home.now, "HH:mm")
        }

        ClockWidget {
            id: clock
            anchors.top: status.bottom
            anchors.topMargin: 56
            anchors.horizontalCenter: parent.horizontalCenter
            now: home.now
        }

        PlatinumLogo {
            id: logo
            anchors.top: clock.bottom
            anchors.topMargin: 28
            anchors.horizontalCenter: parent.horizontalCenter
            width: 300
            height: 260
        }

        AppGrid {
            id: apps
            anchors.top: logo.bottom
            anchors.topMargin: 24
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: Theme.screenMargin
            anchors.rightMargin: Theme.screenMargin

            model: [
                { label: "Calendar",     glyph: "\u{1F4C5}" },
                { label: "Clock",        glyph: "\u{1F551}" },
                { label: "Contacts",     glyph: "\u{1F464}" },
                { label: "Platinum OS",  glyph: "◎",    color: Theme.accentSoft },

                { label: "AI Assistant", glyph: "●",    color: Theme.accent },
                { label: "Messages",     glyph: "\u{1F4AC}" },
                { label: "Files",        glyph: "\u{1F4C1}" },
                { label: "Notes",        glyph: "\u{1F4DD}" },

                { label: "Gallery",      glyph: "\u{1F5BC}" },
                { label: "Settings",     glyph: "⚙",    color: "#5b5b6b" },
                { label: "Security",     glyph: "\u{1F6E1}" },
                { label: "Store",        glyph: "\u{1F6CD}" }
            ]
        }

        FocusCard {
            id: focus
            anchors.top: apps.bottom
            anchors.topMargin: 26
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: Theme.screenMargin
            anchors.rightMargin: Theme.screenMargin
        }

        Dock {
            anchors.bottom: indicator.top
            anchors.bottomMargin: 14
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: Theme.screenMargin
            anchors.rightMargin: Theme.screenMargin

            model: [
                { label: "Home",     glyph: "\u{1F3E0}" },
                { label: "Apps",     glyph: "☷",    color: "#5b5b6b" },
                { label: "Phone",    glyph: "\u{1F4DE}" },
                { label: "Messages", glyph: "\u{1F4AC}" }
            ]
        }

        // Жест-бар: место под системный жест «домой».
        Rectangle {
            id: indicator
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 10
            anchors.horizontalCenter: parent.horizontalCenter
            width: parent.width * 0.36
            height: 5
            radius: 2.5
            color: Qt.rgba(0.25, 0.25, 0.35, 0.45)
        }
    }

}
