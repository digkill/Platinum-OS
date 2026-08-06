//! Проверяемое описание разметки дискового образа.
//!
//! Layout проверяется как данные до любых destructive операций: перекрытие
//! разделов или неверное смещение обнаруживаются на пустом образе, а не после
//! часа записи файловых систем.

use std::{fmt, str::FromStr};

use thiserror::Error;

/// Размер сектора, которым адресуются разделы MBR.
pub const SECTOR_SIZE: u64 = 512;

/// Секторов в одном mebibyte.
pub const SECTORS_PER_MIB: u64 = 1024 * 1024 / SECTOR_SIZE;

/// Максимум первичных разделов в таблице MBR.
const MAX_PARTITIONS: usize = 4;

/// Ошибки описания разметки.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LayoutError {
    /// Образ без разделов не с чего загружать.
    #[error("образ должен содержать хотя бы один раздел")]
    Empty,
    /// MBR хранит ровно четыре первичных раздела.
    #[error("в таблице MBR не может быть больше {MAX_PARTITIONS} разделов, описано {count}")]
    TooManyPartitions {
        /// Количество разделов в конфигурации.
        count: usize,
    },
    /// Имя нужно для диагностики и сопоставления с fstab.
    #[error("имя раздела не должно быть пустым")]
    EmptyName,
    /// Метка попадает в fstab как `LABEL=`, поэтому обязана быть непустой.
    #[error("метка раздела `{name}` не должна быть пустой")]
    EmptyLabel {
        /// Раздел без метки.
        name: String,
    },
    /// Слишком длинную метку e2fsprogs молча обрежет, и fstab перестанет
    /// находить раздел.
    #[error("метка `{label}` длиннее {limit} символов, допустимых для {filesystem}")]
    LabelTooLong {
        /// Отклонённая метка.
        label: String,
        /// Предел для этой файловой системы.
        limit: usize,
        /// Файловая система раздела.
        filesystem: Filesystem,
    },
    /// Дубликат имени или метки означает ошибку конфигурации.
    #[error("значение `{value}` встречается у нескольких разделов")]
    Duplicate {
        /// Повторяющееся имя или метка.
        value: String,
    },
    /// Нулевой размер создаёт раздел, который невозможно отформатировать.
    #[error("размер раздела `{name}` должен быть больше нуля")]
    ZeroSize {
        /// Раздел с нулевым размером.
        name: String,
    },
    /// Первый mebibyte занят таблицей разделов и загрузчиком.
    #[error("раздел `{name}` начинается с {start_mib} MiB: первый MiB зарезервирован")]
    StartsTooEarly {
        /// Раздел с недопустимым смещением.
        name: String,
        /// Заявленное смещение.
        start_mib: u64,
    },
    /// Раздел попадает в область, куда пишется загрузчик.
    ///
    /// Проверка отдельная от `Overlap`: загрузчик пишется мимо таблицы
    /// разделов, поэтому перекрытие с ним не видно ни `fdisk`, ни сборке
    /// образа — файловая система создаётся успешно и повреждается уже после.
    #[error(
        "раздел `{name}` начинается с {start_mib} MiB и попадает в область загрузчика \
         ({reserved_mib} MiB); увеличьте `start_mib` либо `reserved_mib`"
    )]
    StartsInsideBootloader {
        /// Раздел с недопустимым смещением.
        name: String,
        /// Заявленное смещение.
        start_mib: u64,
        /// Область, зарезервированная под загрузчик.
        reserved_mib: u64,
    },
    /// Перекрытие разделов уничтожает данные соседа.
    #[error("раздел `{name}` начинается с {start_mib} MiB и перекрывает предыдущий")]
    Overlap {
        /// Раздел, нарушающий порядок.
        name: String,
        /// Заявленное смещение.
        start_mib: u64,
    },
    /// Активным в MBR может быть только один раздел.
    #[error("bootable может быть только один раздел")]
    SeveralBootable,
    /// Точка монтирования попадает в fstab и обязана быть абсолютной.
    #[error("точка монтирования `{mount_point}` должна быть абсолютным путём")]
    RelativeMountPoint {
        /// Отклонённое значение.
        mount_point: String,
    },
    /// Файловая система не поддерживается сборщиком образа.
    #[error("неизвестная файловая система `{filesystem}`")]
    UnknownFilesystem {
        /// Отклонённое значение.
        filesystem: String,
    },
    /// Таблица MBR адресует сектора 32-битным числом.
    #[error("раздел `{name}` не помещается в 32-битную адресацию MBR")]
    TooLargeForMbr {
        /// Раздел, выходящий за предел адресации.
        name: String,
    },
}

/// Файловая система, которую умеет создавать сборщик образа.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filesystem {
    /// ext4 для корня и всего, что не читает прошивка платы.
    Ext4,
    /// FAT32 для загрузочных разделов, которые читает прошивка до запуска ядра.
    Fat32,
}

impl Filesystem {
    /// Возвращает утилиту создания файловой системы.
    pub fn mkfs_command(self) -> &'static str {
        match self {
            Self::Ext4 => "mkfs.ext4",
            Self::Fat32 => "mkfs.vfat",
        }
    }

    /// Возвращает предел длины метки, который допускает файловая система.
    ///
    /// У FAT метка живёт в загрузочном секторе и ограничена 11 символами.
    /// Более длинную `mkfs.vfat` обрежет, и `LABEL=` в fstab перестанет
    /// находить раздел — отказ, заметный только на устройстве.
    pub fn max_label_length(self) -> usize {
        match self {
            Self::Ext4 => 16,
            Self::Fat32 => 11,
        }
    }

    /// Возвращает код типа раздела MBR.
    ///
    /// Прошивка Raspberry Pi ищет загрузочный раздел по типу `0x0c`
    /// (FAT32 LBA); помеченный как Linux она пропустит и не загрузится.
    pub fn mbr_partition_type(self) -> u8 {
        match self {
            Self::Ext4 => 0x83,
            Self::Fat32 => 0x0c,
        }
    }

    /// Сообщает, заполняется ли раздел утилитой `mkfs` напрямую.
    ///
    /// `mkfs.ext4` умеет `-d`, а `mkfs.vfat` — нет: для FAT содержимое
    /// копируется отдельно через `mcopy` из mtools, без loop-устройств.
    pub fn populated_by_mkfs(self) -> bool {
        matches!(self, Self::Ext4)
    }
}

impl fmt::Display for Filesystem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ext4 => formatter.write_str("ext4"),
            Self::Fat32 => formatter.write_str("vfat"),
        }
    }
}

impl FromStr for Filesystem {
    type Err = LayoutError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ext4" => Ok(Self::Ext4),
            "fat32" | "vfat" => Ok(Self::Fat32),
            _ => Err(LayoutError::UnknownFilesystem {
                filesystem: value.to_owned(),
            }),
        }
    }
}

/// Один раздел конечного дискового образа.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionSpec {
    /// Логическое имя раздела для логов и диагностики.
    pub name: String,
    /// Метка файловой системы, по которой раздел находит fstab.
    pub label: String,
    /// Файловая система раздела.
    pub filesystem: Filesystem,
    /// Смещение начала раздела в mebibyte.
    pub start_mib: u64,
    /// Размер раздела в mebibyte.
    pub size_mib: u64,
    /// Точка монтирования в готовой системе, если раздел монтируется.
    pub mount_point: Option<String>,
    /// Отмечать ли раздел активным в таблице MBR.
    pub bootable: bool,
    /// Является ли раздел системным разделом UEFI.
    pub esp: bool,
}

impl PartitionSpec {
    /// Создаёт проверенное описание раздела.
    pub fn new(
        name: String,
        label: String,
        filesystem: Filesystem,
        start_mib: u64,
        size_mib: u64,
        mount_point: Option<String>,
    ) -> Result<Self, LayoutError> {
        if name.trim().is_empty() {
            return Err(LayoutError::EmptyName);
        }

        if label.trim().is_empty() {
            return Err(LayoutError::EmptyLabel { name });
        }

        if label.len() > filesystem.max_label_length() {
            return Err(LayoutError::LabelTooLong {
                label,
                limit: filesystem.max_label_length(),
                filesystem,
            });
        }

        if size_mib == 0 {
            return Err(LayoutError::ZeroSize { name });
        }

        if start_mib < 1 {
            return Err(LayoutError::StartsTooEarly { name, start_mib });
        }

        if let Some(mount_point) = &mount_point
            && !mount_point.starts_with('/')
        {
            return Err(LayoutError::RelativeMountPoint {
                mount_point: mount_point.clone(),
            });
        }

        Ok(Self {
            name,
            label,
            filesystem,
            start_mib,
            size_mib,
            mount_point,
            bootable: false,
            esp: false,
        })
    }

    /// Отмечает раздел активным в таблице MBR.
    pub fn bootable(mut self, bootable: bool) -> Self {
        self.bootable = bootable;

        self
    }

    /// Отмечает раздел как системный раздел UEFI.
    ///
    /// Флаги задаются отдельными методами, а не аргументами конструктора: два
    /// соседних `bool` в вызове переставляются местами незаметно, а разница
    /// между «активный» и «ESP» видна только на устройстве, которое не
    /// загрузилось.
    pub fn esp(mut self, esp: bool) -> Self {
        self.esp = esp;

        self
    }

    /// Возвращает код типа раздела для таблицы MBR.
    ///
    /// ESP имеет собственный тип: прошивка UEFI ищет раздел именно по нему и
    /// помеченный как обычный FAT пропустит.
    pub fn partition_type(&self) -> u8 {
        if self.esp {
            0xEF
        } else {
            self.filesystem.mbr_partition_type()
        }
    }

    /// Возвращает первый сектор раздела.
    pub fn start_sector(&self) -> u64 {
        self.start_mib * SECTORS_PER_MIB
    }

    /// Возвращает длину раздела в секторах.
    pub fn sectors(&self) -> u64 {
        self.size_mib * SECTORS_PER_MIB
    }

    /// Возвращает смещение раздела в байтах.
    pub fn offset_bytes(&self) -> u64 {
        self.start_mib * 1024 * 1024
    }

    /// Возвращает размер раздела в байтах.
    pub fn size_bytes(&self) -> u64 {
        self.size_mib * 1024 * 1024
    }

    /// Возвращает границу раздела в mebibyte.
    pub fn end_mib(&self) -> u64 {
        self.start_mib + self.size_mib
    }
}

/// Полная разметка дискового образа.
///
/// Используется таблица MBR, а не GPT: BROM Allwinner читает SPL начиная с
/// 8 КиБ, а первичный заголовок GPT занимает сектора 1..33 и был бы затёрт
/// загрузчиком.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageLayout {
    partitions: Vec<PartitionSpec>,
    reserved_mib: u64,
}

impl ImageLayout {
    /// Создаёт разметку, проверив порядок и границы разделов.
    ///
    /// `reserved_mib` — область в начале образа, куда `platform_install.sh`
    /// пишет загрузчик. Её размер задаётся данными платы, а не константой:
    /// смещения записи принадлежат family BSP, и у другой платы они другие.
    pub fn new(partitions: Vec<PartitionSpec>, reserved_mib: u64) -> Result<Self, LayoutError> {
        if partitions.is_empty() {
            return Err(LayoutError::Empty);
        }

        if partitions.len() > MAX_PARTITIONS {
            return Err(LayoutError::TooManyPartitions {
                count: partitions.len(),
            });
        }

        if partitions
            .iter()
            .filter(|partition| partition.bootable)
            .count()
            > 1
        {
            return Err(LayoutError::SeveralBootable);
        }

        let mut seen = Vec::new();
        for partition in &partitions {
            for value in [&partition.name, &partition.label] {
                if seen.contains(&value.as_str()) {
                    return Err(LayoutError::Duplicate {
                        value: value.clone(),
                    });
                }

                seen.push(value.as_str());
            }
        }

        // Разделы проверяются в объявленном порядке: конфигурация, которую
        // нужно мысленно пересортировать, чтобы понять, слишком легко читается
        // неверно.
        let mut boundary = 1;
        for partition in &partitions {
            // Область загрузчика проверяется до перекрытия разделов: запись
            // загрузчика идёт мимо таблицы разделов, и её след иначе виден
            // только как повреждённая файловая система на устройстве.
            if partition.start_mib < reserved_mib {
                return Err(LayoutError::StartsInsideBootloader {
                    name: partition.name.clone(),
                    start_mib: partition.start_mib,
                    reserved_mib,
                });
            }

            if partition.start_mib < boundary {
                return Err(LayoutError::Overlap {
                    name: partition.name.clone(),
                    start_mib: partition.start_mib,
                });
            }

            boundary = partition.end_mib();
        }

        Ok(Self {
            partitions,
            reserved_mib,
        })
    }

    /// Возвращает область, зарезервированную под загрузчик, в mebibyte.
    pub fn reserved_mib(&self) -> u64 {
        self.reserved_mib
    }

    /// Возвращает разделы в порядке объявления.
    pub fn partitions(&self) -> &[PartitionSpec] {
        &self.partitions
    }

    /// Возвращает размер образа в mebibyte.
    ///
    /// Размер определяется границей последнего раздела: незанятый хвост не
    /// нужен, а место под рост файловой системы даёт resize при первом запуске.
    pub fn size_mib(&self) -> u64 {
        self.partitions
            .iter()
            .map(PartitionSpec::end_mib)
            .max()
            .unwrap_or(1)
    }

    /// Возвращает размер образа в байтах.
    pub fn size_bytes(&self) -> u64 {
        self.size_mib() * 1024 * 1024
    }
}

#[cfg(test)]
mod tests {
    use super::{Filesystem, ImageLayout, LayoutError, PartitionSpec};

    fn partition(name: &str, label: &str, start_mib: u64, size_mib: u64) -> PartitionSpec {
        PartitionSpec::new(
            name.into(),
            label.into(),
            Filesystem::Ext4,
            start_mib,
            size_mib,
            Some("/".into()),
        )
        .expect("описание раздела должно быть корректным")
        .bootable(true)
    }

    #[test]
    fn computes_the_image_size_from_the_last_partition() {
        let layout = ImageLayout::new(vec![partition("root", "platinum-root", 16, 2048)], 1)
            .expect("разметка должна быть корректной");

        assert_eq!(layout.size_mib(), 2064);
        assert_eq!(layout.size_bytes(), 2064 * 1024 * 1024);
    }

    #[test]
    fn converts_offsets_to_sectors() {
        let root = partition("root", "platinum-root", 16, 2048);

        assert_eq!(root.start_sector(), 32_768);
        assert_eq!(root.sectors(), 4_194_304);
        assert_eq!(root.offset_bytes(), 16 * 1024 * 1024);
    }

    #[test]
    fn rejects_overlapping_partitions() {
        let error = ImageLayout::new(
            vec![
                partition("boot", "platinum-boot", 16, 256),
                PartitionSpec::new(
                    "root".into(),
                    "platinum-root".into(),
                    Filesystem::Ext4,
                    200,
                    2048,
                    Some("/".into()),
                )
                .expect("описание раздела должно быть корректным"),
            ],
            1,
        )
        .expect_err("перекрытие разделов должно отклоняться");

        assert!(matches!(error, LayoutError::Overlap { .. }));
    }

    #[test]
    fn rejects_a_partition_at_the_start_of_the_image() {
        let error = PartitionSpec::new(
            "root".into(),
            "platinum-root".into(),
            Filesystem::Ext4,
            0,
            2048,
            None,
        )
        .expect_err("раздел не должен начинаться с нулевого смещения");

        assert!(matches!(error, LayoutError::StartsTooEarly { .. }));
    }

    /// Раздел внутри области загрузчика обязан отклоняться разметкой.
    ///
    /// Проверка живая, а не теоретическая: `boot_package.fex` Allwinner пишется
    /// на 16400 KiB, и раздел с 16 MiB был собран без единой ошибки, после чего
    /// `e2fsck` нашёл разрушенный resize-inode. Таблица разделов такого
    /// перекрытия не показывает, поэтому поймать его может только разметка.
    #[test]
    fn rejects_a_partition_inside_the_bootloader_area() {
        let error = ImageLayout::new(vec![partition("root", "platinum-root", 16, 2048)], 32)
            .expect_err("раздел в области загрузчика должен отклоняться");

        assert_eq!(
            error,
            LayoutError::StartsInsideBootloader {
                name: "root".into(),
                start_mib: 16,
                reserved_mib: 32,
            }
        );
    }

    /// Граница области загрузчика включительна: с неё раздел уже допустим.
    #[test]
    fn accepts_a_partition_starting_at_the_reserved_boundary() {
        ImageLayout::new(vec![partition("root", "platinum-root", 32, 2048)], 32)
            .expect("раздел с границы области загрузчика должен приниматься");
    }

    #[test]
    fn rejects_a_label_longer_than_e2fsprogs_allows() {
        let error = PartitionSpec::new(
            "root".into(),
            "platinum-root-partition".into(),
            Filesystem::Ext4,
            16,
            2048,
            None,
        )
        .expect_err("слишком длинная метка должна отклоняться");

        assert!(matches!(error, LayoutError::LabelTooLong { .. }));
    }

    #[test]
    fn rejects_two_active_partitions() {
        let error = ImageLayout::new(
            vec![
                partition("boot", "platinum-boot", 16, 256),
                partition("root", "platinum-root", 272, 2048),
            ],
            1,
        )
        .expect_err("два активных раздела должны отклоняться");

        assert_eq!(error, LayoutError::SeveralBootable);
    }

    #[test]
    fn rejects_more_partitions_than_mbr_holds() {
        let partitions = (0..5)
            .map(|index| {
                PartitionSpec::new(
                    format!("part{index}"),
                    format!("platinum-{index}"),
                    Filesystem::Ext4,
                    16 + index * 16,
                    16,
                    None,
                )
                .expect("описание раздела должно быть корректным")
                .bootable(false)
                .esp(false)
            })
            .collect();

        let error =
            ImageLayout::new(partitions, 1).expect_err("пятый первичный раздел должен отклоняться");

        assert!(matches!(error, LayoutError::TooManyPartitions { count: 5 }));
    }
}
