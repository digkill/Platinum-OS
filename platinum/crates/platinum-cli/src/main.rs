//! Точка входа CLI Platinum OS One.
//!
//! Разбор аргументов остаётся на границе приложения: BuildEngine получает уже
//! проверенные BoardConfig и BuildContext и не зависит от окружения процесса.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use platinum_armbian_bsp::{ArmbianBspRunner, ArmbianCheckout, BspInventory};
use platinum_board::{BoardConfig, PackagesConfig, PartitionsConfig, SystemConfig};
use platinum_builder::{BuildEngine, BuildOptions};
use platinum_core::{BuildContext, BuildPaths};
use tracing::warn;

/// Сборщик образов Platinum OS One.
#[derive(Debug, Parser)]
#[command(name = "platinum", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Печатает версию сборщика.
    Version,
    /// Собирает образ Platinum OS для платы.
    Build(BuildArgs),
    /// Синхронизирует pinned Armbian checkout платы.
    BspSync(BspArgs),
    /// Собирает kernel и DTB официальным target Armbian.
    BspBuildKernel(BspArgs),
    /// Собирает U-Boot официальным target Armbian.
    BspBuildUboot(BspArgs),
    /// Показывает BSP-артефакты, найденные в Armbian checkout.
    BspArtifacts(BspArgs),
}

#[derive(Debug, Args)]
struct BuildArgs {
    /// Путь к board.toml собираемой платы.
    board: PathBuf,
    /// Каталог промежуточных результатов сборки.
    #[arg(long)]
    work_dir: PathBuf,
    /// Каталог загруженных артефактов.
    #[arg(long)]
    downloads_dir: PathBuf,
    /// Каталог, переиспользуемый между сборками.
    #[arg(long)]
    cache_dir: PathBuf,
    /// Каталог готовых образов.
    #[arg(long)]
    output_dir: PathBuf,
    /// Включает сборку BSP: checkout Armbian, kernel и DTB.
    ///
    /// Требует сети и часов компиляции, поэтому выключено по умолчанию.
    #[arg(long)]
    with_bsp: bool,
    /// Устанавливает Platinum userspace из `packages.toml` рядом с board.toml.
    ///
    /// Установка идёт в chroot и требует root, а для чужой архитектуры — ещё и
    /// qemu-user-static, поэтому она не включается по умолчанию.
    #[arg(long)]
    with_packages: bool,
    /// Путь к packages.toml, если состав userspace лежит вне каталога платы.
    ///
    /// Указание пути само включает установку пакетов: молча проигнорированный
    /// флаг был бы худшим из вариантов.
    #[arg(long)]
    packages: Option<PathBuf>,
    /// Настраивает систему по `system.toml` рядом с board.toml.
    ///
    /// Требует те же права, что и установка пакетов: часть шагов выполняется
    /// внутри chroot.
    #[arg(long)]
    with_system: bool,
    /// Путь к system.toml, если конфигурация лежит вне каталога платы.
    #[arg(long)]
    system: Option<PathBuf>,
    /// Собирает дисковый образ по `partitions.toml` рядом с board.toml.
    ///
    /// Требует `mkfs.ext4`; без `--with-bsp` образ выйдет без загрузчика, о чём
    /// сборка предупреждает.
    #[arg(long)]
    with_image: bool,
    /// Путь к partitions.toml, если разметка лежит вне каталога платы.
    #[arg(long)]
    partitions: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct BspArgs {
    /// Путь к board.toml платы.
    board: PathBuf,
    /// Каталог pinned Armbian checkout.
    checkout_dir: PathBuf,
}

fn main() -> Result<()> {
    platinum_logger::init().context("не удалось включить логирование")?;

    match Cli::parse().command {
        Command::Version => {
            println!("Platinum OS One {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::Build(arguments) => run_build(arguments),
        Command::BspSync(arguments) => run_bsp_sync(arguments),
        Command::BspBuildKernel(arguments) => run_bsp_build_kernel(arguments),
        Command::BspBuildUboot(arguments) => run_bsp_build_uboot(arguments),
        Command::BspArtifacts(arguments) => run_bsp_artifacts(arguments),
    }
}

/// Запускает pipeline сборки образа для указанной платы.
fn run_build(arguments: BuildArgs) -> Result<()> {
    let board = load_board(&arguments.board)?;
    let packages = load_packages(&arguments)?;
    let system = load_system(&arguments)?;
    let partitions = load_partitions(&arguments)?;

    let paths = BuildPaths::new(
        arguments.work_dir,
        arguments.downloads_dir,
        arguments.cache_dir,
        arguments.output_dir,
    )
    .context("некорректные пути сборки")?;
    let mut context = BuildContext::new(paths);

    let engine = BuildEngine::new(
        board,
        BuildOptions {
            with_bsp: arguments.with_bsp,
            packages,
            system,
            partitions,
        },
    )
    .context("не удалось построить pipeline сборки")?;

    engine
        .run(&mut context)
        .context("сборка Platinum OS не выполнена")?;

    // Результаты печатаются в stdout, а не только в лог: их удобно передавать
    // следующим шагам скрипта, не разбирая структурированные записи tracing.
    for (key, path) in context.outputs() {
        println!("{key} = {}", path.display());
    }

    Ok(())
}

/// Синхронизирует pinned Armbian checkout платы.
///
/// Checkout-путь передаётся явно, чтобы CLI не прятал десятки гигабайт и
/// сетевые операции в неочевидном каталоге проекта.
fn run_bsp_sync(arguments: BspArgs) -> Result<()> {
    let board = load_board(&arguments.board)?;
    let checkout = ArmbianCheckout::new(arguments.checkout_dir, require_armbian(&board)?)
        .context("некорректная Armbian-конфигурация платы")?;

    checkout
        .sync()
        .context("не удалось синхронизировать pinned Armbian checkout")?;

    println!(
        "Armbian checkout синхронизирован: {}",
        checkout.checkout_dir().display()
    );

    Ok(())
}

/// Синхронизирует Armbian source и собирает kernel с DTB.
///
/// Команда не создаёт Platinum image: она выпускает только BSP-артефакты,
/// которые отдельный stage установит в собственный Ubuntu rootfs.
fn run_bsp_build_kernel(arguments: BspArgs) -> Result<()> {
    let board = load_board(&arguments.board)?;
    let checkout = ArmbianCheckout::new(arguments.checkout_dir, require_armbian(&board)?)
        .context("некорректная Armbian-конфигурация платы")?;

    checkout
        .sync()
        .context("не удалось синхронизировать pinned Armbian checkout")?;

    ArmbianBspRunner::new(
        checkout.checkout_dir().to_path_buf(),
        require_armbian(&board)?,
    )
    .context("не удалось подготовить Armbian BSP runner")?
    .build_kernel()
    .context("Armbian не смог собрать kernel и DTB")?;

    print_bsp_artifacts(checkout.checkout_dir(), &board)
}

/// Синхронизирует Armbian source и собирает U-Boot платы.
///
/// U-Boot вынесен в отдельную команду: его сборка занимает минуты вместо часов,
/// и повторять ради неё компиляцию ядра не нужно.
fn run_bsp_build_uboot(arguments: BspArgs) -> Result<()> {
    let board = load_board(&arguments.board)?;
    let checkout = ArmbianCheckout::new(arguments.checkout_dir, require_armbian(&board)?)
        .context("некорректная Armbian-конфигурация платы")?;

    checkout
        .sync()
        .context("не удалось синхронизировать pinned Armbian checkout")?;

    ArmbianBspRunner::new(
        checkout.checkout_dir().to_path_buf(),
        require_armbian(&board)?,
    )
    .context("не удалось подготовить Armbian BSP runner")?
    .build_uboot()
    .context("Armbian не смог собрать U-Boot")?;

    // Печатается только загрузчик: ядро собирается отдельной командой, и
    // требовать его пакеты здесь значило бы возвращать ошибку после успешной
    // сборки U-Boot.
    print_uboot_artifact(checkout.checkout_dir(), &board)
}

/// Выводит путь пакета U-Boot, собранного target `uboot`.
fn print_uboot_artifact(checkout_dir: &Path, board: &BoardConfig) -> Result<()> {
    let uboot = BspInventory::for_board(checkout_dir, board)
        .context("плата не использует Armbian: в board.toml нет секции [armbian]")?
        .uboot_artifact()
        .context("не удалось найти пакет U-Boot")?;

    println!("uboot = {}", uboot.display());

    Ok(())
}

/// Печатает BSP-артефакты, уже собранные в Armbian checkout.
fn run_bsp_artifacts(arguments: BspArgs) -> Result<()> {
    let board = load_board(&arguments.board)?;

    print_bsp_artifacts(&arguments.checkout_dir, &board)
}

/// Выводит пути найденных пакетов ядра, DTB и загрузчика.
fn print_bsp_artifacts(checkout_dir: &Path, board: &BoardConfig) -> Result<()> {
    let inventory = BspInventory::for_board(checkout_dir, board)
        .context("плата не использует Armbian: в board.toml нет секции [armbian]")?;

    let artifacts = inventory
        .kernel_artifacts()
        .context("не удалось найти артефакты Armbian")?;

    println!("kernel image = {}", artifacts.image_deb.display());
    println!("kernel dtb   = {}", artifacts.dtb_deb.display());

    if let Some(headers) = artifacts.headers_deb {
        println!("kernel headers = {}", headers.display());
    }

    // Отсутствие загрузчика не ошибка: команда работает и на checkout, где
    // собирали только ядро.
    match inventory.uboot_artifact() {
        Ok(uboot) => println!("uboot = {}", uboot.display()),
        Err(error) => warn!(%error, "пакет U-Boot не найден"),
    }

    Ok(())
}

/// Возвращает Armbian-конфигурацию платы или понятную ошибку.
///
/// Команды `bsp-*` работают только с платами, чей BSP собирается через Armbian.
/// У остальных ядро приходит из архива Ubuntu, и синхронизировать нечего.
fn require_armbian(board: &BoardConfig) -> Result<platinum_board::ArmbianConfig> {
    board.armbian.clone().with_context(|| {
        format!(
            "плата `{}` не использует Armbian: в board.toml нет секции [armbian]",
            board.id
        )
    })
}

/// Загружает board-конфигурацию с понятным сообщением об ошибке.
fn load_board(path: &Path) -> Result<BoardConfig> {
    BoardConfig::load(path)
        .with_context(|| format!("не удалось загрузить board-конфигурацию {}", path.display()))
}

/// Загружает состав userspace, если сборка должна устанавливать пакеты.
///
/// Путь к `packages.toml` включает установку сам по себе, поэтому невозможно
/// запросить пакеты и получить сборку без них.
fn load_packages(arguments: &BuildArgs) -> Result<Option<PackagesConfig>> {
    if !arguments.with_packages && arguments.packages.is_none() {
        return Ok(None);
    }

    let path = arguments
        .packages
        .clone()
        .unwrap_or_else(|| PackagesConfig::default_path(&arguments.board));

    let packages = PackagesConfig::load(&path)
        .with_context(|| format!("не удалось загрузить состав userspace {}", path.display()))?;

    Ok(Some(packages))
}

/// Загружает системную конфигурацию, если сборка должна настраивать систему.
fn load_system(arguments: &BuildArgs) -> Result<Option<SystemConfig>> {
    if !arguments.with_system && arguments.system.is_none() {
        return Ok(None);
    }

    let path = arguments
        .system
        .clone()
        .unwrap_or_else(|| SystemConfig::default_path(&arguments.board));

    let system = SystemConfig::load(&path).with_context(|| {
        format!(
            "не удалось загрузить системную конфигурацию {}",
            path.display()
        )
    })?;

    Ok(Some(system))
}

/// Загружает разметку образа, если сборка должна выпустить `.img`.
fn load_partitions(arguments: &BuildArgs) -> Result<Option<PartitionsConfig>> {
    if !arguments.with_image && arguments.partitions.is_none() {
        return Ok(None);
    }

    let path = arguments
        .partitions
        .clone()
        .unwrap_or_else(|| PartitionsConfig::default_path(&arguments.board));

    let partitions = PartitionsConfig::load(&path)
        .with_context(|| format!("не удалось загрузить разметку образа {}", path.display()))?;

    Ok(Some(partitions))
}
