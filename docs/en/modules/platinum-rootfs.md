# platinum-rootfs

[← Modules](README.md)

## Purpose

Prepare Ubuntu Base rootfs: unpack, apt/dpkg in chroot, system files, boot
configuration, shell/agents installation.

## Major components

| Component | Role |
| --- | --- |
| `RootfsSpec`, `RootfsUnpacker`, `RootfsExpander` | Spec + unpack |
| `Chroot` / `ChrootSession` | Controlled chroot sessions |
| `AptInstaller`, `DpkgInstaller`, `PackageSet` | Package install |
| `FirmwareInstaller` | Vendor firmware |
| `SystemConfigurator` | hostname, fstab, users, network, … |
| Boot configurators | extlinux / boot.scr / Raspberry Pi / UEFI |
| Shell / splash / cloud-init helpers | Desktop shell bring-up |
| Agents | `ConsoleAgent`, `LauncherAgent`, `SettingsAgent` |

External tools (`tar`, `chroot`, `apt`, `dpkg`, `mkfs` elsewhere) are invoked
explicitly; business validation stays in Rust types.

## Dependencies

`thiserror`, `tracing`
