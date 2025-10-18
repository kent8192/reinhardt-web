//! Humanize integration tests
//!
//! Based on Django's humanize tests from:
//! - django/tests/humanize_tests/tests.py

use chrono::{Duration, Utc};
use reinhardt_contrib::humanize;

#[test]
fn test_contrib_humanize_intcomma() {
    assert_eq!(humanize::intcomma(100), "100");
    assert_eq!(humanize::intcomma(1000), "1,000");
    assert_eq!(humanize::intcomma(10123), "10,123");
    assert_eq!(humanize::intcomma(10311), "10,311");
    assert_eq!(humanize::intcomma(1000000), "1,000,000");
    assert_eq!(humanize::intcomma(1234567), "1,234,567");
    assert_eq!(humanize::intcomma(-100), "-100");
    assert_eq!(humanize::intcomma(-1000), "-1,000");
    assert_eq!(humanize::intcomma(-1234567), "-1,234,567");
}

#[test]
fn test_intword() {
    assert_eq!(humanize::intword(100), "100");
    assert_eq!(humanize::intword(1000), "1 thousand");
    assert_eq!(humanize::intword(12000), "12 thousand");
    assert_eq!(humanize::intword(1200000), "1.2 million");
    assert_eq!(humanize::intword(1290000), "1.3 million");
    assert_eq!(humanize::intword(1000000), "1 million");
    assert_eq!(humanize::intword(1000000000), "1 billion");
    assert_eq!(humanize::intword(1000000000000), "1 trillion");
    assert_eq!(humanize::intword(-1200000), "-1.2 million");
}

#[test]
fn test_contrib_humanize_ordinal() {
    assert_eq!(humanize::ordinal(1), "1st");
    assert_eq!(humanize::ordinal(2), "2nd");
    assert_eq!(humanize::ordinal(3), "3rd");
    assert_eq!(humanize::ordinal(4), "4th");
    assert_eq!(humanize::ordinal(11), "11th");
    assert_eq!(humanize::ordinal(12), "12th");
    assert_eq!(humanize::ordinal(13), "13th");
    assert_eq!(humanize::ordinal(21), "21st");
    assert_eq!(humanize::ordinal(22), "22nd");
    assert_eq!(humanize::ordinal(23), "23rd");
    assert_eq!(humanize::ordinal(101), "101st");
    assert_eq!(humanize::ordinal(102), "102nd");
    assert_eq!(humanize::ordinal(103), "103rd");
    assert_eq!(humanize::ordinal(111), "111th");
}

#[test]
fn test_contrib_humanize_filesize() {
    assert_eq!(humanize::filesizeformat(0), "0 bytes");
    assert_eq!(humanize::filesizeformat(50), "50 bytes");
    assert_eq!(humanize::filesizeformat(1023), "1023 bytes");
    assert_eq!(humanize::filesizeformat(1024), "1.0 KB");
    assert_eq!(humanize::filesizeformat(10 * 1024), "10.0 KB");
    assert_eq!(humanize::filesizeformat(1024 * 1024), "1.0 MB");
    assert_eq!(humanize::filesizeformat(1024 * 1024 * 1024), "1.0 GB");
}

#[test]
fn test_naturalday_today() {
    let now = Utc::now();
    let result = humanize::naturalday(&now);
    assert_eq!(result, "today");
}

#[test]
fn test_naturalday_yesterday() {
    let yesterday = Utc::now() - Duration::days(1);
    let result = humanize::naturalday(&yesterday);
    assert_eq!(result, "yesterday");
}

#[test]
fn test_naturalday_tomorrow() {
    let tomorrow = Utc::now() + Duration::days(1);
    let result = humanize::naturalday(&tomorrow);
    assert_eq!(result, "tomorrow");
}

#[test]
fn test_naturaltime_seconds() {
    let now = Utc::now();
    let few_seconds_ago = now - Duration::seconds(3);
    let result = humanize::naturaltime(&few_seconds_ago);
    assert!(result.contains("second"));
    assert!(result.contains("ago"));
}

#[test]
fn test_naturaltime_minutes() {
    let now = Utc::now();
    let minutes_ago = now - Duration::minutes(5);
    let result = humanize::naturaltime(&minutes_ago);
    assert!(result.contains("minute"));
    assert!(result.contains("ago"));
}

#[test]
fn test_naturaltime_hours() {
    let now = Utc::now();
    let hours_ago = now - Duration::hours(2);
    let result = humanize::naturaltime(&hours_ago);
    assert!(result.contains("hour"));
    assert!(result.contains("ago"));
}

#[test]
fn test_naturaltime_days() {
    let now = Utc::now();
    let days_ago = now - Duration::days(3);
    let result = humanize::naturaltime(&days_ago);
    assert!(result.contains("day"));
    assert!(result.contains("ago"));
}

#[test]
fn test_naturaltime_future() {
    let now = Utc::now();
    let future = now + Duration::minutes(10);
    let result = humanize::naturaltime(&future);
    assert!(result.contains("in"));
    assert!(result.contains("minute"));
}

#[test]
fn test_timesince_minutes() {
    let now = Utc::now();
    let past = now - Duration::minutes(30);
    let result = humanize::timesince(&past);
    assert!(result.contains("30 minutes"));
}

#[test]
fn test_timesince_hours() {
    let now = Utc::now();
    let past = now - Duration::hours(3);
    let result = humanize::timesince(&past);
    assert!(result.contains("hour"));
}

#[test]
fn test_negative_numbers() {
    assert_eq!(humanize::intcomma(-5000), "-5,000");
    assert_eq!(humanize::intword(-2000000), "-2 million");
}
