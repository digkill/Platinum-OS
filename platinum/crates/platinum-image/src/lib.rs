//! Разметка и сборка дисковых образов Platinum OS One.
//!
//! Разметка описывается данными и проверяется отдельно от записи: ошибка в
//! смещении или размере обнаруживается до создания файловых систем, а не после
//! часа записи гигабайтов.

mod build;
mod layout;
mod mbr;
mod uboot;

pub use build::{ImageBuilder, ImageError};
pub use layout::{
    Filesystem, ImageLayout, LayoutError, PartitionSpec, SECTOR_SIZE, SECTORS_PER_MIB,
};
pub use mbr::render_boot_sector;
pub use uboot::{UbootError, write_uboot};
