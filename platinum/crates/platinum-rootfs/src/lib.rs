//! Подготовка Ubuntu Base rootfs.
//!
//! Спецификация проверяется отдельно от файловых операций: ошибка в release,
//! архитектуре или имени пакета обнаруживается до того, как сборка потратит
//! время на загрузку архива и привилегированные операции в chroot.

mod apt;
mod boot;
mod bootscript;
mod chroot;
mod configure;
mod dpkg;
mod firmware;
mod packages;
mod raspberrypi;
mod resize;
mod spec;
mod sys;
mod system;
mod unpack;

pub use apt::{AptError, AptInstaller};
pub use boot::{BootArtifacts, BootConfigurator, BootError, BootSpec};
pub use bootscript::{BootScriptConfigurator, BootScriptError, BootScriptSpec};
pub use chroot::{Chroot, ChrootError, ChrootSession};
pub use configure::{ConfigureError, SystemConfigurator};
pub use dpkg::{DpkgError, DpkgInstaller};
pub use firmware::{FirmwareError, FirmwareInstaller, FirmwareSpec};
pub use packages::{PackageError, PackageSet};
pub use raspberrypi::{RaspberryPiConfigurator, RaspberryPiError, RaspberryPiSpec};
pub use resize::{ResizeError, RootfsExpander};
pub use spec::{RootfsError, RootfsSpec};
pub use system::{Filesystem, SystemError, SystemSpec, User};
pub use unpack::{RootfsUnpacker, UnpackError};
