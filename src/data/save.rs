//! Writing project files back to disk.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::model::Project;

use super::format::to_pretty;
use super::load::project_path;

/// Write a project file atomically: a crash mid-write leaves the old file intact.
pub fn save_project(dir: &Path, slug: &str, p: &Project) -> Result<()> {
    let path = project_path(dir, slug);
    let text = to_pretty(&serde_json::to_value(p)?);
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text).with_context(|| format!("cannot write {}", tmp.display()))?;
    fs::rename(&tmp, &path).with_context(|| format!("cannot replace {}", path.display()))
}
