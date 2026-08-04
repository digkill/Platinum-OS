use anyhow::{Context, Result};
use platinum_board::SystemConfig;
use platinum_core::{BuildContext, Stage};
use platinum_rootfs::{Chroot, Filesystem, SystemConfigurator, SystemSpec, User};

use crate::outputs;

/// Настройка `/etc` готового rootfs: имя устройства, локаль, пользователи.
///
/// Stage идёт последним: он опирается на пакеты (`locales`, `tzdata`) и должен
/// видеть систему в том виде, в каком она попадёт в образ.
pub struct ConfigureSystemStage {
    architecture: String,
    configurator: SystemConfigurator,
}

impl ConfigureSystemStage {
    /// Создаёт stage для архитектуры платы и проверенной конфигурации.
    pub fn new(architecture: String, spec: SystemSpec) -> Self {
        Self {
            architecture,
            configurator: SystemConfigurator::new(spec),
        }
    }
}

impl Stage for ConfigureSystemStage {
    fn name(&self) -> &'static str {
        "configure-system"
    }

    fn execute(&self, context: &mut BuildContext) -> Result<()> {
        let rootfs = context.require_output(outputs::ROOTFS_DIR)?.to_path_buf();

        let chroot = Chroot::new(rootfs, self.architecture.clone())
            .context("каталог rootfs непригоден для chroot")?;

        self.configurator
            .apply(&chroot)
            .context("не удалось применить системную конфигурацию к rootfs")?;

        Ok(())
    }
}

/// Переводит конфигурацию из TOML в проверенную спецификацию.
///
/// Преобразование живёт в builder, а не в platinum-board: crate конфигурации не
/// должен знать правила валидации целевой системы, а platinum-rootfs — формат
/// файлов на диске.
///
/// `image_filesystems` приходят из `partitions.toml` и идут в fstab первыми:
/// корень системы должен монтироваться раньше всего, что описано вручную.
pub fn system_spec(
    config: SystemConfig,
    image_filesystems: Vec<Filesystem>,
    modules: Vec<String>,
) -> Result<SystemSpec> {
    let mut users = Vec::with_capacity(config.users.len());
    for user in config.users {
        users.push(
            User::new(
                user.name.clone(),
                user.password_hash,
                user.groups,
                user.shell,
                user.force_password_change,
            )
            .with_context(|| format!("некорректная учётная запись `{}`", user.name))?,
        );
    }

    let mut filesystems = image_filesystems;
    filesystems.reserve(config.filesystems.len());
    for filesystem in config.filesystems {
        filesystems.push(
            Filesystem::new(
                filesystem.source,
                filesystem.mount_point.clone(),
                filesystem.filesystem,
                filesystem.options,
                filesystem.dump,
                filesystem.pass,
            )
            .with_context(|| {
                format!("некорректная запись fstab для `{}`", filesystem.mount_point)
            })?,
        );
    }

    let expand_rootfs = config.expand_rootfs;

    let spec = SystemSpec::new(config.hostname, config.timezone, config.locale)
        .context("некорректная системная конфигурация")?
        .with_users(users)
        .with_filesystems(filesystems)
        .with_dhcp_interfaces(config.network.dhcp_interfaces)
        .context("некорректный список сетевых интерфейсов")?
        // Модули приходят из board.toml, а не из system.toml: список драйверов
        // определяется железом платы, а не политикой образа.
        .with_modules(modules)
        .with_rootfs_expansion(expand_rootfs);

    Ok(spec)
}
