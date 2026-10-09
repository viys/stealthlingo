//! Timestamp helpers. All timestamps are stored as fixed-width RFC 3339 UTC
//! strings so that SQLite can compare them lexicographically.

use anyhow::{Context, Result};
use chrono::{DateTime, Local, SecondsFormat, TimeZone, Utc};

pub fn to_db(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn from_db(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .with_context(|| format!("invalid timestamp in database: {value}"))
}

/// Start of the current local calendar day, expressed in UTC.
pub fn local_day_start(now: DateTime<Utc>) -> DateTime<Utc> {
    let local = now.with_timezone(&Local);
    let midnight = local
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("midnight is always valid");
    Local
        .from_local_datetime(&midnight)
        .earliest()
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or(now)
}

pub fn format_local(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Local)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

/// Short human description of when something is due, e.g. "now", "in 3h".
pub fn describe_due(due: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let secs = (due - now).num_seconds();
    if secs <= 0 {
        return "now".to_string();
    }
    let minutes = secs / 60;
    if minutes < 60 {
        format!("in {}m", minutes.max(1))
    } else if minutes < 60 * 24 {
        format!("in {}h", minutes / 60)
    } else {
        format!("in {}d", minutes / (60 * 24))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn db_round_trip_is_fixed_width() {
        let now = Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap();
        let s = to_db(now);
        assert_eq!(s, "2026-01-02T03:04:05Z");
        assert_eq!(from_db(&s).unwrap(), now);
    }

    #[test]
    fn describe_due_buckets() {
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(describe_due(now - Duration::hours(1), now), "now");
        assert_eq!(describe_due(now + Duration::minutes(10), now), "in 10m");
        assert_eq!(describe_due(now + Duration::hours(5), now), "in 5h");
        assert_eq!(describe_due(now + Duration::days(3), now), "in 3d");
    }
}
