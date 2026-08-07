# platinum-rootfs

[← 模块](README.md)

## 目的

准备 Ubuntu Base rootfs：解压、chroot 中 apt/dpkg、系统文件、启动配置、
安装 shell/agents。

## 主要组件

`RootfsSpec` / unpacker、`Chroot`、`AptInstaller`、`DpkgInstaller`、
`FirmwareInstaller`、`SystemConfigurator`、boot configurators
（extlinux / boot.scr / Pi / UEFI）、shell/splash/cloud-init helpers、
agents（`ConsoleAgent`、`LauncherAgent`、`SettingsAgent`）。

外部工具（`tar`、`chroot`、`apt`、`dpkg`）显式调用；业务校验留在 Rust 类型中。

## 依赖

`thiserror`、`tracing`
