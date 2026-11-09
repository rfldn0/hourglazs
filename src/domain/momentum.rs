//! Momentum: how consistently work is happening.
//!
//! An *active day* is any day with at least one action: a log entry (note,
//! completion, status change, new task) or a task whose `done_on` is that day.
//! From those days come a streak, a weekly comparison, a heatmap and
//! per-project "quiet for N days" signals.

use std::collections::{BTreeMap, HashSet};

use chrono::{Days, NaiveDate};

use super::project::ProjectIndex;

/// A project is "quiet" once this many days pass without activity.
pub const QUIET_AFTER: i64 = 7;

/// Actions per day, for the given projects.
pub fn activity(projects: &[&ProjectIndex]) -> BTreeMap<NaiveDate, u32> {
    let mut days = BTreeMap::new();
    for p in projects {
        let mut logged: HashSet<(&str, NaiveDate)> = HashSet::new();
        for l in &p.log {
            *days.entry(l.at.date()).or_insert(0) += 1;
            if let Some(t) = &l.task {
                logged.insert((t.as_str(), l.at.date()));
            }
        }
        // Completions entered by hand have no log entry; count them once.
        for n in &p.nodes {
            if let Some(d) = n.task.done_on
                && !logged.contains(&(n.task.id.as_str(), d))
            {
                *days.entry(d).or_insert(0) += 1;
            }
        }
    }
    days
}

/// The most recent active day.
pub fn last_active(project: &ProjectIndex) -> Option<NaiveDate> {
    activity(&[project]).keys().next_back().copied()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Momentum {
    /// Consecutive active days ending today, or yesterday if today has
    /// nothing yet (the streak is still alive until the day ends).
    pub streak: u32,
    pub longest: u32,
    pub active_today: bool,
    pub last_active: Option<NaiveDate>,
    /// Actions in the 7 days ending today, and the 7 before that.
    pub this_week: u32,
    pub last_week: u32,
}

impl Momentum {
    pub fn new(days: &BTreeMap<NaiveDate, u32>, today: NaiveDate) -> Self {
        let active = |d: NaiveDate| days.get(&d).is_some_and(|&n| n > 0);
        let active_today = active(today);

        let mut streak = 0;
        let mut d = if active_today { today } else { today - Days::new(1) };
        while active(d) {
            streak += 1;
            d = d - Days::new(1);
        }

        let (mut longest, mut run, mut prev): (u32, u32, Option<NaiveDate>) = (0, 0, None);
        for (&day, _) in days.iter().filter(|(d, n)| **n > 0 && **d <= today) {
            run = if prev.is_some_and(|p| (day - p).num_days() == 1) { run + 1 } else { 1 };
            longest = longest.max(run);
            prev = Some(day);
        }

        let sum = |from: i64, to: i64| -> u32 {
            days.iter()
                .filter(|(d, _)| (from..to).contains(&(today - **d).num_days()))
                .map(|(_, n)| n)
                .sum()
        };

        Self {
            streak,
            longest,
            active_today,
            last_active: days.iter().rev().find(|(d, n)| **n > 0 && **d <= today).map(|(d, _)| *d),
            this_week: sum(0, 7),
            last_week: sum(7, 14),
        }
    }

    /// Days since the last action (0 = today).
    pub fn quiet_days(&self, today: NaiveDate) -> Option<i64> {
        self.last_active.map(|d| (today - d).num_days())
    }

    /// Yesterday counted but today doesn't yet.
    pub fn at_risk(&self) -> bool {
        self.streak > 0 && !self.active_today
    }
}

/// Heat level 0–4 for a day's action count.
pub fn level(n: u32) -> u8 {
    match n {
        0 => 0,
        1 => 1,
        2..=3 => 2,
        4..=6 => 3,
        _ => 4,
    }
}

/// `weeks` full weeks (Monday-first columns) ending with the week of `today`.
/// Days after today are `None`.
pub fn heatmap(days: &BTreeMap<NaiveDate, u32>, today: NaiveDate, weeks: u64) -> Vec<[Option<(NaiveDate, u32)>; 7]> {
    let start = super::schedule::monday_on_or_before(today) - Days::new(7 * (weeks - 1));
    (0..weeks)
        .map(|w| {
            std::array::from_fn(|i| {
                let d = start + Days::new(w * 7 + i as u64);
                (d <= today).then(|| (d, days.get(&d).copied().unwrap_or(0)))
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    fn days(list: &[(&str, u32)]) -> BTreeMap<NaiveDate, u32> {
        list.iter().map(|(s, n)| (d(s), *n)).collect()
    }

    #[test]
    fn streak_survives_until_the_day_ends() {
        let a = days(&[("2026-11-01", 1), ("2026-11-02", 1), ("2026-11-05", 2), ("2026-11-06", 1), ("2026-11-07", 3)]);
        let m = Momentum::new(&a, d("2026-11-07"));
        assert_eq!((m.streak, m.longest, m.active_today, m.at_risk()), (3, 3, true, false));

        let m = Momentum::new(&a, d("2026-11-08"));
        assert_eq!((m.streak, m.at_risk(), m.quiet_days(d("2026-11-08"))), (3, true, Some(1)));

        let m = Momentum::new(&a, d("2026-11-09"));
        assert_eq!((m.streak, m.longest, m.at_risk()), (0, 3, false));
    }

    #[test]
    fn weeks_and_heatmap() {
        let a = days(&[("2026-10-28", 4), ("2026-11-02", 1), ("2026-11-07", 2)]);
        let m = Momentum::new(&a, d("2026-11-07"));
        assert_eq!((m.this_week, m.last_week), (3, 4));

        let h = heatmap(&a, d("2026-11-07"), 2);
        assert_eq!(h.len(), 2);
        assert_eq!(h[0][0], Some((d("2026-10-26"), 0)));
        assert_eq!(h[1][0], Some((d("2026-11-02"), 1)));
        assert_eq!(h[1][5], Some((d("2026-11-07"), 2)));
        assert_eq!(h[1][6], None);
        assert_eq!((level(0), level(1), level(3), level(9)), (0, 1, 2, 4));
    }
}
