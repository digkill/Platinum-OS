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

    // Единицы раскладки: экран устройства. Всё остальное считается в них.
    readonly property int designWidth: 720
    readonly property int designHeight: 1280

    // Ориентация определяется экраном и меняется на лету: свойства width и
    // height — привязки, поэтому смена разрешения или поворот панели
    // пересобирают раскладку сами, без перезапуска оболочки.
    readonly property bool landscape: width > height

    // Как вести себя на ландшафтном экране:
    //   "adapt"  — оставить портретную раскладку, добрав место полями
    //   "rotate" — развернуть её; для панели, установленной боком
    property string landscapeMode: "adapt"

    readonly property bool rotated: landscape && landscapeMode === "rotate"

    // Масштаб раскладки: по узкой стороне, поэтому содержимое не обрезается.
    //
    // Раньше холст просто занимал экран, а раскладка считалась в пикселях: на
    // мониторе 1024x768 сетка наезжала на док, а низ экрана уходил за край.
    // Поймано на живой машине в Parallels.
    readonly property real canvasScale: rotated
                                        ? Math.min(width / designHeight, height / designWidth)
                                        : Math.min(width / designWidth, height / designHeight)

    // Фон: мягкий градиент, поверх которого «стекло» читается без размытия.
    gradient: Gradient {
        GradientStop { position: 0.0; color: Theme.backgroundTop }
        GradientStop { position: 0.55; color: Theme.backgroundMiddle }
        GradientStop { position: 1.0; color: Theme.backgroundBottom }
    }

    // Время держит состояние устройства: два независимых таймера разошлись бы
    // между часами и строкой состояния.
    readonly property date now: DeviceState.now

    Item {
        id: canvas

        // Холст — это экран, пересчитанный в единицы раскладки. В режиме rotate
        // он остаётся портретным и разворачивается, поэтому размер фиксирован.
        width: home.rotated ? home.designWidth : home.width / home.canvasScale
        height: home.rotated ? home.designHeight : home.height / home.canvasScale
        anchors.centerIn: parent
        rotation: home.rotated ? 90 : 0
        scale: home.canvasScale

        // Содержимое не растягивается на всю ширину монитора: оболочка
        // рассчитана на телефон, и сетка в четыре колонки во весь широкий
        // экран выглядит не «адаптивно», а сломанно. Лишнее место — поля.
        Item {
            id: frame

            width: Math.min(parent.width, home.designWidth * 1.15)
            height: parent.height
            anchors.horizontalCenter: parent.horizontalCenter

            StatusBar {
                id: status
                anchors.top: parent.top
                anchors.left: parent.left
                anchors.right: parent.right
            }

            // Сцена между строкой состояния и доком. Поверхности и приложения
            // делят её целиком, поэтому каждая занимает всё место и
            // переключается видимостью, а не пересчётом привязок.
            Item {
                id: stage

                anchors.top: status.bottom
                anchors.bottom: dock.top
                anchors.bottomMargin: Theme.spacingMedium
                anchors.left: parent.left
                anchors.right: parent.right

                Item {
                    id: homeSurface

                    anchors.fill: parent
                    visible: !Navigation.inApp && Navigation.surface === "home"

                    ClockWidget {
                        id: clock
                        anchors.top: parent.top
                        anchors.topMargin: 18
                        anchors.horizontalCenter: parent.horizontalCenter
                        now: home.now
                    }

                    PlatinumLogo {
                        id: logo
                        anchors.top: clock.bottom
                        anchors.topMargin: 14
                        anchors.horizontalCenter: parent.horizontalCenter
                        width: 300
                        height: 250
                    }

                    PlatinumWordmark {
                        id: wordmark
                        anchors.top: logo.bottom
                        anchors.topMargin: -10
                        anchors.horizontalCenter: parent.horizontalCenter
                    }

                    AppGrid {
                        id: apps
                        anchors.top: wordmark.bottom
                        anchors.topMargin: 26
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.leftMargin: Theme.screenMargin
                        anchors.rightMargin: Theme.screenMargin

                        // Состав приходит из реестра: домашний экран, список
                        // приложений и док обязаны показывать один набор.
                        model: Apps.modules
                        onLaunch: function (id) { Navigation.open(id); }
                    }

                    PageDots {
                        anchors.bottom: parent.bottom
                        anchors.bottomMargin: 4
                        anchors.horizontalCenter: parent.horizontalCenter
                    }
                }

                AppsScreen {
                    anchors.fill: parent
                    visible: !Navigation.inApp && Navigation.surface === "apps"
                }

                AppHost {
                    anchors.fill: parent
                    visible: Navigation.inApp
                }
            }

            Dock {
                id: dock

                anchors.bottom: indicator.top
                anchors.bottomMargin: 14
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.leftMargin: Theme.screenMargin
                anchors.rightMargin: Theme.screenMargin

                model: Apps.dock
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

                // Нажатие на жест-бар возвращает домой: на устройстве без
                // кнопок это единственный выход из приложения, кроме полосы
                // возврата.
                MouseArea {
                    anchors.fill: parent
                    anchors.margins: -18
                    onClicked: Navigation.show("home")
                }
            }
        }
    }
}
