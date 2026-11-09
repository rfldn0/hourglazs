//! The public "Past projects" page: finished work, newest first, by year.
//!
//! A project is finished when its file says `"finished": "YYYY-MM-DD"` or
//! when every task is done. Projects that were never tracked here can be
//! listed with just a name, a description, `finished` and a `url`.

use chrono::{Datelike, NaiveDate};
use maud::{Markup, html};

use crate::domain::ProjectIndex;

use crate::render::components::logo;
use crate::render::layout::{self, PublicNav};
use crate::render::routes::Ctx;

use super::public::status_pill;

pub fn page(ctx: &Ctx) -> Markup {
    let mut done: Vec<&ProjectIndex> = ctx.site.projects.iter().filter(|p| p.public && p.is_finished()).collect();
    // Newest first; undated last.
    done.sort_by_key(|p| std::cmp::Reverse(p.finished_on().map_or(i64::MIN, |d| d.num_days_from_ce().into())));

    let mut years: Vec<(Option<i32>, Vec<&ProjectIndex>)> = Vec::new();
    for p in done.iter().copied() {
        let y = p.finished_on().map(|d| d.year());
        match years.last_mut() {
            Some((year, list)) if *year == y => list.push(p),
            _ => years.push((y, vec![p])),
        }
    }
    let tasks: usize = done.iter().map(|p| p.overall().done).sum();

    let content = html! {
        div.wrap.stack {
            section.hero aria-labelledby="title" {
                p.hero-date { "Archive" }
                h1 id="title" { "Past projects" }
                p.lede {
                    @if done.is_empty() {
                        "Nothing finished yet. Finished projects will be listed here."
                    } @else {
                        (count(done.len(), "finished project", "finished projects"))
                        @if tasks > 0 { ", " (count(tasks, "task", "tasks")) " completed" }
                        "."
                    }
                }
            }
            @for (year, list) in &years {
                @let id = year.map_or_else(|| "undated".to_string(), |y| format!("y{y}"));
                section aria-labelledby=(id) {
                    div.sec-head {
                        h2 id=(id) { @match year { Some(y) => (y), None => "Undated" } }
                        span.mono { (count(list.len(), "project", "projects")) }
                    }
                    div.projects.past-list { @for p in list { (card(p)) } }
                }
            }
        }
    };
    layout::public_page(ctx, PublicNav::Past, status_pill(ctx), content)
}

fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn full(d: NaiveDate) -> String {
    d.format("%b %-d, %Y").to_string()
}

/// "3 weeks", "5 days", "4 months".
fn duration(days: i64) -> String {
    match days {
        d if d < 14 => count(d.max(1) as usize, "day", "days"),
        d if d < 60 => count((d / 7) as usize, "week", "weeks"),
        d => count((d / 30) as usize, "month", "months"),
    }
}

fn card(p: &ProjectIndex) -> Markup {
    let pr = p.overall();
    let finished = p.finished_on();
    let milestones = p.nodes.iter().filter(|n| n.task.milestone && n.is_done()).count();
    let took = match (p.span(), finished) {
        (Some(s), Some(f)) if f >= s.start => Some(duration((f - s.start).num_days() + 1)),
        _ => None,
    };
    let heading_id = format!("p-{}", p.slug);
    html! {
        article.proj.past aria-labelledby=(heading_id) {
            // Mood art, not information: empty alt so screen readers skip it.
            @if p.image.is_empty() {
                div.cover.placeholder aria-hidden="true" { (logo()) }
            } @else {
                img.cover src=(p.image) alt="" loading="lazy" decoding="async";
            }
            div.past-body {
            div.proj-head {
                div {
                    p.status.done { span aria-hidden="true" { "✓ " } "Finished" }
                    h3 id=(heading_id) {
                        @if p.url.is_empty() {
                            (p.name)
                        } @else {
                            a.ext href=(p.url) rel="noopener" {
                                (p.name)
                                span aria-hidden="true" { " ↗" }
                            }
                        }
                    }
                }
                @if let Some(d) = finished {
                    time.mono datetime=(d.to_string()) { (full(d)) }
                }
            }
            @if !p.description.is_empty() { p.proj-desc { (p.description) } }
            @let facts = [
                (pr.total > 0).then(|| count(pr.done, "task", "tasks")),
                (milestones > 0).then(|| count(milestones, "milestone", "milestones")),
                took.map(|t| format!("took {t}")),
            ];
            @if facts.iter().any(Option::is_some) {
                p.mono.past-facts {
                    @for (i, f) in facts.iter().flatten().enumerate() {
                        @if i > 0 { " · " }
                        (f)
                    }
                }
            }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::duration;

    #[test]
    fn durations_read_naturally() {
        assert_eq!(duration(1), "1 day");
        assert_eq!(duration(10), "10 days");
        assert_eq!(duration(21), "3 weeks");
        assert_eq!(duration(120), "4 months");
    }
}
