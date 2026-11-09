//! A project file: `data/projects/<slug>.json`.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::{LogEntry, Task};

/// Accent colour used for the project's dot and timeline lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Accent {
    #[default]
    Red,
    Amber,
    Green,
    Violet,
    Cyan,
}

impl Accent {
    pub fn css(self) -> &'static str {
        match self {
            Accent::Red => "var(--red)",
            Accent::Amber => "var(--amber)",
            Accent::Green => "var(--green)",
            Accent::Violet => "var(--violet)",
            Accent::Cyan => "var(--cyan)",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    /// Optional pointer to the JSON schema, for editor autocomplete.
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub name: String,
    /// Short prefix for task IDs, e.g. `HG` for `HG-12`.
    pub key: String,
    #[serde(default)]
    pub color: Accent,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Show this project on the public progress page.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub public: bool,
    /// When the project was finished. Projects whose tasks are all done
    /// count as finished too; this also covers past work never tracked here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished: Option<NaiveDate>,
    /// Repo or demo link, shown on the public "Past projects" page.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
    /// Cover picture, a file in `data/images/` (e.g. `images/compiler.webp`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub image: String,
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub log: Vec<LogEntry>,
}

fn yes() -> bool {
    true
}

fn is_true(b: &bool) -> bool {
    *b
}
