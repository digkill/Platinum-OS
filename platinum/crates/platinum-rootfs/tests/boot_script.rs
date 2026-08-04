//! Проверка подготовки загрузки настоящим `mkimage`.
//!
//! Тест требует u-boot-tools и пропускается там, где их нет: содержимое
//! `armbianEnv.txt` покрыто unit-тестами, а здесь проверяется именно связка с
//! внешней утилитой — заголовки uImage и раскладка файлов в `/boot`.

#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use platinum_rootfs::{BootScriptConfigurator, BootScriptSpec};

/// Магическое число заголовка uImage.
const UIMAGE_MAGIC: [u8; 4] = [0x27, 0x05, 0x19, 0x56];

/// Версия ядра, общая для всех файлов тестового `/boot`.
const VERSION: &str = "6.6.0-vendor-sun60iw2";

/// DTB тестовой платы внутри каталога пакета.
const DTB: &str = "allwinner/sun60i-a733-orangepi-zero3w.dtb";

/// Сообщает, доступна ли утилита сборки образов U-Boot.
fn mkimage_available() -> bool {
    Command::new("mkimage")
        .arg("-V")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Создаёт rootfs с `/boot` в том виде, в каком его оставляют пакеты Armbian.
fn rootfs_with_installed_kernel(root: &Path) -> PathBuf {
    let rootfs = root.join("rootfs");
    let boot = rootfs.join("boot");
    let dtb_directory = format!("dtb-{VERSION}");

    fs::create_dir_all(boot.join(&dtb_directory).join("allwinner"))
        .expect("каталог DTB должен создаваться");
    fs::write(boot.join(format!("vmlinuz-{VERSION}")), b"kernel")
        .expect("ядро должно записываться");
    // Сигнатура gzip: сборка определяет сжатие initramfs по ней, а не по
    // конфигурации.
    fs::write(
        boot.join(format!("initrd.img-{VERSION}")),
        [0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00],
    )
    .expect("initramfs должен записываться");
    fs::write(boot.join(&dtb_directory).join(DTB), b"dtb").expect("DTB должен записываться");

    // Симлинки создаёт postinst пакетов ядра; boot-скрипт грузится только по ним.
    symlink(format!("vmlinuz-{VERSION}"), boot.join("Image"))
        .expect("симлинк ядра должен создаваться");
    symlink(&dtb_directory, boot.join("dtb")).expect("симлинк каталога DTB должен создаваться");

    rootfs
}

/// Создаёт checkout Armbian с boot-скриптом и файлом окружения.
fn checkout_with_boot_script(root: &Path) -> PathBuf {
    let checkout = root.join("armbian");
    fs::create_dir_all(checkout.join("config/bootscripts"))
        .expect("каталог скриптов должен создаваться");
    fs::create_dir_all(checkout.join("config/bootenv"))
        .expect("каталог окружения должен создаваться");

    fs::write(
        checkout.join("config/bootscripts/boot-sun60iw2.cmd"),
        "setenv kernel_addr_r \"0x41000000\"\n\
         load ${devtype} ${devnum} ${kernel_addr_r} ${prefix}Image\n\
         booti ${kernel_addr_r} ${ramdisk_addr_r} ${fdt_addr_r}\n",
    )
    .expect("boot-скрипт должен записываться");
    fs::write(
        checkout.join("config/bootenv/sun60iw2.txt"),
        "verbosity=1\nconsole=both\nextraargs=coherent_pool=2M\n",
    )
    .expect("файл окружения должен записываться");

    checkout
}

fn spec() -> BootScriptSpec {
    BootScriptSpec {
        root_source: "LABEL=platinum-root".into(),
        root_filesystem: "ext4".into(),
        extra_arguments: Vec::new(),
        script: "boot-sun60iw2.cmd".into(),
        environment: "sun60iw2.txt".into(),
        initrd_architecture: "arm".into(),
        overlay_prefix: Some("sun60i-a733".into()),
    }
}

fn test_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "platinum-boot-script-{label}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("системное время должно быть позже Unix epoch")
            .as_nanos()
    ))
}

#[test]
fn prepares_every_file_the_boot_script_loads() {
    if !mkimage_available() {
        eprintln!("mkimage недоступен: проверка подготовки загрузки пропущена");

        return;
    }

    let root = test_root("apply");
    let rootfs = rootfs_with_installed_kernel(&root);
    let checkout = checkout_with_boot_script(&root);

    let script = BootScriptConfigurator::new(spec())
        .apply(&rootfs, &checkout, DTB)
        .expect("загрузка должна подготавливаться");

    assert_eq!(script, rootfs.join("boot/boot.scr"));

    let compiled = fs::read(&script).expect("boot.scr должен читаться");
    assert!(
        compiled.starts_with(&UIMAGE_MAGIC),
        "boot.scr обязан быть образом U-Boot, а не текстом скрипта"
    );

    let uinitrd = fs::read(rootfs.join("boot/uInitrd")).expect("uInitrd должен читаться");
    assert!(
        uinitrd.starts_with(&UIMAGE_MAGIC),
        "initramfs обязан быть завёрнут в uImage"
    );

    // Исходник скрипта остаётся рядом: комментарий upstream описывает
    // перекомпиляцию именно из него.
    assert!(rootfs.join("boot/boot.cmd").is_file());

    let environment =
        fs::read_to_string(rootfs.join("boot/armbianEnv.txt")).expect("окружение должно читаться");
    assert!(environment.contains("extraargs=coherent_pool=2M\n"));
    assert!(environment.contains(&format!("fdtfile={DTB}\n")));
    assert!(environment.contains("rootdev=LABEL=platinum-root\n"));
    assert!(environment.contains("rootfstype=ext4\n"));

    fs::remove_dir_all(root).expect("временный каталог должен удаляться");
}

/// Без симлинка ядра образ собрался бы, но не стартовал.
#[test]
fn refuses_a_boot_directory_without_the_kernel_link() {
    let root = test_root("no-link");
    let rootfs = rootfs_with_installed_kernel(&root);
    let checkout = checkout_with_boot_script(&root);
    fs::remove_file(rootfs.join("boot/Image")).expect("симлинк ядра должен удаляться");

    let error = BootScriptConfigurator::new(spec())
        .apply(&rootfs, &checkout, DTB)
        .expect_err("отсутствие симлинка ядра должно отклоняться");

    assert!(error.to_string().contains("Image"));

    fs::remove_dir_all(root).expect("временный каталог должен удаляться");
}

/// Путь DTB проверяется тем же способом, каким его читает скрипт.
#[test]
fn refuses_a_device_tree_of_another_board() {
    let root = test_root("other-dtb");
    let rootfs = rootfs_with_installed_kernel(&root);
    let checkout = checkout_with_boot_script(&root);

    let error = BootScriptConfigurator::new(spec())
        .apply(
            &rootfs,
            &checkout,
            "allwinner/sun50i-h618-orangepi-zero3.dtb",
        )
        .expect_err("чужой DTB не должен приниматься");

    assert!(error.to_string().contains("sun50i-h618-orangepi-zero3.dtb"));

    fs::remove_dir_all(root).expect("временный каталог должен удаляться");
}
