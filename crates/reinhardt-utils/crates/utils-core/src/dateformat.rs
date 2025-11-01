//! Date formatting utilities
//!
//! Provides date and time formatting similar to Django's dateformat module.

use chrono::{DateTime, Datelike, Timelike, Utc, Weekday};

/// Format codes similar to Django/PHP date format
///
/// Common format codes:
/// - Y: 4-digit year (e.g., 2025)
/// - y: 2-digit year (e.g., 25)
/// - m: Month with leading zero (01-12)
/// - n: Month without leading zero (1-12)
/// - d: Day with leading zero (01-31)
/// - j: Day without leading zero (1-31)
/// - H: Hour in 24-hour format with leading zero (00-23)
/// - i: Minutes with leading zero (00-59)
/// - s: Seconds with leading zero (00-59)
/// - A: AM/PM
/// - l: Full weekday name (e.g., Monday)
/// - F: Full month name (e.g., January)
/// Format codes similar to Django/PHP date format
///
/// Common format codes:
/// - Y: 4-digit year (e.g., 2025)
/// - y: 2-digit year (e.g., 25)
/// - m: Month with leading zero (01-12)
/// - n: Month without leading zero (1-12)
/// - d: Day with leading zero (01-31)
/// - j: Day without leading zero (1-31)
/// - H: Hour in 24-hour format with leading zero (00-23)
/// - i: Minutes with leading zero (00-59)
/// - s: Seconds with leading zero (00-59)
/// - A: AM/PM
/// - l: Full weekday name (e.g., Monday)
/// - F: Full month name (e.g., January)
/// Format a datetime using Django-style format string
///
/// # Examples
///
/// ```
/// use reinhardt_utils_core::dateformat::format;
/// use chrono::{TimeZone, Utc};
///
/// let dt = Utc.with_ymd_and_hms(2025, 1, 15, 14, 30, 45).unwrap();
/// assert_eq!(format(&dt, "Y-m-d"), "2025-01-15");
/// assert_eq!(format(&dt, "H:i:s"), "14:30:45");
/// assert_eq!(format(&dt, "l, F j, Y"), "Wednesday, January 15, 2025");
/// ```
pub fn format(dt: &DateTime<Utc>, format_str: &str) -> String {
	let mut result = String::new();
	let mut chars = format_str.chars().peekable();

	while let Some(c) = chars.next() {
		if c == '\\' {
			// Escape character - output next char literally
			if let Some(next) = chars.next() {
				result.push(next);
			}
			continue;
		}

		let replacement = match c {
			// Year
			'Y' => format!("{:04}", dt.year()),
			'y' => format!("{:02}", dt.year() % 100),

			// Month
			'm' => format!("{:02}", dt.month()),
			'n' => dt.month().to_string(),
			'F' => month_name(dt.month()),
			'M' => month_abbr(dt.month()),

			// Day
			'd' => format!("{:02}", dt.day()),
			'j' => dt.day().to_string(),
			'l' => weekday_name(dt.weekday()),
			'D' => weekday_abbr(dt.weekday()),

			// Hour
			'H' => format!("{:02}", dt.hour()),
			'h' => {
				let hour12 = if dt.hour() == 0 || dt.hour() == 12 {
					12
				} else {
					dt.hour() % 12
				};
				format!("{:02}", hour12)
			}
			'G' => dt.hour().to_string(),
			'g' => {
				let hour12 = if dt.hour() == 0 || dt.hour() == 12 {
					12
				} else {
					dt.hour() % 12
				};
				hour12.to_string()
			}

			// Minute
			'i' => format!("{:02}", dt.minute()),

			// Second
			's' => format!("{:02}", dt.second()),

			// AM/PM
			'A' => if dt.hour() < 12 { "AM" } else { "PM" }.to_string(),
			'a' => if dt.hour() < 12 { "am" } else { "pm" }.to_string(),

			// Default: output character as-is
			_ => c.to_string(),
		};

		result.push_str(&replacement);
	}

	result
}

/// Get full month name
fn month_name(month: u32) -> String {
	match month {
		1 => "January",
		2 => "February",
		3 => "March",
		4 => "April",
		5 => "May",
		6 => "June",
		7 => "July",
		8 => "August",
		9 => "September",
		10 => "October",
		11 => "November",
		12 => "December",
		_ => "Unknown",
	}
	.to_string()
}

/// Get abbreviated month name
fn month_abbr(month: u32) -> String {
	match month {
		1 => "Jan",
		2 => "Feb",
		3 => "Mar",
		4 => "Apr",
		5 => "May",
		6 => "Jun",
		7 => "Jul",
		8 => "Aug",
		9 => "Sep",
		10 => "Oct",
		11 => "Nov",
		12 => "Dec",
		_ => "Unk",
	}
	.to_string()
}

/// Get full weekday name
fn weekday_name(weekday: Weekday) -> String {
	match weekday {
		Weekday::Mon => "Monday",
		Weekday::Tue => "Tuesday",
		Weekday::Wed => "Wednesday",
		Weekday::Thu => "Thursday",
		Weekday::Fri => "Friday",
		Weekday::Sat => "Saturday",
		Weekday::Sun => "Sunday",
	}
	.to_string()
}

/// Get abbreviated weekday name
fn weekday_abbr(weekday: Weekday) -> String {
	match weekday {
		Weekday::Mon => "Mon",
		Weekday::Tue => "Tue",
		Weekday::Wed => "Wed",
		Weekday::Thu => "Thu",
		Weekday::Fri => "Fri",
		Weekday::Sat => "Sat",
		Weekday::Sun => "Sun",
	}
	.to_string()
}

/// Common date format shortcuts
pub mod shortcuts {
	use super::*;
	/// ISO 8601 format: YYYY-MM-DD
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_utils_core::dateformat::shortcuts::iso_date;
	/// use chrono::{TimeZone, Utc};
	///
	/// let dt = Utc.with_ymd_and_hms(2025, 1, 15, 14, 30, 45).unwrap();
	/// assert_eq!(iso_date(&dt), "2025-01-15");
	/// ```
	pub fn iso_date(dt: &DateTime<Utc>) -> String {
		format(dt, "Y-m-d")
	}
	/// ISO 8601 datetime: YYYY-MM-DD HH:MM:SS
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_utils_core::dateformat::shortcuts::iso_datetime;
	/// use chrono::{TimeZone, Utc};
	///
	/// let dt = Utc.with_ymd_and_hms(2025, 1, 15, 14, 30, 45).unwrap();
	/// assert_eq!(iso_datetime(&dt), "2025-01-15 14:30:45");
	/// ```
	pub fn iso_datetime(dt: &DateTime<Utc>) -> String {
		format(dt, "Y-m-d H:i:s")
	}
	/// US date format: MM/DD/YYYY
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_utils_core::dateformat::shortcuts::us_date;
	/// use chrono::{TimeZone, Utc};
	///
	/// let dt = Utc.with_ymd_and_hms(2025, 1, 15, 14, 30, 45).unwrap();
	/// assert_eq!(us_date(&dt), "01/15/2025");
	/// ```
	pub fn us_date(dt: &DateTime<Utc>) -> String {
		format(dt, "m/d/Y")
	}
	/// European date format: DD/MM/YYYY
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_utils_core::dateformat::shortcuts::eu_date;
	/// use chrono::{TimeZone, Utc};
	///
	/// let dt = Utc.with_ymd_and_hms(2025, 1, 15, 14, 30, 45).unwrap();
	/// assert_eq!(eu_date(&dt), "15/01/2025");
	/// ```
	pub fn eu_date(dt: &DateTime<Utc>) -> String {
		format(dt, "d/m/Y")
	}
	/// Full text date: Monday, January 1, 2025
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_utils_core::dateformat::shortcuts::full_date;
	/// use chrono::{TimeZone, Utc};
	///
	/// let dt = Utc.with_ymd_and_hms(2025, 1, 15, 14, 30, 45).unwrap();
	/// let full = full_date(&dt);
	/// assert!(full.contains("Wednesday"));
	/// assert!(full.contains("January"));
	/// assert!(full.contains("15"));
	/// assert!(full.contains("2025"));
	/// ```
	pub fn full_date(dt: &DateTime<Utc>) -> String {
		format(dt, "l, F j, Y")
	}
	/// Short text date: Jan 1, 2025
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_utils_core::dateformat::shortcuts::short_date;
	/// use chrono::{TimeZone, Utc};
	///
	/// let dt = Utc.with_ymd_and_hms(2025, 1, 15, 14, 30, 45).unwrap();
	/// let short = short_date(&dt);
	/// assert!(short.contains("Jan"));
	/// assert!(short.contains("15"));
	/// assert!(short.contains("2025"));
	/// ```
	pub fn short_date(dt: &DateTime<Utc>) -> String {
		format(dt, "M j, Y")
	}
	/// Time 24-hour: 14:30:00
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_utils_core::dateformat::shortcuts::time_24;
	/// use chrono::{TimeZone, Utc};
	///
	/// let dt = Utc.with_ymd_and_hms(2025, 1, 15, 14, 30, 45).unwrap();
	/// assert_eq!(time_24(&dt), "14:30:45");
	/// ```
	pub fn time_24(dt: &DateTime<Utc>) -> String {
		format(dt, "H:i:s")
	}
	/// Time 12-hour: 2:30:00 PM
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_utils_core::dateformat::shortcuts::time_12;
	/// use chrono::{TimeZone, Utc};
	///
	/// let dt = Utc.with_ymd_and_hms(2025, 1, 15, 14, 30, 45).unwrap();
	/// assert_eq!(time_12(&dt), "2:30:45 PM");
	/// ```
	pub fn time_12(dt: &DateTime<Utc>) -> String {
		format(dt, "g:i:s A")
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use chrono::TimeZone;

	#[test]
	fn test_year_formats() {
		let dt = Utc.with_ymd_and_hms(2025, 1, 15, 14, 30, 45).unwrap();

		assert_eq!(format(&dt, "Y"), "2025");
		assert_eq!(format(&dt, "y"), "25");
	}

	#[test]
	fn test_month_formats() {
		let dt = Utc.with_ymd_and_hms(2025, 1, 15, 14, 30, 45).unwrap();

		assert_eq!(format(&dt, "m"), "01");
		assert_eq!(format(&dt, "n"), "1");
		assert_eq!(format(&dt, "F"), "January");
		assert_eq!(format(&dt, "M"), "Jan");
	}

	#[test]
	fn test_day_formats() {
		let dt = Utc.with_ymd_and_hms(2025, 1, 5, 14, 30, 45).unwrap();

		assert_eq!(format(&dt, "d"), "05");
		assert_eq!(format(&dt, "j"), "5");
	}

	#[test]
	fn test_time_formats() {
		let dt = Utc.with_ymd_and_hms(2025, 1, 15, 14, 5, 9).unwrap();

		assert_eq!(format(&dt, "H:i:s"), "14:05:09");
		assert_eq!(format(&dt, "g:i:s A"), "2:05:09 PM");
	}

	#[test]
	fn test_shortcuts() {
		let dt = Utc.with_ymd_and_hms(2025, 1, 15, 14, 30, 45).unwrap();

		assert_eq!(shortcuts::iso_date(&dt), "2025-01-15");
		assert_eq!(shortcuts::iso_datetime(&dt), "2025-01-15 14:30:45");
		assert_eq!(shortcuts::us_date(&dt), "01/15/2025");
		assert_eq!(shortcuts::eu_date(&dt), "15/01/2025");
	}

	#[test]
	fn test_escape_character() {
		let dt = Utc.with_ymd_and_hms(2025, 1, 15, 14, 30, 45).unwrap();

		assert_eq!(format(&dt, "Y\\Y"), "2025Y");
		assert_eq!(format(&dt, "\\d\\a\\y"), "day");
	}

	#[test]
	fn test_12_hour_formats() {
		let dt_afternoon = Utc.with_ymd_and_hms(2025, 1, 15, 14, 30, 45).unwrap();
		let dt_midnight = Utc.with_ymd_and_hms(2025, 1, 15, 0, 30, 45).unwrap();
		let dt_noon = Utc.with_ymd_and_hms(2025, 1, 15, 12, 30, 45).unwrap();

		// h format (01-12 with leading zero)
		assert_eq!(format(&dt_afternoon, "h"), "02");
		assert_eq!(format(&dt_midnight, "h"), "12");
		assert_eq!(format(&dt_noon, "h"), "12");

		// g format (1-12 without leading zero)
		assert_eq!(format(&dt_afternoon, "g"), "2");
		assert_eq!(format(&dt_midnight, "g"), "12");
		assert_eq!(format(&dt_noon, "g"), "12");

		// G format (0-23 without leading zero)
		assert_eq!(format(&dt_afternoon, "G"), "14");
		assert_eq!(format(&dt_midnight, "G"), "0");
		assert_eq!(format(&dt_noon, "G"), "12");
	}

	#[test]
	fn test_weekday_formats() {
		let monday = Utc.with_ymd_and_hms(2025, 1, 13, 14, 30, 45).unwrap(); // Monday
		let friday = Utc.with_ymd_and_hms(2025, 1, 17, 14, 30, 45).unwrap(); // Friday
		let sunday = Utc.with_ymd_and_hms(2025, 1, 19, 14, 30, 45).unwrap(); // Sunday

		// l format (full weekday name)
		assert_eq!(format(&monday, "l"), "Monday");
		assert_eq!(format(&friday, "l"), "Friday");
		assert_eq!(format(&sunday, "l"), "Sunday");

		// D format (abbreviated weekday name)
		assert_eq!(format(&monday, "D"), "Mon");
		assert_eq!(format(&friday, "D"), "Fri");
		assert_eq!(format(&sunday, "D"), "Sun");
	}

	#[test]
	fn test_all_month_names() {
		for month in 1..=12 {
			let dt = Utc.with_ymd_and_hms(2025, month, 15, 14, 30, 45).unwrap();
			let full_name = format(&dt, "F");
			let abbr_name = format(&dt, "M");

			// Verify they are not empty
			assert!(!full_name.is_empty());
			assert!(!abbr_name.is_empty());
			assert_ne!(full_name, "Unknown");
			assert_ne!(abbr_name, "Unk");
		}
	}

	#[test]
	fn test_all_weekday_names() {
		// Test a full week (Jan 13-19, 2025 is Mon-Sun)
		for day in 13..=19 {
			let dt = Utc.with_ymd_and_hms(2025, 1, day, 14, 30, 45).unwrap();
			let full_name = format(&dt, "l");
			let abbr_name = format(&dt, "D");

			// Verify they are not empty
			assert!(!full_name.is_empty());
			assert!(!abbr_name.is_empty());
		}
	}

	#[test]
	fn test_am_pm_formats() {
		let morning = Utc.with_ymd_and_hms(2025, 1, 15, 9, 30, 45).unwrap();
		let evening = Utc.with_ymd_and_hms(2025, 1, 15, 21, 30, 45).unwrap();

		// A format (uppercase AM/PM)
		assert_eq!(format(&morning, "A"), "AM");
		assert_eq!(format(&evening, "A"), "PM");

		// a format (lowercase am/pm)
		assert_eq!(format(&morning, "a"), "am");
		assert_eq!(format(&evening, "a"), "pm");
	}

	#[test]
	fn test_shortcuts_full_date() {
		let dt = Utc.with_ymd_and_hms(2025, 1, 15, 14, 30, 45).unwrap();
		let full = shortcuts::full_date(&dt);
		assert!(full.contains("Wednesday"));
		assert!(full.contains("January"));
		assert!(full.contains("15"));
		assert!(full.contains("2025"));
	}

	#[test]
	fn test_shortcuts_short_date() {
		let dt = Utc.with_ymd_and_hms(2025, 1, 15, 14, 30, 45).unwrap();
		let short = shortcuts::short_date(&dt);
		assert!(short.contains("Jan"));
		assert!(short.contains("15"));
		assert!(short.contains("2025"));
	}

	#[test]
	fn test_shortcuts_time_formats() {
		let dt = Utc.with_ymd_and_hms(2025, 1, 15, 14, 30, 45).unwrap();

		assert_eq!(shortcuts::time_24(&dt), "14:30:45");
		assert_eq!(shortcuts::time_12(&dt), "2:30:45 PM");
	}

	#[test]
	fn test_combined_format() {
		let dt = Utc.with_ymd_and_hms(2025, 1, 15, 14, 30, 45).unwrap();
		let result = format(&dt, "l, F j, Y - g:i A");
		assert!(result.contains("Wednesday"));
		assert!(result.contains("January"));
		assert!(result.contains("15"));
		assert!(result.contains("2025"));
		assert!(result.contains("2:30"));
		assert!(result.contains("PM"));
	}
}

#[cfg(test)]
mod proptests {
	use super::*;
	use chrono::TimeZone;
	use proptest::prelude::*;

	proptest! {
		#[test]
		fn prop_year_format_4_digits(year in 1000i32..9999i32, month in 1u32..=12, day in 1u32..=28) {
			let dt = Utc.with_ymd_and_hms(year, month, day, 12, 0, 0).unwrap();
			let result = format(&dt, "Y");
			assert_eq!(result.len(), 4);
			assert!(result.chars().all(|c| c.is_ascii_digit()));
			assert_eq!(result.parse::<i32>().unwrap(), year);
		}

		#[test]
		fn prop_month_format_range(year in 2000i32..2100, month in 1u32..=12, day in 1u32..=28) {
			let dt = Utc.with_ymd_and_hms(year, month, day, 12, 0, 0).unwrap();
			let result = format(&dt, "m");
			assert_eq!(result.len(), 2);
			let month_val = result.parse::<u32>().unwrap();
			assert!(month_val >= 1 && month_val <= 12);
			assert_eq!(month_val, month);
		}

		#[test]
		fn prop_day_format_range(year in 2000i32..2100, month in 1u32..=12, day in 1u32..=28) {
			let dt = Utc.with_ymd_and_hms(year, month, day, 12, 0, 0).unwrap();
			let result = format(&dt, "d");
			assert_eq!(result.len(), 2);
			let day_val = result.parse::<u32>().unwrap();
			assert!(day_val >= 1 && day_val <= 31);
			assert_eq!(day_val, day);
		}

		#[test]
		fn prop_hour_format_range(year in 2000i32..2100, hour in 0u32..=23) {
			let dt = Utc.with_ymd_and_hms(year, 1, 1, hour, 0, 0).unwrap();
			let result = format(&dt, "H");
			assert_eq!(result.len(), 2);
			let hour_val = result.parse::<u32>().unwrap();
			assert!(hour_val <= 23);
			assert_eq!(hour_val, hour);
		}

		#[test]
		fn prop_minute_format_range(year in 2000i32..2100, minute in 0u32..=59) {
			let dt = Utc.with_ymd_and_hms(year, 1, 1, 12, minute, 0).unwrap();
			let result = format(&dt, "i");
			assert_eq!(result.len(), 2);
			let minute_val = result.parse::<u32>().unwrap();
			assert!(minute_val <= 59);
			assert_eq!(minute_val, minute);
		}

		#[test]
		fn prop_second_format_range(year in 2000i32..2100, second in 0u32..=59) {
			let dt = Utc.with_ymd_and_hms(year, 1, 1, 12, 0, second).unwrap();
			let result = format(&dt, "s");
			assert_eq!(result.len(), 2);
			let second_val = result.parse::<u32>().unwrap();
			assert!(second_val <= 59);
			assert_eq!(second_val, second);
		}

		#[test]
		fn prop_escape_character(year in 2000i32..2100, c in "\\PC") {
			let dt = Utc.with_ymd_and_hms(year, 1, 1, 12, 0, 0).unwrap();
			let format_str = format!("\\{}", c);
			let result = format(&dt, &format_str);
			assert_eq!(result, c.to_string());
		}

		#[test]
		fn prop_shortcuts_valid_lengths(year in 2000i32..2100, month in 1u32..=12, day in 1u32..=28, hour in 0u32..=23, minute in 0u32..=59, second in 0u32..=59) {
			let dt = Utc.with_ymd_and_hms(year, month, day, hour, minute, second).unwrap();

			// ISO date is always 10 chars: YYYY-MM-DD
			assert_eq!(shortcuts::iso_date(&dt).len(), 10);

			// ISO datetime is always 19 chars: YYYY-MM-DD HH:MM:SS
			assert_eq!(shortcuts::iso_datetime(&dt).len(), 19);

			// US date is always 10 chars: MM/DD/YYYY
			assert_eq!(shortcuts::us_date(&dt).len(), 10);

			// EU date is always 10 chars: DD/MM/YYYY
			assert_eq!(shortcuts::eu_date(&dt).len(), 10);

			// Time 24 is always 8 chars: HH:MM:SS
			assert_eq!(shortcuts::time_24(&dt).len(), 8);

			// Time 12 format has variable length due to AM/PM and hour without leading zero
			let time_12 = shortcuts::time_12(&dt);
			assert!(time_12.len() >= 10 && time_12.len() <= 11);
		}
	}
}
