//! Background publishing: batch edits, then commit `data/` and push.
//!
//! Each saved edit sends a one-line summary here. Once no edit has arrived for
//! the quiet period, everything since the last publish becomes one commit,
//! which is pushed so the GitHub Pages workflow rebuilds the public page.

use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use anyhow::{Result, bail};

pub struct Publisher {
    tx: Sender<String>,
}

impl Publisher {
    pub fn notify(&self, summary: String) {
        let _ = self.tx.send(summary);
    }
}

pub fn spawn(data: PathBuf, quiet: Duration) -> Result<Publisher> {
    if git(&data, &["rev-parse", "--is-inside-work-tree"]).is_err() {
        bail!("--publish needs {} to be inside a git repository", data.display());
    }
    let push = git(&data, &["remote"]).is_ok_and(|r| !r.trim().is_empty());
    if !push {
        eprintln!("note: no git remote yet; edits will be committed locally but not pushed");
    }

    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || {
        while let Ok(first) = rx.recv() {
            let mut batch = vec![first];
            loop {
                match rx.recv_timeout(quiet) {
                    Ok(s) => batch.push(s),
                    Err(RecvTimeoutError::Timeout) => break,
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
            match publish(&data, &batch, push) {
                Ok(Some(msg)) => println!("published: {msg}"),
                Ok(None) => {}
                Err(e) => eprintln!("publish failed (will retry with the next edit): {e:#}"),
            }
        }
    });
    Ok(Publisher { tx })
}

/// Commit only the data directory; leaves anything else you have staged alone.
fn publish(data: &PathBuf, batch: &[String], push: bool) -> Result<Option<String>> {
    git(data, &["add", "--", "."])?;
    // `diff --quiet` exits non-zero when there are staged changes.
    if git(data, &["diff", "--cached", "--quiet", "--", "."]).is_ok() {
        return Ok(None);
    }
    let subject = match batch {
        [one] => format!("tracker: {one}"),
        many => format!("tracker: {} updates", many.len()),
    };
    let body: String = batch.iter().map(|s| format!("- {s}\n")).collect();
    git(data, &["commit", "-m", &subject, "-m", &body, "--", "."])?;
    if push {
        git(data, &["push"])?;
    }
    Ok(Some(subject))
}

fn git(dir: &PathBuf, args: &[&str]) -> Result<String> {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output()?;
    if !out.status.success() {
        bail!("git {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}
