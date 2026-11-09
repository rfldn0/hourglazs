//! The public progress page.
//!
//! Reads top to bottom as: is work happening (momentum), how far along is each
//! project, what's next (milestones), and what got done. No notes, task
//! descriptions or links into the private app.
//!
//! Accessibility: one h1 → h2 per section → h3 per project; every status uses a
//! shape *and* a word (never colour alone, WCAG 1.4.1); every progress bar has
//! a name and a spoken value, and the number is also visible as text; the
//! heatmap is one labelled image whose facts are also written out beside it.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use maud::{Markup, html};

use crate::domain::momentum::{self, Momentum, QUIET_AFTER};
use crate::domain::schedule::{long, relative_days, short};
use crate::domain::{Progress, ProjectIndex, TaskRef};
use crate::model::Status;

use crate::render::components::title_of;
use crate::render::layout::{self, PublicNav};
use crate::render::routes::{self, Ctx};

const RECENT_LIMIT: usize = 12;
const NOW_LIMIT: usize = 4;
/// A year of activity; phones show the most recent `HEAT_WEEKS_SMALL` (see public.css).
const HEAT_WEEKS: u64 = 52;
const HEAT_WEEKS_SMALL: u64 = 20;

pub fn page(ctx: &Ctx) -> Markup {
    let site = ctx.site;
    let today = site.clock.today;
    let projects: Vec<&ProjectIndex> = site.projects.iter().filter(|p| p.public).collect();
    let all: Vec<TaskRef> = projects
        .iter()
        .flat_map(|p| (0..p.nodes.len()).map(move |idx| TaskRef { project: p, idx }))
        .collect();
    let days = momentum::activity(&projects);
    let many = projects.len() > 1;

    let mut milestones: Vec<TaskRef> = all.iter().copied().filter(|t| t.node().task.milestone).collect();
    milestones.sort_by_key(|t| (t.node().due(), t.node().task.id.clone()));

    let content = html! {
        div.wrap.stack {
            section.hero aria-labelledby="title" {
                p.hero-date { "Progress report · " (long(today)) }
                h1 id="title" { "What " (site.owner) " is building" }
                p.lede { "How far along each project is, what's next, and whether work happened today." }
            }
            (momentum_section(&projects, &days, today))
            @let active: Vec<&&ProjectIndex> = projects.iter().filter(|p| !p.is_finished()).collect();
            @let finished = projects.len() - active.len();
            section aria-labelledby="projects" {
                (heading("projects", "In progress", &active.len().to_string()))
                @if active.is_empty() { p.empty { "Nothing in progress right now." } }
                div.projects { @for p in &active { (project(p, today)) } }
                @if finished > 0 {
                    p.more {
                        a href=(routes::PUBLIC_PAST) {
                            (plural(finished as i64, "finished project", "finished projects")) " in Past projects"
                            span aria-hidden="true" { " →" }
                        }
                    }
                }
            }
            div.split {
            @if !milestones.is_empty() {
                @let reached = milestones.iter().filter(|t| t.node().is_done()).count();
                section aria-labelledby="milestones" {
                    (heading("milestones", "Milestones", &format!("{reached} / {} reached", milestones.len())))
                    ol.plain.timeline { @for t in &milestones { (milestone(t, today, many)) } }
                }
            }
            (recently_completed(&all, today, many))
            }
        }
    };
    layout::public_page(ctx, PublicNav::Progress, live(ctx, &days), content)
}

/// The top-bar pill for any public page.
pub fn status_pill(ctx: &Ctx) -> Markup {
    let projects: Vec<&ProjectIndex> = ctx.site.projects.iter().filter(|p| p.public).collect();
    live(ctx, &momentum::activity(&projects))
}

/// Top-bar pill: is work happening, and how fresh is this page.
fn live(ctx: &Ctx, days: &BTreeMap<NaiveDate, u32>) -> Markup {
    let clock = ctx.site.clock;
    let m = Momentum::new(days, clock.today);
    let (class, word) = match m.quiet_days(clock.today) {
        Some(0) => ("on", "Active today".to_string()),
        Some(1) => ("risk", "Active yesterday".to_string()),
        Some(n) => ("quiet", format!("Quiet {n}d")),
        None => ("quiet", "No activity yet".to_string()),
    };
    // A real instant when there is one, so app.js can say "2h ago".
    let stamp = clock.built.map_or_else(|| clock.today.to_string(), |b| b.to_rfc3339());
    html! {
        p.live.(class) {
            span.dot aria-hidden="true" {}
            span.live-word { (word) }
            span.sep aria-hidden="true" { "·" }
            span.upd { "updated " time datetime=(stamp) data-rel { (short(clock.today)) } }
        }
    }
}

fn heading(id: &str, title: &str, meta: &str) -> Markup {
    html! {
        div.sec-head {
            h2 id=(id) { (title) }
            span.mono { (meta) }
        }
    }
}

fn plural(n: impl Into<i64>, one: &str, many: &str) -> String {
    let n = n.into();
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Named progress bar; the visible number lives next to it.
fn bar(pct: u8, label: &str, spoken: &str) -> Markup {
    html! {
        span.bar role="progressbar" aria-label=(label) aria-valuemin="0" aria-valuemax="100"
            aria-valuenow=(pct) aria-valuetext=(spoken) {
            span.bar-fill style=(format!("width:{pct}%")) {}
        }
    }
}

/* ---------------------------------------------------------------- momentum */

fn momentum_section(projects: &[&ProjectIndex], days: &BTreeMap<NaiveDate, u32>, today: NaiveDate) -> Markup {
    let m = Momentum::new(days, today);
    let overall = projects.iter().map(|p| p.overall()).fold(Progress { done: 0, total: 0 }, |a, b| Progress {
        done: a.done + b.done,
        total: a.total + b.total,
    });

    // The one-line verdict: the honest public nudge.
    let (pulse_class, pulse_icon, pulse) = match m.quiet_days(today) {
        Some(0) => ("on", "●", "Worked on today".to_string()),
        Some(1) => ("risk", "◐", "Last worked on yesterday. The streak holds if work lands today.".to_string()),
        Some(n) => ("quiet", "○", format!("Quiet for {n} days")),
        None => ("quiet", "○", "No activity yet".to_string()),
    };
    let trend = i64::from(m.this_week) - i64::from(m.last_week);
    let (trend_icon, trend_word) = match trend {
        t if t > 0 => ("▲", format!("+{t} vs last week")),
        t if t < 0 => ("▼", format!("−{} vs last week", -t)),
        _ => ("＝", "same as last week".to_string()),
    };

    let grid = momentum::heatmap(days, today, HEAT_WEEKS);
    let cells = grid.iter().flatten().flatten();
    let active_days = cells.clone().filter(|(_, n)| *n > 0).count();
    let actions: u32 = cells.map(|(_, n)| n).sum();
    let heat_label = format!(
        "Activity heatmap, last year: {} with {}.",
        plural(active_days as i64, "active day", "active days"),
        plural(actions, "action", "actions")
    );

    html! {
        section.momentum aria-labelledby="momentum" {
            (heading("momentum", "Momentum", "last 12 months"))
            div.momentum-body {
            div.momentum-side {
            div.pulse.(pulse_class) {
                span.pulse-icon aria-hidden="true" { (pulse_icon) }
                span { (pulse) }
            }
                div.streak {
                    p.big.zero[m.streak == 0] { (m.streak) small { (if m.streak == 1 { " day" } else { " days" }) } }
                    p.mono { "Current streak · best " (plural(m.longest, "day", "days")) }
                }
                dl.stats {
                    div {
                        dt.mono { "This week" }
                        dd { (plural(m.this_week, "action", "actions")) }
                        dd.trend.(if trend > 0 { "up" } else if trend < 0 { "down" } else { "flat" }) {
                            span aria-hidden="true" { (trend_icon) " " } (trend_word)
                        }
                    }
                    div {
                        dt.mono { "Overall" }
                        dd { (overall.pct()) "% " small { "· " (overall.done) " / " (overall.total) " tasks" } }
                        dd { (bar(overall.pct(), "Overall progress", &format!("{}%, {} of {} tasks done", overall.pct(), overall.done, overall.total))) }
                    }
                }
            }
            figure.heat {
                div.heat-grid role="img" aria-label=(heat_label) {
                    @for (w, week) in grid.iter().enumerate() {
                        @let old = (w as u64) < HEAT_WEEKS - HEAT_WEEKS_SMALL;
                        @for cell in week {
                            @match cell {
                                Some((d, n)) => span.(format!("l{}", momentum::level(*n))).today[*d == today].old[old]
                                    title=(format!("{}: {}", short(*d), plural(*n, "action", "actions"))) {},
                                None => span.future.old[old] {},
                            }
                        }
                    }
                }
                figcaption.mono {
                    span { (plural(active_days as i64, "active day", "active days")) " in the last year" }
                    span.legend aria-hidden="true" {
                        "Less "
                        @for l in 0..5 { span.(format!("l{l}")) {} }
                        " More"
                    }
                }
            }
            }
        }
    }
}

/* ---------------------------------------------------------------- projects */

fn is_active_leaf(t: &TaskRef) -> bool {
    let n = t.node();
    n.is_leaf() && !n.task.milestone && n.task.status == Status::Doing
}

fn project(p: &ProjectIndex, today: NaiveDate) -> Markup {
    let pr = p.overall();
    let refs = (0..p.nodes.len()).map(|idx| TaskRef { project: p, idx });
    let active: Vec<usize> = refs.clone().filter(is_active_leaf).map(|t| t.idx).collect();
    let overdue = refs.clone().filter(|t| t.node().is_leaf() && t.node().is_late(today)).count();
    let next_milestone = refs
        .clone()
        .filter(|t| t.node().task.milestone && !t.node().is_done())
        .min_by_key(|t| t.node().due())
        .map(|t| t.idx);
    let latest = refs
        .filter(|t| t.node().is_done() && !t.node().task.milestone && t.node().task.done_on.is_some())
        .max_by_key(|t| t.node().task.done_on)
        .map(|t| t.idx);
    let phases: Vec<usize> = p.roots.iter().copied().filter(|&i| !p.nodes[i].task.milestone).collect();
    let quiet = momentum::last_active(p).map(|d| (today - d).num_days());
    let complete = pr.total > 0 && pr.done == pr.total;

    // Shape + word, so state never depends on colour alone.
    let (class, icon, word) = if complete {
        ("done", "✓", "Done".to_string())
    } else if quiet.is_some_and(|q| q >= QUIET_AFTER) {
        ("quiet", "○", format!("Quiet {}d", quiet.unwrap_or_default()))
    } else if pr.done > 0 || !active.is_empty() {
        ("active", "●", "Active".to_string())
    } else {
        ("idle", "○", "Not started".to_string())
    };
    let heading_id = format!("p-{}", p.slug);

    html! {
        article.proj aria-labelledby=(heading_id) {
            div.proj-head {
                div {
                    p.status.(class) { span aria-hidden="true" { (icon) " " } (word) }
                    h3 id=(heading_id) { (p.name) }
                }
                p.pct aria-hidden="true" { (pr.pct()) small { "%" } }
            }
            (bar(pr.pct(), &format!("{} progress", p.name), &format!("{}%, {} of {} tasks done", pr.pct(), pr.done, pr.total)))
            p.mono.proj-count {
                (pr.done) " / " (pr.total) " tasks"
                @if overdue > 0 { " · " span.late { "! " (overdue) " overdue" } }
            }
            @if !p.description.is_empty() { p.proj-desc { (p.description) } }

            div.facts {
                div {
                    h4.mono { "Working on" }
                    @if active.is_empty() {
                        p.muted { "Nothing in progress right now." }
                    } @else {
                        ul.plain.checks {
                            @for i in active.iter().take(NOW_LIMIT) {
                                li { span.box aria-hidden="true" {} span { (title_of(p.node(*i))) } }
                            }
                            @if active.len() > NOW_LIMIT {
                                li.muted { "and " (active.len() - NOW_LIMIT) " more" }
                            }
                        }
                    }
                }
                div {
                    h4.mono { "Next milestone" }
                    @match next_milestone {
                        Some(i) => {
                            @let n = p.node(i);
                            @let late = n.is_late(today);
                            p {
                                span.late[late] aria-hidden="true" { (if late { "! " } else { "◇ " }) }
                                (title_of(n))
                                @if let Some(d) = n.due() {
                                    br;
                                    span.mono.late[late] { (when_label(d, today, late)) }
                                }
                            }
                        }
                        None => p.muted { "None planned." },
                    }
                }
            }

            @if let Some(i) = latest {
                @let n = p.node(i);
                @let d = n.task.done_on.expect("filtered above");
                div.latest {
                    p.mono { "Latest · " time datetime=(d.to_string()) { (short(d)) } }
                    p { span.box.done aria-hidden="true" {} "Completed " (title_of(n)) }
                }
            }

            @if phases.len() > 1 {
                details.phases {
                    summary { "Phases " span.sr-only { "for " (p.name) } span.mono { "(" (phases.len()) ")" } }
                    ul.plain.phase-list {
                        @for i in &phases {
                            @let pct = p.completion(*i);
                            @let name = title_of(p.node(*i));
                            li {
                                span { (name) }
                                (bar(pct, name, &format!("{pct}%")))
                                span.mono aria-hidden="true" { (pct) "%" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn when_label(d: NaiveDate, today: NaiveDate, late: bool) -> String {
    if late {
        format!("{} · {} overdue", short(d), plural((today - d).num_days(), "day", "days"))
    } else {
        format!("{} · {}", short(d), relative_days(d, today))
    }
}

/* -------------------------------------------------------------- milestones */

fn milestone(t: &TaskRef, today: NaiveDate, show_project: bool) -> Markup {
    let n = t.node();
    let late = n.is_late(today);
    let (class, icon, word) = if n.is_done() {
        ("done", "◆", "Reached")
    } else if late {
        ("late", "!", "Overdue")
    } else {
        ("upcoming", "◇", "Upcoming")
    };
    let date = if n.is_done() { n.task.done_on.or(n.due()) } else { n.due() };
    html! {
        li.(class) {
            span.marker aria-hidden="true" { (icon) }
            span {
                span.m-title { (title_of(n)) }
                @if show_project { span.m-project.mono { (t.project.name) } }
            }
            span.m-when {
                span.status { (word) }
                @if let Some(d) = date {
                    time.mono datetime=(d.to_string()) {
                        (short(d))
                        @if !n.is_done() { " · " (relative_days(d, today)) }
                    }
                }
            }
        }
    }
}

/* ------------------------------------------------------- recently completed */

fn recently_completed(all: &[TaskRef], today: NaiveDate, show_project: bool) -> Markup {
    let mut done: Vec<TaskRef> = all
        .iter()
        .copied()
        .filter(|t| t.node().is_done() && !t.node().task.milestone && t.node().task.done_on.is_some())
        .collect();
    done.sort_by_key(|t| std::cmp::Reverse(t.node().task.done_on));
    done.truncate(RECENT_LIMIT);
    if done.is_empty() {
        return html! {};
    }
    html! {
        section aria-labelledby="recent" {
            (heading("recent", "Recently completed", &format!("last {}", done.len())))
            ul.plain {
                @for t in &done {
                    @let d = t.node().task.done_on.expect("filtered above");
                    li.log-row {
                        time.mono datetime=(d.to_string()) {
                            @match (today - d).num_days() {
                                0 => "Today",
                                1 => "Yesterday",
                                _ => (short(d)),
                            }
                        }
                        span { span.sr-only { "Completed: " } (title_of(t.node())) }
                        @if show_project { span.log-project { (t.project.name) } }
                    }
                }
            }
        }
    }
}
