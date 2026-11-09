//! Everything the renderer needs: config, clock and indexed projects.

use crate::data::Loaded;
use crate::model::LogEntry;

use super::project::{Node, ProjectIndex};
use super::schedule::Clock;

#[derive(Debug, Clone)]
pub struct Site {
    pub title: String,
    pub owner: String,
    pub clock: Clock,
    pub projects: Vec<ProjectIndex>,
}

/// A task together with the project it lives in.
#[derive(Debug, Clone, Copy)]
pub struct TaskRef<'a> {
    pub project: &'a ProjectIndex,
    pub idx: usize,
}

impl<'a> TaskRef<'a> {
    pub fn node(&self) -> &'a Node {
        &self.project.nodes[self.idx]
    }
}

impl Site {
    /// `today` overrides both the system clock and the config's `today`.
    pub fn new(loaded: Loaded, today: Option<chrono::NaiveDate>) -> Self {
        let clock = today.or(loaded.config.today).map_or_else(Clock::system, Clock::fixed);
        Self {
            title: loaded.config.title,
            owner: loaded.config.owner,
            clock,
            projects: loaded
                .projects
                .into_iter()
                .map(|(slug, p)| ProjectIndex::new(slug, p))
                .collect(),
        }
    }

    pub fn tasks(&self) -> impl Iterator<Item = TaskRef<'_>> {
        self.projects
            .iter()
            .flat_map(|p| (0..p.nodes.len()).map(move |idx| TaskRef { project: p, idx }))
    }

    /// Open leaf tasks: the things that can actually be worked on.
    pub fn open_work(&self) -> Vec<TaskRef<'_>> {
        self.tasks()
            .filter(|t| {
                let n = t.node();
                n.is_leaf() && !n.is_done() && !n.task.milestone
            })
            .collect()
    }

    /// Log entries across all projects, newest first.
    pub fn activity(&self, limit: usize) -> Vec<(&ProjectIndex, &LogEntry)> {
        let mut all: Vec<_> = self
            .projects
            .iter()
            .flat_map(|p| p.log.iter().map(move |l| (p, l)))
            .collect();
        all.sort_by_key(|(_, l)| std::cmp::Reverse(l.at));
        all.truncate(limit);
        all
    }
}
