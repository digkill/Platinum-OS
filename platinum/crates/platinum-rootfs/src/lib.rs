//! Подготовка Ubuntu Base rootfs.
//!
//! Спецификация проверяется отдельно от файловых операций: ошибка в release,
//! архитектуре или имени пакета обнаруживается до того, как сборка потратит
//! время на загрузку архива и привилегированные операции в chroot.

mod apt;
mod boot;
mod bootscript;
mod chroot;
mod cloudinit;
mod configure;
mod console_agent;
mod dpkg;
mod firmware;
mod launcher_agent;
mod mounts;
mod packages;
mod raspberrypi;
mod resize;
mod settings_agent;
mod shell;
mod spec;
mod splash;
mod sys;
mod system;
mod uefi;
mod unpack;

pub use apt::{AptError, AptInstaller};
pub use boot::{BootArtifacts, BootConfigurator, BootError, BootSpec};
pub use bootscript::{BootScriptConfigurator, BootScriptError, BootScriptSpec};
pub use chroot::{Chroot, ChrootError, ChrootSession};
pub use cloudinit::{CloudInitConfigurator, CloudInitError, CloudInitSpec};
pub use configure::{ConfigureError, SystemConfigurator};
pub use console_agent::{ConsoleAgent, ConsoleAgentError};
pub use dpkg::{DpkgError, DpkgInstaller};
pub use firmware::{FirmwareError, FirmwareInstaller, FirmwareSpec};
pub use launcher_agent::{LauncherAgent, LauncherAgentError};
pub use mounts::{MountError, ensure_nothing_mounted, mounts_under};
pub use packages::{PackageError, PackageSet};
pub use raspberrypi::{RaspberryPiConfigurator, RaspberryPiError, RaspberryPiSpec};
pub use resize::{ResizeError, RootfsExpander};
pub use settings_agent::{SettingsAgent, SettingsAgentError};
pub use shell::{ShellConfigurator, ShellError, ShellSpec};
pub use spec::{RootfsError, RootfsSpec};
pub use splash::{SplashConfigurator, SplashError, SplashSpec};
pub use system::{Filesystem, SystemError, SystemSpec, User, WifiNetwork};
pub use uefi::{UefiConfigurator, UefiError, UefiSpec};
pub use unpack::{RootfsUnpacker, UnpackError};
