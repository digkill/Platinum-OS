use anyhow::{Context, Result};
use platinum_board::{BootConfig, BootloaderConfig, PartitionsConfig};
use platinum_core::{BuildContext, Stage};
use platinum_rootfs::{
    BootConfigurator, BootScriptConfigurator, BootScriptSpec, BootSpec, RaspberryPiConfigurator,
    RaspberryPiSpec,
};

use crate::outputs;

/// Подготовка загрузки в готовом rootfs.
///
/// Stage идёт после установки ядра и до сборки образа: конфигурация читает
/// имена файлов из `/boot`, а в образ должна попасть уже готовой.
pub struct ConfigureBootStage {
    method: BootMethod,
    dtb: String,
}

/// Способ загрузки, выбранный данными платы.
///
/// Выбор делается один раз при сборке pipeline: stage не должен решать это
/// заново на каждом запуске, а engine не должен знать деталей ни одного способа.
enum BootMethod {
    /// `extlinux.conf`, который U-Boot читает сам.
    Extlinux(BootConfigurator),
    /// Скомпилированный `boot.scr` из скрипта pinned checkout Armbian.
    Script(BootScriptConfigurator),
    /// Файлы прошивки Raspberry Pi на FAT-разделе.
    RaspberryPi(RaspberryPiConfigurator),
}

impl ConfigureBootStage {
    /// Создаёт stage для параметров загрузки и DTB платы.
    pub fn new(spec: BootSpec, bootloader: &BootloaderConfig, dtb: String) -> Self {
        let method = match bootloader {
            BootloaderConfig::Extlinux => BootMethod::Extlinux(BootConfigurator::new(spec)),
            BootloaderConfig::BootScript(script) => {
                BootMethod::Script(BootScriptConfigurator::new(BootScriptSpec {
                    root_source: spec.root_source,
                    root_filesystem: spec.root_filesystem,
                    extra_arguments: spec.extra_arguments,
                    script: script.script.clone(),
                    environment: script.env.clone(),
                    initrd_architecture: script.initrd_arch.clone(),
                    overlay_prefix: script.overlay_prefix.clone(),
                }))
            }
            BootloaderConfig::RaspberryPi(pi) => {
                BootMethod::RaspberryPi(RaspberryPiConfigurator::new(RaspberryPiSpec {
                    root_source: spec.root_source,
                    root_filesystem: spec.root_filesystem,
                    extra_arguments: spec.extra_arguments,
                    firmware_mount_point: pi.firmware_mount_point.clone(),
                    config: pi.config.clone(),
                }))
            }
        };

        Self { method, dtb }
    }
}

impl Stage for ConfigureBootStage {
    fn name(&self) -> &'static str {
        "configure-boot"
    }

    fn execute(&self, context: &mut BuildContext) -> Result<()> {
        let rootfs = context.require_output(outputs::ROOTFS_DIR)?.to_path_buf();

        match &self.method {
            BootMethod::Extlinux(configurator) => {
                let path = configurator
                    .apply(&rootfs, &self.dtb)
                    .context("не удалось подготовить конфигурацию загрузки")?;

                context.record(outputs::EXTLINUX, path);
            }
            BootMethod::Script(configurator) => {
                // Скрипт и файл окружения берутся из того же checkout, что дал
                // ядро: другой commit Armbian описывал бы другую загрузку.
                let checkout = context.require_output(outputs::BSP_CHECKOUT)?.to_path_buf();

                let path = configurator
                    .apply(&rootfs, &checkout, &self.dtb)
                    .context("не удалось подготовить boot-скрипт загрузки")?;

                context.record(outputs::BOOT_SCRIPT, path);
            }
            BootMethod::RaspberryPi(configurator) => {
                let path = configurator
                    .apply(&rootfs, &self.dtb)
                    .context("не удалось подготовить загрузочный раздел Raspberry Pi")?;

                context.record(outputs::BOOT_FIRMWARE_CONFIG, path);
            }
        }

        Ok(())
    }
}

/// Строит параметры загрузки по разметке образа и системной конфигурации.
///
/// Источник корня берётся из `partitions.toml`: командная строка ядра и fstab
/// обязаны указывать на один и тот же раздел, а две независимые записи метки
/// разошлись бы.
pub fn boot_spec(partitions: &PartitionsConfig, boot: &BootConfig) -> Result<BootSpec> {
    let root = partitions
        .partitions
        .iter()
        .find(|partition| partition.mount_point.as_deref() == Some("/"))
        .context("в разметке образа нет раздела, монтируемого в `/`")?;

    Ok(BootSpec {
        root_source: format!("LABEL={}", root.label),
        root_filesystem: root.filesystem.clone(),
        extra_arguments: boot.extra_cmdline.clone(),
        timeout_deciseconds: boot.timeout_deciseconds,
    })
}
