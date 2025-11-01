//! Request Module Tests
//!
//! Tests inspired by Django and Django Rest Framework request handling tests

use bytes::Bytes;
use hyper::{HeaderMap, Method, Uri, Version};
use reinhardt_apps::Request;
use serde::{Deserialize, Serialize};

#[test]
fn test_request_creation() {
	let method = Method::GET;
	let uri = Uri::from_static("/test");
	let headers = HeaderMap::new();
	let body = Bytes::from("test body");

	let request = Request::new(
		method.clone(),
		uri.clone(),
		Version::HTTP_11,
		headers.clone(),
		body.clone(),
	);

	assert_eq!(request.method, method);
	assert_eq!(request.path(), "/test");
	assert_eq!(request.body(), &body);
}

#[test]
fn test_request_path() {
	let uri = Uri::from_static("/api/users/123");
	let request = Request::new(
		Method::GET,
		uri,
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	assert_eq!(request.path(), "/api/users/123");
}

#[test]
fn test_request_query_params_single() {
	let uri = Uri::from_static("/test?foo=bar");
	let request = Request::new(
		Method::GET,
		uri,
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	assert_eq!(request.query_params.get("foo"), Some(&"bar".to_string()));
}

#[test]
fn test_request_query_params_multiple() {
	let uri = Uri::from_static("/test?foo=bar&baz=qux");
	let request = Request::new(
		Method::GET,
		uri,
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	assert_eq!(request.query_params.get("foo"), Some(&"bar".to_string()));
	assert_eq!(request.query_params.get("baz"), Some(&"qux".to_string()));
}

#[test]
fn test_request_query_params_empty() {
	let uri = Uri::from_static("/test");
	let request = Request::new(
		Method::GET,
		uri,
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	assert!(request.query_params.is_empty());
}

#[test]
fn test_request_query_params_value_with_equals() {
	let uri = Uri::from_static("/test?key=value=with=equals");
	let request = Request::new(
		Method::GET,
		uri,
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	// The current implementation splits on the first '=' only
	// Additional '=' characters are not included in the value
	assert_eq!(request.query_params.get("key"), Some(&"value".to_string()));
}

#[test]
fn test_request_query_params_no_value() {
	let uri = Uri::from_static("/test?key");
	let request = Request::new(
		Method::GET,
		uri,
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	assert_eq!(request.query_params.get("key"), Some(&"".to_string()));
}

#[test]
fn test_request_json_deserialization() {
	#[derive(Debug, Serialize, Deserialize, PartialEq)]
	struct TestData {
		name: String,
		age: u32,
	}

	let data = TestData {
		name: "John".to_string(),
		age: 30,
	};

	let json_body = serde_json::to_vec(&data).unwrap();
	let request = Request::new(
		Method::POST,
		Uri::from_static("/api/users"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::from(json_body),
	);

	let parsed: TestData = request.json().unwrap();
	assert_eq!(parsed, data);
}

#[test]
fn test_request_json_deserialization_error() {
	let invalid_json = Bytes::from("not valid json");
	let request = Request::new(
		Method::POST,
		Uri::from_static("/api/users"),
		Version::HTTP_11,
		HeaderMap::new(),
		invalid_json,
	);

	let result: Result<serde_json::Value, _> = request.json();
	assert!(result.is_err());
}

#[test]
fn test_request_method_get() {
	let request = Request::new(
		Method::GET,
		Uri::from_static("/"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	assert_eq!(request.method, Method::GET);
}

#[test]
fn test_request_method_post() {
	let request = Request::new(
		Method::POST,
		Uri::from_static("/"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	assert_eq!(request.method, Method::POST);
}

#[test]
fn test_request_method_put() {
	let request = Request::new(
		Method::PUT,
		Uri::from_static("/"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	assert_eq!(request.method, Method::PUT);
}

#[test]
fn test_request_method_delete() {
	let request = Request::new(
		Method::DELETE,
		Uri::from_static("/"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	assert_eq!(request.method, Method::DELETE);
}

#[test]
fn test_request_method_patch() {
	let request = Request::new(
		Method::PATCH,
		Uri::from_static("/"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	assert_eq!(request.method, Method::PATCH);
}

#[test]
fn test_request_version_http_11() {
	let request = Request::new(
		Method::GET,
		Uri::from_static("/"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	assert_eq!(request.version, Version::HTTP_11);
}

#[test]
fn test_request_version_http_2() {
	let request = Request::new(
		Method::GET,
		Uri::from_static("/"),
		Version::HTTP_2,
		HeaderMap::new(),
		Bytes::new(),
	);

	assert_eq!(request.version, Version::HTTP_2);
}

#[test]
fn test_request_headers() {
	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::CONTENT_TYPE,
		hyper::header::HeaderValue::from_static("application/json"),
	);

	let request = Request::new(
		Method::POST,
		Uri::from_static("/"),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);

	assert_eq!(
		request.headers.get(hyper::header::CONTENT_TYPE).unwrap(),
		"application/json"
	);
}

#[test]
fn test_request_empty_body() {
	let request = Request::new(
		Method::GET,
		Uri::from_static("/"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	assert!(request.body().is_empty());
}

#[test]
fn test_request_path_params_empty_by_default() {
	let request = Request::new(
		Method::GET,
		Uri::from_static("/"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	assert!(request.path_params.is_empty());
}

#[test]
fn test_request_complex_query_string() {
	let uri = Uri::from_static("/search?q=rust+programming&page=1&limit=10");
	let request = Request::new(
		Method::GET,
		uri,
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	assert_eq!(
		request.query_params.get("q"),
		Some(&"rust+programming".to_string())
	);
	assert_eq!(request.query_params.get("page"), Some(&"1".to_string()));
	assert_eq!(request.query_params.get("limit"), Some(&"10".to_string()));
}

#[test]
fn test_request_query_params_special_characters() {
	// Test URL-encoded query parameters
	let uri = Uri::from_static("/test?name=John%20Doe&city=San%20Francisco");
	let request = Request::new(
		Method::GET,
		uri,
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	let decoded = request.decoded_query_params();
	assert_eq!(decoded.get("name"), Some(&"John Doe".to_string()));
	assert_eq!(decoded.get("city"), Some(&"San Francisco".to_string()));
}

#[test]
fn test_request_json_with_nested_data() {
	#[derive(Debug, Serialize, Deserialize, PartialEq)]
	struct Address {
		street: String,
		city: String,
	}

	#[derive(Debug, Serialize, Deserialize, PartialEq)]
	struct User {
		name: String,
		address: Address,
	}

	let user = User {
		name: "John".to_string(),
		address: Address {
			street: "Main St".to_string(),
			city: "New York".to_string(),
		},
	};

	let json_body = serde_json::to_vec(&user).unwrap();
	let request = Request::new(
		Method::POST,
		Uri::from_static("/api/users"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::from(json_body),
	);

	let parsed: User = request.json().unwrap();
	assert_eq!(parsed, user);
}

#[test]
fn test_request_special_characters_in_query() {
	let uri = Uri::from_static("/test?email=user%40example.com&path=%2Fhome%2Fuser");
	let request = Request::new(
		Method::GET,
		uri,
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	let decoded = request.decoded_query_params();
	assert_eq!(decoded.get("email"), Some(&"user@example.com".to_string()));
	assert_eq!(decoded.get("path"), Some(&"/home/user".to_string()));
}

#[test]
fn test_request_unicode_in_query() {
	let uri = Uri::from_static("/test?message=%E3%81%93%E3%82%93%E3%81%AB%E3%81%A1%E3%81%AF");
	let request = Request::new(
		Method::GET,
		uri,
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	let decoded = request.decoded_query_params();
	assert_eq!(decoded.get("message"), Some(&"こんにちは".to_string()));
}

#[test]
fn test_request_plus_sign_in_query() {
	let uri = Uri::from_static("/test?query=hello+world");
	let request = Request::new(
		Method::GET,
		uri,
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	let decoded = request.decoded_query_params();
	// Note: '+' in query params is typically decoded as space in form data,
	// but in standard URL encoding it remains as '+'
	// percent_decode_str treats '+' as '+', not as space
	assert_eq!(decoded.get("query"), Some(&"hello+world".to_string()));
}

#[test]
fn test_request_empty_query_params() {
	let uri = Uri::from_static("/test?empty=&key=value");
	let request = Request::new(
		Method::GET,
		uri,
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	let decoded = request.decoded_query_params();
	assert_eq!(decoded.get("empty"), Some(&"".to_string()));
	assert_eq!(decoded.get("key"), Some(&"value".to_string()));
}
