//! Small reusable pieces of markup.

use chrono::NaiveDateTime;
use maud::{Markup, PreEscaped, html};

use crate::domain::schedule::short;
use crate::domain::{Node, ProjectIndex};
use crate::model::{Priority, Status};

use super::routes::{self, Ctx};

const LOGO: &str = r#"<svg aria-hidden="true" viewBox="0 0 64 64" width="26" height="26" fill="none" stroke-linecap="round" stroke-linejoin="round" class="logo"><g stroke="var(--text)" stroke-width="2.6"><path d="M15 45C10 33 11 20 19 12.5C23 9 28 8 32 8C36 8 41 9 45 12.5C53 20 54 33 49 45"/><path d="M19 12.5C17 10 18 7.5 20.5 8"/><path d="M45 12.5C47 10 46 7.5 43.5 8"/><circle cx="32" cy="5.5" r="1.8"/><rect x="20" y="14.5" width="24" height="4" rx="1.5"/><path d="M22.5 18.5C22.5 26 30 28 30 31.5C30 35 22.5 37 22.5 44.5"/><path d="M41.5 18.5C41.5 26 34 28 34 31.5C34 35 41.5 37 41.5 44.5"/><rect x="18" y="44.5" width="28" height="4.5" rx="1.5"/><path d="M26 49L28 54H36L38 49"/><rect x="21" y="54" width="22" height="4" rx="1.5"/></g><g fill="var(--red)"><path d="M25.5 22H38.5C37.5 26 33 28 32 30.5C31 28 26.5 26 25.5 22Z"/><path d="M25 44.5C26 40.5 29.5 38.5 32 38.5C34.5 38.5 38 40.5 39 44.5Z"/></g><path d="M32 31V38" stroke="var(--red)" stroke-width="1.6"/></svg>"#;

pub fn logo() -> Markup {
    PreEscaped(LOGO.into())
}

/// Done indicator; a toggle button when editing.
pub fn check(ctx: &Ctx, p: &ProjectIndex, i: usize, round: bool) -> Markup {
    let n = p.node(i);
    let done = n.is_done();
    if !ctx.editable {
        return html! {
            span.check.done[done].round[round] role="img" aria-label=(if done { "Done" } else { "Not done" }) {
                @if done { "✓" }
            }
        };
    }
    let label = format!("{}: {}", if done { "Mark not done" } else { "Mark done" }, title_of(n));
    html! {
        button.check.done[done].round[round] type="button" role="checkbox" aria-checked=(done) aria-label=(label)
            data-action="set" data-project=(p.slug) data-id=(n.task.id) data-field="status"
            data-value=(if done { "todo" } else { "done" }) {
            @if done { "✓" }
        }
    }
}

pub fn pri_mark(p: Priority) -> Markup {
    html! {
        @if p == Priority::High {
            span.pri-high title="High priority" { "▲▲" }
        }
    }
}

pub fn status(s: Status) -> Markup {
    html! {
        span.status {
            span.dot style=(format!("background:{}", s.color())) {}
            (s.label())
        }
    }
}

pub fn progress(pct: u8, label: &str) -> Markup {
    html! {
        span.progress role="progressbar" aria-valuenow=(pct) aria-valuemin="0" aria-valuemax="100" aria-label=(label) {
            span style=(format!("width:{pct}%")) {}
        }
    }
}

/// Timestamp that `app.js` upgrades to "3h ago" in the reader's browser.
pub fn when(at: NaiveDateTime) -> Markup {
    html! {
        time datetime=(at.format("%Y-%m-%dT%H:%M")) data-rel { (short(at.date())) }
    }
}

/// Client-side priority filter buttons (handled by `app.js`).
pub fn pri_filters() -> Markup {
    html! {
        div.pri-filters role="group" aria-label="Filter by priority" {
            button type="button" data-pri-btn="all" aria-pressed="true" { "All" }
            @for p in Priority::ALL {
                button type="button" data-pri-btn=(p.key()) aria-pressed="false" { (p.label()) }
            }
        }
    }
}

/// Lowercased text the filter box matches against.
pub fn filter_text(n: &Node) -> String {
    format!("{} {}", n.task.title, n.task.id).to_lowercase()
}

pub fn title_of(n: &Node) -> &str {
    if n.task.title.trim().is_empty() { "Untitled task" } else { &n.task.title }
}

pub fn task_link(ctx: &Ctx, p: &ProjectIndex, i: usize) -> String {
    ctx.href(&routes::task(&p.slug, &p.nodes[i].task.id))
}

pub fn hours(h: Option<f32>) -> String {
    match h {
        Some(h) if h.fract() == 0.0 => format!("{h:.0}h"),
        Some(h) => format!("{h:.1}h"),
        None => String::new(),
    }
}
