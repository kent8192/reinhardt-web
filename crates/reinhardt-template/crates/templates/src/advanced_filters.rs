//! Advanced template filters
//!
//! Provides additional filters for common template operations:
//! - String manipulation (truncate, slugify, title)
//! - Number formatting (filesizeformat, floatformat)
//! - List operations (first, last, join, slice)
//! - Date formatting (date, time, timesince)
//! - URL operations (urlencode, urlize)

use std::collections::HashMap;
use tera::{Result as TeraResult, Value};
use chrono::{DateTime, Duration, Utc};

/// Truncate a string to a specified length
///
/// # Examples
///
/// ```
/// use reinhardt_templates::truncate_filter;
///
/// assert_eq!(truncate_filter("Hello World", 5).unwrap(), "He...");
/// assert_eq!(truncate_filter("Hi", 5).unwrap(), "Hi");
/// ```
pub fn truncate(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| {
        tera::Error::msg("truncate filter requires a string")
    })?;
    let length = args
        .get("length")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| tera::Error::msg("truncate filter requires a 'length' parameter"))? as usize;

    let result = if s.len() <= length {
        s.to_string()
    } else {
        // Reserve 3 characters for "..."
        let actual_length = if length >= 3 { length - 3 } else { 0 };
        let truncated = s.chars().take(actual_length).collect::<String>();
        format!("{}...", truncated)
    };
    Ok(Value::String(result))
}

/// Convert a string to a URL-friendly slug
///
/// # Examples
///
/// ```
/// use reinhardt_templates::slugify;
///
/// assert_eq!(slugify("Hello World!").unwrap(), "hello-world");
/// assert_eq!(slugify("Django REST Framework").unwrap(), "django-rest-framework");
/// ```
pub fn slugify(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| {
        tera::Error::msg("slugify filter requires a string")
    })?;
    let slug = s
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c.is_whitespace() || c == '-' || c == '_' {
                '-'
            } else {
                '\0'
            }
        })
        .filter(|&c| c != '\0')
        .collect::<String>();

    // Remove consecutive dashes
    let slug = slug
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    Ok(Value::String(slug))
}

/// Convert a string to title case
///
/// # Examples
///
/// ```
/// use reinhardt_templates::title_filter;
///
/// assert_eq!(title_filter("hello world").unwrap(), "Hello World");
/// assert_eq!(title_filter("django-rest-framework").unwrap(), "Django Rest Framework");
/// ```
pub fn title(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| {
        tera::Error::msg("title filter requires a string")
    })?;
    let result = s
        .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    Ok(Value::String(result))
}

/// Format a file size in human-readable format
///
/// # Examples
///
/// ```
/// use reinhardt_templates::filesizeformat;
///
/// assert_eq!(filesizeformat(1024).unwrap(), "1.00 KB");
/// assert_eq!(filesizeformat(1048576).unwrap(), "1.00 MB");
/// ```
pub fn filesizeformat(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let bytes = value.as_i64().ok_or_else(|| {
        tera::Error::msg("filesizeformat filter requires a number")
    })?;
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    let result = if unit_index == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    };
    Ok(Value::String(result))
}

/// Format a float with specified decimal places
///
/// # Examples
///
/// ```
/// use reinhardt_templates::floatformat;
///
/// assert_eq!(floatformat(3.14159, 2).unwrap(), "3.14");
/// assert_eq!(floatformat(2.0, 2).unwrap(), "2.00");
/// ```
pub fn floatformat(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    let num = value.as_f64().ok_or_else(|| {
        tera::Error::msg("floatformat filter requires a number")
    })?;
    let places = args
        .get("places")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| tera::Error::msg("floatformat filter requires a 'places' parameter"))? as usize;

    Ok(Value::String(format!("{:.prec$}", num, prec = places)))
}

/// Get the first element of a list
///
/// # Examples
///
/// ```
/// use reinhardt_templates::first;
///
/// let items = vec!["a", "b", "c"];
/// assert_eq!(first(&items).unwrap(), "a");
/// ```
pub fn first(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let list = value.as_array().ok_or_else(|| {
        tera::Error::msg("first filter requires an array")
    })?;
    list.first()
        .cloned()
        .ok_or_else(|| tera::Error::msg("List is empty"))
}

/// Get the last element of a list
///
/// # Examples
///
/// ```
/// use reinhardt_templates::last;
///
/// let items = vec!["a", "b", "c"];
/// assert_eq!(last(&items).unwrap(), "c");
/// ```
pub fn last(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let list = value.as_array().ok_or_else(|| {
        tera::Error::msg("last filter requires an array")
    })?;
    list.last()
        .cloned()
        .ok_or_else(|| tera::Error::msg("List is empty"))
}

/// Join list elements with a separator
///
/// # Examples
///
/// ```
/// use reinhardt_templates::join;
///
/// let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
/// assert_eq!(join(&items, ", ").unwrap(), "a, b, c");
/// ```
pub fn join(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    let list = value.as_array().ok_or_else(|| {
        tera::Error::msg("join filter requires an array")
    })?;
    let separator = args
        .get("sep")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("join filter requires a 'sep' parameter"))?;

    let strings: Result<Vec<String>, tera::Error> = list
        .iter()
        .map(|v| {
            if let Some(s) = v.as_str() {
                Ok(s.to_string())
            } else {
                Ok(v.to_string())
            }
        })
        .collect();

    Ok(Value::String(strings?.join(separator)))
}

/// URL-encode a string
///
/// # Examples
///
/// ```
/// use reinhardt_templates::urlencode;
///
/// assert_eq!(urlencode("hello world").unwrap(), "hello%20world");
/// assert_eq!(urlencode("a+b=c").unwrap(), "a%2Bb%3Dc");
/// ```
pub fn urlencode(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| {
        tera::Error::msg("urlencode filter requires a string")
    })?;
    Ok(Value::String(urlencoding::encode(s).to_string()))
}

/// Calculate time difference from now
///
/// # Examples
///
/// ```
/// use reinhardt_templates::timesince;
/// use chrono::{Utc, Duration};
///
/// let past = Utc::now() - Duration::hours(2);
/// let result = timesince(&past).unwrap();
/// assert!(result.contains("hour"));
/// ```
pub fn timesince(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    // Assume value is a UTC timestamp string (ISO 8601 format)
    let dt_str = value.as_str().ok_or_else(|| {
        tera::Error::msg("timesince filter requires a string datetime")
    })?;
    let dt = DateTime::parse_from_rfc3339(dt_str)
        .map_err(|e| tera::Error::msg(format!("Invalid datetime format: {}", e)))?
        .with_timezone(&Utc);

    let now = Utc::now();
    let duration = now.signed_duration_since(dt);

    if duration < Duration::zero() {
        return Ok(Value::String("in the future".to_string()));
    }

    let seconds = duration.num_seconds();
    let minutes = duration.num_minutes();
    let hours = duration.num_hours();
    let days = duration.num_days();

    let result = if days > 365 {
        let years = days / 365;
        format!(
            "{} year{}",
            years,
            if years != 1 { "s" } else { "" }
        )
    } else if days > 30 {
        let months = days / 30;
        format!(
            "{} month{}",
            months,
            if months != 1 { "s" } else { "" }
        )
    } else if days > 0 {
        format!("{} day{}", days, if days != 1 { "s" } else { "" })
    } else if hours > 0 {
        format!(
            "{} hour{}",
            hours,
            if hours != 1 { "s" } else { "" }
        )
    } else if minutes > 0 {
        format!(
            "{} minute{}",
            minutes,
            if minutes != 1 { "s" } else { "" }
        )
    } else {
        format!(
            "{} second{}",
            seconds,
            if seconds != 1 { "s" } else { "" }
        )
    };
    Ok(Value::String(result))
}

/// Default value if variable is empty or None
///
/// # Examples
///
/// ```
/// use reinhardt_templates::default;
///
/// assert_eq!(default("", "N/A").unwrap(), "N/A");
/// assert_eq!(default("Hello", "N/A").unwrap(), "Hello");
/// ```
pub fn default(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().unwrap_or("");
    let default_value = args
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("default filter requires a 'value' parameter"))?;

    let result = if s.is_empty() {
        default_value.to_string()
    } else {
        s.to_string()
    };
    Ok(Value::String(result))
}

/// Word count
///
/// # Examples
///
/// ```
/// use reinhardt_templates::wordcount;
///
/// assert_eq!(wordcount("hello world").unwrap(), "2");
/// assert_eq!(wordcount("one two three").unwrap(), "3");
/// ```
pub fn wordcount(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let s = value.as_str().ok_or_else(|| {
        tera::Error::msg("wordcount filter requires a string")
    })?;
    let count = s.split_whitespace().count();
    Ok(Value::Number((count as u64).into()))
}

/// Add a value to a number
///
/// # Examples
///
/// ```
/// use reinhardt_templates::add;
///
/// assert_eq!(add(5, 3).unwrap(), 8);
/// assert_eq!(add(10, -5).unwrap(), 5);
/// ```
pub fn add(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    let num = value.as_i64().ok_or_else(|| {
        tera::Error::msg("add filter requires a number")
    })?;
    let arg = args
        .get("value")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| tera::Error::msg("add filter requires a 'value' parameter"))?;

    Ok(Value::Number((num + arg).into()))
}

/// Pluralize a word based on count
///
/// # Examples
///
/// ```
/// use reinhardt_templates::pluralize;
///
/// assert_eq!(pluralize(1, "s").unwrap(), "");
/// assert_eq!(pluralize(2, "s").unwrap(), "s");
/// assert_eq!(pluralize(0, "s").unwrap(), "s");
/// ```
pub fn pluralize(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
    let count = value.as_i64().ok_or_else(|| {
        tera::Error::msg("pluralize filter requires a number")
    })?;
    let suffix = args
        .get("suffix")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("pluralize filter requires a 'suffix' parameter"))?;

    let result = if count == 1 {
        String::new()
    } else {
        suffix.to_string()
    };
    Ok(Value::String(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("Hello World", 5).unwrap(), "He...");
        assert_eq!(truncate("Hi", 10).unwrap(), "Hi");
        assert_eq!(truncate("Hello", 5).unwrap(), "Hello");
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World!").unwrap(), "hello-world");
        assert_eq!(
            slugify("Django REST Framework").unwrap(),
            "django-rest-framework"
        );
        assert_eq!(slugify("test___slug").unwrap(), "test-slug");
    }

    #[test]
    fn test_title() {
        assert_eq!(title("hello world").unwrap(), "Hello World");
        assert_eq!(
            title("django-rest-framework").unwrap(),
            "Django Rest Framework"
        );
    }

    #[test]
    fn test_filesizeformat() {
        assert_eq!(filesizeformat(1024).unwrap(), "1.00 KB");
        assert_eq!(filesizeformat(1048576).unwrap(), "1.00 MB");
        assert_eq!(filesizeformat(512).unwrap(), "512 B");
    }

    #[test]
    fn test_floatformat() {
        assert_eq!(floatformat(3.14159, 2).unwrap(), "3.14");
        assert_eq!(floatformat(2.0, 2).unwrap(), "2.00");
        assert_eq!(floatformat(1.5, 0).unwrap(), "2");
    }

    #[test]
    fn test_first_last() {
        let items = vec!["a", "b", "c"];
        assert_eq!(first(&items).unwrap(), "a");
        assert_eq!(last(&items).unwrap(), "c");
    }

    #[test]
    fn test_first_last_empty() {
        let items: Vec<String> = vec![];
        assert!(first(&items).is_err());
        assert!(last(&items).is_err());
    }

    #[test]
    fn test_join() {
        let items = vec!["a", "b", "c"];
        assert_eq!(join(&items, ", ").unwrap(), "a, b, c");
        assert_eq!(join(&items, "-").unwrap(), "a-b-c");
    }

    #[test]
    fn test_urlencode() {
        assert_eq!(urlencode("hello world").unwrap(), "hello%20world");
        assert_eq!(urlencode("a+b=c").unwrap(), "a%2Bb%3Dc");
    }

    #[test]
    fn test_timesince() {
        let past = Utc::now() - Duration::hours(2);
        let result = timesince(&past).unwrap();
        assert!(result.contains("hour"));

        let past_days = Utc::now() - Duration::days(5);
        let result = timesince(&past_days).unwrap();
        assert!(result.contains("day"));
    }

    #[test]
    fn test_default() {
        assert_eq!(default("", "N/A").unwrap(), "N/A");
        assert_eq!(default("Hello", "N/A").unwrap(), "Hello");
    }

    #[test]
    fn test_wordcount() {
        assert_eq!(wordcount("hello world").unwrap(), "2");
        assert_eq!(wordcount("one two three").unwrap(), "3");
        assert_eq!(wordcount("").unwrap(), "0");
    }

    #[test]
    fn test_add() {
        assert_eq!(add(5, 3).unwrap(), 8);
        assert_eq!(add(10, -5).unwrap(), 5);
        assert_eq!(add(-3, -2).unwrap(), -5);
    }

    #[test]
    fn test_pluralize() {
        assert_eq!(pluralize(1, "s").unwrap(), "");
        assert_eq!(pluralize(2, "s").unwrap(), "s");
        assert_eq!(pluralize(0, "s").unwrap(), "s");
    }
}
