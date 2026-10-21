//! Riga wall-clock views of an RFC 3339 instant: the Excel serial number and the file-name stamp.
//! Owns the time-zone rule (Europe/Riga from the bundled tz database: UTC+2, UTC+3 in summer).
//! Invariant: pure; text that is not an instant gives None, never a wrong time.

use crate::config;

/// Excel serial date-time on the Riga wall clock (days since 1899-12-30, the time as the fraction).
pub fn excel_serial(rfc3339: &str) -> Option<f64> {
    let ts: jiff::Timestamp = rfc3339.parse().ok()?;
    let local = ts.in_tz(config::TIME_ZONE).ok()?.datetime();
    let epoch = jiff::civil::date(1899, 12, 30).at(0, 0, 0, 0);
    Some(local.duration_since(epoch).as_secs_f64() / 86_400.0)
}

/// The Riga date and time for a file name, ASCII only: "2026-10-28 12-15".
pub fn file_stamp(rfc3339: &str) -> Option<String> {
    let ts: jiff::Timestamp = rfc3339.parse().ok()?;
    Some(ts.in_tz(config::TIME_ZONE).ok()?.strftime("%Y-%m-%d %H-%M").to_string())
}
