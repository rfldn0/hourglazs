//! Gantt: one project's task tree against a week-by-week calendar.

use chrono::Days;
use maud::{Markup, html};

use crate::domain::schedule::{monday_on_or_before, short};
use crate::domain::{ProjectIndex, Span};

use crate::render::components::{filter_text, pri_filters, task_link, title_of};
use crate::render::layout::{self, Chrome, Nav, Tab};
use crate::render::routes::Ctx;

use super::tasks::ancestors_attr;

/// Include today in the chart if it is within this many days of the work.
const TODAY_REACH_DAYS: i64 = 28;

pub fn page(ctx: &Ctx, p: &ProjectIndex) -> Markup {
    let today = ctx.site.clock.today;
    let range = chart_range(p.span(), today);
    let total = range.days() as f64;
    let weeks = (range.days() / 7) as usize;
    let frac = |d: chrono::NaiveDate| (d - range.start).num_days() as f64 / total;
    let today_frac = range.contains(today).then(|| frac(today) + 0.5 / total);
    let label = format!("{} – {} · {} weeks", short(range.start), short(range.end), weeks);

    let content = html! {
        div.page-pad.stack {
            div.toolbar {
                button.ghost type="button" data-expand-all { "Expand all" }
                button.ghost type="button" data-collapse-all { "Collapse all" }
                span.grow {}
                (pri_filters())
                span.mono.faint { (label) }
            }
            div.gantt-scroll {
                div.gantt role="table" aria-label="Project schedule" style=(format!("--weeks:{weeks}")) {
                    div.g-row.g-head role="row" {
                        div.g-label { span role="columnheader" { "Task" } span role="columnheader" { "Due" } }
                        div.g-weeks aria-hidden="true" {
                            @for w in 0..weeks { span { (short(range.start + Days::new(w as u64 * 7)).to_uppercase()) } }
                        }
                    }
                    @for i in 0..p.nodes.len() { (row(ctx, p, i, range, total)) }
                    @if let Some(f) = today_frac {
                        div.g-today aria-hidden="true" style=(format!("--at:{f:.5}")) { span { "TODAY" } }
                    }
                }
            }
            @if p.nodes.iter().any(|n| n.span.is_none()) {
                p.faint.small { "Tasks without dates are listed without a bar." }
            }
        }
    };

    layout::page(
        ctx,
        Chrome { nav: Nav::Project(p, Tab::Gantt), title: p.name.clone(), subtitle: "Gantt".into(), filter: true, new_task_parent: None },
        content,
    )
}

/// Whole weeks (Mon–Sun) around the project's work.
fn chart_range(work: Option<Span>, today: chrono::NaiveDate) -> Span {
    let mut span = work.unwrap_or(Span::day(today));
    let gap = (today - span.end).num_days().max((span.start - today).num_days());
    if gap <= TODAY_REACH_DAYS {
        span = span.union(Span::day(today));
    }
    let start = monday_on_or_before(span.start);
    let weeks = ((span.end - start).num_days() / 7 + 1).max(4);
    Span::new(start, start + Days::new(weeks as u64 * 7 - 1))
}

fn row(ctx: &Ctx, p: &ProjectIndex, i: usize, range: Span, total: f64) -> Markup {
    let n = p.node(i);
    let today = ctx.site.clock.today;
    let pct = p.completion(i);
    let bar = n.span.map(|s| {
        let left = (s.start - range.start).num_days() as f64 / total * 100.0;
        let width = s.days() as f64 / total * 100.0;
        if n.task.milestone {
            format!("left:calc({:.3}% - 8px)", left + 0.5 / total * 100.0)
        } else {
            format!("left:{left:.3}%;width:{width:.3}%")
        }
    });
    html! {
        div.g-row role="row" data-item data-text=(filter_text(n)) data-pri=(n.task.priority.key())
            data-ancestors=(ancestors_attr(p, i)) style=(format!("--depth:{}", n.depth)) {
            div.g-label.top[!n.is_leaf() && n.depth == 0] {
                span.g-name role="cell" {
                    @if n.is_leaf() {
                        span.caret-space {}
                    } @else {
                        button.caret type="button" data-toggle=(n.task.id) aria-expanded="true" aria-label=(format!("Collapse {}", title_of(n))) { "▾" }
                    }
                    a.parent[!n.is_leaf()].done[n.is_done()] href=(task_link(ctx, p, i)) { (title_of(n)) }
                }
                span.mono.late[n.is_late(today)] role="cell" { @if let Some(d) = n.due() { (short(d)) } }
            }
            div.g-track {
                @if let Some(style) = bar {
                    a.g-bar.parent[!n.is_leaf()].milestone[n.task.milestone].late[n.is_late(today)].done[n.is_done()]
                        href=(task_link(ctx, p, i)) style=(style) tabindex="-1"
                        title=(format!("{} · {pct}%", title_of(n))) {
                        @if !n.task.milestone { span.g-fill style=(format!("width:{pct}%")) {} }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> chrono::NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn range_snaps_to_whole_weeks_and_includes_nearby_today() {
        let r = chart_range(Some(Span::new(d("2026-11-10"), d("2026-11-20"))), d("2026-11-07"));
        assert_eq!(r.start, d("2026-11-02"));
        assert_eq!(r.days() % 7, 0);
        assert!(r.days() >= 28);
        // Far-away today doesn't stretch the chart.
        let r = chart_range(Some(Span::new(d("2026-11-10"), d("2026-11-20"))), d("2027-06-01"));
        assert!(!r.contains(d("2027-06-01")));
    }
}
