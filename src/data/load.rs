//! Reads the JSON files from the data directory.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;

use crate::model::{Project, SiteConfig};

pub const CONFIG_FILE: &str = "hourglazs.json";
pub const PROJECTS_DIR: &str = "projects";

/// Everything read from disk, before indexing.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub dir: PathBuf,
    pub config: SiteConfig,
    /// `(slug, project)` in sidebar order.
    pub projects: Vec<(String, Project)>,
}

pub fn load(dir: &Path) -> Result<Loaded> {
    let config: SiteConfig = read_json(&dir.join(CONFIG_FILE))?;
    let projects = config
        .projects
        .iter()
        .map(|slug| {
            check_slug(slug)?;
            let project = read_json(&project_path(dir, slug))?;
            Ok((slug.clone(), project))
        })
        .collect::<Result<_>>()?;
    Ok(Loaded { dir: dir.to_path_buf(), config, projects })
}

/// Project names become file names and URLs: keep them simple.
fn check_slug(slug: &str) -> Result<()> {
    if slug.is_empty() {
        bail!("{CONFIG_FILE}: \"projects\" has an empty name; list file names like [\"hourglazs\"]");
    }
    if !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        bail!("{CONFIG_FILE}: project name {slug:?} may only use letters, digits, - and _");
    }
    Ok(())
}

pub fn project_path(dir: &Path, slug: &str) -> PathBuf {
    dir.join(PROJECTS_DIR).join(format!("{slug}.json"))
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("invalid data in {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::check_slug;

    #[test]
    fn project_names_must_be_simple() {
        assert!(check_slug("hourglazs").is_ok());
        assert!(check_slug("my-site_2").is_ok());
        assert!(check_slug("").is_err());
        assert!(check_slug("../etc").is_err());
    }
}
