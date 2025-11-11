//! # Humanize Utilities
//!
//! Human-friendly formatting utilities for numbers, dates, and file sizes.
//!
//! ## Available Functions
//!
//! - `intword` - Convert large numbers to word representation (e.g., "1.5 million")
//! - `filesizeformat` - Format file sizes in human-readable format (e.g., "1.2 MB")
//! - `naturalday` - Format dates as "today", "yesterday", "tomorrow", or actual date
//! - `naturaltime` - Format datetimes as natural language (e.g., "3 seconds ago")
//! - `timesince` - Calculate time since a given datetime
//!
//! Also re-exports from `reinhardt_utils::text`:
//! - `intcomma` - Add commas to numbers (e.g., "1,234,567")
//! - `ordinal` - Convert numbers to ordinals (e.g., "1st", "2nd", "3rd")
//! - `pluralize` - Pluralize words based on count
//!
//! ## Usage
//!
//! ```rust
//! use reinhardt_utils::humanize::{intword, filesizeformat, naturalday};
//! use chrono::Utc;
//!
//! let num = 1_500_000;
//! assert_eq!(intword(num), "1.5 million");
//!
//! let size = 1_234_567;
//! assert_eq!(filesizeformat(size), "1.2 MB");
//!
//! let now = Utc::now();
//! assert_eq!(naturalday(&now), "today");
//! ```

use chrono::{DateTime, Duration, Utc};

pub use crate::text::{intcomma, ordinal, pluralize};

/// Convert a large number to a word representation
///
/// # Examples
///
/// ```rust
/// use reinhardt_utils::humanize::intword;
///
/// assert_eq!(intword(1_500_000), "1.5 million");
/// assert_eq!(intword(2_000_000_000), "2 billion");
/// ```
pub fn intword(n: i64) -> String {
	if n.abs() >= 1_000_000_000_000 {
		let val = n as f64 / 1_000_000_000_000.0;
		if val.fract() == 0.0 {
			format!("{} trillion", val as i64)
		} else {
			format!("{:.1} trillion", val)
		}
	} else if n.abs() >= 1_000_000_000 {
		let val = n as f64 / 1_000_000_000.0;
		if val.fract() == 0.0 {
			format!("{} billion", val as i64)
		} else {
			format!("{:.1} billion", val)
		}
	} else if n.abs() >= 1_000_000 {
		let val = n as f64 / 1_000_000.0;
		if val.fract() == 0.0 {
			format!("{} million", val as i64)
		} else {
			format!("{:.1} million", val)
		}
	} else if n.abs() >= 1_000 {
		let val = n as f64 / 1_000.0;
		if val.fract() == 0.0 {
			format!("{} thousand", val as i64)
		} else {
			format!("{:.1} thousand", val)
		}
	} else {
		n.to_string()
	}
}

/// Format file size in human-readable format
///
/// # Examples
///
/// ```rust
/// use reinhardt_utils::humanize::filesizeformat;
///
/// assert_eq!(filesizeformat(1_234_567), "1.2 MB");
/// assert_eq!(filesizeformat(1024), "1.0 KB");
/// ```
pub fn filesizeformat(bytes: u64) -> String {
	const KB: u64 = 1024;
	const MB: u64 = KB * 1024;
	const GB: u64 = MB * 1024;

	if bytes == 0 {
		"0 bytes".to_string()
	} else if bytes == 1 {
		"1 byte".to_string()
	} else if bytes < KB {
		format!("{} bytes", bytes)
	} else if bytes < MB {
		format!("{:.1} KB", bytes as f64 / KB as f64)
	} else if bytes < GB {
		format!("{:.1} MB", bytes as f64 / MB as f64)
	} else {
		format!("{:.1} GB", bytes as f64 / GB as f64)
	}
}

/// Format a date as "today", "yesterday", "tomorrow", or the actual date
///
/// # Examples
///
/// ```rust
/// use reinhardt_utils::humanize::naturalday;
/// use chrono::Utc;
///
/// let now = Utc::now();
/// assert_eq!(naturalday(&now), "today");
/// ```
pub fn naturalday(dt: &DateTime<Utc>) -> String {
	let now = Utc::now();
	let today = now.date_naive();
	let dt_date = dt.date_naive();

	if dt_date == today {
		"today".to_string()
	} else if dt_date == today - Duration::days(1) {
		"yesterday".to_string()
	} else if dt_date == today + Duration::days(1) {
		"tomorrow".to_string()
	} else {
		dt.format("%b %d, %Y").to_string()
	}
}

/// Format a datetime as natural language (e.g., "3 seconds ago", "2 hours from now")
///
/// # Examples
///
/// ```rust
/// use reinhardt_utils::humanize::naturaltime;
/// use chrono::{Utc, Duration};
///
/// let now = Utc::now();
/// let past = now - Duration::seconds(30);
/// let result = naturaltime(&past);
/// assert!(result.contains("seconds ago"));
/// ```
pub fn naturaltime(dt: &DateTime<Utc>) -> String {
	let now = Utc::now();
	let diff = now.signed_duration_since(*dt);

	if diff.num_seconds() < 60 && diff.num_seconds() >= 0 {
		if diff.num_seconds() < 10 {
			"a few seconds ago".to_string()
		} else {
			format!("{} seconds ago", diff.num_seconds())
		}
	} else if diff.num_seconds() < 0 {
		let future_diff = dt.signed_duration_since(now);
		if future_diff.num_seconds() < 60 {
			"a few seconds from now".to_string()
		} else if future_diff.num_minutes() < 60 {
			format!("{} minutes from now", future_diff.num_minutes())
		} else if future_diff.num_hours() < 24 {
			format!("{} hours from now", future_diff.num_hours())
		} else {
			format!("{} days from now", future_diff.num_days())
		}
	} else if diff.num_minutes() < 60 {
		format!("{} minutes ago", diff.num_minutes())
	} else if diff.num_hours() < 24 {
		format!("{} hours ago", diff.num_hours())
	} else {
		format!("{} days ago", diff.num_days())
	}
}

/// Calculate time since a given datetime
///
/// # Examples
///
/// ```rust
/// use reinhardt_utils::humanize::timesince;
/// use chrono::{Utc, Duration};
///
/// let now = Utc::now();
/// let past = now - Duration::hours(2);
/// assert_eq!(timesince(&past), "2 hours");
/// ```
pub fn timesince(dt: &DateTime<Utc>) -> String {
	let now = Utc::now();
	let diff = now.signed_duration_since(*dt);

	if diff.num_days() > 0 {
		if diff.num_days() == 1 {
			"1 day".to_string()
		} else {
			format!("{} days", diff.num_days())
		}
	} else if diff.num_hours() > 0 {
		if diff.num_hours() == 1 {
			"1 hour".to_string()
		} else {
			format!("{} hours", diff.num_hours())
		}
	} else if diff.num_minutes() > 0 {
		if diff.num_minutes() == 1 {
			"1 minute".to_string()
		} else {
			format!("{} minutes", diff.num_minutes())
		}
	} else {
		"less than a minute".to_string()
	}
}
