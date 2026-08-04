use std::str::FromStr;

use anyhow::{Context, Result};
use platinum_board::PartitionsConfig;
use platinum_core::{BuildContext, Stage};
use platinum_image::{Filesystem, ImageBuilder, ImageLayout, PartitionSpec, write_uboot};
use platinum_rootfs::Filesystem as FstabEntry;
use tracing::warn;

use crate::outputs;

/// Расширение файла готового образа.
const IMAGE_EXTENSION: &str = "img";

/// Сборка загружаемого образа из готового rootfs.
///
/// Stage выполняется последним: он фиксирует состояние rootfs в файловой
/// системе образа, и любое изменение после него в образ уже не попадёт.
pub struct BuildImageStage {
    layout: ImageLayout,
    image_name: String,
    with_uboot: bool,
}

impl BuildImageStage {
    /// Создаёт stage для разметки платы.
    ///
    /// `with_uboot` включается вместе с BSP: без него в rootfs нет
    /// `platform_install.sh`, и образ заведомо остался бы без загрузчика.
    pub fn new(layout: ImageLayout, image_name: String, with_uboot: bool) -> Self {
        Self {
            layout,
            image_name,
            with_uboot,
        }
    }
}

impl Stage for BuildImageStage {
    fn name(&self) -> &'static str {
        "build-image"
    }

    fn execute(&self, context: &mut BuildContext) -> Result<()> {
        let rootfs = context.require_output(outputs::ROOTFS_DIR)?.to_path_buf();
        let image = context
            .paths()
            .output_dir
            .join(format!("{}.{IMAGE_EXTENSION}", self.image_name));

        ImageBuilder::new(self.layout.clone())
            .build(&rootfs, &image)
            .context("не удалось собрать дисковый образ")?;

        if self.with_uboot {
            write_uboot(&rootfs, &image).context("не удалось записать загрузчик в образ")?;
        } else {
            // Образ без загрузчика пригоден для проверки rootfs, но не
            // загрузится: об этом нужно сказать явно, а не молча отдать файл.
            warn!(
                image = %image.display(),
                "образ собран без загрузчика: BSP не участвовал в сборке"
            );
        }

        context.record(outputs::IMAGE, image);

        Ok(())
    }
}

/// Переводит разметку из TOML в проверенную `ImageLayout`.
pub fn image_layout(config: &PartitionsConfig) -> Result<ImageLayout> {
    let mut partitions = Vec::with_capacity(config.partitions.len());

    for partition in &config.partitions {
        let filesystem = Filesystem::from_str(&partition.filesystem)
            .with_context(|| format!("раздел `{}`", partition.name))?;

        partitions.push(
            PartitionSpec::new(
                partition.name.clone(),
                partition.label.clone(),
                filesystem,
                partition.start_mib,
                partition.size_mib,
                partition.mount_point.clone(),
                partition.bootable,
            )
            .with_context(|| format!("раздел `{}`", partition.name))?,
        );
    }

    ImageLayout::new(partitions, config.reserved_mib).context("некорректная разметка образа")
}

/// Строит записи fstab по разметке образа.
///
/// Записи берутся из `partitions.toml`, а не дублируются в `system.toml`: две
/// независимые копии меток разошлись бы, и система не нашла бы корень.
pub fn fstab_entries(config: &PartitionsConfig) -> Result<Vec<FstabEntry>> {
    let mut entries = Vec::new();

    for partition in &config.partitions {
        let Some(mount_point) = &partition.mount_point else {
            continue;
        };

        // В fstab идёт имя, которое понимает ядро, а не имя из конфигурации:
        // `fat32` описывает разметку, а смонтировать такое можно только как
        // `vfat`, и раздел просто не поднялся бы на устройстве.
        let filesystem = Filesystem::from_str(&partition.filesystem)
            .with_context(|| format!("раздел `{}`", partition.name))?;

        entries.push(
            FstabEntry::new(
                format!("LABEL={}", partition.label),
                mount_point.clone(),
                filesystem.to_string(),
                partition.options.clone(),
                0,
                partition.pass,
            )
            .with_context(|| format!("некорректная запись fstab раздела `{}`", partition.name))?,
        );
    }

    Ok(entries)
}
