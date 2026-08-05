//! Настройка образа при первой загрузке через cloud-init.
//!
//! Позволяет задать пользователя, Wi-Fi, SSH-ключи и hostname **после** записи
//! образа на носитель, не пересобирая его. Файлы кладутся на загрузочный
//! FAT-раздел, который виден с любой машины — в том числе из Raspberry Pi
//! Imager, где это встроенный сценарий «OS customisation».
//!
//! Взят cloud-init, а не собственный формат: userspace Platinum — Ubuntu, где
//! это штатный механизм первой загрузки. Свой разбор `platinum.toml` пришлось
//! бы писать и поддерживать, а пользователю — ставить нашу утилиту вместо
//! привычного Imager.
//!
//! Настройка из образа при этом остаётся базовой: сборка задаёт рабочие
//! умолчания, а cloud-init их дополняет или перекрывает. Поэтому поставляемый
//! `user-data` пуст — образ обязан быть пригоден и без единой правки.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use thiserror::Error;
use tracing::info;

/// Каталог drop-in конфигурации cloud-init.
const CONFIG_DIRECTORY: &str = "etc/cloud/cloud.cfg.d";

/// Файл конфигурации, создаваемый сборкой.
const CONFIG_FILE: &str = "99-platinum.cfg";

/// Файл настроек, который правит оператор или Imager.
const USER_DATA_FILE: &str = "user-data";

/// Файл идентификации экземпляра; без него NoCloud не считает seed валидным.
const META_DATA_FILE: &str = "meta-data";

/// Ошибки настройки cloud-init.
#[derive(Debug, Error)]
pub enum CloudInitError {
    /// Каталог seed обязан быть абсолютным путём внутри целевой системы.
    #[error("каталог seed `{directory}` должен быть абсолютным путём")]
    RelativeSeedDirectory {
        /// Отклонённое значение.
        directory: String,
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
}

/// Параметры первой загрузки.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudInitSpec {
    /// Каталог целевой системы, из которого читается `user-data`.
    ///
    /// Обязан лежать на разделе, который видно с чужой машины: смысл в том,
    /// чтобы править настройки после записи образа, не загружая устройство.
    pub seed_directory: String,
}

impl CloudInitSpec {
    /// Создаёт проверенное описание.
    pub fn new(seed_directory: String) -> Result<Self, CloudInitError> {
        if !seed_directory.starts_with('/') {
            return Err(CloudInitError::RelativeSeedDirectory {
                directory: seed_directory,
            });
        }

        Ok(Self { seed_directory })
    }
}

/// Включает cloud-init в готовом rootfs.
#[derive(Debug, Clone)]
pub struct CloudInitConfigurator {
    spec: CloudInitSpec,
}

impl CloudInitConfigurator {
    /// Создаёт конфигуратор.
    pub fn new(spec: CloudInitSpec) -> Self {
        Self { spec }
    }

    /// Пишет конфигурацию датасорса и пустой seed на загрузочный раздел.
    pub fn apply(&self, rootfs: &Path) -> Result<(), CloudInitError> {
        write(
            &rootfs.join(CONFIG_DIRECTORY).join(CONFIG_FILE),
            &render_config(&self.spec),
        )?;

        let seed = rootfs.join(self.spec.seed_directory.trim_start_matches('/'));

        // Файлы не перезаписываются: пересборка не должна стирать настройки,
        // которые оператор уже положил на раздел.
        let user_data = seed.join(USER_DATA_FILE);
        if !user_data.exists() {
            write(&user_data, USER_DATA)?;
        }

        let meta_data = seed.join(META_DATA_FILE);
        if !meta_data.exists() {
            write(&meta_data, META_DATA)?;
        }

        info!(seed = %self.spec.seed_directory, "cloud-init enabled");

        Ok(())
    }
}

/// Записывает файл, создавая родительские каталоги.
fn write(path: &Path, contents: &str) -> Result<(), CloudInitError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| CloudInitError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    fs::write(path, contents).map_err(|source| CloudInitError::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Формирует drop-in конфигурацию cloud-init.
///
/// `users: []` обязателен: без него cloud-init заводит учётную запись дистрибутива
/// по умолчанию, и в образе появился бы лишний пользователь `ubuntu` рядом с
/// созданным сборкой. Значение перекрывается из `user-data`, если оператор
/// действительно хочет своих пользователей.
fn render_config(spec: &CloudInitSpec) -> String {
    format!(
        "# Platinum OS: файл создан сборкой, ручные правки будут перезаписаны.\n\
         #\n\
         # Настройки берутся с загрузочного раздела, чтобы их можно было менять\n\
         # с любой машины после записи образа.\n\
         datasource_list: [NoCloud, None]\n\
         datasource:\n\
         \x20 NoCloud:\n\
         \x20   seedfrom: {}/\n\
         users: []\n\
         disable_root: true\n",
        spec.seed_directory
    )
}

/// Поставляемый `user-data`: рабочий образ без единой правки.
const USER_DATA: &str = r#"#cloud-config
# Platinum OS — настройка первой загрузки.
#
# Файл пуст намеренно: образ уже собран с рабочими умолчаниями, и без правок
# устройство просто загрузится. Раскомментируйте нужное — настройки применятся
# при первом старте.
#
# Правится с любой машины: раздел с этим файлом монтируется и в macOS, и в
# Windows. Raspberry Pi Imager пишет сюда же, когда включена «OS customisation».

# hostname: platinum
# create_hostname_file: true

# users:
#   - name: platinum
#     groups: [sudo, adm, dialout, netdev, video, input, render]
#     shell: /bin/bash
#     lock_passwd: false
#     # openssl passwd -6
#     passwd: "$6$..."
#     ssh_authorized_keys:
#       - ssh-ed25519 AAAA...

# Wi-Fi настраивается через netplan: cloud-init кладёт свой файл поверх
# собранного сборкой, поэтому здесь достаточно описать только сеть.
# network:
#   version: 2
#   wifis:
#     wlan0:
#       dhcp4: true
#       access-points:
#         "имя-сети":
#           password: "PSK или пароль"

# timezone: Europe/Moscow
# locale: ru_RU.UTF-8

# ssh_pwauth: true
"#;

/// Минимальный `meta-data`: без него NoCloud не признаёт seed.
const META_DATA: &str = "instance-id: platinum-os\n";

#[cfg(test)]
mod tests {
    use super::{CloudInitError, CloudInitSpec, META_DATA, USER_DATA, render_config};

    #[test]
    fn points_the_datasource_at_the_boot_partition() {
        let spec = CloudInitSpec::new("/boot/firmware".into()).expect("корректный путь");

        let config = render_config(&spec);

        assert!(config.contains("datasource_list: [NoCloud, None]\n"));
        assert!(config.contains("seedfrom: /boot/firmware/\n"));
    }

    /// Без `users: []` cloud-init заводит учётную запись дистрибутива, и рядом с
    /// пользователем сборки появился бы лишний `ubuntu`.
    #[test]
    fn suppresses_the_default_distribution_user() {
        let spec = CloudInitSpec::new("/boot/firmware".into()).expect("корректный путь");

        assert!(render_config(&spec).contains("users: []\n"));
    }

    /// Поставляемый seed обязан быть безоперационным: образ должен загружаться
    /// без единой правки файла.
    #[test]
    fn ships_a_no_op_seed() {
        assert!(USER_DATA.starts_with("#cloud-config\n"));

        let active = USER_DATA
            .lines()
            .filter(|line| !line.trim_start().starts_with('#') && !line.trim().is_empty())
            .count();
        assert_eq!(
            active, 0,
            "в поставляемом user-data не должно быть директив"
        );

        assert!(META_DATA.contains("instance-id:"));
    }

    #[test]
    fn rejects_a_relative_seed_directory() {
        let error = CloudInitSpec::new("boot/firmware".into())
            .expect_err("относительный путь должен отклоняться");

        assert!(matches!(
            error,
            CloudInitError::RelativeSeedDirectory { .. }
        ));
    }
}
