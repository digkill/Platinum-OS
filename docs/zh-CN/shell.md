# Shell / UI

[← 目录](README.md)

## 位置

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

## 架构

图形 Shell 是运行在 **cage** 内的嵌套 Wayland compositor。Ubuntu 应用窗口以
`xdg-toplevel` 形式出现在 QML 场景中。

应用注册表来自 `/usr/share/applications/*.desktop`。
Shell 使用的终端是 **qterminal**。
虚拟键盘由 `Shell.qml` 中的 Qt VirtualKeyboard `InputPanel` 提供。

## Rootfs agents

由 `platinum-rootfs` 安装：`ConsoleAgent`、`LauncherAgent`、`SettingsAgent`
（基于文件的请求 + systemd user units）。

## 与板配置集成

Shell 变体使用 `packages-shell.toml`、`system-shell.toml`，有时还有更大的
`partitions-shell.toml`。
