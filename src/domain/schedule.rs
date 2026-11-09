//! Date spans, "today", and date formatting.

use chrono::{DateTime, Datelike, Days, Local, NaiveDate, NaiveDateTime, Utc};

use crate::model::Task;

/// Inclusive date range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl Span {
    pub fn new(start: NaiveDate, end: NaiveDate) -> Self {
        if start <= end { Self { start, end } } else { Self { start: end, end: start } }
    }

    pub fn day(d: NaiveDate) -> Self {
        Self { start: d, end: d }
    }

    /// Number of calendar days covered (at least 1).
    pub fn days(&self) -> i64 {
        (self.end - self.start).num_days() + 1
    }

    pub fn union(self, o: Span) -> Span {
        Span { start: self.start.min(o.start), end: self.end.max(o.end) }
    }

    pub fn contains(&self, d: NaiveDate) -> bool {
        self.start <= d && d <= self.end
    }
}

/// The span a task states itself, ignoring subtasks.
pub fn explicit_span(t: &Task) -> Option<Span> {
    let len = |n: u32| Days::new(u64::from(n.max(1) - 1));
    match (t.start, t.due, t.days) {
        (Some(s), Some(d), _) => Some(Span::new(s, d)),
        (Some(s), None, Some(n)) => Some(Span::new(s, s + len(n))),
        (Some(s), None, None) => Some(Span::day(s)),
        (None, Some(d), Some(n)) => Some(Span::new(d - len(n), d)),
        (None, Some(d), None) => Some(Span::day(d)),
        (None, None, _) => None,
    }
}

/// What "now" means for this build.
#[derive(Debug, Clone, Copy)]
pub struct Clock {
    pub today: NaiveDate,
    pub now: NaiveDateTime,
    /// The real instant of the build (absent with a fixed date), so pages
    /// can say "updated 2h ago" in the reader's own time zone.
    pub built: Option<DateTime<Utc>>,
}

impl Clock {
    pub fn system() -> Self {
        let at = Local::now();
        let now = at.naive_local();
        Self { today: now.date(), now, built: Some(at.with_timezone(&Utc)) }
    }

    pub fn fixed(today: NaiveDate) -> Self {
        Self { today, now: today.and_hms_opt(12, 0, 0).expect("noon is valid"), built: None }
    }
}

/// `Nov 7`
pub fn short(d: NaiveDate) -> String {
    d.format("%b %-d").to_string()
}

/// `Saturday, November 7`
pub fn long(d: NaiveDate) -> String {
    d.format("%A, %B %-d").to_string()
}

/// `Overdue` / `Today` / `Tomorrow` / `Nov 9`
pub fn due_label(due: NaiveDate, today: NaiveDate) -> String {
    match (due - today).num_days() {
        d if d < 0 => "Overdue".into(),
        0 => "Today".into(),
        1 => "Tomorrow".into(),
        _ => short(due),
    }
}

/// `today` / `tomorrow` / `in 3 days` / `yesterday` / `5 days ago`
pub fn relative_days(d: NaiveDate, today: NaiveDate) -> String {
    match (d - today).num_days() {
        0 => "today".into(),
        1 => "tomorrow".into(),
        -1 => "yesterday".into(),
        n if n > 1 => format!("in {n} days"),
        n => format!("{} days ago", -n),
    }
}

pub fn monday_on_or_before(d: NaiveDate) -> NaiveDate {
    d - Days::new(u64::from(d.weekday().num_days_from_monday()))
}

pub fn first_of_month(d: NaiveDate) -> NaiveDate {
    d.with_day(1).expect("day 1 exists")
}

pub fn next_month(d: NaiveDate) -> NaiveDate {
    let d = first_of_month(d);
    if d.month() == 12 {
        NaiveDate::from_ymd_opt(d.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(d.year(), d.month() + 1, 1)
    }
    .expect("first of month exists")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    /// Span of a task whose JSON has `fields` besides id/title.
    fn s(fields: &str) -> Option<Span> {
        let sep = if fields.is_empty() { "" } else { "," };
        let t: Task = serde_json::from_str(&format!(r#"{{"id":"A","title":"a"{sep}{fields}}}"#)).unwrap();
        explicit_span(&t)
    }

    #[test]
    fn spans_from_every_field_combination() {
        let nov2_4 = Some(Span::new(d("2026-11-02"), d("2026-11-04")));
        assert_eq!(s(r#""start":"2026-11-02","days":3"#), nov2_4);
        assert_eq!(s(r#""due":"2026-11-04","days":3"#), nov2_4);
        assert_eq!(s(r#""start":"2026-11-02","due":"2026-11-09""#).unwrap().days(), 8);
        assert_eq!(s(r#""due":"2026-11-09""#), Some(Span::day(d("2026-11-09"))));
        assert_eq!(s(""), None);
    }

    #[test]
    fn labels_and_calendar_helpers() {
        let today = d("2026-11-07");
        assert_eq!(due_label(d("2026-11-06"), today), "Overdue");
        assert_eq!(due_label(today, today), "Today");
        assert_eq!(due_label(d("2026-11-08"), today), "Tomorrow");
        assert_eq!(due_label(d("2026-11-20"), today), "Nov 20");
        assert_eq!(relative_days(d("2026-11-10"), today), "in 3 days");
        assert_eq!(relative_days(d("2026-11-08"), today), "tomorrow");
        assert_eq!(relative_days(d("2026-11-04"), today), "3 days ago");
        assert_eq!(monday_on_or_before(today), d("2026-11-02"));
        assert_eq!(next_month(d("2026-12-15")), d("2027-01-01"));
    }
}
