//! DC release info.

use chrono::NaiveDate;
use once_cell::sync::Lazy;

const DATE_STR: &str = include_str!("../release-date.in");

/// Last release date.
pub static DATE: Lazy<NaiveDate> =
    Lazy::new(|| NaiveDate::parse_from_str(DATE_STR, "%Y-%m-%d").unwrap());
