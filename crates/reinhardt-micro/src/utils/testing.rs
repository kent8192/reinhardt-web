//! Testing utilities for microservices
//!
//! This module provides convenient helper functions for testing HTTP handlers
//! and endpoints in microservices.

use crate::{Error, Request, Response, Result};
use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
use serde::de::DeserializeOwned;
use std::fmt::Debug;

/// Create a test request with the given method, path, and optional body
///
/// This is a convenience function for creating HTTP requests in tests.
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::test_request;
/// use hyper::Method;
///
/// let request = test_request(Method::GET, "/api/users", None);
/// assert_eq!(request.method, Method::GET);
/// assert_eq!(request.uri.path(), "/api/users");
/// ```
///
/// # With body
///
/// ```
/// use reinhardt_micro::utils::test_request;
/// use hyper::Method;
///
/// let body = r#"{"name": "Alice"}"#;
/// let request = test_request(Method::POST, "/api/users", Some(body.to_string()));
/// assert_eq!(request.method, Method::POST);
/// assert_eq!(request.body().len(), body.len());
/// ```
pub fn test_request(method: Method, path: &str, body: Option<String>) -> Request {
	let uri = path.parse::<Uri>().expect("Invalid URI");
	let body_bytes = body.map(|b| Bytes::from(b)).unwrap_or_else(Bytes::new);

	Request::new(method, uri, Version::HTTP_11, HeaderMap::new(), body_bytes)
}

/// Assert that the response contains the expected JSON data
///
/// This function deserializes the response body and compares it with the expected value.
/// Returns an error if deserialization fails or values don't match.
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::{assert_json_response, test_request};
/// use reinhardt_micro::Response;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Deserialize, PartialEq, Debug)]
/// struct User {
///     id: i64,
///     name: String,
/// }
///
/// let user = User { id: 1, name: "Alice".to_string() };
/// let json = serde_json::to_string(&user).unwrap();
/// let response = Response::ok()
///     .with_header("Content-Type", "application/json")
///     .with_body(json);
///
/// let expected = User { id: 1, name: "Alice".to_string() };
/// assert!(assert_json_response(response, expected).is_ok());
/// ```
///
/// # Type mismatch
///
/// ```
/// use reinhardt_micro::utils::assert_json_response;
/// use reinhardt_micro::Response;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Deserialize, PartialEq, Debug)]
/// struct User {
///     id: i64,
///     name: String,
/// }
///
/// let json = r#"{"id": 1, "name": "Alice"}"#;
/// let response = Response::ok()
///     .with_header("Content-Type", "application/json")
///     .with_body(json);
///
/// let expected = User { id: 2, name: "Bob".to_string() };
/// assert!(assert_json_response(response, expected).is_err());
/// ```
pub fn assert_json_response<T: DeserializeOwned + PartialEq + Debug>(
	response: Response,
	expected: T,
) -> Result<()> {
	let actual: T = serde_json::from_slice(&response.body)
		.map_err(|e| Error::Serialization(format!("Failed to deserialize response: {}", e)))?;

	if actual == expected {
		Ok(())
	} else {
		Err(Error::Internal(format!(
			"Response body mismatch: expected {:?}, got {:?}",
			expected, actual
		)))
	}
}

/// Assert that the response has the expected status code
///
/// Returns an error if the status code doesn't match.
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::assert_status;
/// use reinhardt_micro::Response;
/// use hyper::StatusCode;
///
/// let response = Response::ok();
/// assert!(assert_status(&response, StatusCode::OK).is_ok());
/// ```
///
/// # Status mismatch
///
/// ```
/// use reinhardt_micro::utils::assert_status;
/// use reinhardt_micro::Response;
/// use hyper::StatusCode;
///
/// let response = Response::ok();
/// assert!(assert_status(&response, StatusCode::NOT_FOUND).is_err());
/// ```
pub fn assert_status(response: &Response, expected: StatusCode) -> Result<()> {
	if response.status == expected {
		Ok(())
	} else {
		Err(Error::Internal(format!(
			"Status code mismatch: expected {}, got {}",
			expected, response.status
		)))
	}
}

/// Extract and deserialize JSON from a response
///
/// Returns the deserialized data or an error if deserialization fails.
///
/// # Examples
///
/// ```
/// use reinhardt_micro::utils::extract_json;
/// use reinhardt_micro::Response;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Deserialize, PartialEq, Debug)]
/// struct User {
///     id: i64,
///     name: String,
/// }
///
/// let user = User { id: 1, name: "Alice".to_string() };
/// let json = serde_json::to_string(&user).unwrap();
/// let response = Response::ok()
///     .with_header("Content-Type", "application/json")
///     .with_body(json);
///
/// let extracted: User = extract_json(response).unwrap();
/// assert_eq!(extracted.id, 1);
/// assert_eq!(extracted.name, "Alice");
/// ```
///
/// # Invalid JSON
///
/// ```
/// use reinhardt_micro::utils::extract_json;
/// use reinhardt_micro::Response;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct User {
///     id: i64,
///     name: String,
/// }
///
/// let response = Response::ok()
///     .with_header("Content-Type", "application/json")
///     .with_body("invalid json");
///
/// let result: Result<User, _> = extract_json(response);
/// assert!(result.is_err());
/// ```
pub fn extract_json<T: DeserializeOwned>(response: Response) -> Result<T> {
	serde_json::from_slice(&response.body)
		.map_err(|e| Error::Serialization(format!("Failed to deserialize response: {}", e)))
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde::{Deserialize, Serialize};

	#[derive(Serialize, Deserialize, PartialEq, Debug)]
	struct TestData {
		id: i64,
		name: String,
	}

	#[test]
	fn test_test_request_without_body() {
		let request = test_request(Method::GET, "/test", None);
		assert_eq!(request.method, Method::GET);
		assert_eq!(request.uri.path(), "/test");
		assert!(request.body().is_empty());
	}

	#[test]
	fn test_test_request_with_body() {
		let body = "test body";
		let request = test_request(Method::POST, "/test", Some(body.to_string()));
		assert_eq!(request.method, Method::POST);
		assert_eq!(*request.body(), Bytes::from(body));
	}

	#[test]
	fn test_assert_json_response_success() {
		let data = TestData {
			id: 1,
			name: "test".to_string(),
		};
		let json = serde_json::to_string(&data).unwrap();
		let response = Response::ok()
			.with_header("Content-Type", "application/json")
			.with_body(json);

		let expected = TestData {
			id: 1,
			name: "test".to_string(),
		};
		assert!(assert_json_response(response, expected).is_ok());
	}

	#[test]
	fn test_assert_json_response_mismatch() {
		let data = TestData {
			id: 1,
			name: "test".to_string(),
		};
		let json = serde_json::to_string(&data).unwrap();
		let response = Response::ok()
			.with_header("Content-Type", "application/json")
			.with_body(json);

		let expected = TestData {
			id: 2,
			name: "different".to_string(),
		};
		assert!(assert_json_response(response, expected).is_err());
	}

	#[test]
	fn test_assert_status_success() {
		let response = Response::ok();
		assert!(assert_status(&response, StatusCode::OK).is_ok());
	}

	#[test]
	fn test_assert_status_mismatch() {
		let response = Response::ok();
		assert!(assert_status(&response, StatusCode::NOT_FOUND).is_err());
	}

	#[test]
	fn test_extract_json_success() {
		let data = TestData {
			id: 1,
			name: "test".to_string(),
		};
		let json = serde_json::to_string(&data).unwrap();
		let response = Response::ok()
			.with_header("Content-Type", "application/json")
			.with_body(json);

		let extracted: TestData = extract_json(response).unwrap();
		assert_eq!(extracted.id, 1);
		assert_eq!(extracted.name, "test");
	}

	#[test]
	fn test_extract_json_invalid() {
		let response = Response::ok()
			.with_header("Content-Type", "application/json")
			.with_body("invalid json");

		let result: Result<TestData> = extract_json(response);
		assert!(result.is_err());
	}
}
