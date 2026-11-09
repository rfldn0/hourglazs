//! Raw data types, exactly as they appear in the JSON files.

mod config;
mod log;
mod project;
mod task;

pub use config::SiteConfig;
pub use log::{LogEntry, LogKind, parse_when};
pub use project::{Accent, Project};
pub use task::{Priority, Status, Task};

/// Lets serde leave default values out of the written files.
pub(crate) fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}
