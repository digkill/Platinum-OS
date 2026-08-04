use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use platinum_armbian_bsp::{ArmbianBspRunner, ArmbianCheckout, BspInventory};
use platinum_board::{ArmbianConfig, BoardConfig};
use platinum_core::{BuildContext, Stage};
use platinum_rootfs::{Chroot, DpkgInstaller};
use tracing::warn;

use crate::outputs;

/// Каталог pinned Armbian checkout внутри cache-dir сборки.
///
/// Checkout лежит в cache, а не в work: он занимает гигабайты и переживает
/// отдельные сборки, тогда как work-dir можно удалять целиком.
const ARMBIAN_DIRECTORY: &str = "armbian";

/// Возвращает путь к Armbian checkout для заданного cache-каталога.
pub fn armbian_checkout_dir(cache_dir: &Path) -> PathBuf {
    cache_dir.join(ARMBIAN_DIRECTORY)
}

/// Синхронизация pinned Armbian Build в cache сборки.
pub struct BspSyncStage {
    config: ArmbianConfig,
}

impl BspSyncStage {
    /// Создаёт stage для Armbian-конфигурации платы.
    pub fn new(config: ArmbianConfig) -> Self {
        Self { config }
    }
}

impl Stage for BspSyncStage {
    fn name(&self) -> &'static str {
        "bsp-sync"
    }

    fn execute(&self, context: &mut BuildContext) -> Result<()> {
        let checkout_dir = armbian_checkout_dir(&context.paths().cache_dir);

        ArmbianCheckout::new(checkout_dir.clone(), self.config.clone())
            .context("некорректная Armbian-конфигурация платы")?
            .sync()
            .context("не удалось синхронизировать pinned Armbian checkout")?;

        context.record(outputs::BSP_CHECKOUT, checkout_dir);

        Ok(())
    }
}

/// Сборка kernel и DTB официальным target Armbian.
pub struct BspKernelStage {
    config: ArmbianConfig,
}

impl BspKernelStage {
    /// Создаёт stage для Armbian-конфигурации платы.
    pub fn new(config: ArmbianConfig) -> Self {
        Self { config }
    }
}

impl Stage for BspKernelStage {
    fn name(&self) -> &'static str {
        "bsp-kernel"
    }

    fn execute(&self, context: &mut BuildContext) -> Result<()> {
        let checkout_dir = context.require_output(outputs::BSP_CHECKOUT)?.to_path_buf();

        ArmbianBspRunner::new(checkout_dir, self.config.clone())
            .context("не удалось подготовить Armbian BSP runner")?
            .build_kernel()
            .context("Armbian не смог собрать kernel и DTB")?;

        Ok(())
    }
}

/// Сборка U-Boot официальным target Armbian.
pub struct BspUbootStage {
    config: ArmbianConfig,
}

impl BspUbootStage {
    /// Создаёт stage для Armbian-конфигурации платы.
    pub fn new(config: ArmbianConfig) -> Self {
        Self { config }
    }
}

impl Stage for BspUbootStage {
    fn name(&self) -> &'static str {
        "bsp-uboot"
    }

    fn execute(&self, context: &mut BuildContext) -> Result<()> {
        let checkout_dir = context.require_output(outputs::BSP_CHECKOUT)?.to_path_buf();

        ArmbianBspRunner::new(checkout_dir, self.config.clone())
            .context("не удалось подготовить Armbian BSP runner")?
            .build_uboot()
            .context("Armbian не смог собрать U-Boot")?;

        Ok(())
    }
}

/// Поиск собранных Armbian пакетов ядра и DTB.
pub struct BspInventoryStage {
    board: BoardConfig,
}

impl BspInventoryStage {
    /// Создаёт stage для конкретной платы.
    pub fn new(board: BoardConfig) -> Self {
        Self { board }
    }
}

impl Stage for BspInventoryStage {
    fn name(&self) -> &'static str {
        "bsp-inventory"
    }

    fn execute(&self, context: &mut BuildContext) -> Result<()> {
        let checkout_dir = context.require_output(outputs::BSP_CHECKOUT)?.to_path_buf();

        let inventory = BspInventory::for_board(&checkout_dir, &self.board);

        let inventory = inventory
            .context("плата не использует Armbian: секция [armbian] в board.toml отсутствует")?;

        let artifacts = inventory
            .kernel_artifacts()
            .context("не удалось найти артефакты Armbian после сборки ядра")?;

        context.record(outputs::BSP_KERNEL_IMAGE, artifacts.image_deb);
        context.record(outputs::BSP_KERNEL_DTB, artifacts.dtb_deb);

        if let Some(headers) = artifacts.headers_deb {
            context.record(outputs::BSP_KERNEL_HEADERS, headers);
        }

        // Загрузчик необязателен: та же inventory используется командой
        // `bsp-artifacts` на checkout, где собирали только ядро.
        match inventory.uboot_artifact() {
            Ok(uboot) => context.record(outputs::BSP_UBOOT, uboot),
            Err(error) => warn!(%error, "пакет U-Boot не найден в Armbian output"),
        }

        Ok(())
    }
}

/// Установка пакета U-Boot в rootfs Platinum.
///
/// В образ попадает не сам загрузчик, а пакет с `platform_install.sh`: он несёт
/// смещения записи SPL и U-Boot для family платы, поэтому stage записи образа
/// не будет хранить эти числа у себя.
pub struct InstallUbootStage {
    architecture: String,
}

impl InstallUbootStage {
    /// Создаёт stage для архитектуры платы.
    pub fn new(architecture: String) -> Self {
        Self { architecture }
    }
}

impl Stage for InstallUbootStage {
    fn name(&self) -> &'static str {
        "install-uboot"
    }

    fn execute(&self, context: &mut BuildContext) -> Result<()> {
        let rootfs = context.require_output(outputs::ROOTFS_DIR)?.to_path_buf();
        let uboot = context
            .require_output(outputs::BSP_UBOOT)
            .context("пакет U-Boot отсутствует: Armbian не собрал его на этом checkout")?
            .to_path_buf();

        let chroot = Chroot::new(rootfs, self.architecture.clone())
            .context("каталог rootfs непригоден для chroot")?;

        DpkgInstaller::new(vec![uboot])
            .context("некорректный пакет U-Boot")?
            .install(&chroot)
            .context("не удалось установить U-Boot в rootfs")?;

        Ok(())
    }
}

/// Установка ядра и DTB, собранных Armbian, в rootfs Platinum.
///
/// Ставятся только `linux-image` и `linux-dtb`: заголовки нужны для сборки
/// модулей на самом устройстве и добавили бы к образу десятки мегабайт, поэтому
/// остаются артефактом сборки, а не частью OS.
pub struct InstallKernelStage {
    architecture: String,
}

impl InstallKernelStage {
    /// Создаёт stage для архитектуры платы.
    pub fn new(architecture: String) -> Self {
        Self { architecture }
    }
}

impl Stage for InstallKernelStage {
    fn name(&self) -> &'static str {
        "install-kernel"
    }

    fn execute(&self, context: &mut BuildContext) -> Result<()> {
        let rootfs = context.require_output(outputs::ROOTFS_DIR)?.to_path_buf();
        let packages = vec![
            context
                .require_output(outputs::BSP_KERNEL_IMAGE)?
                .to_path_buf(),
            context
                .require_output(outputs::BSP_KERNEL_DTB)?
                .to_path_buf(),
        ];

        let chroot = Chroot::new(rootfs, self.architecture.clone())
            .context("каталог rootfs непригоден для chroot")?;

        DpkgInstaller::new(packages)
            .context("некорректный набор пакетов ядра")?
            .install(&chroot)
            // Зависимости пакетов ядра решаются составом packages.toml: dpkg не
            // ходит в сеть, и отсутствующий initramfs-tools виден именно здесь.
            .context(
                "не удалось установить ядро и DTB в rootfs; проверьте зависимости в packages.toml",
            )?;

        Ok(())
    }
}
