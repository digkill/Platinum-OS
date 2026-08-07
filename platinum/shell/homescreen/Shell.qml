import QtQuick
import QtWayland.Compositor
import QtWayland.Compositor.XdgShell
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
