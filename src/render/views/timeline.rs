//! Timeline: every project's top-level work on one month-by-month roadmap.

use chrono::{Datelike, Days, NaiveDate};
use maud::{Markup, html};

use crate::domain::schedule::{first_of_month, next_month, short};
use crate::domain::{ProjectIndex, Span};
use crate::model::Status;

use crate::render::components::{task_link, title_of};
use crate::render::layout::{self, Chrome, Nav};
use crate::render::routes::{self, Ctx};

const MIN_MONTHS: usize = 3;
const LANE_PX: usize = 40;

pub fn page(ctx: &Ctx) -> Markup {
    let site = ctx.site;
    let today = site.clock.today;
    let months = month_range(site.projects.iter().filter_map(ProjectIndex::span), today);
    let r0 = months[0];
    let r1 = next_month(*months.last().expect("at least one month"));
    let total = (r1 - r0).num_days() as f64;
    let pos = |d: NaiveDate| (d - r0).num_days() as f64 / total * 100.0;

    let grid_cols: String = months
        .iter()
        .map(|m| format!("{}fr", (next_month(*m) - *m).num_days()))
        .collect::<Vec<_>>()
        .join(" ");
    let range_label = format!("{} – {}", months[0].format("%b %Y"), months.last().unwrap().format("%b %Y"));
    let today_pct = (pos(today) + 0.5 / total * 100.0).clamp(0.0, 100.0);

    let content = html! {
        div.page-pad.stack {
            div.toolbar { span.dim { (range_label) } }
            div.roadmap style=(format!("--months:{grid_cols}")) {
                div.rm-row.rm-head {
                    div {}
                    div.rm-months {
                        @for m in &months { span { (m.format("%B").to_string().to_uppercase()) @if m.month() == 1 { " " (m.year()) } } }
                    }
                }
                @for p in &site.projects {
                    @let (bars, lanes) = layout_bars(ctx, p, r0, total);
                    div.rm-row {
                        div.rm-project {
                            a.rm-name href=(ctx.href(&routes::gantt(&p.slug))) {
                                span.ring aria-hidden="true" style=(format!("border-color:{}", p.accent.css())) {}
                                (p.name)
                            }
                            span.mono.faint { (p.open_count()) " open tasks" }
                        }
                        div.rm-track style=(format!("height:{}px", lanes.max(1) * LANE_PX + 24)) {
                            div.rm-gridlines aria-hidden="true" { @for _ in &months { span {} } }
                            (bars)
                            div.today-line aria-hidden="true" style=(format!("left:{today_pct:.3}%")) {}
                        }
                    }
                }
            }
            div.legend {
                span { span.swatch.doing {} "In progress" }
                span { span.swatch.plan {} "Planned" }
                span { span.swatch.done {} "Done" }
                span { span.swatch.today {} "Today" }
            }
        }
    };

    layout::page(
        ctx,
        Chrome { nav: Nav::Timeline, title: "Timeline".into(), subtitle: String::new(), filter: false, new_task_parent: None },
        content,
    )
}

/// First-of-month dates covering every span and today, at least `MIN_MONTHS` long.
fn month_range(spans: impl Iterator<Item = Span>, today: NaiveDate) -> Vec<NaiveDate> {
    let all = spans.fold(Span::day(today), Span::union);
    let mut m = first_of_month(all.start);
    let mut out = Vec::new();
    while m <= all.end || out.len() < MIN_MONTHS {
        out.push(m);
        m = next_month(m);
    }
    out
}

/// Bars for one project's top-level tasks, packed into non-overlapping lanes.
fn layout_bars(ctx: &Ctx, p: &ProjectIndex, r0: NaiveDate, total: f64) -> (Markup, usize) {
    let mut items: Vec<(usize, Span)> = p.roots.iter().filter_map(|&i| p.nodes[i].span.map(|s| (i, s))).collect();
    items.sort_by_key(|(_, s)| s.start);

    // Short bars still get a readable minimum width, so pack by visual length.
    let min_days = (total * 0.1).ceil() as i64;
    let mut lane_ends: Vec<NaiveDate> = Vec::new();
    let markup = html! {
        @for (i, span) in &items {
            @let n = p.node(*i);
            @let visual_end = span.start + Days::new(span.days().max(min_days) as u64);
            @let lane = match lane_ends.iter().position(|end| *end <= span.start) {
                Some(l) => { lane_ends[l] = visual_end; l }
                None => { lane_ends.push(visual_end); lane_ends.len() - 1 }
            };
            @let left = (span.start - r0).num_days() as f64 / total * 100.0;
            @let width = span.days().max(min_days) as f64 / total * 100.0;
            @let pct = p.completion(*i);
            @let state = match n.task.status { Status::Done => "done", Status::Doing => "doing", _ => "plan" };
            a.rm-bar.(state).milestone[n.task.milestone]
                href=(task_link(ctx, p, *i))
                title=(format!("{} · {} → {} · {pct}% done", title_of(n), short(span.start), short(span.end)))
                style=(format!("top:{}px;left:{left:.3}%;width:{width:.3}%", 14 + lane * LANE_PX)) {
                span.rm-label { @if n.task.milestone { "◆ " } (title_of(n)) }
                span.rm-pct { @if n.task.milestone { (short(span.start)) } @else { (pct) "%" } }
            }
        }
    };
    (markup, lane_ends.len())
}
