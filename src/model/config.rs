//! Site-wide settings: `data/hourglazs.json`.

use chrono::NaiveDate;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiteConfig {
    #[serde(rename = "$schema", default)]
    pub schema: Option<String>,
    #[serde(default = "default_title")]
    pub title: String,
    /// Name used in the home page greeting.
    pub owner: String,
    /// Pin "today" for reproducible builds. Defaults to the build machine's date.
    #[serde(default)]
    pub today: Option<NaiveDate>,
    /// Project file stems in sidebar order; each maps to `projects/<slug>.json`.
    pub projects: Vec<String>,
}

fn default_title() -> String {
    "hourglazs".into()
}
