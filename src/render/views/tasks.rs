//! Tasks: the whole project as a collapsible tree (flat while filtering).

use maud::{Markup, html};

use crate::domain::ProjectIndex;
use crate::domain::schedule::short;

use crate::render::components::{check, filter_text, hours, pri_filters, pri_mark, status, task_link, title_of};
use crate::render::layout::{self, Chrome, Nav, Tab};
use crate::render::routes::Ctx;

pub fn page(ctx: &Ctx, p: &ProjectIndex) -> Markup {
    let content = html! {
        div.page-pad.stack.narrow {
            @if ctx.editable {
                form.quick-add data-create data-project=(p.slug) {
                    span aria-hidden="true" { "＋" }
                    input name="title" autocomplete="off" aria-label="Quick add task"
                        placeholder="Add a task…   !high  !low  #tag";
                }
            }
            div.toolbar {
                button.ghost type="button" data-expand-all { "Expand all" }
                button.ghost type="button" data-collapse-all { "Collapse all" }
                span.grow {}
                (pri_filters())
            }
            div.tasks role="table" aria-label="Tasks" {
                div.trow.thead role="row" {
                    span role="columnheader" { "Task" }
                    span role="columnheader" { "Status" }
                    span role="columnheader" { "Due" }
                    span role="columnheader" { "Est." }
                }
                @for i in 0..p.nodes.len() { (row(ctx, p, i)) }
                @if p.nodes.is_empty() {
                    p.empty { "No tasks yet. Add some to data/projects/" (p.slug) ".json." }
                }
            }
        }
    };
    layout::page(
        ctx,
        Chrome { nav: Nav::Project(p, Tab::Tasks), title: p.name.clone(), subtitle: "Tasks".into(), filter: true, new_task_parent: None },
        content,
    )
}

/// `data-ancestors` lets `app.js` hide a row when any ancestor is collapsed.
pub fn ancestors_attr(p: &ProjectIndex, i: usize) -> String {
    p.path(i).iter().map(|&a| p.nodes[a].task.id.as_str()).collect::<Vec<_>>().join(" ")
}

fn row(ctx: &Ctx, p: &ProjectIndex, i: usize) -> Markup {
    let n = p.node(i);
    let today = ctx.site.clock.today;
    let pr = p.progress(i);
    let path: Vec<&str> = p.path(i).into_iter().map(|a| title_of(p.node(a))).collect();
    html! {
        div.trow role="row" data-item data-text=(filter_text(n)) data-pri=(n.task.priority.key())
            data-ancestors=(ancestors_attr(p, i)) style=(format!("--depth:{}", n.depth)) {
            span.tmain role="cell" {
                @if n.is_leaf() {
                    span.caret-space {}
                } @else {
                    button.caret type="button" data-toggle=(n.task.id) aria-expanded="true" aria-label=(format!("Collapse {}", title_of(n))) { "▾" }
                }
                @if n.task.milestone { span.diamond aria-label="Milestone" { "◆" } } @else { (check(ctx, p, i, false)) }
                a.ttitle.parent[!n.is_leaf()].done[n.is_done()] href=(task_link(ctx, p, i)) {
                    span.tline {
                        (pri_mark(n.task.priority))
                        span.tname { (title_of(n)) }
                        @if let Some(pr) = pr { span.subs { (pr.done) "/" (pr.total) } }
                    }
                    span.tpath { (n.task.id) @for a in &path { " › " (a) } }
                }
            }
            span role="cell" { (status(n.task.status)) }
            span.mono.late[n.is_late(today)] role="cell" {
                @if let Some(d) = n.due() { @if !n.is_done() { (short(d)) } }
            }
            span.mono.dim role="cell" { (hours(n.task.hours)) }
        }
    }
}
