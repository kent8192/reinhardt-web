//! Advanced template filters
//!
//! Provides additional filters for common template operations:
//! - String manipulation (truncate, slugify, title)
//! - Number formatting (filesizeformat, floatformat)
//! - List operations (first, last, join, slice)
//! - Date formatting (date, time, timesince)
//! - URL operations (urlencode, urlize)

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use tera::{Result as TeraResult, Value};

/// Truncate a string to a specified length
///
/// # Examples
///
/// ```
/// use tera::Value;
/// use std::collections::HashMap;
/// use reinhardt_templates::advanced_filters::truncate;
///
/// let value = Value::String("Hello World".to_string());
/// let mut args = HashMap::new();
/// args.insert("length".to_string(), Value::Number(5.into()));
/// assert_eq!(truncate(&value, &args).unwrap(), Value::String("He...".to_string()));
///
/// let value2 = Value::String("Hi".to_string());
/// let mut args2 = HashMap::new();
/// args2.insert("length".to_string(), Value::Number(5.into()));
/// assert_eq!(truncate(&value2, &args2).unwrap(), Value::String("Hi".to_string()));
/// ```
pub fn truncate(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
	let s = value
		.as_str()
		.ok_or_else(|| tera::Error::msg("truncate filter requires a string"))?;
	let length = args
		.get("length")
		.and_then(|v| v.as_u64())
		.ok_or_else(|| tera::Error::msg("truncate filter requires a 'length' parameter"))?
		as usize;

	let result = if s.len() <= length {
		s.to_string()
	} else {
		// Reserve 3 characters for "..."
		let actual_length = length.saturating_sub(3);
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
/// use tera::Value;
/// use std::collections::HashMap;
/// use reinhardt_templates::advanced_filters::slugify;
///
/// let value = Value::String("Hello World!".to_string());
/// let args = HashMap::new();
/// assert_eq!(slugify(&value, &args).unwrap(), Value::String("hello-world".to_string()));
///
/// let value2 = Value::String("Django REST Framework".to_string());
/// assert_eq!(slugify(&value2, &args).unwrap(), Value::String("django-rest-framework".to_string()));
/// ```
pub fn slugify(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
	let s = value
		.as_str()
		.ok_or_else(|| tera::Error::msg("slugify filter requires a string"))?;
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
/// use tera::Value;
/// use std::collections::HashMap;
/// use reinhardt_templates::advanced_filters::title;
///
/// let value = Value::String("hello world".to_string());
/// let args = HashMap::new();
/// assert_eq!(title(&value, &args).unwrap(), Value::String("Hello World".to_string()));
///
/// let value2 = Value::String("django-rest-framework".to_string());
/// assert_eq!(title(&value2, &args).unwrap(), Value::String("Django Rest Framework".to_string()));
/// ```
pub fn title(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
	let s = value
		.as_str()
		.ok_or_else(|| tera::Error::msg("title filter requires a string"))?;
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
/// use tera::Value;
/// use std::collections::HashMap;
/// use reinhardt_templates::advanced_filters::filesizeformat;
///
/// let value = Value::Number(1024.into());
/// let args = HashMap::new();
/// assert_eq!(filesizeformat(&value, &args).unwrap(), Value::String("1.00 KB".to_string()));
///
/// let value2 = Value::Number(1048576.into());
/// assert_eq!(filesizeformat(&value2, &args).unwrap(), Value::String("1.00 MB".to_string()));
/// ```
pub fn filesizeformat(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
	let bytes = value
		.as_i64()
		.ok_or_else(|| tera::Error::msg("filesizeformat filter requires a number"))?;
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
/// use tera::Value;
/// use std::collections::HashMap;
/// use reinhardt_templates::advanced_filters::floatformat;
///
/// let value = Value::Number(tera::Number::from_f64(3.14159).unwrap());
/// let mut args = HashMap::new();
/// args.insert("places".to_string(), Value::Number(2.into()));
/// assert_eq!(floatformat(&value, &args).unwrap(), Value::String("3.14".to_string()));
///
/// let value2 = Value::Number(tera::Number::from_f64(2.0).unwrap());
/// let mut args2 = HashMap::new();
/// args2.insert("places".to_string(), Value::Number(2.into()));
/// assert_eq!(floatformat(&value2, &args2).unwrap(), Value::String("2.00".to_string()));
/// ```
pub fn floatformat(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
	let num = value
		.as_f64()
		.ok_or_else(|| tera::Error::msg("floatformat filter requires a number"))?;
	let places = args
		.get("places")
		.and_then(|v| v.as_u64())
		.ok_or_else(|| tera::Error::msg("floatformat filter requires a 'places' parameter"))?
		as usize;

	Ok(Value::String(format!("{:.prec$}", num, prec = places)))
}

/// Get the first element of a list
///
/// # Examples
///
/// ```
/// use tera::Value;
/// use std::collections::HashMap;
/// use reinhardt_templates::advanced_filters::first;
///
/// let value = Value::Array(vec![
///     Value::String("a".to_string()),
///     Value::String("b".to_string()),
///     Value::String("c".to_string()),
/// ]);
/// let args = HashMap::new();
/// assert_eq!(first(&value, &args).unwrap(), Value::String("a".to_string()));
/// ```
pub fn first(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
	let list = value
		.as_array()
		.ok_or_else(|| tera::Error::msg("first filter requires an array"))?;
	list.first()
		.cloned()
		.ok_or_else(|| tera::Error::msg("List is empty"))
}

/// Get the last element of a list
///
/// # Examples
///
/// ```
/// use tera::Value;
/// use std::collections::HashMap;
/// use reinhardt_templates::advanced_filters::last;
///
/// let value = Value::Array(vec![
///     Value::String("a".to_string()),
///     Value::String("b".to_string()),
///     Value::String("c".to_string()),
/// ]);
/// let args = HashMap::new();
/// assert_eq!(last(&value, &args).unwrap(), Value::String("c".to_string()));
/// ```
pub fn last(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
	let list = value
		.as_array()
		.ok_or_else(|| tera::Error::msg("last filter requires an array"))?;
	list.last()
		.cloned()
		.ok_or_else(|| tera::Error::msg("List is empty"))
}

/// Join list elements with a separator
///
/// # Examples
///
/// ```
/// use tera::Value;
/// use std::collections::HashMap;
/// use reinhardt_templates::advanced_filters::join;
///
/// let value = Value::Array(vec![
///     Value::String("a".to_string()),
///     Value::String("b".to_string()),
///     Value::String("c".to_string()),
/// ]);
/// let mut args = HashMap::new();
/// args.insert("sep".to_string(), Value::String(", ".to_string()));
/// assert_eq!(join(&value, &args).unwrap(), Value::String("a, b, c".to_string()));
/// ```
pub fn join(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
	let list = value
		.as_array()
		.ok_or_else(|| tera::Error::msg("join filter requires an array"))?;
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
/// use tera::Value;
/// use std::collections::HashMap;
/// use reinhardt_templates::advanced_filters::urlencode;
///
/// let value = Value::String("hello world".to_string());
/// let args = HashMap::new();
/// assert_eq!(urlencode(&value, &args).unwrap(), Value::String("hello%20world".to_string()));
///
/// let value2 = Value::String("a+b=c".to_string());
/// assert_eq!(urlencode(&value2, &args).unwrap(), Value::String("a%2Bb%3Dc".to_string()));
/// ```
pub fn urlencode(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
	let s = value
		.as_str()
		.ok_or_else(|| tera::Error::msg("urlencode filter requires a string"))?;
	Ok(Value::String(urlencoding::encode(s).to_string()))
}

/// Calculate time difference from now
///
/// # Examples
///
/// ```
/// use tera::Value;
/// use std::collections::HashMap;
/// use reinhardt_templates::advanced_filters::timesince;
/// use chrono::{Utc, Duration};
///
/// let past = Utc::now() - Duration::hours(2);
/// let value = Value::String(past.to_rfc3339());
/// let args = HashMap::new();
/// let result = timesince(&value, &args).unwrap();
/// assert!(result.as_str().unwrap().contains("hour"));
/// ```
pub fn timesince(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
	// Assume value is a UTC timestamp string (ISO 8601 format)
	let dt_str = value
		.as_str()
		.ok_or_else(|| tera::Error::msg("timesince filter requires a string datetime"))?;
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
		format!("{} year{}", years, if years != 1 { "s" } else { "" })
	} else if days > 30 {
		let months = days / 30;
		format!("{} month{}", months, if months != 1 { "s" } else { "" })
	} else if days > 0 {
		format!("{} day{}", days, if days != 1 { "s" } else { "" })
	} else if hours > 0 {
		format!("{} hour{}", hours, if hours != 1 { "s" } else { "" })
	} else if minutes > 0 {
		format!("{} minute{}", minutes, if minutes != 1 { "s" } else { "" })
	} else {
		format!("{} second{}", seconds, if seconds != 1 { "s" } else { "" })
	};
	Ok(Value::String(result))
}

/// Default value if variable is empty or None
///
/// # Examples
///
/// ```
/// use tera::Value;
/// use std::collections::HashMap;
/// use reinhardt_templates::advanced_filters::default;
///
/// let value = Value::String("".to_string());
/// let mut args = HashMap::new();
/// args.insert("value".to_string(), Value::String("N/A".to_string()));
/// assert_eq!(default(&value, &args).unwrap(), Value::String("N/A".to_string()));
///
/// let value2 = Value::String("Hello".to_string());
/// assert_eq!(default(&value2, &args).unwrap(), Value::String("Hello".to_string()));
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
/// use tera::Value;
/// use std::collections::HashMap;
/// use reinhardt_templates::advanced_filters::wordcount;
///
/// let value = Value::String("hello world".to_string());
/// let args = HashMap::new();
/// let result = wordcount(&value, &args).unwrap();
/// assert_eq!(result.as_u64().unwrap(), 2);
///
/// let value2 = Value::String("one two three".to_string());
/// let result2 = wordcount(&value2, &args).unwrap();
/// assert_eq!(result2.as_u64().unwrap(), 3);
/// ```
pub fn wordcount(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
	let s = value
		.as_str()
		.ok_or_else(|| tera::Error::msg("wordcount filter requires a string"))?;
	let count = s.split_whitespace().count();
	Ok(Value::Number((count as u64).into()))
}

/// Add a value to a number
///
/// # Examples
///
/// ```
/// use tera::Value;
/// use std::collections::HashMap;
/// use reinhardt_templates::advanced_filters::add;
///
/// let value = Value::Number(5.into());
/// let mut args = HashMap::new();
/// args.insert("value".to_string(), Value::Number(3.into()));
/// let result = add(&value, &args).unwrap();
/// assert_eq!(result.as_i64().unwrap(), 8);
///
/// let value2 = Value::Number(10.into());
/// let mut args2 = HashMap::new();
/// args2.insert("value".to_string(), Value::Number((-5).into()));
/// let result2 = add(&value2, &args2).unwrap();
/// assert_eq!(result2.as_i64().unwrap(), 5);
/// ```
pub fn add(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
	let num = value
		.as_i64()
		.ok_or_else(|| tera::Error::msg("add filter requires a number"))?;
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
/// use tera::Value;
/// use std::collections::HashMap;
/// use reinhardt_templates::advanced_filters::pluralize;
///
/// let value = Value::Number(1.into());
/// let mut args = HashMap::new();
/// args.insert("suffix".to_string(), Value::String("s".to_string()));
/// assert_eq!(pluralize(&value, &args).unwrap(), Value::String("".to_string()));
///
/// let value2 = Value::Number(2.into());
/// let mut args2 = HashMap::new();
/// args2.insert("suffix".to_string(), Value::String("s".to_string()));
/// assert_eq!(pluralize(&value2, &args2).unwrap(), Value::String("s".to_string()));
///
/// let value3 = Value::Number(0.into());
/// let mut args3 = HashMap::new();
/// args3.insert("suffix".to_string(), Value::String("s".to_string()));
/// assert_eq!(pluralize(&value3, &args3).unwrap(), Value::String("s".to_string()));
/// ```
pub fn pluralize(value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
	let count = value
		.as_i64()
		.ok_or_else(|| tera::Error::msg("pluralize filter requires a number"))?;
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
	use tera::Number;

	#[test]
	fn test_truncate() {
		let value = Value::String("Hello World".to_string());
		let mut args = HashMap::new();
		args.insert("length".to_string(), Value::Number(5.into()));
		assert_eq!(
			truncate(&value, &args).unwrap(),
			Value::String("He...".to_string())
		);

		let value2 = Value::String("Hi".to_string());
		let mut args2 = HashMap::new();
		args2.insert("length".to_string(), Value::Number(10.into()));
		assert_eq!(
			truncate(&value2, &args2).unwrap(),
			Value::String("Hi".to_string())
		);

		let value3 = Value::String("Hello".to_string());
		let mut args3 = HashMap::new();
		args3.insert("length".to_string(), Value::Number(5.into()));
		assert_eq!(
			truncate(&value3, &args3).unwrap(),
			Value::String("Hello".to_string())
		);
	}

	#[test]
	fn test_slugify() {
		let value = Value::String("Hello World!".to_string());
		let args = HashMap::new();
		assert_eq!(
			slugify(&value, &args).unwrap(),
			Value::String("hello-world".to_string())
		);

		let value2 = Value::String("Django REST Framework".to_string());
		assert_eq!(
			slugify(&value2, &args).unwrap(),
			Value::String("django-rest-framework".to_string())
		);

		let value3 = Value::String("test___slug".to_string());
		assert_eq!(
			slugify(&value3, &args).unwrap(),
			Value::String("test-slug".to_string())
		);
	}

	#[test]
	fn test_title() {
		let value = Value::String("hello world".to_string());
		let args = HashMap::new();
		assert_eq!(
			title(&value, &args).unwrap(),
			Value::String("Hello World".to_string())
		);

		let value2 = Value::String("django-rest-framework".to_string());
		assert_eq!(
			title(&value2, &args).unwrap(),
			Value::String("Django Rest Framework".to_string())
		);
	}

	#[test]
	fn test_filesizeformat() {
		let value = Value::Number(1024.into());
		let args = HashMap::new();
		assert_eq!(
			filesizeformat(&value, &args).unwrap(),
			Value::String("1.00 KB".to_string())
		);

		let value2 = Value::Number(1048576.into());
		assert_eq!(
			filesizeformat(&value2, &args).unwrap(),
			Value::String("1.00 MB".to_string())
		);

		let value3 = Value::Number(512.into());
		assert_eq!(
			filesizeformat(&value3, &args).unwrap(),
			Value::String("512 B".to_string())
		);
	}

	#[test]
	fn test_floatformat() {
		let value = Value::Number(Number::from_f64(3.14159).unwrap());
		let mut args = HashMap::new();
		args.insert("places".to_string(), Value::Number(2.into()));
		assert_eq!(
			floatformat(&value, &args).unwrap(),
			Value::String("3.14".to_string())
		);

		let value2 = Value::Number(Number::from_f64(2.0).unwrap());
		let mut args2 = HashMap::new();
		args2.insert("places".to_string(), Value::Number(2.into()));
		assert_eq!(
			floatformat(&value2, &args2).unwrap(),
			Value::String("2.00".to_string())
		);

		let value3 = Value::Number(Number::from_f64(1.5).unwrap());
		let mut args3 = HashMap::new();
		args3.insert("places".to_string(), Value::Number(0.into()));
		assert_eq!(
			floatformat(&value3, &args3).unwrap(),
			Value::String("2".to_string())
		);
	}

	#[test]
	fn test_first_last() {
		let items = vec![
			Value::String("a".to_string()),
			Value::String("b".to_string()),
			Value::String("c".to_string()),
		];
		let value = Value::Array(items);
		let args = HashMap::new();
		assert_eq!(
			first(&value, &args).unwrap(),
			Value::String("a".to_string())
		);
		assert_eq!(last(&value, &args).unwrap(), Value::String("c".to_string()));
	}

	#[test]
	fn test_first_last_empty() {
		let items: Vec<Value> = vec![];
		let value = Value::Array(items);
		let args = HashMap::new();
		assert!(first(&value, &args).is_err());
		assert!(last(&value, &args).is_err());
	}

	#[test]
	fn test_join() {
		let items = vec![
			Value::String("a".to_string()),
			Value::String("b".to_string()),
			Value::String("c".to_string()),
		];
		let value = Value::Array(items);
		let mut args = HashMap::new();
		args.insert("sep".to_string(), Value::String(", ".to_string()));
		assert_eq!(
			join(&value, &args).unwrap(),
			Value::String("a, b, c".to_string())
		);

		let mut args2 = HashMap::new();
		args2.insert("sep".to_string(), Value::String("-".to_string()));
		assert_eq!(
			join(&value, &args2).unwrap(),
			Value::String("a-b-c".to_string())
		);
	}

	#[test]
	fn test_urlencode() {
		let value = Value::String("hello world".to_string());
		let args = HashMap::new();
		assert_eq!(
			urlencode(&value, &args).unwrap(),
			Value::String("hello%20world".to_string())
		);

		let value2 = Value::String("a+b=c".to_string());
		assert_eq!(
			urlencode(&value2, &args).unwrap(),
			Value::String("a%2Bb%3Dc".to_string())
		);
	}

	#[test]
	fn test_timesince() {
		let past = Utc::now() - Duration::hours(2);
		let value = Value::String(past.to_rfc3339());
		let args = HashMap::new();
		let result = timesince(&value, &args).unwrap();
		if let Value::String(s) = result {
			assert!(s.contains("hour"));
		} else {
			panic!("Expected String value");
		}

		let past_days = Utc::now() - Duration::days(5);
		let value2 = Value::String(past_days.to_rfc3339());
		let result = timesince(&value2, &args).unwrap();
		if let Value::String(s) = result {
			assert!(s.contains("day"));
		} else {
			panic!("Expected String value");
		}
	}

	#[test]
	fn test_default() {
		let value = Value::String("".to_string());
		let mut args = HashMap::new();
		args.insert("value".to_string(), Value::String("N/A".to_string()));
		assert_eq!(
			default(&value, &args).unwrap(),
			Value::String("N/A".to_string())
		);

		let value2 = Value::String("Hello".to_string());
		assert_eq!(
			default(&value2, &args).unwrap(),
			Value::String("Hello".to_string())
		);
	}

	#[test]
	fn test_wordcount() {
		let value = Value::String("hello world".to_string());
		let args = HashMap::new();
		assert_eq!(wordcount(&value, &args).unwrap(), Value::Number(2.into()));

		let value2 = Value::String("one two three".to_string());
		assert_eq!(wordcount(&value2, &args).unwrap(), Value::Number(3.into()));

		let value3 = Value::String("".to_string());
		assert_eq!(wordcount(&value3, &args).unwrap(), Value::Number(0.into()));
	}

	#[test]
	fn test_add() {
		let value = Value::Number(5.into());
		let mut args = HashMap::new();
		args.insert("value".to_string(), Value::Number(3.into()));
		assert_eq!(add(&value, &args).unwrap(), Value::Number(8.into()));

		let value2 = Value::Number(10.into());
		let mut args2 = HashMap::new();
		args2.insert("value".to_string(), Value::Number((-5).into()));
		assert_eq!(add(&value2, &args2).unwrap(), Value::Number(5.into()));

		let value3 = Value::Number((-3).into());
		let mut args3 = HashMap::new();
		args3.insert("value".to_string(), Value::Number((-2).into()));
		assert_eq!(add(&value3, &args3).unwrap(), Value::Number((-5).into()));
	}

	#[test]
	fn test_pluralize() {
		let value = Value::Number(1.into());
		let mut args = HashMap::new();
		args.insert("suffix".to_string(), Value::String("s".to_string()));
		assert_eq!(
			pluralize(&value, &args).unwrap(),
			Value::String("".to_string())
		);

		let value2 = Value::Number(2.into());
		assert_eq!(
			pluralize(&value2, &args).unwrap(),
			Value::String("s".to_string())
		);

		let value3 = Value::Number(0.into());
		assert_eq!(
			pluralize(&value3, &args).unwrap(),
			Value::String("s".to_string())
		);
	}
}
