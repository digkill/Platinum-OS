import QtQuick
import QtWayland.Compositor

// Переключатель приложений: карусель живых карточек.
//
// Карточки живые, а не снимки: композитор наш, и одну поверхность можно
// рисовать несколькими элементами сразу — приложение в карусели продолжает
// работать и обновляться. Снимок пришлось бы пересоздавать по таймеру, и он
// всё равно врал бы про текущее состояние.
//
// Жесты повторяют привычные по телефонам: карточка выбирается касанием,
// смахивание вверх закрывает приложение, касание мимо карточек уводит домой.
Item {
    id: carousel

    // Размер карточки: доля сцены, чтобы соседние карточки было видно с краёв
    // и переключатель читался как лента, а не как одно окно.
    readonly property real cardWidth: width * 0.62
    readonly property real cardHeight: height * 0.68

    // Затемнение под каруселью: без него живые карточки сливаются с обоями.
    Rectangle {
        anchors.fill: parent
        color: Qt.rgba(0, 0, 0, Theme.dark ? 0.55 : 0.35)

        // Касание мимо карточек — выход домой, как на телефоне.
        MouseArea {
            anchors.fill: parent
            onClicked: Navigation.show("home")
        }
    }

    Text {
        id: caption

        anchors.top: parent.top
        anchors.topMargin: Theme.spacingMedium
        anchors.horizontalCenter: parent.horizontalCenter
        text: Windows.list.length > 0 ? "Открытые приложения" : "Открытых приложений нет"
        font.pixelSize: 15
        font.weight: Font.DemiBold
        color: Theme.statusText
    }

    ListView {
        id: strip

        anchors.top: caption.bottom
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.topMargin: Theme.spacingMedium

        model: Windows.list
        orientation: ListView.Horizontal
        spacing: Theme.spacingMedium
        clip: true

        // Лента останавливается на карточке, а не между ними: половина одной и
        // половина другой не дают понять, что именно откроется касанием.
        snapMode: ListView.SnapOneItem
        highlightRangeMode: ListView.StrictlyEnforceRange
        preferredHighlightBegin: (width - carousel.cardWidth) / 2
        preferredHighlightEnd: (width + carousel.cardWidth) / 2

        // Начинаем с активного приложения: переключатель открывают, чтобы уйти
        // с него, и показывать чужую карточку первой незачем.
        currentIndex: Windows.active

        delegate: Item {
            id: slot

            width: carousel.cardWidth
            height: strip.height

            // Соседние карточки мельче: так видно, какая выбрана, даже когда
            // лента остановилась между делениями.
            readonly property real distance: Math.abs(
                slot.x + slot.width / 2 - strip.contentX - strip.width / 2)
            readonly property real closeness: Math.max(0, 1 - distance / (strip.width * 0.7))

            Item {
                id: card

                width: carousel.cardWidth
                height: carousel.cardHeight
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.verticalCenter: parent.verticalCenter

                scale: 0.86 + 0.14 * slot.closeness
                opacity: 0.5 + 0.5 * slot.closeness
                Behavior on scale { NumberAnimation { duration: 120 } }

                // Смахивание вверх закрывает приложение. Порог в треть высоты:
                // случайное движение пальцем по ленте не должно ничего убивать.
                readonly property real dismissAt: -carousel.cardHeight / 3

                GlassPanel {
                    anchors.fill: parent
                    strong: true
                    radius: 22
                }

                // Предпросмотр: та же поверхность, что и в окне приложения, но
                // без ввода — нажатия по карточке выбирают её, а не попадают
                // внутрь приложения.
                Item {
                    id: preview

                    anchors.fill: parent
                    anchors.margins: 6
                    clip: true

                    ShellSurfaceItem {
                        id: surfaceView

                        shellSurface: modelData.surface
                        inputEventsEnabled: false
                        transformOrigin: Item.TopLeft
                        // Поверхность приходит размером с экран: её нужно
                        // вписать в карточку, сохранив пропорции.
                        scale: (width > 0 && height > 0)
                               ? Math.min(preview.width / width, preview.height / height)
                               : 1
                    }
                }

                Text {
                    anchors.bottom: parent.top
                    anchors.bottomMargin: 6
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: parent.width
                    horizontalAlignment: Text.AlignHCenter
                    elide: Text.ElideRight
                    text: modelData.toplevel.title !== ""
                          ? modelData.toplevel.title
                          : modelData.toplevel.appId
                    font.pixelSize: 13
                    color: Theme.statusText
                }

                MouseArea {
                    anchors.fill: parent

                    drag.target: card
                    drag.axis: Drag.YAxis
                    drag.minimumY: -carousel.cardHeight
                    drag.maximumY: 0

                    onClicked: Windows.activate(index)

                    onReleased: {
                        if (card.y < card.dismissAt) {
                            modelData.toplevel.sendClose();
                        }

                        // Карточка возвращается на место в любом случае: окно
                        // исчезнет само, когда клиент действительно закроется,
                        // а до тех пор оно ещё живо и должно быть видно.
                        card.y = 0;
                    }
                }
            }
        }
    }

    // Пусто — переключать нечего, и единственный осмысленный выход домой.
    ActionButton {
        anchors.centerIn: parent
        visible: Windows.list.length === 0
        text: "На домашний экран"
        onClicked: Navigation.show("home")
    }
}
