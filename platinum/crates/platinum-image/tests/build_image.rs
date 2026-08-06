//! Проверка сборки образа с настоящей файловой системой.
//!
//! Тест требует `mkfs.ext4` и пропускается там, где e2fsprogs нет: разметка и
//! таблица разделов покрыты unit-тестами, а здесь проверяется именно связка с
//! внешней утилитой — набор аргументов и перенос файловой системы по смещению.

use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use platinum_image::{Filesystem, ImageBuilder, ImageLayout, PartitionSpec};

/// Смещение магического числа ext4 внутри суперблока.
const EXT4_MAGIC_OFFSET: u64 = 0x438;

/// Магическое число ext4.
const EXT4_MAGIC: [u8; 2] = [0x53, 0xEF];

/// Сообщает, доступна ли утилита создания файловой системы.
fn mkfs_available() -> bool {
    Command::new("mkfs.ext4")
        .arg("-V")
        .output()
        .map(|output| output.status.success() || output.status.code() == Some(1))
        .unwrap_or(false)
}

#[test]
fn builds_an_image_with_a_real_filesystem() {
    if !mkfs_available() {
        eprintln!("mkfs.ext4 недоступен: проверка сборки образа пропущена");

        return;
    }

    let root: PathBuf = std::env::temp_dir().join(format!(
        "platinum-image-build-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("системное время должно быть позже Unix epoch")
            .as_nanos()
    ));
    let rootfs = root.join("rootfs");
    fs::create_dir_all(rootfs.join("etc")).expect("каталог rootfs должен создаваться");
    fs::write(rootfs.join("etc/os-release"), b"ID=platinum\n")
        .expect("файл rootfs должен записываться");

    let layout = ImageLayout::new(
        vec![
            PartitionSpec::new(
                "root".into(),
                "platinum-root".into(),
                Filesystem::Ext4,
                16,
                32,
                Some("/".into()),
            )
            .expect("описание раздела должно быть корректным")
            .bootable(true),
        ],
        1,
    )
    .expect("разметка должна быть корректной");

    let image = root.join("platinum.img");
    ImageBuilder::new(layout)
        .build(&rootfs, &image)
        .expect("образ должен собираться");

    let size = fs::metadata(&image)
        .expect("метаданные образа должны читаться")
        .len();
    assert_eq!(size, 48 * 1024 * 1024, "размер образа задаётся разметкой");

    let mut file = File::open(&image).expect("образ должен открываться");

    let mut signature = [0_u8; 2];
    file.seek(SeekFrom::Start(510))
        .and_then(|_| file.read_exact(&mut signature))
        .expect("подпись загрузочного сектора должна читаться");
    assert_eq!(signature, [0x55, 0xAA]);

    let mut magic = [0_u8; 2];
    file.seek(SeekFrom::Start(16 * 1024 * 1024 + EXT4_MAGIC_OFFSET))
        .and_then(|_| file.read_exact(&mut magic))
        .expect("суперблок раздела должен читаться");
    assert_eq!(
        magic, EXT4_MAGIC,
        "файловая система должна лежать по смещению раздела"
    );

    assert!(
        !image.with_extension("root.fs").exists(),
        "промежуточный файл файловой системы должен удаляться"
    );

    fs::remove_dir_all(root).expect("временный каталог должен удаляться");
}

/// Загрузчик пишется по каталогу, который объявляет сам `platform_install.sh`.
///
/// Armbian кладёт бинарники в `/usr/lib/linux-u-boot-<branch>-<board>`, а не
/// рядом со скриптом: путь зависит от платы, и жёстко заданный каталог привёл
/// бы к «missing boot0_sdcard.fex» на каждой сборке.
#[test]
fn writes_the_bootloader_using_the_directory_declared_by_the_script() {
    let root: PathBuf = std::env::temp_dir().join(format!(
        "platinum-uboot-write-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("системное время должно быть позже Unix epoch")
            .as_nanos()
    ));
    let rootfs = root.join("rootfs");
    let binaries = rootfs.join("usr/lib/linux-u-boot-vendor-test");
    fs::create_dir_all(rootfs.join("usr/lib/u-boot")).expect("каталог скрипта должен создаваться");
    fs::create_dir_all(&binaries).expect("каталог бинарей должен создаваться");

    fs::write(binaries.join("boot0_sdcard.fex"), b"PLATINUM-BOOT0")
        .expect("blob загрузчика должен записываться");
    fs::write(
        rootfs.join("usr/lib/u-boot/platform_install.sh"),
        "DIR=/usr/lib/linux-u-boot-vendor-test\n\
         write_uboot_platform() {\n\
         \x20 dd conv=notrunc status=none if=\"$1/boot0_sdcard.fex\" of=\"$2\" bs=1k seek=8\n\
         }\n",
    )
    .expect("скрипт установки должен записываться");

    let image = root.join("platinum.img");
    let file = File::create(&image).expect("образ должен создаваться");
    file.set_len(1024 * 1024)
        .expect("размер образа должен задаваться");
    drop(file);

    platinum_image::write_uboot(&rootfs, &image).expect("загрузчик должен записываться");

    let mut written = [0_u8; 14];
    let mut file = File::open(&image).expect("образ должен открываться");
    file.seek(SeekFrom::Start(8 * 1024))
        .and_then(|_| file.read_exact(&mut written))
        .expect("данные загрузчика должны читаться");
    assert_eq!(
        &written, b"PLATINUM-BOOT0",
        "blob пишется со смещения 8 КиБ"
    );

    fs::remove_dir_all(root).expect("временный каталог должен удаляться");
}
