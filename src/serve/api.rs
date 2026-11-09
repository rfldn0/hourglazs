//! `POST /api/<op>`: apply one edit, validate everything, save the project file.

use std::path::Path;

use anyhow::{Context, Result, bail};
use chrono::{Local, NaiveDateTime, Timelike};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::data;
use crate::edit::{self, NewTask, Summary, TaskPatch};
use crate::model::{Priority, Status};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateReq {
    project: String,
    id: String,
    patch: TaskPatch,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateReq {
    project: String,
    #[serde(default)]
    parent: Option<String>,
    title: String,
    #[serde(default)]
    status: Option<Status>,
    #[serde(default)]
    priority: Option<Priority>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeleteReq {
    project: String,
    id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommentReq {
    project: String,
    #[serde(default)]
    task: Option<String>,
    text: String,
}

/// Returns the JSON reply and a one-line summary of what changed.
pub fn handle(data_dir: &Path, op: &str, body: &str) -> Result<(Value, Summary)> {
    let mut loaded = data::load(data_dir)?;
    let now = now();

    let (slug, reply, summary) = match op {
        "update" => {
            let r: UpdateReq = parse(body)?;
            let s = edit::update(project(&mut loaded, &r.project)?, &r.id, r.patch, now)?;
            (r.project, json!({}), s)
        }
        "create" => {
            let r: CreateReq = parse(body)?;
            let new = NewTask { parent: r.parent, title: r.title, status: r.status, priority: r.priority, tags: r.tags };
            let (id, s) = edit::create(project(&mut loaded, &r.project)?, new, now)?;
            (r.project, json!({ "id": id }), s)
        }
        "delete" => {
            let r: DeleteReq = parse(body)?;
            let s = edit::delete(project(&mut loaded, &r.project)?, &r.id, now)?;
            (r.project, json!({}), s)
        }
        "comment" => {
            let r: CommentReq = parse(body)?;
            let s = edit::comment(project(&mut loaded, &r.project)?, r.task.as_deref(), &r.text, now)?;
            (r.project, json!({}), s)
        }
        other => bail!("unknown operation {other:?}"),
    };

    // Never write a file that `check` would reject.
    let report = data::validate(&loaded);
    if !report.is_ok() {
        bail!("{}", report.errors.join("; "));
    }
    let (_, p) = loaded.projects.iter().find(|(s, _)| *s == slug).expect("project was found above");
    data::save_project(data_dir, &slug, p)?;
    Ok((reply, format!("{slug} {summary}")))
}

fn parse<T: DeserializeOwned>(body: &str) -> Result<T> {
    serde_json::from_str(body).context("bad request body")
}

fn project<'a>(loaded: &'a mut data::Loaded, slug: &str) -> Result<&'a mut crate::model::Project> {
    loaded
        .projects
        .iter_mut()
        .find(|(s, _)| s == slug)
        .map(|(_, p)| p)
        .with_context(|| format!("no project {slug:?}"))
}

/// Local time to the minute (that's what the log stores).
fn now() -> NaiveDateTime {
    let t = Local::now().naive_local();
    t.with_second(0).and_then(|t| t.with_nanosecond(0)).unwrap_or(t)
}
