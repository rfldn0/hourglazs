//! Task detail: properties, description, subtasks and activity.
//! Read-only in a static build; a form when served by `hourglazs serve`.

use maud::{Markup, html};

use crate::domain::schedule::short;
use crate::domain::{Node, ProjectIndex};
use crate::model::{LogKind, Priority, Status};

use crate::render::components::{check, hours, pri_mark, progress, status, task_link, title_of, when};
use crate::render::layout::{self, Chrome, Nav, Tab};
use crate::render::routes::{self, Ctx};

pub fn page(ctx: &Ctx, p: &ProjectIndex, i: usize) -> Markup {
    let n = p.node(i);
    let today = ctx.site.clock.today;
    let pr = p.progress(i);
    let close = match n.parent {
        Some(parent) => ctx.href(&routes::board(&p.slug, Some(&p.nodes[parent].task.id))),
        None => ctx.href(&routes::board(&p.slug, None)),
    };

    let content = html! {
        div.page-pad.detail data-project=(p.slug) data-id=(n.task.id) {
            div.detail-top {
                nav.crumbs aria-label="Task path" {
                    a href=(ctx.href(&routes::board(&p.slug, None))) { (p.name) }
                    @for a in p.path(i) {
                        span aria-hidden="true" { "›" }
                        a href=(task_link(ctx, p, a)) { (title_of(p.node(a))) }
                    }
                    span aria-hidden="true" { "›" }
                    span.current { (n.task.id) }
                }
                a.pill href=(close) data-esc { "Esc ✕" }
            }

            @if ctx.editable {
                textarea.title-input data-field="title" rows="1" aria-label="Task title" { (n.task.title) }
            } @else {
                h2.detail-title { @if n.task.milestone { "◆ " } (title_of(n)) }
            }

            @if pr.is_some_and(|pr| pr.done == pr.total) && !n.is_done() {
                div.banner role="status" {
                    span.grow { "All subtasks are done." }
                    @if ctx.editable {
                        button.btn.ok type="button" data-action="set" data-field="status" data-value="done" { "Mark task done" }
                    }
                }
            }
            @if n.is_late(today) {
                div.banner.warn role="status" { "Overdue since " (n.due().map(short).unwrap_or_default()) "." }
            }

            @if ctx.editable { (props_form(n)) } @else { (props_view(n)) }

            @if ctx.editable {
                label.block {
                    span.block-label { "Description" }
                    textarea.field.area data-field="description" rows="4" placeholder="Details, links, acceptance criteria…" { (n.task.description) }
                }
            } @else if !n.task.description.trim().is_empty() {
                section.block aria-label="Description" {
                    @for para in n.task.description.split("\n\n") { p { (para) } }
                }
            }

            (subtasks(ctx, p, i))
            (activity(ctx, p, n))

            @if ctx.editable {
                div.detail-foot {
                    button.btn.danger type="button" data-action="delete" data-title=(title_of(n)) data-next=(close) { "Delete task" }
                    span.faint.small { "Saved to " code { "data/projects/" (p.slug) ".json" } }
                }
            } @else {
                p.source.faint.small { "Source: " code { "data/projects/" (p.slug) ".json" } " → " code { (n.task.id) } }
            }
        }
    };

    layout::page(
        ctx,
        Chrome { nav: Nav::Project(p, Tab::Task), title: p.name.clone(), subtitle: n.task.id.clone(), filter: false, new_task_parent: Some(&n.task.id) },
        content,
    )
}

fn props_view(n: &Node) -> Markup {
    html! {
        dl.props {
            dt { "Status" } dd { (status(n.task.status)) }
            dt { "Priority" }
            dd { span.pri-text style=(format!("color:{}", n.task.priority.color())) { (n.task.priority.glyph()) " " (n.task.priority.label()) } }
            dt { "Schedule" }
            dd {
                @match n.span {
                    Some(s) if n.task.milestone => (short(s.start)),
                    Some(s) => { (short(s.start)) " → " (short(s.end)) " · " (s.days()) "d" }
                    None => span.faint { "Unscheduled" },
                }
                @if let Some(h) = n.task.hours { " · " (hours(Some(h))) " estimated" }
            }
            @if let Some(d) = n.task.done_on { dt { "Completed" } dd { (short(d)) } }
            @if !n.task.tags.is_empty() {
                dt { "Tags" }
                dd.tags { @for t in &n.task.tags { span.tag { (t) } } }
            }
        }
    }
}

fn props_form(n: &Node) -> Markup {
    let date = |d: Option<chrono::NaiveDate>| d.map(|d| d.to_string()).unwrap_or_default();
    // Show the effective span, including one derived from subtasks.
    let (start, due) = match n.span {
        Some(s) if n.task.milestone => (String::new(), s.start.to_string()),
        Some(s) => (s.start.to_string(), s.end.to_string()),
        None => (date(None), date(None)),
    };
    html! {
        div.props {
            label.prop-label for="f-status" { "Status" }
            select.field #f-status data-field="status" {
                @for s in Status::ALL {
                    option value=(s.key()) selected[s == n.task.status] { (s.label()) }
                }
            }
            span.prop-label { "Priority" }
            div.choice role="group" aria-label="Priority" {
                @for pr in Priority::ALL {
                    @let on = pr == n.task.priority;
                    button type="button" aria-pressed=(on) data-action="set" data-field="priority" data-value=(pr.key())
                        style=[on.then(|| format!("color:{0};border-color:{0}", pr.color()))] {
                        (pr.glyph()) " " (pr.label())
                    }
                }
            }
            span.prop-label { "Dates" }
            div.inline-fields {
                @if !n.task.milestone {
                    label { span.faint { "Start" } input.field type="date" data-field="start" value=(start); }
                }
                label { span.faint { @if n.task.milestone { "Date" } @else { "Due" } } input.field type="date" data-field="due" value=(due); }
                @if n.span.is_some() {
                    button.link-quiet type="button" data-action="clear-dates" { "Clear" }
                }
            }
            @if n.task.start.is_none() && n.task.due.is_none() && n.span.is_some() {
                span {} span.faint.small { "From subtasks. Setting dates here pins them on this task." }
            }
            label.prop-label for="f-hours" { "Estimate" }
            div.inline-fields {
                input.field.short #f-hours type="number" min="0" step="0.5" data-field="hours" value=[n.task.hours] placeholder="—";
                span.faint { "hours" }
            }
            label.prop-label for="f-tags" { "Tags" }
            input.field #f-tags type="text" data-field="tags" value=(n.task.tags.join(", ")) placeholder="comma, separated";
            span.prop-label { "Milestone" }
            label.inline-fields {
                input type="checkbox" data-field="milestone" checked[n.task.milestone];
                span.faint { "Show as a ◆ on timelines and the public page" }
            }
            @if let Some(d) = n.task.done_on {
                span.prop-label { "Completed" } span { (short(d)) }
            }
        }
    }
}

fn subtasks(ctx: &Ctx, p: &ProjectIndex, i: usize) -> Markup {
    let n = p.node(i);
    let pr = p.progress(i);
    let subtree: Vec<usize> = p.subtree(i).into_iter().skip(1).collect();
    html! {
        section.block aria-labelledby="subs" {
            div.block-head {
                h3 id="subs" { "Subtasks" }
                span.mono.dim { @match pr { Some(pr) => { (pr.done) " of " (pr.total) " done" } None => "None" } }
                @if !n.is_leaf() {
                    a.link-quiet href=(ctx.href(&routes::board(&p.slug, Some(&n.task.id)))) { "Open as board" }
                }
            }
            @if let Some(pr) = pr { (progress(pr.pct(), "Subtask progress")) }
            ul.subtree {
                @for s in &subtree {
                    @let sn = p.node(*s);
                    li style=(format!("--depth:{}", sn.depth - n.depth - 1)) {
                        (check(ctx, p, *s, false))
                        a.done[sn.is_done()].parent[!sn.is_leaf()] href=(task_link(ctx, p, *s)) { (title_of(sn)) }
                        @if let Some(spr) = p.progress(*s) { span.mono.dim { (spr.done) "/" (spr.total) } }
                        (pri_mark(sn.task.priority))
                    }
                }
                @if ctx.editable {
                    li.add-row {
                        form.grow data-create data-parent=(n.task.id) {
                            input name="title" autocomplete="off" aria-label="Add subtask" placeholder="＋ Add subtask and press Enter";
                        }
                    }
                }
            }
        }
    }
}

fn activity(ctx: &Ctx, p: &ProjectIndex, n: &Node) -> Markup {
    let log: Vec<_> = p.log.iter().filter(|l| l.task.as_deref() == Some(n.task.id.as_str())).collect();
    html! {
        section.block aria-labelledby="act" {
            div.block-head { h3 id="act" { "Activity" } }
            @if ctx.editable {
                form.comment data-comment {
                    textarea.field.area name="text" rows="2" aria-label="Write a note" placeholder="Write a note…  (Ctrl+Enter to save)" {}
                    div { button.btn type="submit" { "Add note" } }
                }
            } @else if log.is_empty() {
                p.faint.small { "No entries yet." }
            }
            ol.feed {
                @for l in &log {
                    li {
                        span.feed-text.note[l.kind == LogKind::Note] { (l.text) }
                        span.feed-meta { (when(l.at)) }
                    }
                }
            }
        }
    }
}
