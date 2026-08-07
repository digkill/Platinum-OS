import QtQuick
import QtWayland.Compositor
import QtWayland.Compositor.XdgShell
import QtQuick.VirtualKeyboard
import Platinum

// Корень оболочки: вложенный Wayland-композитор.
//
// Оболочка сама принимает окна приложений — каждый xdg-toplevel становится
// обычным QML-элементом в её сцене (см. WindowHost). Cage остаётся снаружи и
// даёт DRM, ввод и seat: вложенному композитору не нужно уметь ни того, ни
// другого, поэтому переход на прямой запуск по eglfs, когда появится драйвер
// GPU, не потребует менять эту схему — только способ запуска.
WaylandCompositor {
    id: compositor

    // Свой сокет, а не wayland-0: оболочка сама живёт клиентом внутри cage, и
    // совпадение имён увело бы приложения в cage мимо сцены оболочки.
    socketName: "platinum-0"

    WaylandOutput {
        sizeFollowsWindow: true

        window: Window {
            width: 720
            height: 1280
            visible: true

            Home {
                anchors.fill: parent
            }

            // Экранная клавиатура. Живёт у композитора, а не внутри Home:
            // отсюда она обслуживает и поля самой оболочки, и окна приложений
            // Ubuntu — клиент получает ввод по протоколу text-input, и вторая
            // клавиатура внутри каждого приложения не нужна.
            //
            // Панель поверх всего и вне холста Home: холст масштабируется под
            // разрешение экрана, а клавиатуру пальцу надо показывать в
            // настоящих пикселях, иначе на мониторе она уезжает вместе с ним.
            InputPanel {
                id: keyboard

                anchors.left: parent.left
                anchors.right: parent.right
                z: 100

                // Панель выезжает снизу и не занимает места, когда не нужна.
                y: active ? parent.height - height : parent.height
                Behavior on y { NumberAnimation { duration: 140 } }
            }
        }
    }

    XdgShell {
        onToplevelCreated: function (toplevel, xdgSurface) {
            Windows.add(toplevel, xdgSurface);
        }
    }

    // Клиентов просят не рисовать собственные заголовки: окно занимает сцену
    // целиком, и чужая полоса заголовка выглядела бы второй строкой состояния.
    XdgDecorationManagerV1 {
        preferredMode: XdgToplevel.ServerSideDecoration
    }
}
