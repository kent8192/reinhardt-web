//! Parameter representation utilities for logging
//! Based on SQLAlchemy's _repr_params functionality

use serde_json::Value;

/// Configuration for parameter representation
pub struct ReprParamsConfig {
	pub max_chars: usize,
	pub batches: usize,
	pub is_multi: bool,
}

impl Default for ReprParamsConfig {
	fn default() -> Self {
		Self {
			max_chars: 300,
			batches: 10,
			is_multi: false,
		}
	}
}
/// Represent parameters in a truncated form suitable for logging
///
/// # Examples
///
/// ```
/// use reinhardt_utils::logging::params::{repr_params, ReprParamsConfig};
/// use serde_json::json;
///
// Simple parameter representation
/// let params = json!({"user_id": 42, "email": "test@example.com"});
/// let config = ReprParamsConfig::default();
/// let output = repr_params(&params, &config);
///
// Output includes the parameter values
/// assert!(output.contains("user_id"));
/// assert!(output.contains("42"));
/// ```
pub fn repr_params(params: &Value, config: &ReprParamsConfig) -> String {
	match params {
		Value::Array(arr) if config.is_multi && !arr.is_empty() => repr_multi_params(arr, config),
		Value::Array(arr) => repr_array(arr, config),
		Value::Object(obj) => repr_object(obj, config),
		_ => format!("{:?}", params),
	}
}

fn repr_multi_params(params: &[Value], config: &ReprParamsConfig) -> String {
	let total = params.len();

	if total <= config.batches {
		return format!("{:?}", params);
	}

	let show_count = config.batches / 2;
	let first_items: Vec<String> = params
		.iter()
		.take(show_count)
		.map(|v| format!("{:?}", v))
		.collect();
	let last_items: Vec<String> = params
		.iter()
		.rev()
		.take(show_count)
		.map(|v| format!("{:?}", v))
		.collect::<Vec<_>>()
		.into_iter()
		.rev()
		.collect();

	format!(
		"[{}  ... displaying {} of {} total bound parameter sets ...  {}]",
		first_items.join(", "),
		config.batches,
		total,
		last_items.join(", ")
	)
}

fn repr_array(arr: &[Value], config: &ReprParamsConfig) -> String {
	let repr = format!("{:?}", arr);

	if repr.len() <= config.max_chars {
		return repr;
	}

	let half_chars = config.max_chars / 2;
	let truncated_chars = repr.len() - config.max_chars;

	format!(
		"{}  ... ({} characters truncated) ...  {}",
		&repr[..half_chars],
		truncated_chars,
		&repr[repr.len() - half_chars..]
	)
}

fn repr_object(obj: &serde_json::Map<String, Value>, config: &ReprParamsConfig) -> String {
	let repr = format!("{:?}", obj);

	if repr.len() <= config.max_chars {
		return repr;
	}

	// For huge dicts, truncate from middle
	if obj.len() > 100 {
		let half_chars = config.max_chars / 2;
		let truncated_params = obj.len() - 10; // Show ~10 params

		format!(
			"{} ... {} parameters truncated ... {}",
			&repr[..half_chars],
			truncated_params,
			&repr[repr.len() - half_chars..]
		)
	} else {
		let half_chars = config.max_chars / 2;
		let truncated_chars = repr.len() - config.max_chars;

		format!(
			"{}  ... ({} characters truncated) ...  {}",
			&repr[..half_chars],
			truncated_chars,
			&repr[repr.len() - half_chars..]
		)
	}
}
/// Truncate a single parameter value if it's too long
///
/// # Examples
///
/// ```
/// use reinhardt_utils::logging::params::truncate_param;
///
// Short strings are returned unchanged
/// let short_text = "Hello";
/// assert_eq!(truncate_param(short_text, 100), "Hello");
///
// Long strings are truncated with indication
/// let long_text = "a".repeat(1000);
/// let truncated = truncate_param(&long_text, 50);
/// assert!(truncated.len() < 1000);
/// assert!(truncated.contains("characters truncated"));
/// ```
pub fn truncate_param(value: &str, max_chars: usize) -> String {
	if value.len() <= max_chars {
		return value.to_string();
	}

	let half_chars = max_chars / 2;
	let truncated_chars = value.len() - max_chars;

	format!(
		"{} ... ({} characters truncated) ... {}",
		&value[..half_chars],
		truncated_chars,
		&value[value.len() - half_chars..]
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn test_repr_params_large_list_of_dict() {
		let params: Vec<Value> = (0..100).map(|i| json!({"data": i.to_string()})).collect();

		let config = ReprParamsConfig {
			max_chars: 300,
			batches: 10,
			is_multi: true,
		};

		let result = repr_params(&Value::Array(params), &config);
		eprintln!("LARGE_LIST OUTPUT: {}", result);

		// Should show first few and last few with exact message format
		assert!(
			result.contains("displaying 10 of 100 total bound parameter sets"),
			"Expected truncation message not found in: {}",
			result
		);
		// First element should contain "data": "0"
		assert!(
			result.contains(r#""data""#) && result.contains(r#""0""#),
			"Expected first element with data: 0 not found in: {}",
			result
		);
		// Last element should contain "data": "99"
		assert!(
			result.contains(r#""99""#),
			"Expected last element with data: 99 not found in: {}",
			result
		);
		// Should start with "[" and end with "]"
		assert!(result.starts_with('[') && result.ends_with(']'));
	}

	#[test]
	fn test_repr_params_positional_array() {
		let params = json!([[1, 2, 3], 5]);

		let config = ReprParamsConfig {
			max_chars: 300,
			batches: 10,
			is_multi: false,
		};

		let result = repr_params(&params, &config);
		// Verify exact debug format output
		assert_eq!(
			result,
			"[Array [Number(1), Number(2), Number(3)], Number(5)]"
		);
	}

	#[test]
	fn test_repr_params_unknown_list() {
		let large_array: Vec<i32> = (0..300).collect();
		let params = json!([large_array, 5]);

		let config = ReprParamsConfig {
			max_chars: 80,
			batches: 10,
			is_multi: false,
		};

		let result = repr_params(&params, &config);
		// Should be truncated with exact message format
		assert!(
			result.contains("characters truncated"),
			"Expected truncation message not found in: {}",
			result
		);
		// Should have ellipsis pattern: "...  ... (N characters truncated) ...  ..."
		assert!(
			result.matches("...").count() >= 2,
			"Expected ellipsis pattern not found in: {}",
			result
		);
		// Should start with "[" (array format)
		assert!(
			result.starts_with('['),
			"Expected array start not found in: {}",
			result
		);
	}

	#[test]
	fn test_repr_params_named_dict() {
		let mut params = serde_json::Map::new();
		for i in 0..10 {
			params.insert(format!("key_{}", i), json!(i));
		}

		let config = ReprParamsConfig {
			max_chars: 300,
			batches: 10,
			is_multi: false,
		};

		let result = repr_params(&Value::Object(params.clone()), &config);
		// Should fit without truncation
		assert!(
			!result.contains("truncated"),
			"Unexpected truncation in: {}",
			result
		);
		// Should contain all keys
		for i in 0..10 {
			assert!(
				result.contains(&format!(r#""key_{}""#, i)),
				"Expected key_{}  not found in: {}",
				i,
				result
			);
		}
		// Should start with "{" (object format from Debug)
		assert!(
			result.starts_with('{'),
			"Expected object start not found in: {}",
			result
		);
	}

	#[test]
	fn test_repr_params_huge_named_dict() {
		let mut params = serde_json::Map::new();
		for i in 0..800 {
			params.insert(format!("key_{}", i), json!(i));
		}

		let config = ReprParamsConfig {
			max_chars: 1400,
			batches: 10,
			is_multi: false,
		};

		let result = repr_params(&Value::Object(params), &config);
		// Should be truncated with exact message format for huge dicts
		assert!(
			result.contains("parameters truncated"),
			"Expected parameters truncation message not found in: {}",
			result
		);
		// Should have ellipsis pattern: "... N parameters truncated ..."
		assert!(
			result.matches("...").count() >= 2,
			"Expected ellipsis pattern not found in: {}",
			result
		);
		// Should start with "{" (object format)
		assert!(
			result.starts_with('{'),
			"Expected object start not found in: {}",
			result
		);
	}

	#[test]
	fn test_repr_params_ismulti_named_dict() {
		let param: serde_json::Map<String, Value> =
			(0..10).map(|i| (format!("key_{}", i), json!(i))).collect();

		let params: Vec<Value> = (0..50).map(|_| Value::Object(param.clone())).collect();

		let config = ReprParamsConfig {
			max_chars: 80,
			batches: 5,
			is_multi: true,
		};

		let result = repr_params(&Value::Array(params), &config);
		// Should show truncation with exact message format
		assert!(
			result.contains("displaying 5 of 50 total bound parameter sets"),
			"Expected multi-batch truncation message not found in: {}",
			result
		);
		// Should start with "[" and end with "]"
		assert!(
			result.starts_with('[') && result.ends_with(']'),
			"Expected array format not found in: {}",
			result
		);
	}

	#[test]
	fn test_truncate_param() {
		let large_param = "a".repeat(5000);
		let result = truncate_param(&large_param, 298);

		// Should be truncated to approximately max_chars + overhead
		assert!(
			result.len() < 5000,
			"Expected truncated length, got: {}",
			result.len()
		);
		assert!(
			result.len() > 298,
			"Result should be longer than max_chars due to truncation message"
		);
		// Should contain exact truncation message format
		assert!(
			result.contains("characters truncated"),
			"Expected truncation message not found in: {}",
			result
		);
		// Should start and end with repeated 'a'
		assert!(
			result.starts_with("aaaa"),
			"Expected start pattern not found in: {}",
			result
		);
		assert!(
			result.ends_with("aaaa"),
			"Expected end pattern not found in: {}",
			result
		);
		// Should have ellipsis pattern: "aaa ... (N characters truncated) ... aaa"
		assert!(
			result.matches("...").count() == 2,
			"Expected exactly 2 ellipsis markers, found: {}",
			result.matches("...").count()
		);
	}

	#[test]
	fn test_truncate_param_small() {
		let small_param = "small";
		let result = truncate_param(small_param, 100);

		assert_eq!(result, "small");
	}
}
