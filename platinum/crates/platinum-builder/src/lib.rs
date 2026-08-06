//! Оркестрация сборки образов Platinum OS One.
//!
//! Crate собирает независимые stages в стандартный pipeline. Состав pipeline
//! определяется данными платы и опциями запуска, а не условиями вида
//! `if board == ...`: так поддержка новой платы остаётся вопросом конфигурации.

mod boot;
mod bsp;
mod engine;
mod firmware;
mod image;
mod prepare;
mod rootfs;
mod system;

pub mod outputs;

pub use boot::{ConfigureBootStage, boot_spec};
pub use bsp::{
    BspInventoryStage, BspKernelStage, BspSyncStage, BspUbootStage, InstallKernelStage,
    InstallUbootStage, armbian_checkout_dir,
};
pub use engine::{BuildEngine, BuildOptions};
pub use firmware::{InstallFirmwareStage, firmware_spec};
pub use image::{BuildImageStage, fstab_entries, image_layout};
pub use prepare::PrepareStage;
pub use rootfs::{DownloadRootfsStage, InstallPackagesStage, UnpackRootfsStage};
pub use system::{ConfigureSystemStage, system_spec};

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use platinum_board::{
        ArmbianConfig, BoardConfig, BootConfig, BootloaderConfig, NetworkConfig, PackagesConfig,
        PartitionsConfig, RootfsConfig, SystemConfig, UserConfig,
    };
    use platinum_core::{BuildContext, BuildPaths, Pipeline, Stage};
    use platinum_rootfs::RootfsSpec;

    use super::{BuildEngine, BuildOptions, PrepareStage, UnpackRootfsStage, outputs};

    const SHA256: &str = "b2b46a37324ea1954e93f293fe6d7c2241daf2fc298c4022e6e4caceeed74cab";

    fn test_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "platinum-builder-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("системное время должно быть позже Unix epoch")
                .as_nanos()
        ))
    }

    fn paths_in(root: &Path) -> BuildPaths {
        BuildPaths::new(
            root.join("work"),
            root.join("downloads"),
            root.join("cache"),
            root.join("output"),
        )
        .expect("пути тестовой сборки должны быть корректными")
    }

    fn board(sha256: &str) -> BoardConfig {
        BoardConfig {
            id: "test-board".into(),
            name: "Test Board".into(),
            architecture: "arm64".into(),
            soc: "Test SoC".into(),
            bsp_family: "test-family".into(),
            memory_mib: 1024,
            dtb: "test-board.dtb".into(),
            bootloader: BootloaderConfig::default(),
            modules: Vec::new(),
            firmware: None,
            armbian: Some(ArmbianConfig {
                repository: "https://example.test/armbian.git".into(),
                revision: "0123456789abcdef0123456789abcdef01234567".into(),
                board: "test-board".into(),
                kernel_branch: "vendor".into(),
            }),
            rootfs: RootfsConfig {
                release: "26.04".into(),
                architecture: "arm64".into(),
                url: "https://example.test/ubuntu-base-26.04-base-arm64.tar.gz".into(),
                sha256: sha256.into(),
            },
        }
    }

    fn system_config() -> SystemConfig {
        SystemConfig {
            hostname: "platinum".into(),
            timezone: "Etc/UTC".into(),
            locale: "en_US.UTF-8".into(),
            users: vec![UserConfig {
                name: "platinum".into(),
                password_hash: "$6$salt$hash".into(),
                groups: vec!["sudo".into()],
                shell: None,
                force_password_change: true,
            }],
            filesystems: Vec::new(),
            network: NetworkConfig::default(),
            boot: BootConfig::default(),
            shell: None,
            cloud_init: None,
            splash: None,
            expand_rootfs: true,
        }
    }

    #[test]
    fn prepare_stage_creates_all_build_directories() {
        let root = test_root("prepare");
        let paths = paths_in(&root);
        let mut context = BuildContext::new(paths.clone());

        let mut pipeline = Pipeline::new();
        pipeline.add(PrepareStage);
        pipeline
            .run(&mut context)
            .expect("подготовка директорий должна завершаться успешно");

        assert!(paths.work_dir.is_dir());
        assert!(paths.downloads_dir.is_dir());
        assert!(paths.cache_dir.is_dir());
        assert!(paths.output_dir.is_dir());

        fs::remove_dir_all(root).expect("тестовая директория должна удаляться после проверки");
    }

    #[test]
    fn unpack_stage_reuses_an_existing_rootfs() {
        let root = test_root("reuse");
        let paths = paths_in(&root);
        let rootfs_dir = paths.work_dir.join("rootfs");
        fs::create_dir_all(rootfs_dir.join("etc")).expect("каталог rootfs должен создаваться");
        fs::write(rootfs_dir.join("etc/os-release"), b"ID=ubuntu\n")
            .expect("маркер rootfs должен записываться");

        let mut context = BuildContext::new(paths);
        // Архив заведомо отсутствует: stage обязан переиспользовать готовый
        // rootfs, не обращаясь к файлу.
        context.record(
            outputs::ROOTFS_ARCHIVE,
            PathBuf::from("/absent/base.tar.gz"),
        );

        UnpackRootfsStage::new(
            RootfsSpec::new("26.04".into(), "arm64".into())
                .expect("спецификация должна быть корректной"),
        )
        .execute(&mut context)
        .expect("готовый rootfs должен переиспользоваться");

        assert_eq!(
            context
                .require_output(outputs::ROOTFS_DIR)
                .expect("каталог rootfs должен публиковаться"),
            rootfs_dir
        );

        fs::remove_dir_all(root).expect("тестовая директория должна удаляться после проверки");
    }

    #[test]
    fn engine_rejects_a_board_with_an_invalid_checksum() {
        let error = BuildEngine::new(board("нет-суммы"), BuildOptions::default())
            .expect_err("некорректная контрольная сумма должна отклоняться до запуска stages");

        assert!(error.to_string().contains("test-board"));
    }

    #[test]
    fn engine_adds_bsp_stages_only_on_demand() {
        let without_bsp = BuildEngine::new(board(SHA256), BuildOptions::default())
            .expect("pipeline без BSP должен собираться");
        let with_bsp = BuildEngine::new(
            board(SHA256),
            BuildOptions {
                with_bsp: true,
                ..BuildOptions::default()
            },
        )
        .expect("pipeline с BSP должен собираться");

        assert_eq!(
            without_bsp.stage_names().collect::<Vec<_>>(),
            ["prepare", "download-rootfs", "unpack-rootfs"]
        );
        assert_eq!(
            with_bsp.stage_names().collect::<Vec<_>>(),
            [
                "prepare",
                "download-rootfs",
                "unpack-rootfs",
                "bsp-sync",
                "bsp-kernel",
                "bsp-uboot",
                "bsp-inventory",
                "install-kernel",
                "install-uboot",
            ]
        );
    }

    #[test]
    fn engine_installs_packages_before_building_the_bsp() {
        let engine = BuildEngine::new(
            board(SHA256),
            BuildOptions {
                with_bsp: true,
                packages: Some(PackagesConfig {
                    install_recommends: false,
                    install: vec!["systemd".into()],
                }),
                ..BuildOptions::default()
            },
        )
        .expect("pipeline с пакетами должен собираться");

        assert_eq!(
            engine.stage_names().collect::<Vec<_>>(),
            [
                "prepare",
                "download-rootfs",
                "unpack-rootfs",
                "install-packages",
                "bsp-sync",
                "bsp-kernel",
                "bsp-uboot",
                "bsp-inventory",
                "install-kernel",
                "install-uboot",
            ]
        );
    }

    #[test]
    fn engine_configures_the_system_last() {
        let engine = BuildEngine::new(
            board(SHA256),
            BuildOptions {
                system: Some(system_config()),
                ..BuildOptions::default()
            },
        )
        .expect("pipeline с системной конфигурацией должен собираться");

        assert_eq!(
            engine.stage_names().collect::<Vec<_>>(),
            [
                "prepare",
                "download-rootfs",
                "unpack-rootfs",
                "configure-system",
            ]
        );
    }

    #[test]
    fn engine_rejects_a_plaintext_password_before_any_stage_runs() {
        let mut system = system_config();
        system.users[0].password_hash = "hunter2".into();

        let error = BuildEngine::new(
            board(SHA256),
            BuildOptions {
                system: Some(system),
                ..BuildOptions::default()
            },
        )
        .expect_err("открытый пароль должен отклоняться до запуска stages");

        assert!(error.to_string().contains("test-board"));
    }

    /// Данные платы должны собираться в pipeline без запуска сборки.
    ///
    /// Тест читает реальные файлы репозитория: опечатка в имени пакета или в
    /// hostname иначе обнаружилась бы только на живой сборке под root.
    #[test]
    fn board_data_of_the_repository_builds_a_pipeline() {
        let boards = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../boards/orangepi-zero3w");

        let board = BoardConfig::load(&boards.join("board.toml"))
            .expect("board.toml платы должен читаться");
        let packages = PackagesConfig::load(&boards.join("packages.toml"))
            .expect("packages.toml платы должен читаться");
        let system = SystemConfig::load(&boards.join("system.toml"))
            .expect("system.toml платы должен читаться");
        let partitions = PartitionsConfig::load(&boards.join("partitions.toml"))
            .expect("partitions.toml платы должен читаться");

        // Zero 3W грузится boot-скриптом: у vendor U-Boot 2018.05 поддержка
        // extlinux не подтверждена, и молчаливый откат к ней дал бы образ,
        // который собирается, но не стартует.
        assert!(
            matches!(board.bootloader, BootloaderConfig::BootScript(_)),
            "плата обязана оставаться на boot-скрипте"
        );

        let engine = BuildEngine::new(
            board,
            BuildOptions {
                with_bsp: true,
                packages: Some(packages),
                system: Some(system),
                partitions: Some(partitions),
                config_dir: boards.clone(),
            },
        )
        .expect("данные платы должны давать корректный pipeline");

        let stages: Vec<_> = engine.stage_names().collect();

        // Firmware обязан лечь до установки ядра: пакет ядра пересобирает
        // initramfs, и файлы, добавленные позже, в него не попадут.
        assert!(
            stages
                .iter()
                .position(|stage| *stage == "install-firmware")
                .zip(stages.iter().position(|stage| *stage == "install-kernel"))
                .is_some_and(|(firmware, kernel)| firmware < kernel),
            "install-firmware обязан идти до install-kernel: {stages:?}"
        );
        assert_eq!(
            stages.last(),
            Some(&"build-image"),
            "образ обязан собираться последним"
        );
        assert_eq!(
            stages.iter().position(|stage| *stage == "configure-boot"),
            stages
                .iter()
                .position(|stage| *stage == "build-image")
                .map(|index| index - 1),
            "загрузка обязана быть настроена до сборки образа"
        );
    }

    /// Данные Raspberry Pi 5 должны собираться в pipeline без Armbian.
    ///
    /// Плата проверяет ровно то, ради чего архитектура и разделяла способы:
    /// ядро приходит пакетом, загрузчик — из EEPROM, а engine не знает о плате
    /// ничего сверх её данных.
    #[test]
    fn raspberry_pi_board_data_builds_a_pipeline_without_armbian() {
        let boards = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../boards/raspberrypi-5");

        let board = BoardConfig::load(&boards.join("board.toml"))
            .expect("board.toml платы должен читаться");
        let packages = PackagesConfig::load(&boards.join("packages.toml"))
            .expect("packages.toml платы должен читаться");
        let system = SystemConfig::load(&boards.join("system.toml"))
            .expect("system.toml платы должен читаться");
        let partitions = PartitionsConfig::load(&boards.join("partitions.toml"))
            .expect("partitions.toml платы должен читаться");

        assert!(board.armbian.is_none(), "Pi 5 не использует Armbian");
        assert!(
            !board.bootloader.writes_raw_sectors(),
            "загрузчик Pi лежит в EEPROM, в образ писать нечего"
        );

        let engine = BuildEngine::new(
            board,
            BuildOptions {
                with_bsp: false,
                packages: Some(packages),
                system: Some(system),
                partitions: Some(partitions),
                config_dir: boards.clone(),
            },
        )
        .expect("данные платы должны давать корректный pipeline");

        let stages: Vec<_> = engine.stage_names().collect();

        assert!(
            !stages.iter().any(|stage| stage.starts_with("bsp-")),
            "у платы без Armbian не должно быть BSP-stages: {stages:?}"
        );
        // Загрузка обязана настраиваться без --with-bsp: ядро приносит apt.
        assert!(
            stages.contains(&"configure-boot"),
            "загрузка обязана настраиваться: {stages:?}"
        );
        assert_eq!(stages.last(), Some(&"build-image"));
    }

    /// Данные виртуальной машины UEFI должны собираться в pipeline.
    ///
    /// Третий класс целей после Armbian и прошивки Raspberry Pi: загрузчик
    /// приходит с firmware, поэтому в сырые сектора не пишется ничего, а ESP
    /// обязан получить собственный код типа раздела.
    #[test]
    fn uefi_board_data_builds_a_pipeline() {
        let boards = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../boards/parallels-arm64");

        let board = BoardConfig::load(&boards.join("board.toml"))
            .expect("board.toml платы должен читаться");
        let packages = PackagesConfig::load(&boards.join("packages-shell.toml"))
            .expect("packages.toml платы должен читаться");
        let system = SystemConfig::load(&boards.join("system-shell.toml"))
            .expect("system.toml платы должен читаться");
        let partitions = PartitionsConfig::load(&boards.join("partitions.toml"))
            .expect("partitions.toml платы должен читаться");

        assert!(
            board.armbian.is_none(),
            "виртуальной машине Armbian не нужен"
        );
        assert!(
            !board.bootloader.writes_raw_sectors(),
            "загрузчик приходит с прошивкой, писать в сектора нечего"
        );
        assert!(
            partitions.partitions.iter().any(|partition| partition.esp),
            "без раздела ESP прошивка не найдёт загрузчик"
        );

        let engine = BuildEngine::new(
            board,
            BuildOptions {
                with_bsp: false,
                packages: Some(packages),
                system: Some(system),
                partitions: Some(partitions),
                config_dir: boards.clone(),
            },
        )
        .expect("данные платы должны давать корректный pipeline");

        let stages: Vec<_> = engine.stage_names().collect();
        assert!(stages.contains(&"configure-boot"));
        assert_eq!(stages.last(), Some(&"build-image"));
    }

    /// Метка корня в fstab должна приходить из разметки образа.
    ///
    /// Иначе `partitions.toml` и `system.toml` разошлись бы, и система не нашла
    /// бы корень — отказ, который виден только на живом устройстве.
    #[test]
    fn fstab_is_derived_from_the_partition_layout() {
        let boards = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../boards/orangepi-zero3w");
        let partitions = PartitionsConfig::load(&boards.join("partitions.toml"))
            .expect("partitions.toml платы должен читаться");

        let entries = super::fstab_entries(&partitions).expect("fstab должен строиться разметкой");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, "LABEL=platinum-root");
        assert_eq!(entries[0].mount_point, "/");
        assert_eq!(entries[0].pass, 1);
    }

    #[test]
    fn engine_rejects_an_invalid_package_name_before_any_stage_runs() {
        let error = BuildEngine::new(
            board(SHA256),
            BuildOptions {
                with_bsp: false,
                packages: Some(PackagesConfig {
                    install_recommends: false,
                    install: vec!["--force-yes".into()],
                }),
                ..BuildOptions::default()
            },
        )
        .expect_err("опция apt в списке пакетов должна отклоняться");

        assert!(error.to_string().contains("test-board"));
    }
}
