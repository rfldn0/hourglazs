//! A task as written in a project JSON file.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize, Serializer};

use super::is_default;

/// Workflow column a task sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Backlog,
    #[default]
    Todo,
    Doing,
    Done,
}

impl Status {
    /// Board column order.
    pub const ALL: [Status; 4] = [Status::Backlog, Status::Todo, Status::Doing, Status::Done];

    pub fn key(self) -> &'static str {
        match self {
            Status::Backlog => "backlog",
            Status::Todo => "todo",
            Status::Doing => "doing",
            Status::Done => "done",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Status::Backlog => "Backlog",
            Status::Todo => "To do",
            Status::Doing => "In progress",
            Status::Done => "Done",
        }
    }

    /// CSS colour token for the status dot.
    pub fn color(self) -> &'static str {
        match self {
            Status::Backlog => "var(--faint)",
            Status::Todo => "var(--dusk)",
            Status::Doing => "var(--amber)",
            Status::Done => "var(--green)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    High,
    #[default]
    #[serde(alias = "medium")]
    Med,
    Low,
}

impl Priority {
    pub const ALL: [Priority; 3] = [Priority::High, Priority::Med, Priority::Low];

    pub fn key(self) -> &'static str {
        match self {
            Priority::High => "high",
            Priority::Med => "med",
            Priority::Low => "low",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Priority::High => "High",
            Priority::Med => "Medium",
            Priority::Low => "Low",
        }
    }

    pub fn color(self) -> &'static str {
        match self {
            Priority::High => "var(--red)",
            Priority::Med => "var(--amber)",
            Priority::Low => "var(--dim)",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            Priority::High => "▲▲",
            Priority::Med => "▲",
            Priority::Low => "–",
        }
    }
}

/// One task. Subtasks nest to any depth.
///
/// Scheduling accepts any of: `start` + `due`, `start` + `days`, `due` + `days`,
/// `start` alone, or `due` alone. A task with no dates inherits the span of its
/// subtasks.
///
/// Field order here is the order fields are written back to disk.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub status: Status,
    #[serde(default, skip_serializing_if = "is_default")]
    pub priority: Priority,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due: Option<NaiveDate>,
    /// Duration in calendar days (inclusive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub days: Option<u32>,
    /// Effort estimate in hours.
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "ser_hours")]
    pub hours: Option<f32>,
    /// When the task was finished; set automatically by `hourglazs serve`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_on: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub milestone: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subtasks: Vec<Task>,
}

impl Task {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            status: Status::default(),
            priority: Priority::default(),
            start: None,
            due: None,
            days: None,
            hours: None,
            done_on: None,
            milestone: false,
            description: String::new(),
            tags: Vec::new(),
            subtasks: Vec::new(),
        }
    }
}

/// Write `3` rather than `3.0` for whole hours.
fn ser_hours<S: Serializer>(h: &Option<f32>, s: S) -> Result<S::Ok, S::Error> {
    match h {
        Some(h) if h.fract() == 0.0 && *h >= 0.0 => s.serialize_u64(*h as u64),
        Some(h) => s.serialize_f64(f64::from(*h)),
        None => s.serialize_none(),
    }
}
