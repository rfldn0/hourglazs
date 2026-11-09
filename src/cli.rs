//! Command-line interface.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use chrono::NaiveDate;

use crate::data::{self, Report};
use crate::domain::Site;
use crate::{render, serve};

pub const USAGE: &str = "\
hourglazs — personal project tracker

USAGE:
    hourglazs serve [--data DIR] [--port N] [--publish [--quiet SECS]]
    hourglazs build [--data DIR] [--out DIR] [--today YYYY-MM-DD]
    hourglazs check [--data DIR]

COMMANDS:
    serve   Run the full app locally with editing (saves to the JSON files)
    build   Render the public progress page into --out (what GitHub Pages hosts)
    check   Validate the JSON data only

OPTIONS:
    --data DIR           Data directory                         [default: data]
    --out DIR            Output directory for build             [default: dist]
    --today YYYY-MM-DD   Pretend today is this date             [default: system date]
    --port N             Port for serve                         [default: 4000]
    --publish            While serving, commit + push data/ after edits
    --quiet SECS         Seconds without edits before publishing [default: 60]
";

#[derive(Debug, PartialEq)]
pub enum Command {
    Serve { data: PathBuf, port: u16, publish: Option<Duration> },
    Build { data: PathBuf, out: PathBuf, today: Option<NaiveDate> },
    Check { data: PathBuf },
    Help,
}

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Command> {
    let mut args = args.into_iter();
    let cmd = args.next().unwrap_or_else(|| "help".into());
    let mut data = PathBuf::from("data");
    let mut out = PathBuf::from("dist");
    let mut today = None;
    let mut port = 4000;
    let mut publish = false;
    let mut quiet = 60;

    while let Some(flag) = args.next() {
        let mut value = || args.next().with_context(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--data" => data = value()?.into(),
            "--out" => out = value()?.into(),
            "--today" => {
                let v = value()?;
                today = Some(v.parse().with_context(|| format!("--today {v:?} is not YYYY-MM-DD"))?);
            }
            "--port" => {
                let v = value()?;
                port = v.parse().with_context(|| format!("--port {v:?} is not a port number"))?;
            }
            "--publish" => publish = true,
            "--quiet" => {
                let v = value()?;
                quiet = v.parse().with_context(|| format!("--quiet {v:?} is not a number of seconds"))?;
            }
            "-h" | "--help" => return Ok(Command::Help),
            other => bail!("unknown option {other:?}\n\n{USAGE}"),
        }
    }

    match cmd.as_str() {
        "serve" => Ok(Command::Serve { data, port, publish: publish.then(|| Duration::from_secs(quiet)) }),
        "build" => Ok(Command::Build { data, out, today }),
        "check" => Ok(Command::Check { data }),
        "help" | "-h" | "--help" => Ok(Command::Help),
        other => bail!("unknown command {other:?}\n\n{USAGE}"),
    }
}

pub fn run(cmd: Command) -> Result<()> {
    match cmd {
        Command::Help => print!("{USAGE}"),
        Command::Check { data } => {
            let loaded = data::load(&data)?;
            report(&data::validate(&loaded))?;
            let tasks: usize = loaded.projects.iter().map(|(_, p)| count(&p.tasks)).sum();
            println!("ok: {} projects, {tasks} tasks", loaded.projects.len());
        }
        Command::Build { data, out, today } => {
            let loaded = data::load(&data)?;
            report(&data::validate(&loaded))?;
            let site = Site::new(loaded, today);
            let pages = render::write_public(&site, &out)?;
            println!("built {pages} public pages into {} (today = {})", out.display(), site.clock.today);
        }
        Command::Serve { data, port, publish } => serve::run(serve::Options { data, port, publish })?,
    }
    Ok(())
}

fn count(tasks: &[crate::model::Task]) -> usize {
    tasks.iter().map(|t| 1 + count(&t.subtasks)).sum()
}

fn report(r: &Report) -> Result<()> {
    for w in &r.warnings {
        eprintln!("warning: {w}");
    }
    for e in &r.errors {
        eprintln!("error: {e}");
    }
    if !r.is_ok() {
        bail!("{} problem(s) in the data; nothing was built", r.errors.len());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn parses_commands_and_flags() {
        assert_eq!(
            parse(args("build --data d --out o --today 2026-11-07")).unwrap(),
            Command::Build { data: "d".into(), out: "o".into(), today: Some("2026-11-07".parse().unwrap()) }
        );
        assert_eq!(
            parse(args("serve --port 5000 --publish --quiet 30")).unwrap(),
            Command::Serve { data: "data".into(), port: 5000, publish: Some(Duration::from_secs(30)) }
        );
        assert_eq!(parse(args("serve")).unwrap(), Command::Serve { data: "data".into(), port: 4000, publish: None });
        assert_eq!(parse(args("check")).unwrap(), Command::Check { data: "data".into() });
        assert_eq!(parse(args("")).unwrap(), Command::Help);
        assert!(parse(args("build --today tomorrow")).is_err());
        assert!(parse(args("deploy")).is_err());
    }
}
