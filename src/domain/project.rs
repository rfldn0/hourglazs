//! A project with its nested tasks flattened into an indexed tree.

use std::collections::HashMap;

use chrono::NaiveDate;

use crate::model::{Accent, LogEntry, Project, Status, Task};

use super::schedule::{Span, explicit_span};

/// One task in the flattened tree. `task.subtasks` is always empty here;
/// use `children` instead.
#[derive(Debug, Clone)]
pub struct Node {
    pub task: Task,
    pub parent: Option<usize>,
    pub depth: usize,
    pub children: Vec<usize>,
    /// Own dates, or the union of the subtasks' spans.
    pub span: Option<Span>,
}

impl Node {
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    pub fn is_done(&self) -> bool {
        self.task.status == Status::Done
    }

    pub fn due(&self) -> Option<NaiveDate> {
        self.span.map(|s| if self.task.milestone { s.start } else { s.end })
    }

    pub fn is_late(&self, today: NaiveDate) -> bool {
        !self.is_done() && self.due().is_some_and(|d| d < today)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
}

impl Progress {
    pub fn pct(&self) -> u8 {
        (self.done * 100).checked_div(self.total).unwrap_or(0) as u8
    }
}

#[derive(Debug, Clone)]
pub struct ProjectIndex {
    pub slug: String,
    pub name: String,
    pub key: String,
    pub accent: Accent,
    pub description: String,
    /// Shown on the public progress page.
    pub public: bool,
    /// Tasks in depth-first (file) order.
    pub nodes: Vec<Node>,
    pub roots: Vec<usize>,
    /// Newest first.
    pub log: Vec<LogEntry>,
    by_id: HashMap<String, usize>,
}

impl ProjectIndex {
    pub fn new(slug: String, p: Project) -> Self {
        let mut ix = Self {
            slug,
            name: p.name,
            key: p.key,
            accent: p.color,
            description: p.description,
            public: p.public,
            nodes: Vec::new(),
            roots: Vec::new(),
            log: p.log,
            by_id: HashMap::new(),
        };
        for t in p.tasks {
            let i = ix.insert(t, None, 0);
            ix.roots.push(i);
        }
        // Children always come after their parent, so a reverse sweep sees
        // every child's span before the parent needs it.
        for i in (0..ix.nodes.len()).rev() {
            let own = explicit_span(&ix.nodes[i].task);
            let derived = ix.nodes[i]
                .children
                .iter()
                .filter_map(|&c| ix.nodes[c].span)
                .reduce(Span::union);
            ix.nodes[i].span = own.or(derived);
        }
        ix.log.sort_by_key(|l| std::cmp::Reverse(l.at));
        ix
    }

    fn insert(&mut self, mut task: Task, parent: Option<usize>, depth: usize) -> usize {
        let subs = std::mem::take(&mut task.subtasks);
        let i = self.nodes.len();
        self.by_id.entry(task.id.clone()).or_insert(i);
        self.nodes.push(Node { task, parent, depth, children: Vec::new(), span: None });
        for s in subs {
            let c = self.insert(s, Some(i), depth + 1);
            self.nodes[i].children.push(c);
        }
        i
    }

    pub fn get(&self, id: &str) -> Option<usize> {
        self.by_id.get(id).copied()
    }

    pub fn node(&self, i: usize) -> &Node {
        &self.nodes[i]
    }

    /// Ancestors of `i`, root first.
    pub fn path(&self, i: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut cur = self.nodes[i].parent;
        while let Some(p) = cur {
            out.push(p);
            cur = self.nodes[p].parent;
        }
        out.reverse();
        out
    }

    /// `i` and everything beneath it, in display order.
    pub fn subtree(&self, i: usize) -> Vec<usize> {
        let mut out = vec![i];
        for &c in &self.nodes[i].children {
            out.extend(self.subtree(c));
        }
        out
    }

    /// Leaf tasks under `i` (not counting milestones).
    pub fn leaves(&self, i: usize) -> Vec<usize> {
        self.subtree(i)
            .into_iter()
            .skip(1)
            .filter(|&j| self.nodes[j].is_leaf() && !self.nodes[j].task.milestone)
            .collect()
    }

    /// Done/total over leaf subtasks; `None` for a task without subtasks.
    pub fn progress(&self, i: usize) -> Option<Progress> {
        let leaves = self.leaves(i);
        if leaves.is_empty() {
            return None;
        }
        let done = leaves.iter().filter(|&&j| self.nodes[j].is_done()).count();
        Some(Progress { done, total: leaves.len() })
    }

    /// Percent complete: subtask progress, or 0/100 for a leaf.
    pub fn completion(&self, i: usize) -> u8 {
        match self.progress(i) {
            Some(p) => p.pct(),
            None if self.nodes[i].is_done() => 100,
            None => 0,
        }
    }

    /// Progress over every leaf task in the project.
    pub fn overall(&self) -> Progress {
        let leaves: Vec<_> = self
            .nodes
            .iter()
            .filter(|n| n.is_leaf() && !n.task.milestone)
            .collect();
        Progress { done: leaves.iter().filter(|n| n.is_done()).count(), total: leaves.len() }
    }

    pub fn open_count(&self) -> usize {
        let p = self.overall();
        p.total - p.done
    }

    /// Union of every scheduled task's span.
    pub fn span(&self) -> Option<Span> {
        self.roots.iter().filter_map(|&r| self.nodes[r].span).reduce(Span::union)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ix() -> ProjectIndex {
        let p: Project = serde_json::from_str(
            r#"{"name":"P","key":"P","tasks":[
                {"id":"P-1","title":"Phase","subtasks":[
                    {"id":"P-2","title":"a","status":"done","start":"2026-11-02","days":2},
                    {"id":"P-3","title":"b","subtasks":[
                        {"id":"P-4","title":"c","status":"done","start":"2026-11-05","due":"2026-11-06"},
                        {"id":"P-5","title":"d","start":"2026-11-09","days":3}
                    ]}
                ]},
                {"id":"P-6","title":"Launch","milestone":true,"due":"2026-11-20"}
            ]}"#,
        )
        .unwrap();
        ProjectIndex::new("p".into(), p)
    }

    #[test]
    fn flattens_in_file_order_with_depths() {
        let ix = ix();
        let ids: Vec<_> = ix.nodes.iter().map(|n| (n.task.id.as_str(), n.depth)).collect();
        assert_eq!(ids, [("P-1", 0), ("P-2", 1), ("P-3", 1), ("P-4", 2), ("P-5", 2), ("P-6", 0)]);
        assert_eq!(ix.roots, [0, 5]);
        assert_eq!(ix.path(ix.get("P-5").unwrap()), [0, 2]);
    }

    #[test]
    fn progress_counts_leaves_only() {
        let ix = ix();
        assert_eq!(ix.progress(0), Some(Progress { done: 2, total: 3 }));
        assert_eq!(ix.progress(2), Some(Progress { done: 1, total: 2 }));
        assert_eq!(ix.progress(1), None);
        assert_eq!(ix.completion(1), 100);
        // Milestone is excluded from the totals.
        assert_eq!(ix.overall(), Progress { done: 2, total: 3 });
    }

    #[test]
    fn parent_span_is_derived_from_children() {
        let ix = ix();
        let s = ix.nodes[0].span.unwrap();
        assert_eq!(s.start.to_string(), "2026-11-02");
        assert_eq!(s.end.to_string(), "2026-11-11");
        assert_eq!(ix.nodes[5].due().unwrap().to_string(), "2026-11-20");
    }
}
