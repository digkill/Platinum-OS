//! Формирование таблицы разделов MBR.
//!
//! Таблица собирается в Rust, а не вызовом `sfdisk`: 64 байта фиксированного
//! формата проще проверить unit-тестом, чем вывод внешней утилиты, и сборка не
//! зависит от версии util-linux на машине.

use crate::layout::{ImageLayout, LayoutError, PartitionSpec, SECTOR_SIZE};

/// Смещение таблицы разделов внутри загрузочного сектора.
const TABLE_OFFSET: usize = 446;

/// Смещение подписи диска.
const DISK_SIGNATURE_OFFSET: usize = 440;

/// Размер одной записи таблицы.
const ENTRY_SIZE: usize = 16;

/// Признак активного раздела.
const BOOTABLE_FLAG: u8 = 0x80;

/// Подпись загрузочного сектора.
const BOOT_SIGNATURE: [u8; 2] = [0x55, 0xAA];

/// Подпись диска Platinum.
///
/// Значение фиксировано, а не случайно: одинаковые входные данные должны давать
/// побайтово одинаковый образ. Цена — совпадение идентификатора, если к одной
/// машине подключить две карты Platinum сразу; для сборочной системы это менее
/// важно, чем воспроизводимость.
const DISK_SIGNATURE: [u8; 4] = *b"PLTM";

/// CHS-адрес для разделов за пределами старой геометрии.
///
/// Современные загрузчики читают LBA-поля, а CHS сохраняют по традиции; 0xFEFFFF
/// — общепринятое значение «адрес не выражается в CHS».
const CHS_BEYOND_LIMIT: [u8; 3] = [0xFE, 0xFF, 0xFF];

/// Собирает загрузочный сектор с таблицей разделов.
///
/// Первые 440 байт остаются нулями: место кода MBR занимает SPL, который
/// записывает `platform_install.sh` из пакета U-Boot.
pub fn render_boot_sector(layout: &ImageLayout) -> Result<Vec<u8>, LayoutError> {
    let mut sector = vec![0_u8; SECTOR_SIZE as usize];

    sector[DISK_SIGNATURE_OFFSET..DISK_SIGNATURE_OFFSET + DISK_SIGNATURE.len()]
        .copy_from_slice(&DISK_SIGNATURE);

    for (index, partition) in layout.partitions().iter().enumerate() {
        let entry = render_entry(partition)?;
        let offset = TABLE_OFFSET + index * ENTRY_SIZE;

        sector[offset..offset + ENTRY_SIZE].copy_from_slice(&entry);
    }

    sector[SECTOR_SIZE as usize - 2..].copy_from_slice(&BOOT_SIGNATURE);

    Ok(sector)
}

/// Формирует одну 16-байтовую запись таблицы разделов.
fn render_entry(partition: &PartitionSpec) -> Result<[u8; ENTRY_SIZE], LayoutError> {
    let start =
        u32::try_from(partition.start_sector()).map_err(|_| LayoutError::TooLargeForMbr {
            name: partition.name.clone(),
        })?;
    let sectors = u32::try_from(partition.sectors()).map_err(|_| LayoutError::TooLargeForMbr {
        name: partition.name.clone(),
    })?;

    // Переполнение при сложении означает раздел за пределом 2 TiB: такой образ
    // MBR не адресует, и молча обрезать его нельзя.
    start
        .checked_add(sectors)
        .ok_or_else(|| LayoutError::TooLargeForMbr {
            name: partition.name.clone(),
        })?;

    let mut entry = [0_u8; ENTRY_SIZE];
    entry[0] = if partition.bootable { BOOTABLE_FLAG } else { 0 };
    entry[1..4].copy_from_slice(&CHS_BEYOND_LIMIT);
    entry[4] = partition.partition_type();
    entry[5..8].copy_from_slice(&CHS_BEYOND_LIMIT);
    entry[8..12].copy_from_slice(&start.to_le_bytes());
    entry[12..16].copy_from_slice(&sectors.to_le_bytes());

    Ok(entry)
}

#[cfg(test)]
mod tests {
    use crate::layout::{Filesystem, ImageLayout, PartitionSpec};

    use super::{BOOT_SIGNATURE, TABLE_OFFSET, render_boot_sector};

    fn layout() -> ImageLayout {
        ImageLayout::new(
            vec![
                PartitionSpec::new(
                    "root".into(),
                    "platinum-root".into(),
                    Filesystem::Ext4,
                    16,
                    2048,
                    Some("/".into()),
                )
                .expect("описание раздела должно быть корректным")
                .bootable(true),
            ],
            1,
        )
        .expect("разметка должна быть корректной")
    }

    #[test]
    fn writes_the_partition_entry_in_little_endian_lba() {
        let sector = render_boot_sector(&layout()).expect("сектор должен формироваться");

        assert_eq!(sector.len(), 512);
        assert_eq!(sector[TABLE_OFFSET], 0x80, "раздел должен быть активным");
        assert_eq!(sector[TABLE_OFFSET + 4], 0x83, "тип раздела Linux");
        assert_eq!(
            u32::from_le_bytes(
                sector[TABLE_OFFSET + 8..TABLE_OFFSET + 12]
                    .try_into()
                    .expect("поле LBA занимает четыре байта")
            ),
            32_768
        );
        assert_eq!(
            u32::from_le_bytes(
                sector[TABLE_OFFSET + 12..TABLE_OFFSET + 16]
                    .try_into()
                    .expect("поле длины занимает четыре байта")
            ),
            4_194_304
        );
    }

    #[test]
    fn keeps_the_bootloader_area_and_the_boot_signature() {
        let sector = render_boot_sector(&layout()).expect("сектор должен формироваться");

        assert!(
            sector[..440].iter().all(|byte| *byte == 0),
            "место SPL должно оставаться нулевым"
        );
        assert_eq!(&sector[510..], &BOOT_SIGNATURE);
    }

    #[test]
    fn leaves_unused_entries_empty() {
        let sector = render_boot_sector(&layout()).expect("сектор должен формироваться");

        assert!(
            sector[TABLE_OFFSET + 16..510].iter().all(|byte| *byte == 0),
            "неиспользованные записи таблицы должны быть нулевыми"
        );
    }
}
