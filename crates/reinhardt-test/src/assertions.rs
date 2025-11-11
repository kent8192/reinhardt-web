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

// ========== HTTP Response Assertions ==========

/// Assert that response has expected status code
///
/// This is a unified function combining `assert_status()` from micro
/// and `assert_response_status()` from views.
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_status;
/// use reinhardt_core::apps::Response;
/// use http::StatusCode;
///
/// let response = Response::ok();
/// assert_status(&response, StatusCode::OK);
/// ```
///
/// # Panics
///
/// Panics if status codes don't match.
pub fn assert_status(response: &reinhardt_core::apps::Response, expected: StatusCode) {
	assert_eq!(
		response.status, expected,
		"Expected status {}, got {}",
		expected, response.status
	);
}

// ========== Response Body Assertions ==========

/// Assert that response body contains expected text
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_response_body_contains;
/// use reinhardt_core::apps::Response;
///
/// let response = Response::ok().with_body(b"Hello, World!".to_vec());
/// assert_response_body_contains(&response, "World");
/// ```
///
/// # Panics
///
/// Panics if body doesn't contain the expected text.
pub fn assert_response_body_contains(response: &reinhardt_core::apps::Response, expected: &str) {
	let body_str = String::from_utf8_lossy(&response.body);
	assert!(
		body_str.contains(expected),
		"Expected body to contain '{}', got '{}'",
		expected,
		body_str
	);
}

/// Assert that response body equals expected bytes
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_response_body_equals;
/// use reinhardt_core::apps::Response;
///
/// let expected = b"exact content";
/// let response = Response::ok().with_body(expected.to_vec());
/// assert_response_body_equals(&response, expected);
/// ```
///
/// # Panics
///
/// Panics if body doesn't match expected bytes.
pub fn assert_response_body_equals(response: &reinhardt_core::apps::Response, expected: &[u8]) {
	assert_eq!(
		response.body, expected,
		"Expected body {:?}, got {:?}",
		expected, response.body
	);
}

// ========== JSON Response Assertions ==========

/// Assert that response contains expected JSON data (exact match)
///
/// This function deserializes the response body and compares it with the expected value.
/// For subset matching, use `assert_json_response_contains` instead.
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_json_response;
/// use reinhardt_core::apps::Response;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Deserialize, PartialEq, Debug)]
/// struct User {
///     id: i64,
///     name: String,
/// }
///
/// let user = User { id: 1, name: "Alice".to_string() };
/// let json = serde_json::to_vec(&user).unwrap();
/// let response = Response::ok()
///     .with_header("Content-Type", "application/json")
///     .with_body(json);
///
/// let expected = User { id: 1, name: "Alice".to_string() };
/// assert_json_response(response, expected);
/// ```
///
/// # Panics
///
/// Panics if:
/// - Response body is not valid JSON
/// - Deserialized value doesn't match expected
pub fn assert_json_response<T>(response: reinhardt_core::apps::Response, expected: T)
where
	T: serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
	let actual: T = serde_json::from_slice(&response.body)
		.expect("Failed to deserialize response body as JSON");

	assert_eq!(
		actual, expected,
		"Response body mismatch: expected {:?}, got {:?}",
		expected, actual
	);
}

/// Assert that response is JSON and contains expected field with value
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_json_response_contains;
/// use reinhardt_core::apps::Response;
/// use serde_json::json;
///
/// let json = json!({"name": "Alice", "age": 30, "city": "NYC"});
/// let response = Response::ok()
///     .with_header("Content-Type", "application/json")
///     .with_body(serde_json::to_vec(&json).unwrap());
///
/// assert_json_response_contains(&response, "name", &json!("Alice"));
/// ```
///
/// # Panics
///
/// Panics if:
/// - Response body is not valid JSON
/// - JSON doesn't contain the expected field
/// - Field value doesn't match expected
pub fn assert_json_response_contains(
	response: &reinhardt_core::apps::Response,
	expected_key: &str,
	expected_value: &serde_json::Value,
) {
	let body_str = String::from_utf8_lossy(&response.body);
	let json: serde_json::Value =
		serde_json::from_str(&body_str).expect("Response body should be valid JSON");

	assert!(
		json.get(expected_key).is_some(),
		"JSON should contain key '{}'",
		expected_key
	);
	assert_eq!(
		json.get(expected_key).unwrap(),
		expected_value,
		"Expected field '{}' to equal {:?}, got {:?}",
		expected_key,
		expected_value,
		json.get(expected_key).unwrap()
	);
}

// ========== Error Type Assertions ==========

/// Assert that a result is an error (generic error assertion)
///
/// This function checks if the result is an error without checking the specific error type.
/// Use specific error assertions (like `assert_not_found_error`) for type-specific checks.
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_error;
/// use reinhardt_core::apps::{Error, Result};
///
/// let result: Result<()> = Err(Error::NotFound("Item not found".to_string()));
/// assert_error(result);
/// ```
///
/// # Panics
///
/// Panics if result is `Ok`.
pub fn assert_error<T>(result: reinhardt_core::apps::Result<T>) {
	if result.is_ok() {
		panic!("Expected error, got Ok");
	}
	// Any error is acceptable
}

/// Assert that a result is a NotFound error
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_not_found_error;
/// use reinhardt_core::apps::{Error, Result};
///
/// let result: Result<()> = Err(Error::NotFound("User not found".to_string()));
/// assert_not_found_error(result);
/// ```
///
/// # Panics
///
/// Panics if result is `Ok` or a different error type.
pub fn assert_not_found_error<T>(result: reinhardt_core::apps::Result<T>) {
	match result {
		Ok(_) => panic!("Expected NotFound error, got Ok"),
		Err(reinhardt_core::apps::Error::NotFound(_)) => {}
		Err(error) => panic!("Expected NotFound error, got {:?}", error),
	}
}

/// Assert that a result is a Validation error
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_validation_error;
/// use reinhardt_core::apps::{Error, Result};
///
/// let result: Result<()> = Err(Error::Validation("Invalid email".to_string()));
/// assert_validation_error(result);
/// ```
///
/// # Panics
///
/// Panics if result is `Ok` or a different error type.
pub fn assert_validation_error<T>(result: reinhardt_core::apps::Result<T>) {
	match result {
		Ok(_) => panic!("Expected Validation error, got Ok"),
		Err(reinhardt_core::apps::Error::Validation(_)) => {}
		Err(error) => panic!("Expected Validation error, got {:?}", error),
	}
}

/// Assert that a result is an Internal error
///
/// # Examples
///
/// ```
/// use reinhardt_test::assertions::assert_internal_error;
/// use reinhardt_core::apps::{Error, Result};
///
/// let result: Result<()> = Err(Error::Internal("Database connection failed".to_string()));
/// assert_internal_error(result);
/// ```
///
/// # Panics
///
/// Panics if result is `Ok` or a different error type.
pub fn assert_internal_error<T>(result: reinhardt_core::apps::Result<T>) {
	match result {
		Ok(_) => panic!("Expected Internal error, got Ok"),
		Err(reinhardt_core::apps::Error::Internal(_)) => {}
		Err(error) => panic!("Expected Internal error, got {:?}", error),
	}
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

	#[test]
	fn test_assert_status() {
		let response = reinhardt_core::apps::Response::ok();
		assert_status(&response, StatusCode::OK);
	}

	#[test]
	fn test_assert_response_body_contains() {
		let response = reinhardt_core::apps::Response::ok().with_body(b"Hello, World!".to_vec());
		assert_response_body_contains(&response, "World");
	}

	#[test]
	fn test_assert_response_body_equals() {
		let expected = b"exact content";
		let response = reinhardt_core::apps::Response::ok().with_body(expected.to_vec());
		assert_response_body_equals(&response, expected);
	}

	#[test]
	fn test_assert_json_response() {
		use serde::{Deserialize, Serialize};

		#[derive(Serialize, Deserialize, PartialEq, Debug)]
		struct TestData {
			id: i64,
			name: String,
		}

		let data = TestData {
			id: 1,
			name: "test".to_string(),
		};
		let json = serde_json::to_vec(&data).unwrap();
		let response = reinhardt_core::apps::Response::ok()
			.with_header("Content-Type", "application/json")
			.with_body(json);

		let expected = TestData {
			id: 1,
			name: "test".to_string(),
		};
		assert_json_response(response, expected);
	}

	#[test]
	fn test_assert_json_response_contains() {
		let json = json!({"name": "Alice", "age": 30});
		let response = reinhardt_core::apps::Response::ok()
			.with_header("Content-Type", "application/json")
			.with_body(serde_json::to_vec(&json).unwrap());

		assert_json_response_contains(&response, "name", &json!("Alice"));
		assert_json_response_contains(&response, "age", &json!(30));
	}

	#[test]
	fn test_assert_error() {
		let result: reinhardt_core::apps::Result<()> = Err(reinhardt_core::apps::Error::NotFound(
			"Not found".to_string(),
		));
		assert_error(result);
	}

	#[test]
	fn test_assert_not_found_error() {
		let result: reinhardt_core::apps::Result<()> = Err(reinhardt_core::apps::Error::NotFound(
			"User not found".to_string(),
		));
		assert_not_found_error(result);
	}

	#[test]
	fn test_assert_validation_error() {
		let result: reinhardt_core::apps::Result<()> = Err(
			reinhardt_core::apps::Error::Validation("Invalid input".to_string()),
		);
		assert_validation_error(result);
	}

	#[test]
	fn test_assert_internal_error() {
		let result: reinhardt_core::apps::Result<()> = Err(reinhardt_core::apps::Error::Internal(
			"Database error".to_string(),
		));
		assert_internal_error(result);
	}
}
