//! Выполнение команд внутри подготовленного rootfs.
//!
//! Chroot — единственное место, где Platinum получает права на целевую систему,
//! поэтому окружение здесь создаётся и снимается симметрично: смонтированные
//! `/proc` и `/dev` пережили бы неудачную сборку и заблокировали бы удаление
//! work-dir, а забытый `policy-rc.d` изменил бы поведение готового образа.

use std::{
    fs, io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

use thiserror::Error;
use tracing::{info, warn};

use crate::sys;

/// Файл, по которому каталог распознаётся как распакованный rootfs.
const ROOTFS_MARKER: &str = "etc/os-release";

/// Скрипт, запрещающий запуск сервисов при установке пакетов.
const POLICY_RC_D: &str = "usr/sbin/policy-rc.d";

/// Тело `policy-rc.d`: код 101 означает «действие запрещено политикой».
const POLICY_RC_D_BODY: &str =
    "#!/bin/sh\n# Platinum: сборка не должна запускать сервисы целевой системы.\nexit 101\n";

/// Файл резолвера внутри rootfs.
const RESOLV_CONF: &str = "etc/resolv.conf";

/// Резервная копия исходного резолвера rootfs.
const RESOLV_CONF_BACKUP: &str = "etc/resolv.conf.platinum-backup";

/// Каталог, в который копируется статический qemu.
const QEMU_DIRECTORY: &str = "usr/bin";

/// Каталоги хоста, где ищется статический qemu.
const QEMU_SEARCH_PATHS: [&str; 2] = ["/usr/bin", "/usr/local/bin"];

/// Каталог зарегистрированных интерпретаторов binfmt_misc.
const BINFMT_DIRECTORY: &str = "/proc/sys/fs/binfmt_misc";

/// Способ монтирования служебной файловой системы.
#[derive(Debug, Clone, Copy)]
enum MountKind {
    /// Монтирование по типу файловой системы.
    Virtual(&'static str),
    /// Bind-mount каталога хоста.
    Bind,
}

/// Описание одной служебной точки монтирования внутри rootfs.
#[derive(Debug, Clone, Copy)]
struct MountSpec {
    source: &'static str,
    kind: MountKind,
    target: &'static str,
}

/// Служебные файловые системы, без которых apt и maintainer-scripts падают.
///
/// Порядок важен: `dev/pts` монтируется поверх `dev`, поэтому снимается первым.
const MOUNTS: [MountSpec; 4] = [
    MountSpec {
        source: "proc",
        kind: MountKind::Virtual("proc"),
        target: "proc",
    },
    MountSpec {
        source: "sysfs",
        kind: MountKind::Virtual("sysfs"),
        target: "sys",
    },
    MountSpec {
        source: "/dev",
        kind: MountKind::Bind,
        target: "dev",
    },
    MountSpec {
        source: "/dev/pts",
        kind: MountKind::Bind,
        target: "dev/pts",
    },
];

/// Ошибки подготовки и выполнения chroot.
#[derive(Debug, Error)]
pub enum ChrootError {
    /// chroot и bind-mount существуют только на Linux.
    #[error("chroot в rootfs целевой платы поддерживается только на Linux, а хост — `{os}`")]
    UnsupportedHost {
        /// Семейство ОС хоста.
        os: String,
    },
    /// Без root нельзя ни смонтировать `/proc`, ни выполнить chroot.
    #[error("операции внутри rootfs требуют прав root")]
    NotRoot,
    /// Каталог не похож на распакованный Ubuntu Base.
    #[error("каталог `{path}` не является rootfs: отсутствует {marker}")]
    NotARootfs {
        /// Проверявшийся каталог.
        path: PathBuf,
        /// Ожидавшийся маркер.
        marker: &'static str,
    },
    /// Файловую систему не удалось смонтировать.
    #[error("не удалось смонтировать `{target}`: {stderr}")]
    Mount {
        /// Точка монтирования внутри rootfs.
        target: PathBuf,
        /// Диагностика `mount`.
        stderr: String,
    },
    /// Подготовительная файловая операция не удалась.
    #[error("не удалось подготовить `{path}`: {source}")]
    Prepare {
        /// Проблемный путь.
        path: PathBuf,
        /// Исходная ошибка файловой системы.
        #[source]
        source: io::Error,
    },
    /// Статический qemu не установлен на хосте.
    #[error(
        "для rootfs архитектуры `{architecture}` нужен `{binary}`; установите qemu-user-static"
    )]
    MissingQemu {
        /// Архитектура целевого rootfs.
        architecture: String,
        /// Имя недостающего бинарника.
        binary: String,
    },
    /// Архитектура не поддерживается адаптером qemu.
    #[error("неизвестная архитектура rootfs `{architecture}`")]
    UnknownArchitecture {
        /// Архитектура из board-конфигурации.
        architecture: String,
    },
    /// Процесс не удалось запустить.
    #[error("не удалось запустить `{program}` в rootfs: {source}")]
    StartCommand {
        /// Имя программы.
        program: String,
        /// Исходная ошибка запуска.
        #[source]
        source: io::Error,
    },
    /// Процесс завершился с ненулевым кодом.
    #[error("`{program}` завершился в rootfs с кодом {code}")]
    CommandFailed {
        /// Имя программы.
        program: String,
        /// Код завершения или -1, если ОС не предоставила его.
        code: i32,
    },
}

/// Подготовленный доступ к rootfs целевой платы.
#[derive(Debug, Clone)]
pub struct Chroot {
    root: PathBuf,
    architecture: String,
}

impl Chroot {
    /// Создаёт chroot для каталога rootfs и архитектуры платы.
    ///
    /// Каталог проверяется сразу: chroot в произвольный путь под root — самая
    /// дорогая ошибка, которую может допустить build-система.
    pub fn new(root: PathBuf, architecture: String) -> Result<Self, ChrootError> {
        if !root.join(ROOTFS_MARKER).is_file() {
            return Err(ChrootError::NotARootfs {
                path: root,
                marker: ROOTFS_MARKER,
            });
        }

        Ok(Self { root, architecture })
    }

    /// Возвращает каталог rootfs.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Готовит окружение и возвращает сессию для запуска команд.
    ///
    /// Всё, что создано во время подготовки, снимается при уничтожении сессии,
    /// в том числе при ошибке на середине настройки.
    pub fn enter(&self) -> Result<ChrootSession<'_>, ChrootError> {
        let os = sys::host_os();
        if os != "linux" {
            return Err(ChrootError::UnsupportedHost { os: os.to_owned() });
        }

        if !sys::is_root() {
            return Err(ChrootError::NotRoot);
        }

        let mut session = ChrootSession {
            root: &self.root,
            mounted: Vec::new(),
            qemu: None,
            resolv_conf: false,
            policy_rc_d: false,
        };

        session.mount_virtual_filesystems()?;
        session.install_policy_rc_d()?;
        session.install_resolv_conf()?;
        session.install_qemu(&self.architecture)?;

        info!(
            root = %self.root.display(),
            architecture = %self.architecture,
            "chroot prepared"
        );

        Ok(session)
    }
}

/// Активное окружение chroot.
///
/// Сессия существует только на время выполнения команд: её уничтожение
/// возвращает rootfs в состояние, пригодное для упаковки в образ.
#[derive(Debug)]
pub struct ChrootSession<'a> {
    root: &'a Path,
    mounted: Vec<PathBuf>,
    qemu: Option<PathBuf>,
    resolv_conf: bool,
    policy_rc_d: bool,
}

impl ChrootSession<'_> {
    /// Выполняет команду внутри rootfs, наследуя stdout и stderr.
    ///
    /// Окружение очищается: переменные хоста вроде `LANG` или `http_proxy`
    /// сделали бы результат установки пакетов зависимым от машины сборки.
    pub fn run(&self, program: &str, arguments: &[&str]) -> Result<(), ChrootError> {
        info!(program, ?arguments, "chroot command started");

        let status = Command::new("chroot")
            .arg(self.root)
            .arg(program)
            .args(arguments)
            .env_clear()
            .env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin")
            .env("DEBIAN_FRONTEND", "noninteractive")
            .env("DEBCONF_NONINTERACTIVE_SEEN", "true")
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .status()
            .map_err(|source| ChrootError::StartCommand {
                program: program.to_owned(),
                source,
            })?;

        if !status.success() {
            return Err(ChrootError::CommandFailed {
                program: program.to_owned(),
                code: status.code().unwrap_or(-1),
            });
        }

        Ok(())
    }

    /// Монтирует служебные файловые системы внутрь rootfs.
    fn mount_virtual_filesystems(&mut self) -> Result<(), ChrootError> {
        for spec in MOUNTS {
            let target = self.root.join(spec.target);
            create_directory(&target)?;

            let mut command = Command::new("mount");
            match spec.kind {
                MountKind::Virtual(filesystem) => {
                    command.arg("-t").arg(filesystem).arg(spec.source);
                }
                MountKind::Bind => {
                    command.arg("--bind").arg(spec.source);
                }
            }

            let output =
                command
                    .arg(&target)
                    .output()
                    .map_err(|source| ChrootError::StartCommand {
                        program: "mount".to_owned(),
                        source,
                    })?;

            if !output.status.success() {
                return Err(ChrootError::Mount {
                    target,
                    stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
                });
            }

            self.mounted.push(target);
        }

        Ok(())
    }

    /// Запрещает запуск сервисов целевой системы во время установки пакетов.
    fn install_policy_rc_d(&mut self) -> Result<(), ChrootError> {
        let path = self.root.join(POLICY_RC_D);
        if let Some(parent) = path.parent() {
            create_directory(parent)?;
        }

        fs::write(&path, POLICY_RC_D_BODY).map_err(|source| ChrootError::Prepare {
            path: path.clone(),
            source,
        })?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).map_err(|source| {
            ChrootError::Prepare {
                path: path.clone(),
                source,
            }
        })?;

        self.policy_rc_d = true;

        Ok(())
    }

    /// Подставляет резолвер хоста, сохранив исходный файл rootfs.
    ///
    /// Без DNS apt не разрешит имя зеркала; при этом резолвер хоста не должен
    /// остаться в готовом образе, поэтому он снимается вместе с сессией.
    fn install_resolv_conf(&mut self) -> Result<(), ChrootError> {
        let host = Path::new("/").join(RESOLV_CONF);
        let Ok(contents) = fs::read(&host) else {
            warn!(
                path = %host.display(),
                "резолвер хоста недоступен: установка пакетов возможна только с локальным зеркалом"
            );

            return Ok(());
        };

        let target = self.root.join(RESOLV_CONF);
        if let Some(parent) = target.parent() {
            create_directory(parent)?;
        }

        // symlink_metadata: в Ubuntu Base резолвер может быть симлинком на
        // systemd-resolved, и exists() по нему вернул бы false.
        if target.symlink_metadata().is_ok() {
            let backup = self.root.join(RESOLV_CONF_BACKUP);
            fs::rename(&target, &backup).map_err(|source| ChrootError::Prepare {
                path: target.clone(),
                source,
            })?;
        }

        fs::write(&target, contents).map_err(|source| ChrootError::Prepare {
            path: target.clone(),
            source,
        })?;

        self.resolv_conf = true;

        Ok(())
    }

    /// Копирует статический qemu, если архитектура rootfs чужая для хоста.
    fn install_qemu(&mut self, architecture: &str) -> Result<(), ChrootError> {
        let qemu_architecture =
            qemu_architecture(architecture).ok_or_else(|| ChrootError::UnknownArchitecture {
                architecture: architecture.to_owned(),
            })?;

        if qemu_architecture == sys::host_qemu_architecture() {
            return Ok(());
        }

        let binary = format!("qemu-{qemu_architecture}-static");
        let source = QEMU_SEARCH_PATHS
            .iter()
            .map(|directory| Path::new(directory).join(&binary))
            .find(|candidate| candidate.is_file())
            .ok_or_else(|| ChrootError::MissingQemu {
                architecture: architecture.to_owned(),
                binary: binary.clone(),
            })?;

        warn_when_binfmt_is_not_registered(qemu_architecture);

        let target = self.root.join(QEMU_DIRECTORY).join(&binary);
        if let Some(parent) = target.parent() {
            create_directory(parent)?;
        }

        fs::copy(&source, &target).map_err(|source| ChrootError::Prepare {
            path: target.clone(),
            source,
        })?;
        fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).map_err(|source| {
            ChrootError::Prepare {
                path: target.clone(),
                source,
            }
        })?;

        self.qemu = Some(target);

        Ok(())
    }
}

impl Drop for ChrootSession<'_> {
    /// Снимает всё, что было создано подготовкой сессии.
    ///
    /// Ошибки только логируются: Drop не может их вернуть, а прерывать сборку
    /// паникой на этапе уборки хуже, чем оставить диагностику в логе.
    fn drop(&mut self) {
        for target in self.mounted.iter().rev() {
            unmount(target);
        }

        if let Some(qemu) = &self.qemu {
            remove_quietly(qemu);
        }

        if self.policy_rc_d {
            remove_quietly(&self.root.join(POLICY_RC_D));
        }

        if self.resolv_conf {
            let target = self.root.join(RESOLV_CONF);
            remove_quietly(&target);

            let backup = self.root.join(RESOLV_CONF_BACKUP);
            if backup.symlink_metadata().is_ok()
                && let Err(error) = fs::rename(&backup, &target)
            {
                warn!(path = %target.display(), %error, "не удалось восстановить резолвер rootfs");
            }
        }
    }
}

/// Отмонтирует точку, при неудаче повторяя попытку в ленивом режиме.
///
/// Ленивое размонтирование — компромисс: занятая точка монтирования иначе
/// осталась бы висеть и сделала бы rootfs непригодным для упаковки.
fn unmount(target: &Path) {
    let output = Command::new("umount").arg(target).output();
    match output {
        Ok(output) if output.status.success() => return,
        Ok(output) => warn!(
            path = %target.display(),
            stderr = %String::from_utf8_lossy(&output.stderr).trim(),
            "не удалось отмонтировать точку, повтор в ленивом режиме"
        ),
        Err(error) => warn!(path = %target.display(), %error, "не удалось запустить umount"),
    }

    if let Err(error) = Command::new("umount").arg("-l").arg(target).status() {
        warn!(path = %target.display(), %error, "ленивое размонтирование не выполнено");
    }
}

/// Удаляет файл, сообщая о проблеме в лог.
fn remove_quietly(path: &Path) {
    if let Err(error) = fs::remove_file(path) {
        warn!(path = %path.display(), %error, "не удалось удалить временный файл rootfs");
    }
}

/// Создаёт каталог вместе с родителями.
fn create_directory(path: &Path) -> Result<(), ChrootError> {
    fs::create_dir_all(path).map_err(|source| ChrootError::Prepare {
        path: path.to_path_buf(),
        source,
    })
}

/// Предупреждает, если binfmt_misc не знает нужный интерпретатор.
///
/// Без регистрации chroot чужой архитектуры падает с `Exec format error`, и
/// причина по такому сообщению не очевидна.
fn warn_when_binfmt_is_not_registered(qemu_architecture: &str) {
    let prefix = format!("qemu-{qemu_architecture}");
    let registered = fs::read_dir(BINFMT_DIRECTORY)
        .map(|entries| {
            entries.flatten().any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(prefix.as_str())
            })
        })
        .unwrap_or(false);

    if !registered {
        warn!(
            interpreter = %prefix,
            "интерпретатор не зарегистрирован в binfmt_misc: команды в rootfs могут не запуститься"
        );
    }
}

/// Переводит архитектуру Debian в архитектуру qemu.
fn qemu_architecture(architecture: &str) -> Option<&'static str> {
    match architecture {
        "arm64" => Some("aarch64"),
        "armhf" | "armel" => Some("arm"),
        "amd64" => Some("x86_64"),
        "i386" => Some("i386"),
        "riscv64" => Some("riscv64"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{Chroot, ChrootError, qemu_architecture};

    fn temporary_directory(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "platinum-chroot-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("системное время должно быть позже Unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&path).expect("временный каталог должен создаваться");

        path
    }

    #[test]
    fn maps_debian_architectures_to_qemu() {
        assert_eq!(qemu_architecture("arm64"), Some("aarch64"));
        assert_eq!(qemu_architecture("amd64"), Some("x86_64"));
        assert_eq!(qemu_architecture("sparc"), None);
    }

    #[test]
    fn refuses_a_directory_without_a_rootfs_marker() {
        let root = temporary_directory("marker");

        let error = Chroot::new(root.clone(), "arm64".into())
            .expect_err("каталог без os-release не должен приниматься за rootfs");

        assert!(matches!(error, ChrootError::NotARootfs { .. }));

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }

    #[test]
    fn accepts_a_directory_with_a_rootfs_marker() {
        let root = temporary_directory("valid");
        fs::create_dir_all(root.join("etc")).expect("каталог etc должен создаваться");
        fs::write(root.join("etc/os-release"), b"ID=ubuntu\n")
            .expect("маркер rootfs должен записываться");

        let chroot =
            Chroot::new(root.clone(), "arm64".into()).expect("каталог rootfs должен приниматься");

        assert_eq!(chroot.root(), root);

        fs::remove_dir_all(root).expect("временный каталог должен удаляться");
    }
}
