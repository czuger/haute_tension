//! A timestamp column that gives back exactly the instant it was given.
//!
//! SQLite has no datetime type, and a naive string read back as a naive instant
//! would drop the timezone every instant in this application carries. This
//! column stores the ISO 8601 text of the instant in UTC instead —
//! `2026-09-10T12:00:00.000000+00:00` — which is both what it reads back as an
//! aware instant and a text that sorts in chronological order, so an `ORDER BY`
//! on it is an order in time.
//!
//! [`NOW_IN_SQL`] is that same text written by SQLite itself, for a server
//! default. `CURRENT_TIMESTAMP` would not do: it writes `2026-09-10 12:00:00`,
//! with a space where the `T` goes, which sorts before every instant written
//! from the application on the same day.

use chrono::{DateTime, SecondsFormat, Utc};

/// Wide enough for the fixed-width text [`to_text`] writes.
pub const ISO_WIDTH: usize = 32;

/// SQLite's `%f` is the seconds with three decimals; the three zeros pad them to
/// the six [`to_text`] writes, so a row stamped by either has text of one width.
pub const NOW_IN_SQL: &str = "(strftime('%Y-%m-%dT%H:%M:%f000+00:00', 'now'))";

/// The current instant, in UTC.
pub fn utc_now() -> DateTime<Utc> {
    Utc::now()
}

/// The fixed-width UTC text of an instant, the same for every row.
///
/// Always with microseconds, so two instants compare as text the way they
/// compare in time: `12:00:00` and `12:00:00.5` would otherwise differ at a
/// character that is not a digit.
///
/// # Returns
///
/// `YYYY-MM-DDTHH:MM:SS.ffffff+00:00`.
pub fn to_text(instant: DateTime<Utc>) -> String {
    instant
        .to_rfc3339_opts(SecondsFormat::Micros, false)
        .replace('Z', "+00:00")
}

/// An instant as the application hands it out, the way `isoformat()` writes one.
///
/// The microseconds are left out when they are zero, which is what the memorial
/// and the flagged pages print, and what a reader of those answers expects.
pub fn to_isoformat(instant: DateTime<Utc>) -> String {
    let precision = if instant.timestamp_subsec_micros() == 0 {
        SecondsFormat::Secs
    } else {
        SecondsFormat::Micros
    };
    instant
        .to_rfc3339_opts(precision, false)
        .replace('Z', "+00:00")
}

/// Read a stored instant back, a naive one being taken to be UTC.
///
/// # Returns
///
/// The instant, or nothing when the text is not one.
pub fn from_text(text: &str) -> Option<DateTime<Utc>> {
    if let Ok(instant) = DateTime::parse_from_rfc3339(text) {
        return Some(instant.with_timezone(&Utc));
    }
    chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S%.f")
        .ok()
        .map(|naive| naive.and_utc())
}
