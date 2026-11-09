//! Page shells: the local app (sidebar + header) and the public page.

use maud::{DOCTYPE, Markup, PreEscaped, html};

use crate::domain::ProjectIndex;
use crate::domain::schedule::{long, short};

use super::components::logo;
use super::routes::{self, Ctx};

/// Which part of the app a page belongs to.
#[derive(Clone, Copy)]
pub enum Nav<'a> {
    Home,
    Timeline,
    Project(&'a ProjectIndex, Tab),
}

impl PartialEq for Nav<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Nav::Home, Nav::Home) | (Nav::Timeline, Nav::Timeline) => true,
            (Nav::Project(a, x), Nav::Project(b, y)) => a.slug == b.slug && x == y,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Board,
    Tasks,
    Gantt,
    /// A single task page (no view highlighted).
    Task,
}

pub struct Chrome<'a> {
    pub nav: Nav<'a>,
    pub title: String,
    pub subtitle: String,
    /// Show the "Filter /" box.
    pub filter: bool,
    /// Parent task for the "New task" button (board sub-views).
    pub new_task_parent: Option<&'a str>,
}

/// Applied before first paint so the stored theme doesn't flash.
const THEME_BOOT: &str =
    "try{var t=localStorage.getItem('hourglazs.theme');if(t)document.documentElement.dataset.theme=t}catch(e){}";

/// The app's typeface.
const APP_FONTS: &str = "https://fonts.googleapis.com/css2?family=Geist:wght@400;500;600;700&family=Geist+Mono:wght@400;500&display=swap";

fn head(ctx: &Ctx, title: &str, stylesheet: &str, fonts: &str, extra: Markup) -> Markup {
    html! {
        head {
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1";
            title { (title) }
            (extra)
            link rel="icon" type="image/svg+xml" href=(ctx.href(routes::ICON));
            link rel="preconnect" href="https://fonts.googleapis.com";
            link rel="preconnect" href="https://fonts.gstatic.com" crossorigin;
            link rel="stylesheet" href=(fonts);
            link rel="stylesheet" href=(ctx.href(stylesheet));
            script { (PreEscaped(THEME_BOOT)) }
            script src=(ctx.href(routes::SCRIPT)) defer {}
            @if ctx.editable { script src=(ctx.href(routes::EDIT_SCRIPT)) defer {} }
        }
    }
}

/// The local app: sidebar navigation, header, content.
pub fn page(ctx: &Ctx, chrome: Chrome, content: Markup) -> Markup {
    let site = ctx.site;
    let doc_title = if chrome.title.is_empty() {
        site.title.clone()
    } else {
        format!("{} · {}", chrome.title, site.title)
    };
    let project = match chrome.nav {
        Nav::Project(p, _) => Some(p),
        _ => None,
    };
    html! {
        (DOCTYPE)
        html lang="en" data-theme="dark" {
            (head(ctx, &doc_title, routes::STYLE, APP_FONTS, html! {}))
            body data-project=[project.map(|p| &p.slug)] data-editable[ctx.editable] {
                div.app {
                    (sidebar(ctx, chrome.nav))
                    main.main {
                        header.topbar {
                            div.topbar-titles {
                                h1 { (chrome.title) }
                                @if !chrome.subtitle.is_empty() {
                                    span.subtitle { (chrome.subtitle) }
                                }
                            }
                            @if chrome.filter {
                                label.search {
                                    span aria-hidden="true" { "⌕" }
                                    input type="search" data-filter placeholder="Filter  /" aria-label="Filter tasks by title or ID";
                                }
                            }
                            @if let (true, Some(p)) = (ctx.editable, project) {
                                button.btn.primary type="button" data-action="new-task" data-key="n"
                                    data-parent=[chrome.new_task_parent]
                                    data-task-base=(ctx.href(&routes::task_dir(&p.slug))) {
                                    "＋ New task"
                                }
                            }
                        }
                        div.content { (content) }
                    }
                }
                @if ctx.editable { div.toast role="status" aria-live="polite" data-toast {} }
            }
        }
    }
}

/// Which public page is showing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PublicNav {
    Progress,
    Past,
}

/// The public site's shell: no sidebar, no links into the private app.
/// Follows the visitor's light/dark preference unless they pick one.
/// `status` is the pill in the top bar (see views::public).
pub fn public_page(ctx: &Ctx, nav: PublicNav, status: Markup, content: Markup) -> Markup {
    let site = ctx.site;
    let clock = site.clock;
    let (title, description) = match nav {
        PublicNav::Progress => (
            format!("{} · progress", site.title),
            format!("What {} is building, and how far along it is.", site.owner),
        ),
        PublicNav::Past => (format!("Past projects · {}", site.title), format!("Projects {} has finished.", site.owner)),
    };
    let extra = html! {
        meta name="description" content=(description);
        meta name="color-scheme" content="light dark";
        meta name="theme-color" media="(prefers-color-scheme: light)" content="#f5f4f1";
        meta name="theme-color" media="(prefers-color-scheme: dark)" content="#0a0a0c";
        meta property="og:title" content=(title);
        meta property="og:description" content=(description);
    };
    // Public pages are siblings: link by bare file name.
    let tab = |href: &str, label: &str, me: PublicNav| {
        html! { a href=(href) aria-current=[(nav == me).then_some("page")] { (label) } }
    };
    html! {
        (DOCTYPE)
        html lang="en" {
            (head(ctx, &title, routes::PUBLIC_STYLE, APP_FONTS, extra))
            body {
                a.skip href="#main" { "Skip to content" }
                header.top {
                    div.wrap {
                        a.brand href=(routes::PUBLIC) { (logo()) span { (site.title) } }
                        nav.tabs aria-label="Pages" {
                            (tab(routes::PUBLIC, "Progress", PublicNav::Progress))
                            (tab(routes::PUBLIC_PAST, "Past projects", PublicNav::Past))
                        }
                        (status)
                        button.theme-btn type="button" data-theme-toggle aria-label="Switch colour theme" {
                            span aria-hidden="true" data-theme-icon { "◐" }
                        }
                    }
                }
                main id="main" { (content) }
                footer.foot {
                    div.wrap {
                        span { "Updated " (long(clock.today)) " · rebuilt on every change and daily" }
                        span { "Built with hourglazs" }
                    }
                }
            }
        }
    }
}

fn sidebar(ctx: &Ctx, nav: Nav) -> Markup {
    let site = ctx.site;
    let current_project = match nav {
        Nav::Project(p, _) => Some(p.slug.as_str()),
        _ => None,
    };
    // Project views point at the current project; elsewhere at the first one
    // (app.js swaps in the last project you visited).
    let target = match nav {
        Nav::Project(p, _) => Some(p),
        _ => site.projects.first(),
    };
    let tab = match nav {
        Nav::Project(_, t) => Some(t),
        _ => None,
    };
    html! {
        aside.sidebar aria-label="Main navigation" {
            a.brand href=(ctx.href(routes::HOME)) {
                (logo())
                span { (site.title) }
            }
            nav.nav-list {
                (nav_item(ctx.href(routes::HOME), "●", "Home", "1", nav == Nav::Home, false))
                (nav_item(ctx.href(routes::TIMELINE), "═", "Timeline", "2", nav == Nav::Timeline, false))
                @if let Some(p) = target {
                    (nav_item(ctx.href(&routes::board(&p.slug, None)), "▦", "Board", "3", tab == Some(Tab::Board), true))
                    (nav_item(ctx.href(&routes::tasks(&p.slug)), "☰", "Tasks", "4", tab == Some(Tab::Tasks), true))
                    (nav_item(ctx.href(&routes::gantt(&p.slug)), "▤", "Gantt", "5", tab == Some(Tab::Gantt), true))
                }
            }
            div.nav-list {
                div.nav-label { "PROJECTS" }
                @for p in &site.projects {
                    @let on = current_project == Some(p.slug.as_str());
                    a.nav-item.on[on] href=(ctx.href(&routes::board(&p.slug, None))) data-project-slug=(p.slug) aria-current=[on.then_some("page")] {
                        span.ring aria-hidden="true" style=(format!("border-color:{}", p.accent.css())) {}
                        span.grow { (p.name) }
                        @if !p.public { span.private title="Hidden from the public page" { "private" } }
                    }
                }
            }
            div.sidebar-foot {
                @if ctx.editable {
                    a.nav-item href=(ctx.href(routes::PUBLIC_PREVIEW)) target="_blank" {
                        span.nav-icon aria-hidden="true" { "↗" }
                        span.grow { "Public page" }
                    }
                }
                button.nav-item type="button" data-theme-toggle aria-label="Switch colour theme" {
                    span.nav-icon aria-hidden="true" data-theme-icon { "◐" }
                    span.grow data-theme-label { "Dark theme" }
                    kbd { "T" }
                }
                div.built {
                    span.dot-green aria-hidden="true" {}
                    @if ctx.editable { "Live · saving to data/" } @else {
                        "Built " (short(site.clock.today)) ", " (site.clock.now.format("%H:%M"))
                    }
                }
            }
        }
    }
}

/// `project_view` links follow whichever project you last opened (see app.js).
fn nav_item(href: String, icon: &str, label: &str, key: &str, on: bool, project_view: bool) -> Markup {
    html! {
        a.nav-item.on[on] href=(href) data-key=(key) data-project-view[project_view] aria-current=[on.then_some("page")] {
            span.nav-icon aria-hidden="true" { (icon) }
            span.grow { (label) }
            kbd { (key) }
        }
    }
}
