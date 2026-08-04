//! Ключи, под которыми stages публикуют результаты в BuildContext.
//!
//! Ключи собраны в одном месте намеренно: строковый литерал, продублированный в
//! двух stages, рано или поздно разойдётся, и зависимость между этапами
//! сломается молча.

/// Скачанный и проверенный архив Ubuntu Base.
pub const ROOTFS_ARCHIVE: &str = "rootfs.archive";

/// Каталог распакованного rootfs.
pub const ROOTFS_DIR: &str = "rootfs.dir";

/// Каталог pinned checkout Armbian Build.
pub const BSP_CHECKOUT: &str = "bsp.checkout";

/// Каталог pinned checkout репозитория firmware.
pub const FIRMWARE_CHECKOUT: &str = "firmware.checkout";

/// Пакет с ядром, собранный Armbian.
pub const BSP_KERNEL_IMAGE: &str = "bsp.kernel.image";

/// Пакет с Device Tree blobs.
pub const BSP_KERNEL_DTB: &str = "bsp.kernel.dtb";

/// Пакет с заголовками ядра, если Armbian его собрал.
pub const BSP_KERNEL_HEADERS: &str = "bsp.kernel.headers";

/// Пакет загрузчика U-Boot вместе с `platform_install.sh`.
pub const BSP_UBOOT: &str = "bsp.uboot";

/// Конфигурация загрузки, прочитанная U-Boot.
pub const EXTLINUX: &str = "boot.extlinux";

/// Скомпилированный boot-скрипт для плат, не использующих extlinux.
pub const BOOT_SCRIPT: &str = "boot.script";

/// `config.txt` загрузочного раздела Raspberry Pi.
pub const BOOT_FIRMWARE_CONFIG: &str = "boot.firmware.config";

/// Готовый дисковый образ Platinum OS.
pub const IMAGE: &str = "image";
