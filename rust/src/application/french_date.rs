//! A date as the templates print it: "10 septembre 2026".
//!
//! A filter rather than a field built in `core::db`: the memorial stores when a
//! hero fell as an instant, and how that instant reads is the page's business.

use crate::core::models::utc_datetime::from_text;

pub const MONTHS_FR: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

/// Print an ISO 8601 instant as a French date.
///
/// # Arguments
///
/// * `instant` - The instant as `core::db` hands it out, or nothing.
///
/// # Returns
///
/// The day, month and year in French, or an empty string for nothing.
pub fn french_date(instant: Option<&str>) -> String {
    let Some(instant) = instant.filter(|text| !text.is_empty()) else {
        return String::new();
    };
    let Some(moment) = from_text(instant) else {
        return String::new();
    };
    let month = usize::try_from(chrono::Datelike::month(&moment)).unwrap_or(1);
    format!(
        "{} {} {}",
        chrono::Datelike::day(&moment),
        MONTHS_FR[month.saturating_sub(1).min(11)],
        chrono::Datelike::year(&moment),
    )
}
