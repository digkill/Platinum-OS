# Shell / UI

[← Index](README.md)

## Location

```text
platinum/shell/
├── homescreen/
│   ├── Shell.qml              # Wayland compositor scene
│   ├── Home.qml
│   ├── Platinum/              # QML module
│   └── icons/
└── tools/
    ├── dev.sh                 # live QML preview (macOS)
    ├── dev-vm.sh              # sync into Parallels VM
    ├── to-parallels.sh
    └── …
```

## Architecture

The graphical shell is a nested Wayland compositor hosted inside **cage**.
Ubuntu application windows appear as `xdg-toplevel` surfaces in the QML scene.

Application registry comes from `/usr/share/applications/*.desktop`.
Terminal app used by the shell is **qterminal** (protocol compatibility).

Virtual keyboard input is provided via Qt VirtualKeyboard `InputPanel` in
`Shell.qml`.

## Rootfs agents

Installed by `platinum-rootfs` (not under `shell/`):

- `ConsoleAgent`
- `LauncherAgent`
- `SettingsAgent`

They use file-based requests plus systemd user units.

## Board integration

Shell-oriented board variants use `packages-shell.toml`, `system-shell.toml`,
and sometimes larger `partitions-shell.toml`. Example: Zero 3W shell image
points homescreen assets at `../../shell/homescreen`.
