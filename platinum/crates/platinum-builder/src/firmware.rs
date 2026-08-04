use anyhow::{Context, Result};
use platinum_board::FirmwareConfig;
use platinum_core::{BuildContext, Stage};
use platinum_rootfs::{FirmwareInstaller, FirmwareSpec};

use crate::outputs;

/// Каталог, в котором переиспользуется checkout репозитория firmware.
const FIRMWARE_DIRECTORY: &str = "firmware";

/// Установка vendor-firmware платы в rootfs.
///
/// Stage идёт до сборки BSP: пакет ядра при установке пересобирает initramfs, и
/// firmware, положенный позже, в него бы не попал.
pub struct InstallFirmwareStage {
    installer: FirmwareInstaller,
}

impl InstallFirmwareStage {
    /// Создаёт stage для описания firmware платы.
    pub fn new(spec: FirmwareSpec) -> Self {
        Self {
            installer: FirmwareInstaller::new(spec),
        }
    }
}

impl Stage for InstallFirmwareStage {
    fn name(&self) -> &'static str {
        "install-firmware"
    }

    fn execute(&self, context: &mut BuildContext) -> Result<()> {
        let rootfs = context.require_output(outputs::ROOTFS_DIR)?.to_path_buf();
        let checkout = context.paths().cache_dir.join(FIRMWARE_DIRECTORY);

        self.installer
            .install(&rootfs, &checkout)
            .context("не удалось установить firmware платы")?;

        context.record(outputs::FIRMWARE_CHECKOUT, checkout);

        Ok(())
    }
}

/// Строит описание firmware по board-конфигурации.
pub fn firmware_spec(config: &FirmwareConfig) -> Result<FirmwareSpec> {
    FirmwareSpec::new(
        config.repository.clone(),
        config.revision.clone(),
        config.directories.clone(),
        config.links.clone(),
    )
    .context("некорректное описание firmware")
}
