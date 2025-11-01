//! Custom Tera functions (tags) for template rendering
//!
//! This module provides Django-inspired custom functions for Tera templates.
//! In Tera, these are called "functions" but serve a similar purpose to Django template tags.

#[cfg(feature = "templates")]
use serde_json::Value;
#[cfg(feature = "templates")]
use std::collections::HashMap;
#[cfg(feature = "templates")]
use tera::{Function, Result as TeraResult};

/// Generate a range of numbers
///
/// Similar to Django's `{% for i in range(10) %}` tag.
///
/// # Usage in templates
///
/// ```jinja
/// {% for i in range(start=0, end=10) %}
///   {{ i }}
/// {% endfor %}
/// ```
///
/// # Examples
///
/// ```
/// use reinhardt_shortcuts::tera_functions::RangeFunction;
/// use tera::Function;
/// use serde_json::json;
/// use std::collections::HashMap;
///
/// let func = RangeFunction;
/// let mut args = HashMap::new();
/// args.insert("start".to_string(), json!(0));
/// args.insert("end".to_string(), json!(5));
///
/// let result = func.call(&args).unwrap();
/// assert_eq!(result, json!([0, 1, 2, 3, 4]));
/// ```
#[cfg(feature = "templates")]
#[derive(Debug, Clone)]
pub struct RangeFunction;

#[cfg(feature = "templates")]
impl Function for RangeFunction {
	fn call(&self, args: &HashMap<String, Value>) -> TeraResult<Value> {
		let start = args.get("start").and_then(|v| v.as_i64()).unwrap_or(0);

		let end = args
			.get("end")
			.and_then(|v| v.as_i64())
			.ok_or_else(|| tera::Error::msg("Function `range` requires `end` argument"))?;

		let step = args.get("step").and_then(|v| v.as_i64()).unwrap_or(1);

		if step == 0 {
			return Err(tera::Error::msg("Function `range` step cannot be 0"));
		}

		let mut result = Vec::new();
		let mut current = start;

		if step > 0 {
			while current < end {
				result.push(Value::Number(current.into()));
				current += step;
			}
		} else {
			while current > end {
				result.push(Value::Number(current.into()));
				current += step;
			}
		}

		Ok(Value::Array(result))
	}
}

/// Get the current date and time
///
/// Similar to Django's `{% now %}` tag.
///
/// # Usage in templates
///
/// ```jinja
/// {{ now(format="%Y-%m-%d %H:%M:%S") }}
/// ```
#[cfg(feature = "templates")]
#[derive(Debug, Clone)]
pub struct NowFunction;

#[cfg(feature = "templates")]
impl Function for NowFunction {
	fn call(&self, args: &HashMap<String, Value>) -> TeraResult<Value> {
		let format = args
			.get("format")
			.and_then(|v| v.as_str())
			.unwrap_or("%Y-%m-%d %H:%M:%S");

		let now = chrono::Local::now();
		let formatted = now.format(format).to_string();

		Ok(Value::String(formatted))
	}
}

/// Cycle through values
///
/// Similar to Django's `{% cycle %}` tag.
///
/// # Usage in templates
///
/// ```jinja
/// {% for item in items %}
///   <div class="{{ cycle(values=["odd", "even"], index=loop.index0) }}">
///     {{ item }}
///   </div>
/// {% endfor %}
/// ```
///
/// # Examples
///
/// ```
/// use reinhardt_shortcuts::tera_functions::CycleFunction;
/// use tera::Function;
/// use serde_json::json;
/// use std::collections::HashMap;
///
/// let func = CycleFunction;
/// let mut args = HashMap::new();
/// args.insert("values".to_string(), json!(["odd", "even"]));
/// args.insert("index".to_string(), json!(0));
///
/// let result = func.call(&args).unwrap();
/// assert_eq!(result, json!("odd"));
///
/// args.insert("index".to_string(), json!(1));
/// let result = func.call(&args).unwrap();
/// assert_eq!(result, json!("even"));
/// ```
#[cfg(feature = "templates")]
#[derive(Debug, Clone)]
pub struct CycleFunction;

#[cfg(feature = "templates")]
impl Function for CycleFunction {
	fn call(&self, args: &HashMap<String, Value>) -> TeraResult<Value> {
		let values = args
			.get("values")
			.and_then(|v| v.as_array())
			.ok_or_else(|| tera::Error::msg("Function `cycle` requires `values` array argument"))?;

		let index = args
			.get("index")
			.and_then(|v| v.as_u64())
			.ok_or_else(|| tera::Error::msg("Function `cycle` requires `index` argument"))?
			as usize;

		if values.is_empty() {
			return Err(tera::Error::msg(
				"Function `cycle` requires non-empty `values` array",
			));
		}

		let cycle_index = index % values.len();
		Ok(values[cycle_index].clone())
	}
}

/// Generate a static file URL
///
/// Similar to Django's `{% static %}` tag.
///
/// # Usage in templates
///
/// ```jinja
/// <img src="{{ static(path="images/logo.png") }}">
/// ```
#[cfg(feature = "templates")]
#[derive(Debug, Clone)]
pub struct StaticFunction {
	pub static_url: String,
}

#[cfg(feature = "templates")]
impl StaticFunction {
	pub fn new(static_url: String) -> Self {
		Self { static_url }
	}
}

#[cfg(feature = "templates")]
impl Function for StaticFunction {
	fn call(&self, args: &HashMap<String, Value>) -> TeraResult<Value> {
		let path = args
			.get("path")
			.and_then(|v| v.as_str())
			.ok_or_else(|| tera::Error::msg("Function `static` requires `path` argument"))?;

		let url = format!(
			"{}/{}",
			self.static_url.trim_end_matches('/'),
			path.trim_start_matches('/')
		);
		Ok(Value::String(url))
	}
}

/// Generate a URL from a route name
///
/// Similar to Django's `{% url %}` tag.
///
/// # Usage in templates
///
/// ```jinja
/// <a href="{{ url(name="user_profile", id=user.id) }}">Profile</a>
/// ```
#[cfg(feature = "templates")]
#[derive(Debug, Clone)]
pub struct UrlFunction;

#[cfg(feature = "templates")]
impl Function for UrlFunction {
	fn call(&self, args: &HashMap<String, Value>) -> TeraResult<Value> {
		let name = args
			.get("name")
			.and_then(|v| v.as_str())
			.ok_or_else(|| tera::Error::msg("Function `url` requires `name` argument"))?;

		// For now, return a placeholder URL with the route name
		// In a real implementation, this would use the router to generate URLs
		let mut url = format!("/{}", name);

		// Add query parameters if provided
		for (key, value) in args.iter() {
			if key != "name" {
				if let Some(v) = value.as_str() {
					url = url.replace(&format!("{{{}}}", key), v);
				} else if let Some(v) = value.as_i64() {
					url = url.replace(&format!("{{{}}}", key), &v.to_string());
				}
			}
		}

		Ok(Value::String(url))
	}
}

#[cfg(all(test, feature = "templates"))]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn test_range_function() {
		let func = RangeFunction;
		let mut args = HashMap::new();
		args.insert("start".to_string(), json!(0));
		args.insert("end".to_string(), json!(5));

		let result = func.call(&args).unwrap();
		assert_eq!(result, json!([0, 1, 2, 3, 4]));
	}

	#[test]
	fn test_range_function_with_step() {
		let func = RangeFunction;
		let mut args = HashMap::new();
		args.insert("start".to_string(), json!(0));
		args.insert("end".to_string(), json!(10));
		args.insert("step".to_string(), json!(2));

		let result = func.call(&args).unwrap();
		assert_eq!(result, json!([0, 2, 4, 6, 8]));
	}

	#[test]
	fn test_range_function_negative_step() {
		let func = RangeFunction;
		let mut args = HashMap::new();
		args.insert("start".to_string(), json!(10));
		args.insert("end".to_string(), json!(0));
		args.insert("step".to_string(), json!(-2));

		let result = func.call(&args).unwrap();
		assert_eq!(result, json!([10, 8, 6, 4, 2]));
	}

	#[test]
	fn test_cycle_function() {
		let func = CycleFunction;
		let mut args = HashMap::new();
		args.insert("values".to_string(), json!(["odd", "even"]));

		args.insert("index".to_string(), json!(0));
		let result = func.call(&args).unwrap();
		assert_eq!(result, json!("odd"));

		args.insert("index".to_string(), json!(1));
		let result = func.call(&args).unwrap();
		assert_eq!(result, json!("even"));

		args.insert("index".to_string(), json!(2));
		let result = func.call(&args).unwrap();
		assert_eq!(result, json!("odd"));

		args.insert("index".to_string(), json!(3));
		let result = func.call(&args).unwrap();
		assert_eq!(result, json!("even"));
	}

	#[test]
	fn test_static_function() {
		let func = StaticFunction::new("/static".to_string());
		let mut args = HashMap::new();
		args.insert("path".to_string(), json!("images/logo.png"));

		let result = func.call(&args).unwrap();
		assert_eq!(result, json!("/static/images/logo.png"));
	}

	#[test]
	fn test_url_function() {
		let func = UrlFunction;
		let mut args = HashMap::new();
		args.insert("name".to_string(), json!("user_profile"));
		args.insert("id".to_string(), json!(42));

		let result = func.call(&args).unwrap();
		// Should replace {id} placeholder
		assert!(result.as_str().unwrap().contains("user_profile"));
	}

	#[test]
	fn test_now_function() {
		let func = NowFunction;
		let mut args = HashMap::new();
		args.insert("format".to_string(), json!("%Y-%m-%d"));

		let result = func.call(&args).unwrap();
		let date_str = result.as_str().unwrap();

		// Check that it matches YYYY-MM-DD format
		assert!(date_str.len() == 10);
		assert!(date_str.contains('-'));
	}
}
