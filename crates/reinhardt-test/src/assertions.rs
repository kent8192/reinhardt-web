//! Additional assertion helpers for testing

use http::StatusCode;
use serde_json::Value;
/// Assert that JSON contains a field with a specific value
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_json_field_eq;
/// use serde_json::json;
///
/// let data = json!({"name": "John", "age": 30});
/// assert_json_field_eq(&data, "name", &json!("John"));
/// ```
pub fn assert_json_field_eq(json: &Value, field: &str, expected: &Value) {
	let actual = json.get(field);
	assert_eq!(
		actual,
		Some(expected),
		"Expected field '{}' to equal {:?}, got {:?}",
		field,
		expected,
		actual
	);
}
/// Assert that JSON contains a field
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_json_has_field;
/// use serde_json::json;
///
/// let data = json!({"name": "John", "age": 30});
/// assert_json_has_field(&data, "name");
/// ```
pub fn assert_json_has_field(json: &Value, field: &str) {
	assert!(
		json.get(field).is_some(),
		"Expected JSON to have field '{}'",
		field
	);
}
/// Assert that JSON does not contain a field
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_json_missing_field;
/// use serde_json::json;
///
/// let data = json!({"name": "John"});
/// assert_json_missing_field(&data, "age");
/// ```
pub fn assert_json_missing_field(json: &Value, field: &str) {
	assert!(
		json.get(field).is_none(),
		"Expected JSON to not have field '{}'",
		field
	);
}
/// Assert that JSON array has a specific length
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_json_array_len;
/// use serde_json::json;
///
/// let data = json!([1, 2, 3]);
/// assert_json_array_len(&data, 3);
/// ```
pub fn assert_json_array_len(json: &Value, expected_len: usize) {
	if let Value::Array(arr) = json {
		assert_eq!(
			arr.len(),
			expected_len,
			"Expected array length {}, got {}",
			expected_len,
			arr.len()
		);
	} else {
		panic!("Expected JSON array, got {:?}", json);
	}
}
/// Assert that JSON array is empty
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_json_array_empty;
/// use serde_json::json;
///
/// let data = json!([]);
/// assert_json_array_empty(&data);
/// ```
pub fn assert_json_array_empty(json: &Value) {
	assert_json_array_len(json, 0);
}
/// Assert that JSON array is not empty
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_json_array_not_empty;
/// use serde_json::json;
///
/// let data = json!([1, 2, 3]);
/// assert_json_array_not_empty(&data);
/// ```
pub fn assert_json_array_not_empty(json: &Value) {
	if let Value::Array(arr) = json {
		assert!(!arr.is_empty(), "Expected non-empty array");
	} else {
		panic!("Expected JSON array, got {:?}", json);
	}
}
/// Assert that JSON array contains a value
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_json_array_contains;
/// use serde_json::json;
///
/// let data = json!([1, 2, 3]);
/// assert_json_array_contains(&data, &json!(2));
/// ```
pub fn assert_json_array_contains(json: &Value, expected: &Value) {
	if let Value::Array(arr) = json {
		assert!(
			arr.contains(expected),
			"Expected array to contain {:?}, got {:?}",
			expected,
			arr
		);
	} else {
		panic!("Expected JSON array, got {:?}", json);
	}
}
/// Assert that JSON matches a pattern (subset matching)
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_json_matches;
/// use serde_json::json;
///
/// let actual = json!({"name": "John", "age": 30, "city": "NYC"});
/// let pattern = json!({"name": "John", "age": 30});
/// assert_json_matches(&actual, &pattern);
/// ```
pub fn assert_json_matches(actual: &Value, pattern: &Value) {
	match (actual, pattern) {
		(Value::Object(actual_map), Value::Object(pattern_map)) => {
			for (key, pattern_value) in pattern_map {
				let actual_value = actual_map.get(key);
				assert!(
					actual_value.is_some(),
					"Expected field '{}' in {:?}",
					key,
					actual_map
				);
				assert_json_matches(actual_value.unwrap(), pattern_value);
			}
		}
		(Value::Array(actual_arr), Value::Array(pattern_arr)) => {
			assert_eq!(actual_arr.len(), pattern_arr.len(), "Array length mismatch");
			for (actual_item, pattern_item) in actual_arr.iter().zip(pattern_arr.iter()) {
				assert_json_matches(actual_item, pattern_item);
			}
		}
		_ => {
			assert_eq!(actual, pattern, "Value mismatch");
		}
	}
}
/// Assert that response body contains text
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_contains;
///
/// let text = "Hello, World!";
/// assert_contains(text, "World");
/// ```
pub fn assert_contains(text: &str, substring: &str) {
	assert!(
		text.contains(substring),
		"Expected text to contain '{}', got: {}",
		substring,
		text
	);
}
/// Assert that response body does not contain text
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_not_contains;
///
/// let text = "Hello, World!";
/// assert_not_contains(text, "Goodbye");
/// ```
pub fn assert_not_contains(text: &str, substring: &str) {
	assert!(
		!text.contains(substring),
		"Expected text to not contain '{}', got: {}",
		substring,
		text
	);
}
/// Assert that two status codes are equal
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_status_eq;
/// use http::StatusCode;
///
/// let status = StatusCode::OK;
/// assert_status_eq(status, StatusCode::OK);
/// ```
pub fn assert_status_eq(actual: StatusCode, expected: StatusCode) {
	assert_eq!(
		actual, expected,
		"Expected status {}, got {}",
		expected, actual
	);
}
/// Assert that status is in 2xx range
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_status_success;
/// use http::StatusCode;
///
/// let status = StatusCode::OK;
/// assert_status_success(status);
/// ```
pub fn assert_status_success(status: StatusCode) {
	assert!(
		status.is_success(),
		"Expected success status (2xx), got {}",
		status
	);
}
/// Assert that status is in 4xx range
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_status_client_error;
/// use http::StatusCode;
///
/// let status = StatusCode::BAD_REQUEST;
/// assert_status_client_error(status);
/// ```
pub fn assert_status_client_error(status: StatusCode) {
	assert!(
		status.is_client_error(),
		"Expected client error status (4xx), got {}",
		status
	);
}
/// Assert that status is in 5xx range
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_status_server_error;
/// use http::StatusCode;
///
/// let status = StatusCode::INTERNAL_SERVER_ERROR;
/// assert_status_server_error(status);
/// ```
pub fn assert_status_server_error(status: StatusCode) {
	assert!(
		status.is_server_error(),
		"Expected server error status (5xx), got {}",
		status
	);
}
/// Assert that status is an error (4xx or 5xx)
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_status_error;
/// use http::StatusCode;
///
/// let status = StatusCode::NOT_FOUND;
/// assert_status_error(status);
/// ```
pub fn assert_status_error(status: StatusCode) {
	assert!(
		status.is_client_error() || status.is_server_error(),
		"Expected error status (4xx or 5xx), got {}",
		status
	);
}
/// Assert that status is in 3xx range
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_status_redirect;
/// use http::StatusCode;
///
/// let status = StatusCode::FOUND;
/// assert_status_redirect(status);
/// ```
pub fn assert_status_redirect(status: StatusCode) {
	assert!(
		status.is_redirection(),
		"Expected redirect status (3xx), got {}",
		status
	);
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn test_assert_json_field_eq() {
		let data = json!({"name": "Alice", "age": 30});
		assert_json_field_eq(&data, "name", &json!("Alice"));
		assert_json_field_eq(&data, "age", &json!(30));
	}

	#[test]
	fn test_assert_json_has_field() {
		let data = json!({"name": "Alice"});
		assert_json_has_field(&data, "name");
	}

	#[test]
	fn test_assert_json_missing_field() {
		let data = json!({"name": "Alice"});
		assert_json_missing_field(&data, "age");
	}

	#[test]
	fn test_assert_json_array_len() {
		let data = json!([1, 2, 3]);
		assert_json_array_len(&data, 3);
	}

	#[test]
	fn test_assert_json_array_contains() {
		let data = json!([1, 2, 3]);
		assert_json_array_contains(&data, &json!(2));
	}

	#[test]
	fn test_assert_json_matches() {
		let actual = json!({
			"name": "Alice",
			"age": 30,
			"email": "alice@example.com"
		});
		let pattern = json!({
			"name": "Alice",
			"age": 30
		});
		assert_json_matches(&actual, &pattern);
	}

	#[test]
	fn test_assert_contains() {
		let text = "Hello, world!";
		assert_contains(text, "world");
	}

	#[test]
	fn test_assert_not_contains() {
		let text = "Hello, world!";
		assert_not_contains(text, "foo");
	}
}
