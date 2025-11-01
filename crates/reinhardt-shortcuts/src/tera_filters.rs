//! Custom Tera filters for template rendering
//!
//! This module provides Django-inspired filters for Tera templates,
//! extending the built-in Tera filters with additional functionality.

#[cfg(feature = "templates")]
use serde_json::Value;
#[cfg(feature = "templates")]
use std::collections::HashMap;
#[cfg(feature = "templates")]
use tera::{Filter, Result as TeraResult};

/// Truncate a string to a specified length with optional suffix
///
/// Similar to Django's `truncatechars` filter.
///
/// # Usage in templates
///
/// ```jinja
/// {{ long_text | truncate_chars(length=50, suffix="...") }}
/// ```
///
/// # Examples
///
/// ```
/// use reinhardt_shortcuts::tera_filters::TruncateCharsFilter;
/// use tera::Filter;
/// use serde_json::json;
/// use std::collections::HashMap;
///
/// let filter = TruncateCharsFilter;
/// let mut args = HashMap::new();
/// args.insert("length".to_string(), json!(10));
/// args.insert("suffix".to_string(), json!("..."));
///
/// let result = filter.filter(&json!("This is a long text"), &args).unwrap();
/// assert_eq!(result, json!("This is..."));
/// ```
#[cfg(feature = "templates")]
#[derive(Debug, Clone)]
pub struct TruncateCharsFilter;

#[cfg(feature = "templates")]
impl Filter for TruncateCharsFilter {
	fn filter(&self, value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
		let s = value
			.as_str()
			.ok_or_else(|| tera::Error::msg("Filter `truncate_chars` received non-string value"))?;

		let length =
			args.get("length").and_then(|v| v.as_u64()).ok_or_else(|| {
				tera::Error::msg("Filter `truncate_chars` requires `length` argument")
			})? as usize;

		let suffix = args.get("suffix").and_then(|v| v.as_str()).unwrap_or("...");

		if s.len() <= length {
			Ok(Value::String(s.to_string()))
		} else {
			let truncated = s
				.chars()
				.take(length.saturating_sub(suffix.len()))
				.collect::<String>();
			Ok(Value::String(format!("{}{}", truncated, suffix)))
		}
	}
}

/// Add a class to an HTML element
///
/// Similar to Django's `add_class` filter.
///
/// # Usage in templates
///
/// ```jinja
/// {{ field_html | add_class(class="form-control") }}
/// ```
#[cfg(feature = "templates")]
#[derive(Debug, Clone)]
pub struct AddClassFilter;

#[cfg(feature = "templates")]
impl Filter for AddClassFilter {
	fn filter(&self, value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
		let html = value
			.as_str()
			.ok_or_else(|| tera::Error::msg("Filter `add_class` received non-string value"))?;

		let class = args
			.get("class")
			.and_then(|v| v.as_str())
			.ok_or_else(|| tera::Error::msg("Filter `add_class` requires `class` argument"))?;

		// Simple implementation: add class attribute if not present, append if present
		if html.contains("class=\"") {
			Ok(Value::String(
				html.replace("class=\"", &format!("class=\"{} ", class)),
			))
		} else if html.contains("<input") || html.contains("<select") || html.contains("<textarea")
		{
			Ok(Value::String(
				html.replace(">", &format!(" class=\"{}\">", class)),
			))
		} else {
			Ok(Value::String(html.to_string()))
		}
	}
}

/// Format a number with thousand separators
///
/// Similar to Django's `intcomma` filter.
///
/// # Usage in templates
///
/// ```jinja
/// {{ number | intcomma }}
/// ```
///
/// # Examples
///
/// ```
/// use reinhardt_shortcuts::tera_filters::IntCommaFilter;
/// use tera::Filter;
/// use serde_json::json;
/// use std::collections::HashMap;
///
/// let filter = IntCommaFilter;
/// let args = HashMap::new();
///
/// let result = filter.filter(&json!(1234567), &args).unwrap();
/// assert_eq!(result, json!("1,234,567"));
/// ```
#[cfg(feature = "templates")]
#[derive(Debug, Clone)]
pub struct IntCommaFilter;

#[cfg(feature = "templates")]
impl Filter for IntCommaFilter {
	fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
		let num = if let Some(n) = value.as_i64() {
			n.to_string()
		} else if let Some(n) = value.as_f64() {
			format!("{:.0}", n)
		} else {
			return Err(tera::Error::msg("Filter `intcomma` requires a number"));
		};

		let mut result = String::new();
		let chars: Vec<char> = num.chars().collect();
		let len = chars.len();

		for (i, c) in chars.iter().enumerate() {
			result.push(*c);
			let remaining = len - i - 1;
			if remaining > 0 && remaining % 3 == 0 && c.is_ascii_digit() {
				result.push(',');
			}
		}

		Ok(Value::String(result))
	}
}

/// Pluralize a word based on count
///
/// Similar to Django's `pluralize` filter.
///
/// # Usage in templates
///
/// ```jinja
/// {{ count }} item{{ count | pluralize }}
/// {{ count }} cand{{ count | pluralize(singular="y", plural="ies") }}
/// ```
///
/// # Examples
///
/// ```
/// use reinhardt_shortcuts::tera_filters::PluralizeFilter;
/// use tera::Filter;
/// use serde_json::json;
/// use std::collections::HashMap;
///
/// let filter = PluralizeFilter;
/// let args = HashMap::new();
///
/// let result = filter.filter(&json!(0), &args).unwrap();
/// assert_eq!(result, json!("s"));
///
/// let result = filter.filter(&json!(1), &args).unwrap();
/// assert_eq!(result, json!(""));
///
/// let result = filter.filter(&json!(2), &args).unwrap();
/// assert_eq!(result, json!("s"));
/// ```
#[cfg(feature = "templates")]
#[derive(Debug, Clone)]
pub struct PluralizeFilter;

#[cfg(feature = "templates")]
impl Filter for PluralizeFilter {
	fn filter(&self, value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
		let count = value
			.as_i64()
			.ok_or_else(|| tera::Error::msg("Filter `pluralize` requires a number"))?;

		if count == 1 {
			if let Some(singular) = args.get("singular").and_then(|v| v.as_str()) {
				Ok(Value::String(singular.to_string()))
			} else {
				Ok(Value::String(String::new()))
			}
		} else {
			if let Some(plural) = args.get("plural").and_then(|v| v.as_str()) {
				Ok(Value::String(plural.to_string()))
			} else {
				Ok(Value::String("s".to_string()))
			}
		}
	}
}

/// Default value if the input is empty or falsy
///
/// Similar to Django's `default` filter.
///
/// # Usage in templates
///
/// ```jinja
/// {{ value | default(value="N/A") }}
/// ```
#[cfg(feature = "templates")]
#[derive(Debug, Clone)]
pub struct DefaultFilter;

#[cfg(feature = "templates")]
impl Filter for DefaultFilter {
	fn filter(&self, value: &Value, args: &HashMap<String, Value>) -> TeraResult<Value> {
		let is_empty = value.is_null()
			|| (value.is_string() && value.as_str().unwrap_or("").is_empty())
			|| (value.is_array() && value.as_array().unwrap_or(&vec![]).is_empty())
			|| (value.is_object() && value.as_object().unwrap_or(&Default::default()).is_empty());

		if is_empty {
			args.get("value")
				.cloned()
				.ok_or_else(|| tera::Error::msg("Filter `default` requires `value` argument"))
		} else {
			Ok(value.clone())
		}
	}
}

#[cfg(all(test, feature = "templates"))]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn test_truncate_chars_filter() {
		let filter = TruncateCharsFilter;
		let mut args = HashMap::new();
		args.insert("length".to_string(), json!(10));
		args.insert("suffix".to_string(), json!("..."));

		let result = filter
			.filter(&json!("This is a very long text"), &args)
			.unwrap();
		assert_eq!(result, json!("This is..."));

		let result = filter.filter(&json!("Short"), &args).unwrap();
		assert_eq!(result, json!("Short"));
	}

	#[test]
	fn test_intcomma_filter() {
		let filter = IntCommaFilter;
		let args = HashMap::new();

		let result = filter.filter(&json!(1234567), &args).unwrap();
		assert_eq!(result, json!("1,234,567"));

		let result = filter.filter(&json!(100), &args).unwrap();
		assert_eq!(result, json!("100"));

		let result = filter.filter(&json!(1000), &args).unwrap();
		assert_eq!(result, json!("1,000"));
	}

	#[test]
	fn test_pluralize_filter() {
		let filter = PluralizeFilter;
		let args = HashMap::new();

		let result = filter.filter(&json!(0), &args).unwrap();
		assert_eq!(result, json!("s"));

		let result = filter.filter(&json!(1), &args).unwrap();
		assert_eq!(result, json!(""));

		let result = filter.filter(&json!(2), &args).unwrap();
		assert_eq!(result, json!("s"));

		let result = filter.filter(&json!(100), &args).unwrap();
		assert_eq!(result, json!("s"));
	}

	#[test]
	fn test_pluralize_filter_with_custom_forms() {
		let filter = PluralizeFilter;
		let mut args = HashMap::new();
		args.insert("singular".to_string(), json!("y"));
		args.insert("plural".to_string(), json!("ies"));

		let result = filter.filter(&json!(1), &args).unwrap();
		assert_eq!(result, json!("y"));

		let result = filter.filter(&json!(2), &args).unwrap();
		assert_eq!(result, json!("ies"));
	}

	#[test]
	fn test_default_filter() {
		let filter = DefaultFilter;
		let mut args = HashMap::new();
		args.insert("value".to_string(), json!("N/A"));

		let result = filter.filter(&json!(null), &args).unwrap();
		assert_eq!(result, json!("N/A"));

		let result = filter.filter(&json!(""), &args).unwrap();
		assert_eq!(result, json!("N/A"));

		let result = filter.filter(&json!("Some value"), &args).unwrap();
		assert_eq!(result, json!("Some value"));
	}

	#[test]
	fn test_add_class_filter() {
		let filter = AddClassFilter;
		let mut args = HashMap::new();
		args.insert("class".to_string(), json!("form-control"));

		let result = filter
			.filter(&json!("<input type=\"text\">"), &args)
			.unwrap();
		assert!(result.as_str().unwrap().contains("class=\"form-control\""));
	}
}
