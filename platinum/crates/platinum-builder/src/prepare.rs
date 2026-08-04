use std::fs;

use anyhow::Result;
use platinum_core::{BuildContext, Stage};

/// Начальный stage, который подготавливает директории сборки.
///
/// Каталоги создаются до загрузки артефактов, чтобы последующие stages могли
/// работать с объявленными путями и не зависели от порядка вызовов в CLI.
pub struct PrepareStage;

impl Stage for PrepareStage {
    fn name(&self) -> &'static str {
        "prepare"
    }

    fn execute(&self, context: &mut BuildContext) -> Result<()> {
        let paths = context.paths();

        fs::create_dir_all(&paths.work_dir)?;
        fs::create_dir_all(&paths.downloads_dir)?;
        fs::create_dir_all(&paths.cache_dir)?;
        fs::create_dir_all(&paths.output_dir)?;

        Ok(())
    }
}
