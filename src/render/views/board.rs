//! Board: status columns for one level of the task tree.
//!
//! The project root board shows top-level tasks; every task with subtasks also
//! gets its own board, reachable via the ⊞ link on its card. When editing,
//! cards drag between columns and each column has an add field.

use maud::{Markup, html};

use crate::domain::ProjectIndex;
use crate::domain::schedule::due_label;
use crate::model::Status;

use crate::render::components::{filter_text, pri_filters, pri_mark, task_link, title_of};
use crate::render::layout::{self, Chrome, Nav, Tab};
use crate::render::routes::{self, Ctx};

pub fn page(ctx: &Ctx, p: &ProjectIndex, parent: Option<usize>) -> Markup {
    let scope: Vec<usize> = match parent {
        Some(i) => p.nodes[i].children.clone(),
        None => p.roots.clone(),
    }
    .into_iter()
    .filter(|&i| !p.nodes[i].task.milestone)
    .collect();
    let parent_id = parent.map(|i| p.nodes[i].task.id.as_str());

    let subtitle = match parent {
        Some(i) => {
            let mut crumbs: Vec<&str> = p.path(i).into_iter().map(|a| title_of(p.node(a))).collect();
            crumbs.push(title_of(p.node(i)));
            format!("Board › {}", crumbs.join(" › "))
        }
        None => p.description.clone(),
    };

    let content = html! {
        div.page-pad.stack {
            div.toolbar {
                @if let Some(i) = parent {
                    @let up = p.nodes[i].parent;
                    a.pill href=(ctx.href(&routes::board(&p.slug, up.map(|u| p.nodes[u].task.id.as_str())))) {
                        "↑ Back to " @match up { Some(u) => (title_of(p.node(u))), None => "project board" }
                    }
                    a.pill href=(task_link(ctx, p, i)) { "Open " (p.nodes[i].task.id) }
                }
                span.grow {}
                @if ctx.editable { span.faint.small { "Drag cards, or Shift + ←/→" } }
                (pri_filters())
            }
            div.board {
                @for s in Status::ALL {
                    @let cards: Vec<usize> = scope.iter().copied().filter(|&i| p.nodes[i].task.status == s).collect();
                    section.column aria-label=(s.label()) data-count-scope data-drop-status=[ctx.editable.then_some(s.key())] {
                        div.column-head {
                            span.dot aria-hidden="true" style=(format!("background:{}", s.color())) {}
                            span.column-title { (s.label()) }
                            span.faint data-count { (cards.len()) }
                        }
                        @for i in cards { (card(ctx, p, i)) }
                        @if ctx.editable {
                            form.column-add data-create data-project=(p.slug) data-status=(s.key()) data-parent=[parent_id] {
                                input name="title" autocomplete="off" placeholder="＋ Add" aria-label=(format!("Add task to {}", s.label()));
                            }
                        }
                    }
                }
            }
        }
    };

    layout::page(
        ctx,
        Chrome { nav: Nav::Project(p, Tab::Board), title: p.name.clone(), subtitle, filter: true, new_task_parent: parent_id },
        content,
    )
}

fn card(ctx: &Ctx, p: &ProjectIndex, i: usize) -> Markup {
    let n = p.node(i);
    let today = ctx.site.clock.today;
    let pr = p.progress(i);
    let due = if n.is_done() { None } else { n.due().map(|d| due_label(d, today)) };
    html! {
        div.card.done[n.is_done()] data-item data-text=(filter_text(n)) data-pri=(n.task.priority.key())
            data-id=(n.task.id) data-status=(n.task.status.key()) draggable=[ctx.editable.then_some("true")] {
            a.card-title href=(task_link(ctx, p, i)) draggable=[ctx.editable.then_some("false")] { (title_of(n)) }
            div.card-meta {
                (pri_mark(n.task.priority))
                @if let Some(d) = due { span.late[n.is_late(today)] { (d) } }
                @if let Some(pr) = pr {
                    a.card-drill href=(ctx.href(&routes::board(&p.slug, Some(&n.task.id)))) aria-label=(format!("Open {} as board", title_of(n))) {
                        "⊞ " (pr.done) "/" (pr.total)
                    }
                }
                span.card-id { (n.task.id) }
            }
        }
    }
}
