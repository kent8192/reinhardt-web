//! Timezone utilities
//!
//! Provides timezone-aware datetime handling similar to Django's timezone utilities.

use std::borrow::Cow;

use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
/// Get the current time in UTC
///
/// # Examples
///
/// ```
/// use reinhardt_utils::timezone::now;
///
/// let dt = now();
/// assert_eq!(dt.timezone(), chrono::Utc);
/// ```
pub fn now() -> DateTime<Utc> {
    Utc::now()
}
/// Get the current local time
///
/// # Examples
///
/// ```
/// use reinhardt_utils::timezone::localtime;
/// use chrono::{DateTime, Local};
///
/// let local_dt = localtime();
/// // Verify it returns a DateTime<Local>
/// let _: DateTime<Local> = local_dt;
/// ```
pub fn localtime() -> DateTime<Local> {
    Local::now()
}
/// Convert a UTC datetime to local time
///
/// # Examples
///
/// ```
/// use reinhardt_utils::timezone::{now, to_local};
///
/// let utc_now = now();
/// let local = to_local(utc_now);
/// // Should be the same instant in time
/// assert_eq!(utc_now.timestamp(), local.timestamp());
/// ```
pub fn to_local(dt: DateTime<Utc>) -> DateTime<Local> {
    dt.with_timezone(&Local)
}
/// Convert a local datetime to UTC
///
/// # Examples
///
/// ```
/// use reinhardt_utils::timezone::{localtime, to_utc, to_local};
///
/// let local = localtime();
/// let utc = to_utc(local);
/// let back_to_local = to_local(utc);
/// // Should represent the same instant
/// assert_eq!(local.timestamp(), back_to_local.timestamp());
/// ```
pub fn to_utc(dt: DateTime<Local>) -> DateTime<Utc> {
    dt.with_timezone(&Utc)
}
/// Check if a datetime is aware (has timezone info)
///
/// # Examples
///
/// ```
/// use reinhardt_utils::timezone::{now, is_aware};
///
/// let utc_dt = now();
/// assert!(is_aware(&utc_dt));
/// ```
pub fn is_aware<Tz: TimeZone>(_dt: &DateTime<Tz>) -> bool {
    true // In Rust's chrono, all DateTime objects are timezone-aware
}
/// Make a naive datetime aware in UTC
///
/// # Examples
///
/// ```
/// use reinhardt_utils::timezone::{make_aware_utc, is_aware};
/// use chrono::NaiveDateTime;
/// use std::str::FromStr;
///
/// let naive = NaiveDateTime::from_str("2025-01-01T12:00:00").unwrap();
/// let aware = make_aware_utc(naive);
/// assert!(is_aware(&aware));
/// ```
pub fn make_aware_utc(dt: NaiveDateTime) -> DateTime<Utc> {
    DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc)
}
/// Make a naive datetime aware in local timezone
///
/// # Examples
///
/// ```
/// use reinhardt_utils::timezone::make_aware_local;
/// use chrono::{NaiveDateTime, DateTime, Local};
/// use std::str::FromStr;
///
/// let naive = NaiveDateTime::from_str("2025-01-01T12:00:00").unwrap();
/// let aware = make_aware_local(naive);
/// let _: DateTime<Local> = aware;
/// assert_eq!(aware.naive_local(), naive);
/// ```
pub fn make_aware_local(dt: NaiveDateTime) -> DateTime<Local> {
    Local.from_local_datetime(&dt).earliest().unwrap()
}
/// Convert datetime to a specific timezone by IANA name
///
/// # Examples
///
/// ```
/// use reinhardt_utils::timezone::{now, to_timezone};
///
/// let dt = now();
/// let result = to_timezone(dt, "UTC");
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap(), dt);
///
/// let result_unsupported = to_timezone(dt, "America/New_York");
/// assert!(result_unsupported.is_err());
/// ```
pub fn to_timezone(dt: DateTime<Utc>, tz_name: &str) -> Result<DateTime<Utc>, String> {
    // Note: Full IANA timezone support would require chrono-tz crate
    // For now, we'll support UTC and Local only
    match tz_name {
        "UTC" => Ok(dt),
        _ => Err(format!(
            "Timezone {} not supported in basic implementation. Add chrono-tz for full support.",
            tz_name
        )),
    }
}
/// Get timezone name from UTC DateTime
///
/// # Examples
///
/// ```
/// use reinhardt_utils::timezone::{now, get_timezone_name_utc};
///
/// let dt = now();
/// let tz_name = get_timezone_name_utc(&dt);
/// assert_eq!(tz_name, "UTC");
/// ```
pub fn get_timezone_name_utc(_dt: &DateTime<Utc>) -> &'static str {
    "UTC"
}
/// Get timezone name from Local DateTime
///
/// # Examples
///
/// ```
/// use reinhardt_utils::timezone::{localtime, get_timezone_name_local};
///
/// let dt = localtime();
/// let tz_name = get_timezone_name_local(&dt);
/// // The timezone name will vary by system, but should not be empty
/// assert!(!tz_name.is_empty());
/// ```
pub fn get_timezone_name_local(_dt: &DateTime<Local>) -> Cow<'static, str> {
    /// // Try to get timezone from environment variable
    #[cfg(target_os = "windows")]
    {
        std::env::var("TZ")
            .map(Cow::Owned)
            .unwrap_or(Cow::Borrowed("Local"))
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("TZ")
            .map(Cow::Owned)
            .unwrap_or(Cow::Borrowed("Local"))
    }
}
/// Parse ISO 8601 datetime string
///
/// # Examples
///
/// ```
/// use reinhardt_utils::timezone::parse_datetime;
/// use chrono::Datelike;
///
/// let dt_str = "2025-01-01T12:00:00Z";
/// let dt = parse_datetime(dt_str).unwrap();
/// assert_eq!(dt.year(), 2025);
/// assert_eq!(dt.month(), 1);
/// assert_eq!(dt.day(), 1);
///
/// let result = parse_datetime("invalid datetime");
/// assert!(result.is_err());
/// ```
pub fn parse_datetime(s: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc))
}
/// Format datetime as ISO 8601 string
///
/// # Examples
///
/// ```
/// use reinhardt_utils::timezone::{parse_datetime, format_datetime};
///
/// let dt_str = "2025-01-01T12:00:00Z";
/// let dt = parse_datetime(dt_str).unwrap();
/// let formatted = format_datetime(&dt);
/// assert!(formatted.contains("2025-01-01"));
/// assert!(formatted.contains("12:00:00"));
/// ```
pub fn format_datetime(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_utils_timezone_now() {
        let dt = now();
        assert_eq!(dt.timezone(), Utc);
    }

    #[test]
    fn test_to_local_and_back() {
        let utc_now = now();
        let local = to_local(utc_now);
        let back_to_utc = to_utc(local);

        // Should be the same instant in time
        assert_eq!(utc_now.timestamp(), back_to_utc.timestamp());
    }

    #[test]
    fn test_make_aware() {
        let naive = NaiveDateTime::from_str("2025-01-01T12:00:00").unwrap();
        let aware = make_aware_utc(naive);

        assert!(is_aware(&aware));
    }

    #[test]
    fn test_parse_and_format() {
        let dt_str = "2025-01-01T12:00:00Z";
        let dt = parse_datetime(dt_str).unwrap();
        let formatted = format_datetime(&dt);

        assert!(formatted.contains("2025-01-01"));
        assert!(formatted.contains("12:00:00"));
    }

    #[test]
    fn test_localtime() {
        let local_dt = localtime();
        // Just verify it returns a DateTime<Local> by checking the type compiles
        let _: DateTime<Local> = local_dt;
    }

    #[test]
    fn test_is_aware_utc() {
        let utc_dt = now();
        assert!(is_aware(&utc_dt));
    }

    #[test]
    fn test_is_aware_local() {
        let local_dt = localtime();
        assert!(is_aware(&local_dt));
    }

    #[test]
    fn test_make_aware_local() {
        let naive = NaiveDateTime::from_str("2025-01-01T12:00:00").unwrap();
        let aware = make_aware_local(naive);

        // Verify the type and that naive_local matches
        let _: DateTime<Local> = aware;
        assert_eq!(aware.naive_local(), naive);
    }

    #[test]
    fn test_to_timezone_utc() {
        let dt = now();
        let result = to_timezone(dt, "UTC");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), dt);
    }

    #[test]
    fn test_to_timezone_unsupported() {
        let dt = now();
        let result = to_timezone(dt, "America/New_York");

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("not supported in basic implementation"));
    }

    #[test]
    fn test_get_timezone_name_utc() {
        let dt = now();
        let tz_name = get_timezone_name_utc(&dt);

        assert_eq!(tz_name, "UTC");
    }

    #[test]
    fn test_get_timezone_name_local() {
        let dt = localtime();
        let tz_name = get_timezone_name_local(&dt);

        // The timezone name will vary by system, but should not be empty
        assert!(!tz_name.is_empty());
        // Should be either "Local" or a TZ environment variable value
        assert!(tz_name == "Local" || tz_name.len() > 0);
    }

    #[test]
    fn test_parse_datetime_with_offset() {
        let dt_str = "2025-01-01T12:00:00+09:00";
        let dt = parse_datetime(dt_str);

        assert!(dt.is_ok());
        let parsed = dt.unwrap();
        assert_eq!(parsed.timezone(), Utc);
    }

    #[test]
    fn test_parse_datetime_invalid() {
        let dt_str = "invalid datetime";
        let result = parse_datetime(dt_str);

        assert!(result.is_err());
    }

    #[test]
    fn test_format_datetime_roundtrip() {
        let original = now();
        let formatted = format_datetime(&original);
        let parsed = parse_datetime(&formatted).unwrap();

        // Should represent the same instant in time
        assert_eq!(original.timestamp(), parsed.timestamp());
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_to_local_preserves_timestamp(timestamp in 0i64..2147483647) {
            let utc_dt = DateTime::<Utc>::from_timestamp(timestamp, 0).unwrap();
            let local = to_local(utc_dt);
            assert_eq!(utc_dt.timestamp(), local.timestamp());
        }

        #[test]
        fn prop_to_utc_preserves_timestamp(timestamp in 0i64..2147483647) {
            // Create a local datetime via UTC conversion
            let utc_dt = DateTime::<Utc>::from_timestamp(timestamp, 0).unwrap();
            let local = to_local(utc_dt);
            let back_to_utc = to_utc(local);
            assert_eq!(local.timestamp(), back_to_utc.timestamp());
        }

        #[test]
        fn prop_make_aware_utc_roundtrip(timestamp in 0i64..2147483647) {
            let utc_dt = DateTime::<Utc>::from_timestamp(timestamp, 0).unwrap();
            let naive = utc_dt.naive_utc();
            let aware = make_aware_utc(naive);
            assert_eq!(aware.timestamp(), utc_dt.timestamp());
        }

        #[test]
        fn prop_format_parse_roundtrip(timestamp in 0i64..2147483647) {
            let original = DateTime::<Utc>::from_timestamp(timestamp, 0).unwrap();
            let formatted = format_datetime(&original);
            let parsed = parse_datetime(&formatted);
            assert!(parsed.is_ok());
            assert_eq!(original.timestamp(), parsed.unwrap().timestamp());
        }

        #[test]
        fn prop_is_aware_always_true(timestamp in 0i64..2147483647) {
            let utc_dt = DateTime::<Utc>::from_timestamp(timestamp, 0).unwrap();
            let local_dt = to_local(utc_dt);
            assert!(is_aware(&utc_dt));
            assert!(is_aware(&local_dt));
        }
    }
}
