//! Подготовка загрузки через скомпилированный `boot.scr` Armbian.
//!
//! Часть плат не может грузиться по `extlinux.conf`: vendor U-Boot либо старше
//! его поддержки, либо требует собственных адресов загрузки. Такие платы Armbian
//! обслуживает boot-скриптом, и Platinum использует тот же скрипт из pinned
//! checkout вместо собственной копии его логики.
//!
//! Сборка делает три вещи, которые на устройствах Armbian делает пакет
//! `armbian-bsp-cli`: компилирует `boot.cmd` в `boot.scr`, дописывает
//! `armbianEnv.txt` под разметку Platinum и заворачивает initramfs в uImage.
//! Пакет Armbian при этом не устанавливается: он тянет за собой политику
//! системы, которой Platinum управляет сам.

use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

use thiserror::Error;
use tracing::info;

use crate::boot::{BOOT_DIRECTORY, BootError, discover};

/// Каталог boot-скриптов внутри checkout Armbian.
const BOOTSCRIPT_DIRECTORY: &str = "config/bootscripts";

/// Каталог файлов окружения внутри checkout Armbian.
const BOOTENV_DIRECTORY: &str = "config/bootenv";

/// Имя исходного скрипта внутри `/boot`.
///
/// Имя фиксировано: `BOOTSCRIPT` Armbian всегда указывает назначение
/// `boot.cmd`, и его же ищет комментарий upstream о перекомпиляции.
const BOOT_COMMAND_FILE: &str = "boot.cmd";

/// Имя скомпилированного скрипта, который ищет U-Boot.
const BOOT_SCRIPT_FILE: &str = "boot.scr";

/// Имя файла окружения, читаемого скриптом.
const BOOT_ENVIRONMENT_FILE: &str = "armbianEnv.txt";

/// Имя обёрнутого initramfs, который загружает скрипт.
const UINITRD_FILE: &str = "uInitrd";

/// Симлинк на последнее установленное ядро; его загружает скрипт.
const KERNEL_LINK: &str = "Image";

/// Симлинк на каталог DTB установленного ядра.
const DTB_LINK: &str = "dtb";

/// Архитектура заголовка скомпилированного скрипта.
///
/// Armbian компилирует boot-скрипты с `-A arm` независимо от архитектуры платы
/// (`mkimage -C none -A arm -T script`), и U-Boot сверяет это поле.
const SCRIPT_ARCHITECTURE: &str = "arm";

/// Ошибки подготовки загрузки через boot-скрипт.
#[derive(Debug, Error)]
pub enum BootScriptError {
    /// Загрузочные файлы ядра не найдены или несогласованны.
    #[error(transparent)]
    Boot(#[from] BootError),
    /// В checkout Armbian нет файла, объявленного данными платы.
    #[error("в checkout Armbian нет `{path}`: проверьте `{field}` в board.toml")]
    MissingSource {
        /// Ожидавшийся путь внутри checkout.
        path: PathBuf,
        /// Поле board-конфигурации, которое задало имя файла.
        field: &'static str,
    },
    /// Симлинк, который загружает скрипт, не создан пакетом ядра.
    #[error("в `{path}` нет `{name}`: пакет ядра не создал симлинк, загрузка невозможна")]
    MissingKernelLink {
        /// Каталог `/boot` внутри rootfs.
        path: PathBuf,
        /// Имя отсутствующего симлинка.
        name: &'static str,
    },
    /// DTB платы не разрешается по пути, который использует скрипт.
    #[error("DTB платы `{dtb}` не найден: нет `{path}`")]
    MissingDeviceTree {
        /// Путь DTB из board-конфигурации.
        dtb: String,
        /// Ожидавшийся путь внутри rootfs.
        path: PathBuf,
    },
    /// Файл не удалось прочитать.
    #[error("не удалось прочитать `{path}`: {source}")]
    Read {
        /// Проблемный путь.
        path: PathBuf,
        /// Исходная ошибка файловой системы.
        #[source]
        source: io::Error,
    },
    /// Файл не удалось записать.
    #[error("не удалось записать `{path}`: {source}")]
    Write {
        /// Проблемный путь.
        path: PathBuf,
        /// Исходная ошибка файловой системы.
        #[source]
        source: io::Error,
    },
    /// Формат сжатия initramfs не выражается заголовком uImage.
    #[error("неизвестное сжатие initramfs `{path}`: первые байты {magic}")]
    UnknownRamdiskCompression {
        /// Файл initramfs, который не удалось опознать.
        path: PathBuf,
        /// Начало файла в шестнадцатеричном виде.
        magic: String,
    },
    /// `mkimage` отсутствует на хосте сборки.
    #[error("не удалось запустить `mkimage` ({operation}); установите u-boot-tools: {source}")]
    StartMkimage {
        /// Что именно собиралось: скрипт или initramfs.
        operation: &'static str,
        /// Исходная ошибка запуска процесса.
        #[source]
        source: io::Error,
    },
    /// `mkimage` завершился с ошибкой.
    #[error("`mkimage` ({operation}) завершился с кодом {code}: {stderr}")]
    MkimageFailed {
        /// Что именно собиралось: скрипт или initramfs.
        operation: &'static str,
        /// Код завершения или -1, если ОС не предоставила его.
        code: i32,
        /// Диагностика `mkimage`.
        stderr: String,
    },
}

/// Параметры загрузки платы, использующей boot-скрипт.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootScriptSpec {
    /// Источник корневой файловой системы, например `LABEL=platinum-root`.
    pub root_source: String,
    /// Тип корневой файловой системы.
    pub root_filesystem: String,
    /// Дополнительные аргументы командной строки ядра.
    ///
    /// Уходят в `extraboardargs`, а не в `extraargs`: второй занят файлом
    /// окружения upstream, и запись Platinum затёрла бы его значения.
    pub extra_arguments: Vec<String>,
    /// Имя boot-скрипта в `config/bootscripts` checkout Armbian.
    pub script: String,
    /// Имя файла окружения в `config/bootenv` checkout Armbian.
    pub environment: String,
    /// Архитектура заголовка uImage для initramfs.
    pub initrd_architecture: String,
    /// Префикс имён DT overlay, если плата их использует.
    pub overlay_prefix: Option<String>,
}

/// Готовит `boot.scr`, `armbianEnv.txt` и `uInitrd` в rootfs.
#[derive(Debug, Clone)]
pub struct BootScriptConfigurator {
    spec: BootScriptSpec,
}

impl BootScriptConfigurator {
    /// Создаёт конфигуратор для параметров загрузки платы.
    pub fn new(spec: BootScriptSpec) -> Self {
        Self { spec }
    }

    /// Готовит загрузку и возвращает путь к скомпилированному `boot.scr`.
    ///
    /// `checkout` — pinned checkout Armbian, откуда берутся скрипт и файл
    /// окружения. `dtb` — путь DTB платы внутри каталога пакета, например
    /// `allwinner/sun60i-a733-orangepi-zero3w.dtb`.
    pub fn apply(
        &self,
        rootfs: &Path,
        checkout: &Path,
        dtb: &str,
    ) -> Result<PathBuf, BootScriptError> {
        let boot = rootfs.join(BOOT_DIRECTORY);
        let artifacts = discover(&boot)?;

        // Скрипт грузит ядро и DTB по симлинкам, которые создаёт postinst
        // пакета ядра. Их отсутствие означает, что пакет ставился при
        // смонтированном FAT32 `/boot` и разложил файлы иначе.
        if !boot.join(KERNEL_LINK).is_file() {
            return Err(BootScriptError::MissingKernelLink {
                path: boot.clone(),
                name: KERNEL_LINK,
            });
        }

        let device_tree = boot.join(DTB_LINK).join(dtb);
        if !device_tree.is_file() {
            return Err(BootScriptError::MissingDeviceTree {
                dtb: dtb.to_owned(),
                path: device_tree,
            });
        }

        let command = self.install_boot_command(checkout, &boot)?;
        let script = compile_boot_script(&command, &boot.join(BOOT_SCRIPT_FILE))?;
        self.write_environment(checkout, &boot, dtb)?;
        self.wrap_initramfs(&boot, &artifacts.initrd)?;

        info!(
            version = %artifacts.version,
            path = %script.display(),
            "boot script prepared"
        );

        Ok(script)
    }

    /// Копирует boot-скрипт Armbian в `/boot/boot.cmd`.
    ///
    /// Скрипт копируется без изменений: правки Platinum разошлись бы с pinned
    /// checkout молча, а вся настройка выражается файлом окружения.
    fn install_boot_command(
        &self,
        checkout: &Path,
        boot: &Path,
    ) -> Result<PathBuf, BootScriptError> {
        let source = checkout.join(BOOTSCRIPT_DIRECTORY).join(&self.spec.script);
        if !source.is_file() {
            return Err(BootScriptError::MissingSource {
                path: source,
                field: "bootloader.script",
            });
        }

        let contents = fs::read(&source).map_err(|source_error| BootScriptError::Read {
            path: source.clone(),
            source: source_error,
        })?;

        let path = boot.join(BOOT_COMMAND_FILE);
        fs::write(&path, contents).map_err(|source| BootScriptError::Write {
            path: path.clone(),
            source,
        })?;

        Ok(path)
    }

    /// Пишет `armbianEnv.txt`: значения upstream плюс параметры Platinum.
    ///
    /// Порядок важен: `env import -t` оставляет последнее значение ключа,
    /// поэтому записи сборки идут после файла upstream и перекрывают его.
    fn write_environment(
        &self,
        checkout: &Path,
        boot: &Path,
        dtb: &str,
    ) -> Result<(), BootScriptError> {
        let source = checkout
            .join(BOOTENV_DIRECTORY)
            .join(&self.spec.environment);
        if !source.is_file() {
            return Err(BootScriptError::MissingSource {
                path: source,
                field: "bootloader.env",
            });
        }

        let upstream =
            fs::read_to_string(&source).map_err(|source_error| BootScriptError::Read {
                path: source.clone(),
                source: source_error,
            })?;

        let path = boot.join(BOOT_ENVIRONMENT_FILE);
        fs::write(&path, render_environment(&self.spec, &upstream, dtb)).map_err(|source| {
            BootScriptError::Write {
                path: path.clone(),
                source,
            }
        })
    }

    /// Заворачивает initramfs в uImage, который умеет грузить U-Boot.
    ///
    /// Результат пишется файлом, а не парой `uInitrd-<версия>` + симлинк, как у
    /// Armbian: разрешать симлинк пришлось бы vendor-загрузчику, а версия здесь
    /// ровно одна и выбирать не из чего.
    fn wrap_initramfs(&self, boot: &Path, initrd: &str) -> Result<(), BootScriptError> {
        let source = boot.join(initrd);
        let compression = ramdisk_compression(&source)?;
        let path = boot.join(UINITRD_FILE);

        run_mkimage(
            "initramfs",
            &[
                "-A",
                &self.spec.initrd_architecture,
                "-O",
                "linux",
                "-T",
                "ramdisk",
                "-C",
                compression,
                "-n",
                UINITRD_FILE,
                "-d",
                &source.display().to_string(),
                &path.display().to_string(),
            ],
        )
    }
}

/// Компилирует `boot.cmd` в `boot.scr` и возвращает путь результата.
fn compile_boot_script(command: &Path, script: &Path) -> Result<PathBuf, BootScriptError> {
    run_mkimage(
        "boot script",
        &[
            "-C",
            "none",
            "-A",
            SCRIPT_ARCHITECTURE,
            "-T",
            "script",
            "-d",
            &command.display().to_string(),
            &script.display().to_string(),
        ],
    )?;

    Ok(script.to_path_buf())
}

/// Запускает `mkimage` с готовым набором аргументов.
fn run_mkimage(operation: &'static str, arguments: &[&str]) -> Result<(), BootScriptError> {
    let output = Command::new("mkimage")
        .args(arguments)
        .output()
        .map_err(|source| BootScriptError::StartMkimage { operation, source })?;

    if output.status.success() {
        return Ok(());
    }

    Err(BootScriptError::MkimageFailed {
        operation,
        code: output.status.code().unwrap_or(-1),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

/// Формирует содержимое `armbianEnv.txt`.
///
/// Комментариев в файле нет: его читает `env import -t` внутри U-Boot, и
/// поведение парсера на комментариях зависит от версии загрузчика.
fn render_environment(spec: &BootScriptSpec, upstream: &str, dtb: &str) -> String {
    let mut contents = upstream.to_owned();
    if !contents.ends_with('\n') {
        contents.push('\n');
    }

    if let Some(prefix) = &spec.overlay_prefix {
        contents.push_str(&format!("overlay_prefix={prefix}\n"));
    }

    contents.push_str(&format!("fdtfile={dtb}\n"));

    if !spec.extra_arguments.is_empty() {
        contents.push_str(&format!(
            "extraboardargs={}\n",
            spec.extra_arguments.join(" ")
        ));
    }

    contents.push_str(&format!("rootdev={}\n", spec.root_source));
    contents.push_str(&format!("rootfstype={}\n", spec.root_filesystem));

    contents
}

/// Определяет сжатие initramfs по сигнатуре файла.
///
/// Заголовок uImage обязан описывать фактическое содержимое: initramfs-tools
/// выбирает формат сам, а зафиксированное в коде значение разошлось бы с ним
/// после смены выпуска Ubuntu.
fn ramdisk_compression(path: &Path) -> Result<&'static str, BootScriptError> {
    let contents = fs::read(path).map_err(|source| BootScriptError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    // Имена соответствуют значениям `mkimage -C`.
    const SIGNATURES: &[(&[u8], &str)] = &[
        (&[0x1f, 0x8b], "gzip"),
        (&[0x28, 0xb5, 0x2f, 0xfd], "zstd"),
        (&[0x42, 0x5a, 0x68], "bzip2"),
        (&[0x04, 0x22, 0x4d, 0x18], "lz4"),
        (&[0x89, 0x4c, 0x5a, 0x4f], "lzo"),
        (&[0x5d, 0x00, 0x00], "lzma"),
    ];

    for (magic, name) in SIGNATURES {
        if contents.starts_with(magic) {
            return Ok(name);
        }
    }

    Err(BootScriptError::UnknownRamdiskCompression {
        path: path.to_path_buf(),
        magic: contents
            .iter()
            .take(4)
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join(" "),
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{BootScriptError, BootScriptSpec, ramdisk_compression, render_environment};

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

    fn temporary(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "platinum-bootscript-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("системное время должно быть позже Unix epoch")
                .as_nanos()
        ))
    }

    /// Параметры Platinum обязаны идти после файла upstream.
    ///
    /// `env import -t` оставляет последнее значение ключа: при обратном порядке
    /// значение upstream перекрыло бы метку корня, заданную разметкой образа.
    #[test]
    fn platinum_values_override_the_upstream_environment() {
        let upstream = "verbosity=1\nconsole=both\nrootdev=/dev/mmcblk0p1\n";

        let contents = render_environment(
            &spec(),
            upstream,
            "allwinner/sun60i-a733-orangepi-zero3w.dtb",
        );

        let rootdev: Vec<&str> = contents
            .lines()
            .filter(|line| line.starts_with("rootdev="))
            .collect();
        assert_eq!(rootdev.last(), Some(&"rootdev=LABEL=platinum-root"));
        assert!(contents.contains("fdtfile=allwinner/sun60i-a733-orangepi-zero3w.dtb\n"));
        assert!(contents.contains("overlay_prefix=sun60i-a733\n"));
        assert!(contents.contains("rootfstype=ext4\n"));
    }

    /// Аргументы Platinum не должны затирать `extraargs` файла upstream.
    #[test]
    fn extra_arguments_go_to_the_board_specific_variable() {
        let mut spec = spec();
        spec.extra_arguments = vec!["quiet".into(), "loglevel=3".into()];

        let contents = render_environment(&spec, "extraargs=coherent_pool=2M\n", "board.dtb");

        assert!(contents.contains("extraargs=coherent_pool=2M\n"));
        assert!(contents.contains("extraboardargs=quiet loglevel=3\n"));
    }

    /// Пустой список не должен превращаться в пустую переменную окружения.
    #[test]
    fn omits_board_arguments_when_there_are_none() {
        let contents = render_environment(&spec(), "verbosity=1\n", "board.dtb");

        assert!(!contents.contains("extraboardargs"));
    }

    /// Файл окружения без завершающего перевода строки не должен склеиваться.
    #[test]
    fn separates_appended_values_from_an_unterminated_upstream_file() {
        let contents = render_environment(&spec(), "console=both", "board.dtb");

        assert!(contents.contains("console=both\n"));
    }

    #[test]
    fn recognises_the_compression_of_an_initramfs() {
        let root = temporary("compression");
        fs::create_dir_all(&root).expect("временный каталог должен создаваться");

        let gzip = root.join("initrd.img-gzip");
        fs::write(&gzip, [0x1f, 0x8b, 0x08, 0x00]).expect("initramfs должен записываться");
        assert_eq!(
            ramdisk_compression(&gzip).expect("gzip должен опознаваться"),
            "gzip"
        );

        let zstd = root.join("initrd.img-zstd");
        fs::write(&zstd, [0x28, 0xb5, 0x2f, 0xfd]).expect("initramfs должен записываться");
        assert_eq!(
            ramdisk_compression(&zstd).expect("zstd должен опознаваться"),
            "zstd"
        );

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }

    /// Неопознанный формат обязан быть ошибкой, а не молчаливым `none`.
    #[test]
    fn rejects_an_initramfs_of_an_unknown_format() {
        let root = temporary("unknown");
        fs::create_dir_all(&root).expect("временный каталог должен создаваться");

        let path = root.join("initrd.img-plain");
        fs::write(&path, b"not compressed").expect("initramfs должен записываться");

        let error =
            ramdisk_compression(&path).expect_err("неизвестный формат не должен приниматься");
        assert!(matches!(
            error,
            BootScriptError::UnknownRamdiskCompression { .. }
        ));

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }

    /// Имя скрипта из данных платы должно проверяться, а не подставляться молча.
    #[test]
    fn reports_a_boot_script_missing_from_the_checkout() {
        let root = temporary("checkout");
        let boot = root.join("rootfs/boot");
        fs::create_dir_all(&boot).expect("каталог boot должен создаваться");
        fs::create_dir_all(root.join("checkout/config/bootscripts"))
            .expect("каталог скриптов должен создаваться");

        let error = super::BootScriptConfigurator::new(spec())
            .install_boot_command(&root.join("checkout"), &boot)
            .expect_err("отсутствующий скрипт должен отклоняться");

        assert!(matches!(error, BootScriptError::MissingSource { .. }));

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }
}
