pragma Singleton

import QtQuick
import QtCore

// Контакты и переписка устройства.
//
// Перенос `app_store.rs` из GTK-оболочки. Хранилище живёт в оболочке, а не в
// приложениях: звонок из списка контактов и сообщение в переписке трогают одни
// и те же записи, и две независимые копии разошлись бы.
//
// Данные сохраняются в настройках Qt, а не в своём файле: формат уже решает
// вопросы атомарной записи и прав доступа, а объём здесь — единицы килобайт.
QtObject {
    id: store

    // Записи держатся в свойствах, чтобы разметка перерисовывалась сама.
    property var contacts: []
    property var threads: []

    property Settings settings: Settings {
        category: "store"
        property string contacts: ""
        property string threads: ""
    }

    Component.onCompleted: load()

    /// Читает сохранённое или заполняет начальными записями.
    function load() {
        store.contacts = parse(settings.contacts, seedContacts());
        store.threads = parse(settings.threads, seedThreads());
    }

    /// Разбирает JSON, возвращая запасное значение при любой ошибке.
    ///
    /// Повреждённое хранилище не должно ронять оболочку: устройство обязано
    /// загрузиться, пусть и с пустым списком.
    function parse(text, fallback) {
        if (text === "") {
            return fallback;
        }

        try {
            const value = JSON.parse(text);

            return Array.isArray(value) ? value : fallback;
        } catch (error) {
            console.warn("store: повреждённая запись, взяты начальные данные");

            return fallback;
        }
    }

    /// Сохраняет обе таблицы.
    function save() {
        settings.contacts = JSON.stringify(store.contacts);
        settings.threads = JSON.stringify(store.threads);
    }

    /// Добавляет контакт и возвращает его.
    function addContact(name, phone, note) {
        const now = Date.now();
        const contact = {
            name: name,
            phone: phone,
            note: note === undefined ? "" : note,
            createdAt: now,
            updatedAt: now,
            lastCalledAt: 0,
            lastMessageAt: 0
        };

        store.contacts = store.contacts.concat([contact]);
        save();

        return contact;
    }

    /// Отмечает звонок: список недавних строится по этой метке.
    function markCalled(phone) {
        store.contacts = store.contacts.map(function (contact) {
            if (contact.phone !== phone) {
                return contact;
            }

            return Object.assign({}, contact, {
                lastCalledAt: Date.now(),
                updatedAt: Date.now()
            });
        });
        save();
    }

    // Производные списки — свойства, а не функции: результат вызова функции
    // разметка не пересчитывает, и список недавних вызовов не обновлялся бы
    // после звонка, пока экран не откроют заново.

    /// Контакты, которым звонили, — от свежих к старым.
    readonly property var recents: store.contacts
        .filter(function (contact) { return contact.lastCalledAt > 0; })
        .sort(function (first, second) {
            return second.lastCalledAt - first.lastCalledAt;
        })

    /// Контакты по алфавиту: порядок хранения — порядок добавления, а список
    /// людей, меняющий порядок при каждом звонке, нечитаем.
    readonly property var sorted: store.contacts.slice().sort(function (first, second) {
        return first.name.localeCompare(second.name);
    })

    /// Возвращает переписку с контактом, создавая её при необходимости.
    function threadFor(phone, name) {
        for (let index = 0; index < store.threads.length; ++index) {
            if (store.threads[index].phone === phone) {
                return index;
            }
        }

        const contact = store.contacts.find(function (item) {
            return item.phone === phone;
        });
        const title = contact !== undefined ? contact.name
                    : (name === undefined || name === "" ? phone : name);
        const now = Date.now();

        store.threads = store.threads.concat([{
            title: title,
            phone: phone,
            status: "",
            preview: "",
            kind: "direct",
            messages: [],
            createdAt: now,
            lastMessageAt: now
        }]);
        save();

        return store.threads.length - 1;
    }

    /// Дописывает исходящее сообщение в переписку.
    function appendMessage(index, body, outgoing) {
        if (index < 0 || index >= store.threads.length) {
            return;
        }

        const now = Date.now();
        const message = {
            author: outgoing ? "" : store.threads[index].title,
            body: body,
            outgoing: outgoing,
            createdAt: now
        };

        store.threads = store.threads.map(function (thread, position) {
            if (position !== index) {
                return thread;
            }

            return Object.assign({}, thread, {
                messages: thread.messages.concat([message]),
                // Предпросмотр держится рядом с сообщениями: список переписок
                // иначе пришлось бы разворачивать целиком ради одной строки.
                preview: body,
                lastMessageAt: now
            });
        });
        save();
    }

    /// Начальные контакты первого запуска.
    ///
    /// Пустой список выглядел бы поломкой: первый запуск показывал бы пустые
    /// экраны, и отличить «нет данных» от «не работает хранилище» было бы
    /// нельзя. Записи перезаписываются, как только появляются настоящие.
    function seedContacts() {
        const now = Date.now();

        return [
            seedContact("Operator Desk", "+7 912 440 77 90", "Primary", now - 240000),
            seedContact("Base Station", "+7 912 440 12 01", "Pinned", now - 180000),
            seedContact("Field Team", "+7 912 440 53 20", "Shared", now - 120000),
            seedContact("Emergency Link", "112", "Priority", now - 60000)
        ];
    }

    /// Один начальный контакт.
    function seedContact(name, phone, note, timestamp) {
        return {
            name: name,
            phone: phone,
            note: note,
            createdAt: timestamp,
            updatedAt: timestamp,
            lastCalledAt: 0,
            lastMessageAt: 0
        };
    }

    /// Начальные переписки первого запуска.
    function seedThreads() {
        const now = Date.now();

        return [
            {
                title: "Field Team",
                phone: "+7 912 440 53 20",
                status: "3 unread",
                preview: "Rendezvous updated for 22:30, confirm route lock.",
                kind: "group",
                messages: [
                    seedMessage("Field Team", "Rendezvous updated for 22:30.", false, now - 300000),
                    seedMessage("", "Received. Syncing route and battery window.", true, now - 240000),
                    seedMessage("Field Team", "Confirm route lock when ready.", false, now - 180000)
                ],
                createdAt: now - 300000,
                lastMessageAt: now - 180000
            },
            {
                title: "Contacts Queue",
                phone: "",
                status: "Updated",
                preview: "Three follow-ups extracted from recent assistant activity.",
                kind: "system",
                messages: [
                    seedMessage("Contacts Queue",
                                "Three follow-ups extracted from assistant context.",
                                false, now - 160000),
                    seedMessage("", "Queue them for morning review.", true, now - 130000)
                ],
                createdAt: now - 160000,
                lastMessageAt: now - 130000
            },
            {
                title: "Relay Bridge",
                phone: "",
                // Состояние моста зависит от сети: на устройстве без связи
                // «Linked» ввело бы в заблуждение.
                status: DeviceState.online ? "Linked" : "Standby",
                preview: "Relay tunnel prepared for device-to-device handoff.",
                kind: "relay",
                messages: [
                    seedMessage("Relay Bridge", "Relay tunnel prepared for device handoff.",
                                false, now - 110000),
                    seedMessage("Relay Bridge",
                                DeviceState.online
                                    ? "Transport available. Ready for outbound sync."
                                    : "Transport unavailable. Waiting for network.",
                                false, now - 90000)
                ],
                createdAt: now - 110000,
                lastMessageAt: now - 90000
            }
        ];
    }

    /// Одно начальное сообщение.
    function seedMessage(author, body, outgoing, timestamp) {
        return {
            author: author,
            body: body,
            outgoing: outgoing,
            createdAt: timestamp
        };
    }
}
