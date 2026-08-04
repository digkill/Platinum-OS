//! Общие типы и контракты Platinum OS One.
//!
//! Этот crate не знает о конкретной плате, формате образа или способе запуска.
//! Он определяет минимальный язык, которым BuildEngine и независимые stages
//! описывают процесс сборки.

mod context;
mod paths;
mod pipeline;
mod stage;

pub use context::{BuildContext, MissingOutput};
pub use paths::{BuildPaths, BuildPathsError};
pub use pipeline::Pipeline;
pub use stage::Stage;
