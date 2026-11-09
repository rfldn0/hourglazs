//! Output paths for every page, plus the per-page link context.
//!
//! All links are relative and always end in `.html`, so pages work from any
//! sub-path of a static host and straight from `file://`.

use crate::domain::Site;

// Local app
pub const HOME: &str = "index.html";
pub const TIMELINE: &str = "timeline.html";
/// Where the local server previews the public page.
pub const PUBLIC_PREVIEW: &str = "public/index.html";
// Public site (what `hourglazs build` writes)
pub const PUBLIC: &str = "index.html";

pub const STYLE: &str = "assets/style.css";
pub const PUBLIC_STYLE: &str = "assets/public.css";
pub const SCRIPT: &str = "assets/app.js";
pub const EDIT_SCRIPT: &str = "assets/edit.js";
pub const ICON: &str = "assets/hourglass.svg";

pub fn board(slug: &str, parent: Option<&str>) -> String {
    match parent {
        None => format!("p/{slug}/index.html"),
        Some(id) => format!("p/{slug}/board/{id}.html"),
    }
}

pub fn tasks(slug: &str) -> String {
    format!("p/{slug}/tasks.html")
}

pub fn gantt(slug: &str) -> String {
    format!("p/{slug}/gantt.html")
}

pub fn task(slug: &str, id: &str) -> String {
    format!("{}{id}.html", task_dir(slug))
}

/// Folder holding a project's task pages (the client appends `<id>.html`).
pub fn task_dir(slug: &str) -> String {
    format!("p/{slug}/t/")
}

/// Rendering context for one page: the site, how to reach the output root,
/// and whether editing controls should be rendered.
pub struct Ctx<'a> {
    pub site: &'a Site,
    pub editable: bool,
    root: String,
}

impl<'a> Ctx<'a> {
    pub fn at(site: &'a Site, path: &str, editable: bool) -> Self {
        Self { site, editable, root: "../".repeat(path.matches('/').count()) }
    }

    /// Turn a root-relative route into a link that works from this page.
    pub fn href(&self, route: &str) -> String {
        format!("{}{}", self.root, route)
    }
}
