import QtQuick

// Консоль оболочки: поле команды и вывод последнего запуска.
//
// Это не терминал. Команду выполняет служба пользователя (см. Console.qml),
// вывод приходит целиком по завершении, поэтому интерактивные программы вроде
// `vim` и `top` здесь не работают — и экран не делает вид, что работают.
AppScreen {
    id: screen

    title: "Console"
    subtitle: "Команды выполняются от имени пользователя оболочки, без повышения прав."

    function submit() {
        Console.run(input.text);
    }

    GlassPanel {
        width: parent.width
        height: 56
        radius: 18
        strong: true

        TextInput {
            id: input

            anchors.left: parent.left
            anchors.right: runButton.left
            anchors.leftMargin: Theme.spacingMedium
            anchors.rightMargin: Theme.spacingSmall
            anchors.verticalCenter: parent.verticalCenter
            font.pixelSize: 15
            font.family: "monospace"
            color: Theme.textPrimary
            clip: true
            enabled: !Console.running
            onAccepted: screen.submit()

            // У TextInput нет собственной подсказки — она рисуется поверх.
            Text {
                anchors.verticalCenter: parent.verticalCenter
                visible: input.text === "" && !input.activeFocus
                text: "uname -a"
                font: input.font
                color: Theme.textSecondary
            }
        }

        ActionButton {
            id: runButton

            anchors.right: parent.right
            anchors.rightMargin: Theme.spacingSmall
            anchors.verticalCenter: parent.verticalCenter
            text: Console.running ? "…" : "Run"
            enabled: !Console.running
            onClicked: screen.submit()
        }
    }

    AppCard {
        width: parent.width
        visible: Console.running || Console.output !== ""
        title: "Вывод"
        subtitle: Console.running ? "Команда выполняется…" : ""

        Text {
            width: parent.width
            visible: Console.output !== ""
            text: Console.output
            textFormat: Text.PlainText
            font.family: "monospace"
            font.pixelSize: 13
            color: Theme.textPrimary
            wrapMode: Text.WrapAnywhere
        }
    }
}
