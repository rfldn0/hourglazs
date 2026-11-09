//! Edits applied to a project, plus the activity-log entries they produce.

use anyhow::{Context, Result, bail};
use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Deserializer};

use crate::model::{LogEntry, LogKind, Priority, Project, Status, Task};

use super::tree;

/// Partial update. For the clearable fields, a missing key leaves the value
/// alone and `null` clears it.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub status: Option<Status>,
    pub priority: Option<Priority>,
    #[serde(default, deserialize_with = "clearable")]
    pub start: Option<Option<NaiveDate>>,
    #[serde(default, deserialize_with = "clearable")]
    pub due: Option<Option<NaiveDate>>,
    #[serde(default, deserialize_with = "clearable")]
    pub days: Option<Option<u32>>,
    #[serde(default, deserialize_with = "clearable")]
    pub hours: Option<Option<f32>>,
    pub milestone: Option<bool>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

fn clearable<'de, T: Deserialize<'de>, D: Deserializer<'de>>(d: D) -> Result<Option<Option<T>>, D::Error> {
    Option::<T>::deserialize(d).map(Some)
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewTask {
    #[serde(default)]
    pub parent: Option<String>,
    pub title: String,
    #[serde(default)]
    pub status: Option<Status>,
    #[serde(default)]
    pub priority: Option<Priority>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// A short human summary of each edit, used for commit messages.
pub type Summary = String;

pub fn update(p: &mut Project, id: &str, patch: TaskPatch, now: NaiveDateTime) -> Result<Summary> {
    let t = tree::find_mut(&mut p.tasks, id).with_context(|| format!("no task {id:?}"))?;
    let mut events = Vec::new();
    let mut changed = Vec::new();

    if let Some(title) = patch.title {
        let title = title.trim();
        if title.is_empty() {
            bail!("title can't be empty");
        }
        if title != t.title {
            t.title = title.into();
            changed.push("title");
        }
    }
    if let Some(s) = patch.status.filter(|s| *s != t.status) {
        t.status = s;
        t.done_on = (s == Status::Done).then_some(now.date());
        events.push(if s == Status::Done {
            format!("completed {}", t.title)
        } else {
            format!("moved {} to {}", t.title, s.label())
        });
    }
    if let Some(pr) = patch.priority.filter(|pr| *pr != t.priority) {
        t.priority = pr;
        events.push(format!("set priority of {} to {}", t.title, pr.label()));
    }
    if let Some(v) = patch.start { t.start = v; changed.push("dates"); }
    if let Some(v) = patch.due { t.due = v; changed.push("dates"); }
    if let Some(v) = patch.days { t.days = v; changed.push("dates"); }
    if let Some(v) = patch.hours { t.hours = v; changed.push("estimate"); }
    if let Some(v) = patch.milestone { t.milestone = v; changed.push("milestone"); }
    if let Some(v) = patch.description { t.description = v.trim_end().into(); changed.push("description"); }
    if let Some(v) = patch.tags {
        t.tags = v.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        changed.push("tags");
    }
    // start + due already pins the span; a stale `days` would contradict it.
    if t.start.is_some() && t.due.is_some() {
        t.days = None;
    }

    changed.dedup();
    let summary = if events.is_empty() {
        format!("{id}: edit {}", changed.join(", "))
    } else {
        format!("{id}: {}", events.join(", "))
    };
    for text in events {
        log(p, now, Some(id), LogKind::Event, text);
    }
    Ok(summary)
}

pub fn create(p: &mut Project, new: NewTask, now: NaiveDateTime) -> Result<(String, Summary)> {
    let title = new.title.trim();
    if title.is_empty() {
        bail!("title can't be empty");
    }
    let id = tree::next_id(&p.key, &p.tasks);
    let mut task = Task::new(id.clone(), title);
    task.status = new.status.unwrap_or_default();
    task.priority = new.priority.unwrap_or_default();
    task.tags = new.tags;
    if task.status == Status::Done {
        task.done_on = Some(now.date());
    }

    match new.parent.as_deref() {
        Some(pid) => {
            let parent = tree::find_mut(&mut p.tasks, pid).with_context(|| format!("no task {pid:?}"))?;
            // New work under a finished task re-opens it.
            if parent.status == Status::Done {
                parent.status = Status::Doing;
                parent.done_on = None;
            }
            parent.subtasks.push(task);
        }
        None => p.tasks.push(task),
    }
    log(p, now, Some(&id), LogKind::Event, format!("created {title}"));
    Ok((id.clone(), format!("{id}: create {title}")))
}

pub fn delete(p: &mut Project, id: &str, now: NaiveDateTime) -> Result<Summary> {
    let removed = tree::remove(&mut p.tasks, id).with_context(|| format!("no task {id:?}"))?;
    let gone = tree::ids(&removed);
    // Keep the history, just unlink it from tasks that no longer exist.
    for entry in &mut p.log {
        if entry.task.as_ref().is_some_and(|t| gone.contains(t)) {
            entry.task = None;
        }
    }
    let extra = if gone.len() > 1 { format!(" and {} subtasks", gone.len() - 1) } else { String::new() };
    log(p, now, None, LogKind::Event, format!("deleted {} ({id}){extra}", removed.title));
    Ok(format!("{id}: delete {}{extra}", removed.title))
}

pub fn comment(p: &mut Project, task: Option<&str>, text: &str, now: NaiveDateTime) -> Result<Summary> {
    let text = text.trim();
    if text.is_empty() {
        bail!("comment is empty");
    }
    if let Some(id) = task {
        tree::find(&p.tasks, id).with_context(|| format!("no task {id:?}"))?;
    }
    log(p, now, task, LogKind::Note, text.into());
    Ok(format!("{}: note", task.unwrap_or(&p.key)))
}

/// Newest entries go first, matching how the files are written by hand.
fn log(p: &mut Project, at: NaiveDateTime, task: Option<&str>, kind: LogKind, text: String) {
    p.log.insert(0, LogEntry { at, task: task.map(String::from), kind, text });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> NaiveDateTime {
        "2026-11-07T10:30:00".parse().unwrap()
    }

    fn project() -> Project {
        serde_json::from_str(
            r#"{"name":"P","key":"P","tasks":[
                {"id":"P-1","title":"Phase","status":"done","subtasks":[{"id":"P-2","title":"a","status":"done"}]},
                {"id":"P-7","title":"b","start":"2026-11-01","days":3}
            ],"log":[{"at":"2026-11-01","task":"P-2","text":"hi"}]}"#,
        )
        .unwrap()
    }

    fn patch(json: &str) -> TaskPatch {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn status_change_sets_done_on_and_logs() {
        let mut p = project();
        update(&mut p, "P-7", patch(r#"{"status":"done"}"#), now()).unwrap();
        let t = tree::find(&p.tasks, "P-7").unwrap();
        assert_eq!(t.done_on.unwrap().to_string(), "2026-11-07");
        assert_eq!(p.log[0].text, "completed b");
        update(&mut p, "P-7", patch(r#"{"status":"doing"}"#), now()).unwrap();
        assert!(tree::find(&p.tasks, "P-7").unwrap().done_on.is_none());
    }

    #[test]
    fn null_clears_and_missing_keeps() {
        let mut p = project();
        update(&mut p, "P-7", patch(r#"{"due":"2026-11-09"}"#), now()).unwrap();
        let t = tree::find(&p.tasks, "P-7").unwrap();
        assert_eq!((t.start.is_some(), t.due.is_some(), t.days), (true, true, None));
        update(&mut p, "P-7", patch(r#"{"start":null,"due":null}"#), now()).unwrap();
        let t = tree::find(&p.tasks, "P-7").unwrap();
        assert!(t.start.is_none() && t.due.is_none());
    }

    #[test]
    fn create_picks_next_id_and_reopens_done_parent() {
        let mut p = project();
        let (id, _) = create(&mut p, NewTask { parent: Some("P-1".into()), title: "c".into(), ..Default::default() }, now()).unwrap();
        assert_eq!(id, "P-8");
        assert_eq!(tree::find(&p.tasks, "P-1").unwrap().status, Status::Doing);
        assert!(create(&mut p, NewTask { title: "  ".into(), ..Default::default() }, now()).is_err());
    }

    #[test]
    fn delete_removes_subtree_and_unlinks_log() {
        let mut p = project();
        let s = delete(&mut p, "P-1", now()).unwrap();
        assert!(s.contains("1 subtasks"));
        assert!(tree::find(&p.tasks, "P-2").is_none());
        assert!(p.log.iter().all(|l| l.task.is_none()));
    }

    #[test]
    fn comment_requires_existing_task() {
        let mut p = project();
        assert!(comment(&mut p, Some("P-99"), "x", now()).is_err());
        comment(&mut p, Some("P-7"), " looks good ", now()).unwrap();
        assert_eq!(p.log[0].text, "looks good");
    }
}
