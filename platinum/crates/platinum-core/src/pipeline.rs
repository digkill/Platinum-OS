use std::{fmt, time::Instant};

use anyhow::Result;
use tracing::{error, info};

use crate::{BuildContext, Stage};

/// Упорядоченная последовательность stages одной сборки.
///
/// Pipeline владеет stages через trait objects. Владение здесь необходимо:
/// BuildEngine собирает pipeline один раз, а затем может запускать его без
/// привязки времени жизни stages к локальным переменным конструктора.
#[derive(Default)]
pub struct Pipeline {
    stages: Vec<Box<dyn Stage>>,
}

impl Pipeline {
    /// Создаёт пустой pipeline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Добавляет stage в конец последовательности выполнения.
    pub fn add<S>(&mut self, stage: S)
    where
        S: Stage + 'static,
    {
        self.stages.push(Box::new(stage));
    }

    /// Возвращает имена stages в порядке выполнения.
    ///
    /// Состав pipeline — часть контракта сборки, поэтому он доступен для
    /// вывода в логах и проверки в тестах без запуска самих stages.
    pub fn stage_names(&self) -> impl Iterator<Item = &'static str> {
        self.stages.iter().map(|stage| stage.name())
    }

    /// Выполняет stages по порядку и останавливается на первой ошибке.
    pub fn run(&self, context: &mut BuildContext) -> Result<()> {
        for stage in &self.stages {
            let started_at = Instant::now();
            let name = stage.name();

            info!(stage = name, "stage started");
            if let Err(error) = stage.execute(context) {
                error!(
                    stage = name,
                    duration_ms = started_at.elapsed().as_millis(),
                    error = %error,
                    "stage failed"
                );

                return Err(error);
            }

            info!(
                stage = name,
                duration_ms = started_at.elapsed().as_millis(),
                "stage finished"
            );
        }

        Ok(())
    }
}

/// Показывает состав pipeline: сами stages не обязаны быть `Debug`, но их
/// порядок — главная отладочная информация о сборке.
impl fmt::Debug for Pipeline {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Pipeline")
            .field("stages", &self.stage_names().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{BuildContext, Pipeline, Stage};

    struct NoopStage;

    impl Stage for NoopStage {
        fn name(&self) -> &'static str {
            "noop"
        }

        fn execute(&self, _context: &mut BuildContext) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn reports_stage_names_in_execution_order() {
        let mut pipeline = Pipeline::new();
        pipeline.add(NoopStage);
        pipeline.add(NoopStage);

        assert_eq!(pipeline.stage_names().collect::<Vec<_>>(), ["noop", "noop"]);
    }
}
