//! Home: what's due, project progress, recent activity.

use chrono::Days;
use maud::{Markup, html};

use crate::domain::momentum::{self, Momentum, QUIET_AFTER};
use crate::domain::{ProjectIndex, TaskRef};
use crate::domain::schedule::{long, short};
use crate::model::LogKind;

use crate::render::components::{check, pri_mark, progress, task_link, title_of, when};
use crate::render::layout::{self, Chrome, Nav};
use crate::render::routes::{self, Ctx};

/// "Up next" covers this many days from today.
const HORIZON_DAYS: u64 = 10;

pub fn page(ctx: &Ctx) -> Markup {
    let site = ctx.site;
    let today = site.clock.today;
    let horizon = today + Days::new(HORIZON_DAYS - 1);

    let mut overdue = Vec::new();
    let mut soon = Vec::new();
    let mut later = 0;
    let mut unscheduled = 0;
    for t in site.open_work() {
        match t.node().due() {
            Some(d) if d < today => overdue.push(t),
            Some(d) if d <= horizon => soon.push(t),
            Some(_) => later += 1,
            None => unscheduled += 1,
        }
    }
    overdue.sort_by_key(|t| t.node().due());
    soon.sort_by_key(|t| t.node().due());

    let mut milestones: Vec<TaskRef> = site
        .tasks()
        .filter(|t| t.node().task.milestone && !t.node().is_done() && t.node().due().is_some_and(|d| d >= today))
        .collect();
    milestones.sort_by_key(|t| t.node().due());
    milestones.truncate(3);

    let mut summary = String::new();
    if !overdue.is_empty() {
        summary += &format!("{} overdue and ", overdue.len());
    }
    summary += &format!("{} due in the next {HORIZON_DAYS} days. {later} more later", soon.len());
    if unscheduled > 0 {
        summary += &format!(", {unscheduled} unscheduled");
    }
    summary.push('.');

    let groups = [("Overdue", "late", overdue), (&*format!("Next {HORIZON_DAYS} days"), "", soon), ("Milestones", "", milestones)];

    let content = html! {
        div.home {
            div.hero {
                div.hero-date { (long(today)) }
                div.hero-greeting data-greeting=(site.owner) { "Hello, " (site.owner) "." }
                div.hero-summary { (summary) }
            }
            (momentum_strip(ctx))
            section aria-labelledby="up-next" {
                h2.section-label id="up-next" { "Up next" }
                @if groups.iter().all(|g| g.2.is_empty()) {
                    p.empty { "Nothing due soon. Enjoy it." }
                }
                @for (label, class, list) in &groups {
                    @if !list.is_empty() {
                        div.agenda-group {
                            div.agenda-label.late[*class == "late"] { (label) " · " (list.len()) }
                            @for t in list { (agenda_row(ctx, *t)) }
                        }
                    }
                }
            }
            div.home-columns {
                section aria-labelledby="projects" {
                    h2.section-label id="projects" { "Projects" }
                    @for p in &site.projects {
                        @let pr = p.overall();
                        a.project-row href=(ctx.href(&routes::board(&p.slug, None))) {
                            span.dot aria-hidden="true" style=(format!("background:{}", p.accent.css())) {}
                            span.project-name { (p.name) }
                            (progress(pr.pct(), &format!("{} progress", p.name)))
                            span.mono.dim { (pr.pct()) "%" }
                        }
                    }
                }
                section aria-labelledby="activity" {
                    h2.section-label id="activity" { "Recent activity" }
                    ol.feed {
                        @for (p, l) in site.activity(8) {
                            @let task = l.task.as_deref().and_then(|id| p.get(id));
                            li {
                                span.feed-text.note[l.kind == LogKind::Note] {
                                    @if let Some(i) = task {
                                        a href=(task_link(ctx, p, i)) { (title_of(p.node(i))) }
                                        @if l.kind == LogKind::Note { ": " } @else { " · " }
                                    }
                                    (l.text)
                                }
                                span.feed-meta {
                                    @if let Some(i) = task { (p.node(i).task.id) " · " }
                                    (when(l.at))
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    layout::page(
        ctx,
        Chrome { nav: Nav::Home, title: "Home".into(), subtitle: String::new(), filter: false, new_task_parent: None },
        content,
    )
}

/// The daily push: streak, the last two weeks, and projects going quiet.
fn momentum_strip(ctx: &Ctx) -> Markup {
    let site = ctx.site;
    let today = site.clock.today;
    let all: Vec<&ProjectIndex> = site.projects.iter().collect();
    let days = momentum::activity(&all);
    let m = Momentum::new(&days, today);
    let nudge = if m.active_today {
        format!("Today counts. {} this week.", plural(m.this_week, "action"))
    } else if m.streak > 0 {
        format!("Nothing logged today yet. Finish a task or leave a note to keep your {}-day streak.", m.streak)
    } else {
        match m.quiet_days(today) {
            Some(n) => format!("Quiet for {}. One small task restarts the streak.", plural(n as u32, "day")),
            None => "Finish a task or leave a note to start a streak.".into(),
        }
    };
    let recent = momentum::heatmap(&days, today, 8);
    let quiet: Vec<(&ProjectIndex, i64)> = all
        .iter()
        .filter(|p| p.overall().done < p.overall().total)
        .filter_map(|p| momentum::last_active(p).map(|d| (*p, (today - d).num_days())))
        .filter(|(_, q)| *q >= QUIET_AFTER)
        .collect();
    html! {
        section.momentum.risk[m.at_risk()] aria-label="Momentum" {
            div.momentum-num {
                span.momentum-streak { (m.streak) }
                span.momentum-unit { "day streak" br; "best " (m.longest) }
            }
            div.momentum-body {
                p.momentum-nudge { (nudge) }
                div.momentum-days role="img" aria-label=(format!("Last 8 weeks: {} active days", recent.iter().flatten().flatten().filter(|(_, n)| *n > 0).count())) {
                    @for (d, n) in recent.iter().flatten().flatten() {
                        span.(format!("l{}", momentum::level(*n))).today[*d == today] title=(format!("{}: {}", short(*d), plural(*n, "action"))) {}
                    }
                }
                @if !quiet.is_empty() {
                    p.momentum-quiet {
                        "Going quiet: "
                        @for (i, (p, q)) in quiet.iter().enumerate() {
                            @if i > 0 { ", " }
                            a href=(ctx.href(&routes::board(&p.slug, None))) { (p.name) } " (" (q) "d)"
                        }
                    }
                }
            }
        }
    }
}

fn plural(n: u32, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

fn agenda_row(ctx: &Ctx, t: TaskRef) -> Markup {
    let n = t.node();
    let today = ctx.site.clock.today;
    html! {
        div.agenda-row {
            @if n.task.milestone { span.diamond aria-label="Milestone" { "◆" } } @else { (check(ctx, t.project, t.idx, true)) }
            a.agenda-title href=(task_link(ctx, t.project, t.idx)) { (title_of(n)) }
            (pri_mark(n.task.priority))
            span.agenda-project { (t.project.name) }
            span.agenda-due.late[n.is_late(today)] { @if let Some(d) = n.due() { (short(d)) } }
        }
    }
}
