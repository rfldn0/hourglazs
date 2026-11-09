//! Helpers for the nested task tree as stored in JSON.

use crate::model::Task;

pub fn find<'a>(tasks: &'a [Task], id: &str) -> Option<&'a Task> {
    tasks.iter().find_map(|t| if t.id == id { Some(t) } else { find(&t.subtasks, id) })
}

pub fn find_mut<'a>(tasks: &'a mut [Task], id: &str) -> Option<&'a mut Task> {
    for t in tasks {
        if t.id == id {
            return Some(t);
        }
        if let Some(found) = find_mut(&mut t.subtasks, id) {
            return Some(found);
        }
    }
    None
}

/// Detach a task (with its subtasks) from wherever it sits.
pub fn remove(tasks: &mut Vec<Task>, id: &str) -> Option<Task> {
    if let Some(i) = tasks.iter().position(|t| t.id == id) {
        return Some(tasks.remove(i));
    }
    tasks.iter_mut().find_map(|t| remove(&mut t.subtasks, id))
}

/// Every ID in a subtree, the root included.
pub fn ids(t: &Task) -> Vec<String> {
    let mut out = vec![t.id.clone()];
    for s in &t.subtasks {
        out.extend(ids(s));
    }
    out
}

/// `KEY-n` with `n` one above the highest number already used.
pub fn next_id(key: &str, tasks: &[Task]) -> String {
    let prefix = format!("{key}-");
    let max = tasks
        .iter()
        .flat_map(ids)
        .filter_map(|id| id.strip_prefix(&prefix)?.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    format!("{prefix}{}", max + 1)
}
