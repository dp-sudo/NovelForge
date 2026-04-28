use chrono::{SecondsFormat, Utc};

pub fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Returns today's date as YYYY-MM-DD (e.g. "2026-04-28").
/// Used to check if a daily backup has already been made.
pub fn today_date_str() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}
