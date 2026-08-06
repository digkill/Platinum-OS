#!/bin/sh
# Быстрый цикл разработки на работающей виртуальной машине.
#
# Полная пересборка образа занимает десятки минут, а правка оболочки или темы
# загрузки — секунды. Скрипт копирует изменённое прямо в запущенную машину и
# перезапускает то, что нужно.
#
# Так проверяется всё, чего не видно на macOS: сессия, композитор, заставка,
# поведение служб. Собирать образ нужно только под выпуск.
set -eu

VM="${VM:-platinum@platinum-vm.local}"
HERE=$(cd "$(dirname "$0")/.." && pwd)
WHAT="${1:-shell}"

ask() {
    # Пароль ставится сборкой и одинаков у всех образов разработки.
    SSHPASS_HELPER=$(mktemp)
    printf '#!/bin/sh\nprintf %%s %s\n' "'platinum'" > "$SSHPASS_HELPER"
    chmod 700 "$SSHPASS_HELPER"
    SSH_ASKPASS="$SSHPASS_HELPER" SSH_ASKPASS_REQUIRE=force \
    ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null \
        -o PreferredAuthentications=password -o PubkeyAuthentication=no \
        -o ConnectTimeout=15 "$VM" "$@"
    rm -f "$SSHPASS_HELPER"
}

case "$WHAT" in
    shell)
        echo "Обновляю домашний экран на $VM..."
        tar -C "$HERE" -cf - homescreen |
            ask 'sudo tar -C /usr/share/platinum -xf - --strip-components=1'
        # Сессия перезапускается целиком: composer держит QML в памяти.
        ask 'sudo systemctl restart sddm'
        echo "Готово: оболочка перезапущена."
        ;;
    splash)
        echo "Обновляю заставку на $VM..."
        ask 'sudo plymouthd --debug --debug-file=/tmp/plymouth.log 2>/dev/null || true'
        ask 'sudo update-initramfs -u'
        echo "Готово: перезагрузите машину, чтобы увидеть заставку."
        ;;
    log)
        # Причину, по которой Plymouth ушёл на текстовую тему, видно только здесь.
        ask 'sudo journalctl -b -u plymouth-start --no-pager | tail -30; \
             echo "--- отладка темы ---"; \
             sudo cat /tmp/plymouth.log 2>/dev/null | tail -40'
        ;;
    *)
        echo "использование: $0 [shell|splash|log]" >&2
        exit 1
        ;;
esac
