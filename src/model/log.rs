//! Activity log entries: a lightweight dev journal per project.

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

use super::is_default;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogKind {
    /// Free-form comment ("Fixed the focus order, testing VoiceOver next").
    #[default]
    Note,
    /// Something that happened ("completed Wireframes").
    Event,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LogEntry {
    /// `YYYY-MM-DD`, `YYYY-MM-DDTHH:MM` or `YYYY-MM-DDTHH:MM:SS`.
    #[serde(deserialize_with = "de_when", serialize_with = "ser_when")]
    pub at: NaiveDateTime,
    /// Task ID this entry belongs to, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub kind: LogKind,
    pub text: String,
}

fn de_when<'de, D: Deserializer<'de>>(d: D) -> Result<NaiveDateTime, D::Error> {
    let s = String::deserialize(d)?;
    parse_when(&s).ok_or_else(|| {
        D::Error::custom(format!(
            "invalid timestamp {s:?}; use YYYY-MM-DD or YYYY-MM-DDTHH:MM"
        ))
    })
}

/// Date-only entries stay date-only; everything else is written to the minute.
fn ser_when<S: Serializer>(at: &NaiveDateTime, s: S) -> Result<S::Ok, S::Error> {
    let text = if at.time() == NaiveTime::MIN {
        at.format("%Y-%m-%d").to_string()
    } else {
        at.format("%Y-%m-%dT%H:%M").to_string()
    };
    s.serialize_str(&text)
}

pub fn parse_when(s: &str) -> Option<NaiveDateTime> {
    ["%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M"]
        .iter()
        .find_map(|f| NaiveDateTime::parse_from_str(s, f).ok())
        .or_else(|| {
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.and_hms_opt(0, 0, 0))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_formats() {
        assert!(parse_when("2026-11-07").is_some());
        assert!(parse_when("2026-11-07T09:30").is_some());
        assert!(parse_when("2026-11-07T09:30:15").is_some());
        assert!(parse_when("2026-11-07 09:30").is_some());
        assert!(parse_when("07/11/2026").is_none());
    }

    #[test]
    fn round_trips_timestamps() {
        for s in ["2026-11-07", "2026-11-07T09:30"] {
            let e: LogEntry = serde_json::from_str(&format!(r#"{{"at":"{s}","text":"x"}}"#)).unwrap();
            assert_eq!(serde_json::to_value(&e).unwrap()["at"], s);
        }
    }
}
