//! Consistency checks that serde alone can't express.

use std::collections::HashSet;
use std::fs;

use crate::model::{Project, Task};

use super::load::{Loaded, PROJECTS_DIR};

#[derive(Debug, Default)]
pub struct Report {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl Report {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// IDs and slugs end up in file names and URLs, so keep them simple.
pub fn is_safe_id(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// `images/<name>.<ext>`: one plain file name inside `data/images/`, so a
/// project file can't point the build at anything else on disk.
pub fn is_image_path(p: &str) -> bool {
    let Some(name) = p.strip_prefix("images/") else { return false };
    let ok_name = !name.is_empty()
        && !name.starts_with('.')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    let ext = name.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
    ok_name && name.contains('.') && matches!(ext.as_str(), "webp" | "png" | "jpg" | "jpeg" | "avif")
}

pub fn validate(loaded: &Loaded) -> Report {
    let mut r = Report::default();

    let mut slugs = HashSet::new();
    for (slug, project) in &loaded.projects {
        if !is_safe_id(slug) {
            r.errors.push(format!("project slug {slug:?} may only use letters, digits, '-' and '_'"));
        }
        if !slugs.insert(slug.as_str()) {
            r.errors.push(format!("project {slug:?} is listed twice in the config"));
        }
        check_project(slug, project, &mut r);
        if !project.image.is_empty() {
            if !is_image_path(&project.image) {
                r.errors.push(format!(
                    "{slug}: image must look like \"images/name.webp\" (webp, png, jpg or avif in data/images/)"
                ));
            } else if !loaded.dir.join(&project.image).is_file() {
                r.errors.push(format!("{slug}: image {:?} not found in the data folder", project.image));
            }
        }
    }

    // Project files that exist but aren't listed are easy to forget about.
    if let Ok(entries) = fs::read_dir(loaded.dir.join(PROJECTS_DIR)) {
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().is_some_and(|x| x == "json") {
                let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                if !slugs.contains(stem.as_ref()) {
                    r.warnings.push(format!(
                        "{} is not listed in \"projects\" and will not be built",
                        path.display()
                    ));
                }
            }
        }
    }
    r
}

fn check_project(slug: &str, p: &Project, r: &mut Report) {
    if p.name.trim().is_empty() {
        r.errors.push(format!("{slug}: project name is empty"));
    }
    if p.key.trim().is_empty() {
        r.errors.push(format!("{slug}: project key is empty"));
    }
    // Only web links: nothing like `javascript:` ends up in an href.
    if !p.url.is_empty() && !(p.url.starts_with("https://") || p.url.starts_with("http://")) {
        r.errors.push(format!("{slug}: url must start with https:// or http://"));
    }

    let mut ids = HashSet::new();
    let mut stack: Vec<&Task> = p.tasks.iter().collect();
    while let Some(t) = stack.pop() {
        let at = format!("{slug}/{}", t.id);
        if !is_safe_id(&t.id) {
            r.errors.push(format!("{at}: task id may only use letters, digits, '-' and '_'"));
        }
        if !ids.insert(t.id.as_str()) {
            r.errors.push(format!("{at}: duplicate task id"));
        }
        if t.title.trim().is_empty() {
            r.warnings.push(format!("{at}: task has no title"));
        }
        if t.due.is_some() && t.days.is_some() && t.start.is_some() {
            r.errors.push(format!("{at}: set two of start/due/days, not all three"));
        }
        if let (Some(s), Some(d)) = (t.start, t.due)
            && s > d
        {
            r.errors.push(format!("{at}: start {s} is after due {d}"));
        }
        if t.days == Some(0) {
            r.errors.push(format!("{at}: days must be at least 1"));
        }
        if t.hours.is_some_and(|h| h < 0.0 || !h.is_finite()) {
            r.errors.push(format!("{at}: hours must be a positive number"));
        }
        if t.milestone && !t.subtasks.is_empty() {
            r.warnings.push(format!("{at}: milestones with subtasks are drawn as plain tasks"));
        }
        stack.extend(t.subtasks.iter());
    }

    for entry in &p.log {
        if let Some(task) = &entry.task
            && !ids.contains(task.as_str())
        {
            r.errors.push(format!("{slug}: log entry at {} refers to unknown task {task:?}", entry.at));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SiteConfig;

    fn loaded(json: &str) -> Loaded {
        let project: Project = serde_json::from_str(json).unwrap();
        Loaded {
            dir: "does-not-exist".into(),
            config: SiteConfig {
                schema: None,
                title: "t".into(),
                owner: "o".into(),
                today: None,
                projects: vec!["p".into()],
            },
            projects: vec![("p".into(), project)],
        }
    }

    #[test]
    fn catches_duplicates_and_bad_refs() {
        let r = validate(&loaded(
            r#"{"name":"P","key":"P","tasks":[
                {"id":"P-1","title":"a","subtasks":[{"id":"P-1","title":"b"}]},
                {"id":"P 2","title":"c","start":"2026-11-10","due":"2026-11-01"}
            ],"log":[{"at":"2026-11-01","task":"P-9","text":"x"}]}"#,
        ));
        let all = r.errors.join("\n");
        assert!(all.contains("duplicate task id"), "{all}");
        assert!(all.contains("task id may only use"), "{all}");
        assert!(all.contains("is after due"), "{all}");
        assert!(all.contains("unknown task"), "{all}");
    }

    #[test]
    fn clean_project_passes() {
        let r = validate(&loaded(
            r#"{"name":"P","key":"P","tasks":[{"id":"P-1","title":"a","start":"2026-11-01","days":3}],
                "log":[{"at":"2026-11-01T10:00","task":"P-1","text":"started"}]}"#,
        ));
        assert!(r.is_ok(), "{:?}", r.errors);
    }

    #[test]
    fn image_paths_stay_in_data_images() {
        assert!(is_image_path("images/fifa.webp"));
        assert!(is_image_path("images/a_b-1.JPG"));
        assert!(!is_image_path("images/../secret.png"));
        assert!(!is_image_path("../images/x.png"));
        assert!(!is_image_path("images/sub/x.png"));
        assert!(!is_image_path("images/x.svg"));
        assert!(!is_image_path("images/.png"));
    }

    #[test]
    fn only_web_links() {
        let ok = validate(&loaded(r#"{"name":"P","key":"P","finished":"2025-06-01","url":"https://github.com/x/y"}"#));
        assert!(ok.is_ok(), "{:?}", ok.errors);
        let bad = validate(&loaded(r#"{"name":"P","key":"P","url":"javascript:alert(1)"}"#));
        assert!(bad.errors.join("
").contains("url must start"), "{:?}", bad.errors);
    }
}
