use anyhow::{Context, Result};
use platinum_core::{BuildContext, Stage};
use platinum_downloader::{Artifact, Downloader};
use platinum_rootfs::{AptInstaller, Chroot, PackageSet, RootfsSpec, RootfsUnpacker};
use tracing::info;

use crate::outputs;

/// Имя каталога rootfs внутри work-dir сборки.
const ROOTFS_DIRECTORY: &str = "rootfs";

/// Файл, по которому распознаётся уже распакованный Ubuntu Base.
///
/// Проверяется содержимое, а не сам факт существования каталога: пустой или
/// частично распакованный каталог не должен считаться готовым rootfs.
const ROOTFS_MARKER: &str = "etc/os-release";

/// Загрузка base-архива Ubuntu с проверкой SHA-256.
pub struct DownloadRootfsStage {
    artifact: Artifact,
}

impl DownloadRootfsStage {
    /// Создаёт stage для конкретного проверяемого артефакта.
    pub fn new(artifact: Artifact) -> Self {
        Self { artifact }
    }
}

impl Stage for DownloadRootfsStage {
    fn name(&self) -> &'static str {
        "download-rootfs"
    }

    fn execute(&self, context: &mut BuildContext) -> Result<()> {
        let path = Downloader::new()
            .fetch(&self.artifact, &context.paths().downloads_dir)
            .with_context(|| format!("не удалось получить {}", self.artifact.url))?;

        context.record(outputs::ROOTFS_ARCHIVE, path);

        Ok(())
    }
}

/// Распаковка base-архива в каталог rootfs текущей сборки.
pub struct UnpackRootfsStage {
    spec: RootfsSpec,
}

impl UnpackRootfsStage {
    /// Создаёт stage для проверенной спецификации rootfs.
    pub fn new(spec: RootfsSpec) -> Self {
        Self { spec }
    }
}

impl Stage for UnpackRootfsStage {
    fn name(&self) -> &'static str {
        "unpack-rootfs"
    }

    fn execute(&self, context: &mut BuildContext) -> Result<()> {
        let archive = context
            .require_output(outputs::ROOTFS_ARCHIVE)?
            .to_path_buf();
        let target = context.paths().work_dir.join(ROOTFS_DIRECTORY);

        // Повторный запуск сборки не должен заново распаковывать десятки тысяч
        // файлов, поэтому готовый rootfs переиспользуется по маркеру.
        if target.join(ROOTFS_MARKER).is_file() {
            info!(
                path = %target.display(),
                release = %self.spec.release,
                architecture = %self.spec.architecture,
                "rootfs reused from work directory"
            );
            context.record(outputs::ROOTFS_DIR, target);

            return Ok(());
        }

        let unpacker = RootfsUnpacker::new(archive, target);
        unpacker.unpack().with_context(|| {
            format!(
                "не удалось распаковать Ubuntu Base {} {}",
                self.spec.release, self.spec.architecture
            )
        })?;

        context.record(outputs::ROOTFS_DIR, unpacker.target().to_path_buf());

        Ok(())
    }
}

/// Установка Platinum userspace в распакованный rootfs.
///
/// Stage выполняется в chroot и потому требует root и qemu-user-static для
/// чужой архитектуры. Это осознанная граница: единственная альтернатива —
/// собирать userspace на самой плате, что несовместимо с воспроизводимостью.
pub struct InstallPackagesStage {
    architecture: String,
    installer: AptInstaller,
}

impl InstallPackagesStage {
    /// Создаёт stage для архитектуры платы и проверенного набора пакетов.
    pub fn new(architecture: String, packages: PackageSet, install_recommends: bool) -> Self {
        Self {
            architecture,
            installer: AptInstaller::new(packages, install_recommends),
        }
    }
}

impl Stage for InstallPackagesStage {
    fn name(&self) -> &'static str {
        "install-packages"
    }

    fn execute(&self, context: &mut BuildContext) -> Result<()> {
        let rootfs = context.require_output(outputs::ROOTFS_DIR)?.to_path_buf();

        let chroot = Chroot::new(rootfs, self.architecture.clone())
            .context("каталог rootfs непригоден для chroot")?;

        self.installer
            .install(&chroot)
            .context("не удалось установить пакеты Platinum в rootfs")?;

        Ok(())
    }
}
