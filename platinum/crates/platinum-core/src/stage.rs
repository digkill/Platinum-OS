use anyhow::Result;

use crate::BuildContext;

/// Независимый этап конвейера сборки образа.
///
/// Stage получает только BuildContext и не знает, кто построил pipeline. Это
/// позволяет заменять, добавлять и тестировать этапы без изменения CLI или
/// BuildEngine.
pub trait Stage {
    /// Возвращает стабильное имя этапа для логов и будущего resume-механизма.
    fn name(&self) -> &'static str;

    /// Выполняет работу этапа и обновляет общий контекст при необходимости.
    fn execute(&self, context: &mut BuildContext) -> Result<()>;
}
