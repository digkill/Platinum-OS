use anyhow::{Context, Result};
use platinum_board::{BoardConfig, PackagesConfig, PartitionsConfig, SystemConfig};
use platinum_core::{BuildContext, Pipeline};
use platinum_downloader::Artifact;
use platinum_rootfs::{PackageSet, RootfsSpec};

use crate::{
    BspInventoryStage, BspKernelStage, BspSyncStage, BspUbootStage, BuildImageStage,
    ConfigureBootStage, ConfigureSystemStage, DownloadRootfsStage, InstallFirmwareStage,
    InstallKernelStage, InstallPackagesStage, InstallUbootStage, PrepareStage, UnpackRootfsStage,
    boot_spec, firmware_spec, fstab_entries, image_layout, system_spec,
};

/// Настройки состава pipeline для одного запуска сборки.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildOptions {
    /// Включать ли сборку BSP (checkout Armbian, kernel, DTB).
    ///
    /// Выключено по умолчанию: BSP тянет гигабайты исходников и часы
    /// компиляции, поэтому решение должно быть осознанным, а не побочным
    /// эффектом обычного `build`.
    pub with_bsp: bool,
    /// Состав userspace, если сборка должна установить пакеты.
    ///
    /// Наличие конфигурации и включение stage — одно и то же значение: пара из
    /// флага и опционального пути допускала бы состояние «включено, но нечего
    /// устанавливать».
    pub packages: Option<PackagesConfig>,
    /// Системная конфигурация, если сборка должна настроить `/etc`.
    pub system: Option<SystemConfig>,
    /// Разметка образа, если сборка должна выпустить `.img`.
    pub partitions: Option<PartitionsConfig>,
}

/// Оркестратор стандартного процесса сборки Platinum OS.
///
/// Engine выбирает состав stages по данным платы, но не содержит правил
/// конкретных плат. Добавление новой платы меняет TOML, а не этот код.
#[derive(Debug)]
pub struct BuildEngine {
    pipeline: Pipeline,
}

impl BuildEngine {
    /// Собирает pipeline для платы и выбранных опций.
    ///
    /// Конфигурация проверяется здесь, до первого stage: ошибка в URL или
    /// контрольной сумме не должна обнаруживаться после создания каталогов и
    /// начала сетевых операций.
    pub fn new(board: BoardConfig, options: BuildOptions) -> Result<Self> {
        // Идентификатор, архитектура и DTB нужны stages после того, как board
        // уйдёт в BSP inventory, поэтому копируются один раз в начале.
        let id = board.id.clone();
        let architecture = board.architecture.clone();
        let dtb = board.dtb.clone();
        let bootloader = board.bootloader.clone();
        let modules = board.modules.clone();
        let uses_armbian = board.armbian.is_some();
        // Загрузчик пишется в сырые сектора не у всех плат: у Raspberry Pi он
        // лежит в EEPROM, и писать в образ нечего.
        let writes_bootloader = options.with_bsp && bootloader.writes_raw_sectors();

        // Параметры загрузки живут в системной конфигурации, но нужны и без
        // неё: образ с ядром обязан получить extlinux.conf в любом случае.
        let boot = options
            .system
            .as_ref()
            .map(|system| system.boot.clone())
            .unwrap_or_default();

        let artifact = Artifact::new(board.rootfs.url.clone(), board.rootfs.sha256.clone())
            .with_context(|| format!("некорректное описание rootfs платы `{id}`"))?;
        let spec = RootfsSpec::new(
            board.rootfs.release.clone(),
            board.rootfs.architecture.clone(),
        )
        .with_context(|| format!("некорректная спецификация rootfs платы `{id}`"))?;

        let mut pipeline = Pipeline::new();
        pipeline.add(PrepareStage);
        pipeline.add(DownloadRootfsStage::new(artifact));
        pipeline.add(UnpackRootfsStage::new(spec));

        if let Some(packages) = options.packages {
            let set = PackageSet::new(packages.install)
                .with_context(|| format!("некорректный состав userspace платы `{id}`"))?;

            pipeline.add(InstallPackagesStage::new(
                architecture.clone(),
                set,
                packages.install_recommends,
            ));
        }

        // Firmware ставится до BSP: пакет ядра при установке пересобирает
        // initramfs, и файлы, положенные позже, в него бы не попали.
        if let Some(firmware) = &board.firmware {
            let spec = firmware_spec(firmware)
                .with_context(|| format!("некорректный firmware платы `{id}`"))?;

            pipeline.add(InstallFirmwareStage::new(spec));
        }

        // BSP собирается только там, где он есть: ядро части плат приходит из
        // архива Ubuntu обычным `apt`, и Armbian им не нужен вовсе.
        if let (true, Some(armbian)) = (options.with_bsp, board.armbian.clone()) {
            pipeline.add(BspSyncStage::new(armbian.clone()));
            pipeline.add(BspKernelStage::new(armbian.clone()));
            pipeline.add(BspUbootStage::new(armbian));
            pipeline.add(BspInventoryStage::new(board));
            // Пакеты ставятся сразу после inventory: собранные `.deb` нужны в
            // rootfs, а не рядом с ним, иначе образ останется без ядра и DTB.
            pipeline.add(InstallKernelStage::new(architecture.clone()));
            pipeline.add(InstallUbootStage::new(architecture.clone()));
        }

        // Разметка проверяется до системной конфигурации: fstab строится из неё,
        // и ошибка в метке раздела не должна доходить до записи `/etc`.
        let layout = match &options.partitions {
            Some(partitions) => Some(
                image_layout(partitions)
                    .with_context(|| format!("некорректная разметка образа платы `{id}`"))?,
            ),
            None => None,
        };

        if let Some(system) = options.system {
            let filesystems = match &options.partitions {
                Some(partitions) => fstab_entries(partitions).with_context(|| {
                    format!("разметка образа платы `{id}` несовместима с fstab")
                })?,
                None => Vec::new(),
            };

            let spec = system_spec(system, filesystems, modules)
                .with_context(|| format!("некорректная конфигурация системы платы `{id}`"))?;

            // Настройка идёт перед сборкой образа: всё, что изменено позже, в
            // файловую систему образа уже не попадёт.
            pipeline.add(ConfigureSystemStage::new(architecture, spec));
        }

        // Конфигурация загрузки нужна там, где в rootfs окажется ядро и есть
        // раздел с корнем. У плат с Armbian ядро приносит BSP, поэтому нужен
        // `--with-bsp`; у остальных оно приходит пакетом из архива Ubuntu, и
        // ждать BSP, которого нет, значило бы никогда не настроить загрузку.
        let kernel_expected = options.with_bsp || !uses_armbian;

        if let (true, Some(partitions)) = (kernel_expected, &options.partitions) {
            let spec = boot_spec(partitions, &boot)
                .with_context(|| format!("некорректные параметры загрузки платы `{id}`"))?;

            pipeline.add(ConfigureBootStage::new(spec, &bootloader, dtb));
        }

        if let Some(layout) = layout {
            pipeline.add(BuildImageStage::new(layout, id, writes_bootloader));
        }

        Ok(Self { pipeline })
    }

    /// Возвращает имена stages в порядке выполнения.
    pub fn stage_names(&self) -> impl Iterator<Item = &'static str> {
        self.pipeline.stage_names()
    }

    /// Запускает все stages для переданного контекста сборки.
    pub fn run(&self, context: &mut BuildContext) -> Result<()> {
        self.pipeline.run(context)
    }
}
