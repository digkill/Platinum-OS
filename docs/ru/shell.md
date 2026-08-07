# Shell / UI

[← Содержание](README.md)

## Расположение

```text
platinum/shell/
├── homescreen/
│   ├── Shell.qml
│   ├── Home.qml
│   ├── Platinum/
│   └── icons/
└── tools/
    ├── dev.sh
    ├── dev-vm.sh
    └── …
```

## Архитектура

Графическая оболочка — вложенный Wayland-композитор внутри **cage**. Окна
Ubuntu появляются как `xdg-toplevel` в QML-сцене.

Реестр приложений — `/usr/share/applications/*.desktop`.
Терминал оболочки — **qterminal**.
Экранная клавиатура — Qt VirtualKeyboard `InputPanel` в `Shell.qml`.

## Агенты rootfs

Ставятся через `platinum-rootfs`: `ConsoleAgent`, `LauncherAgent`,
`SettingsAgent` (file-based requests + systemd user units).

## Интеграция с платами

Shell-варианты используют `packages-shell.toml`, `system-shell.toml` и иногда
больший `partitions-shell.toml`.
