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

    /// Контакты, которым звонили, — от свежих к старым.
    function recents() {
        return store.contacts
            .filter(function (contact) { return contact.lastCalledAt > 0; })
            .sort(function (first, second) {
                return second.lastCalledAt - first.lastCalledAt;
            });
    }

    /// Возвращает переписку с контактом, создавая её при необходимости.
    function threadFor(phone) {
        for (let index = 0; index < store.threads.length; ++index) {
            if (store.threads[index].phone === phone) {
                return index;
            }
        }

        const contact = store.contacts.find(function (item) {
            return item.phone === phone;
        });
        const now = Date.now();

        store.threads = store.threads.concat([{
            title: contact === undefined ? phone : contact.name,
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
    function seedContacts() {
        return [];
    }

    /// Начальные переписки первого запуска.
    function seedThreads() {
        return [];
    }
}
