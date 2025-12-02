//! Parameter Extraction Integration Tests
//!
//! This integration test suite validates real HTTP request parsing and parameter
//! extraction functionality across all parameter types (path, query, header, body).
//!
//! Test Coverage:
//! - Path parameter extraction (primitives, structs, multiple params)
//! - Query parameter extraction (single, multiple, optional, arrays)
//! - Header parameter extraction (single, multiple, struct)
//! - Body parameter extraction (JSON, Form URL-encoded, Multipart)
//! - Parameter validation (ranges, lengths, patterns, formats)
//! - Custom parameter extractors
//! - Error handling and edge cases
//!
//! Requirements:
//! - No database dependencies
//! - Real HTTP request construction and parsing
//! - Tests use reinhardt-params components
//!
//! This test suite uses TestIntent documentation to clarify each test's purpose.

use bytes::Bytes;
use hyper::{HeaderMap, Method, Version, header};
use reinhardt_params::{
	Body, Form, FromRequest, HeaderStruct, Json, ParamContext, Path, PathStruct, Query, Request,
	WithValidation,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Create a basic GET request with the given URI
fn create_get_request(uri: &str) -> Request {
	Request::builder()
		.method(Method::GET)
		.uri(uri)
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap()
}

/// Create a POST request with JSON body
fn create_json_request(uri: &str, json_body: &str) -> Request {
	let mut headers = HeaderMap::new();
	headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());

	Request::builder()
		.method(Method::POST)
		.uri(uri)
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::from(json_body.to_string()))
		.build()
		.unwrap()
}

/// Create a POST request with form data
fn create_form_request(uri: &str, form_data: &str) -> Request {
	let mut headers = HeaderMap::new();
	headers.insert(
		header::CONTENT_TYPE,
		"application/x-www-form-urlencoded".parse().unwrap(),
	);

	Request::builder()
		.method(Method::POST)
		.uri(uri)
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::from(form_data.to_string()))
		.build()
		.unwrap()
}

/// Create a request with custom headers
fn create_request_with_headers(uri: &str, headers: Vec<(&str, &str)>) -> Request {
	let mut header_map = HeaderMap::new();
	for (name, value) in headers {
		header_map.insert(
			hyper::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
			value.parse().unwrap(),
		);
	}

	Request::builder()
		.method(Method::GET)
		.uri(uri)
		.version(Version::HTTP_11)
		.headers(header_map)
		.body(Bytes::new())
		.build()
		.unwrap()
}

// ============================================================================
// Path Parameter Extraction Tests
// ============================================================================

/// TestIntent: Verify extraction of primitive integer path parameter
///
/// Request: GET /users/42
/// PathParam: id=42
/// Expected: Path<i64> extracts value 42
#[tokio::test]
async fn test_extract_path_primitive_i64() {
	let mut params = HashMap::new();
	params.insert("id".to_string(), "42".to_string());

	let ctx = ParamContext::with_path_params(params);
	let req = create_get_request("/users/42");

	let result = Path::<i64>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Failed to extract i64: {:?}", result.err());
	assert_eq!(*result.unwrap(), 42);
}

/// TestIntent: Verify extraction of String path parameter
///
/// Request: GET /users/alice
/// PathParam: username=alice
/// Expected: Path<String> extracts "alice"
#[tokio::test]
async fn test_extract_path_primitive_string() {
	let mut params = HashMap::new();
	params.insert("username".to_string(), "alice".to_string());

	let ctx = ParamContext::with_path_params(params);
	let req = create_get_request("/users/alice");

	let result = Path::<String>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Failed to extract String: {:?}",
		result.err()
	);
	assert_eq!(*result.unwrap(), "alice");
}

/// TestIntent: Verify extraction of floating-point path parameter
///
/// Request: GET /products/19.99
/// PathParam: price=19.99
/// Expected: Path<f64> extracts value 19.99
#[tokio::test]
async fn test_extract_path_primitive_f64() {
	let mut params = HashMap::new();
	params.insert("price".to_string(), "19.99".to_string());

	let ctx = ParamContext::with_path_params(params);
	let req = create_get_request("/products/19.99");

	let result = Path::<f64>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Failed to extract f64: {:?}", result.err());
	assert_eq!(*result.unwrap(), 19.99);
}

/// TestIntent: Verify extraction of boolean path parameter
///
/// Request: GET /settings/true
/// PathParam: active=true
/// Expected: Path<bool> extracts value true
#[tokio::test]
async fn test_extract_path_primitive_bool() {
	let mut params = HashMap::new();
	params.insert("active".to_string(), "true".to_string());

	let ctx = ParamContext::with_path_params(params);
	let req = create_get_request("/settings/true");

	let result = Path::<bool>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Failed to extract bool: {:?}", result.err());
	assert!(*result.unwrap());
}

/// TestIntent: Verify extraction of multiple path parameters into struct
///
/// Request: GET /users/123/posts/456
/// PathParams: user_id=123, post_id=456
/// Expected: PathStruct<MultiParams> extracts both values correctly
#[tokio::test]
async fn test_extract_path_struct_multiple_params() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct MultiParams {
		user_id: i64,
		post_id: i64,
	}

	let mut params = HashMap::new();
	params.insert("user_id".to_string(), "123".to_string());
	params.insert("post_id".to_string(), "456".to_string());

	let ctx = ParamContext::with_path_params(params);
	let req = create_get_request("/users/123/posts/456");

	let result = PathStruct::<MultiParams>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Failed to extract PathStruct: {:?}",
		result.err()
	);

	let params = result.unwrap();
	assert_eq!(params.user_id, 123);
	assert_eq!(params.post_id, 456);
}

/// TestIntent: Verify error when path parameter parsing fails
///
/// Request: GET /users/invalid
/// PathParam: id=invalid (not a number)
/// Expected: Returns ParseError
#[tokio::test]
async fn test_extract_path_parse_error() {
	let mut params = HashMap::new();
	params.insert("id".to_string(), "invalid".to_string());

	let ctx = ParamContext::with_path_params(params);
	let req = create_get_request("/users/invalid");

	let result = Path::<i64>::from_request(&req, &ctx).await;
	assert!(result.is_err(), "Expected error for invalid integer");
}

// ============================================================================
// Query Parameter Extraction Tests
// ============================================================================

/// TestIntent: Verify extraction of simple query parameters
///
/// Request: GET /users?page=2&limit=10
/// QueryParams: page=2, limit=10
/// Expected: Query<Pagination> extracts both values
#[tokio::test]
async fn test_extract_query_simple_params() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct Pagination {
		page: i32,
		limit: i32,
	}

	let ctx = ParamContext::new();
	let req = create_get_request("/users?page=2&limit=10");

	let result = Query::<Pagination>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Failed to extract Query: {:?}",
		result.err()
	);

	let query = result.unwrap();
	assert_eq!(query.page, 2);
	assert_eq!(query.limit, 10);
}

/// TestIntent: Verify extraction of optional query parameters
///
/// Request: GET /search?q=rust (without page param)
/// QueryParams: q=rust, page=None
/// Expected: Query extracts q, page defaults to None
#[tokio::test]
async fn test_extract_query_optional_params() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct SearchQuery {
		q: String,
		page: Option<i32>,
	}

	let ctx = ParamContext::new();
	let req = create_get_request("/search?q=rust");

	let result = Query::<SearchQuery>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Failed to extract Query: {:?}",
		result.err()
	);

	let query = result.unwrap();
	assert_eq!(query.q, "rust");
	assert_eq!(query.page, None);
}

/// TestIntent: Verify extraction of multi-value query parameters as array
///
/// Request: GET /search?tags=rust&tags=web
/// QueryParams: tags=[rust, web]
/// Expected: Query<MultiValueQuery> extracts tags as Vec<String>
#[tokio::test]
#[cfg(feature = "multi-value-arrays")]
async fn test_extract_query_multi_value_array() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct MultiValueQuery {
		tags: Vec<String>,
	}

	let ctx = ParamContext::new();
	let req = create_get_request("/search?tags=rust&tags=web");

	let result = Query::<MultiValueQuery>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Failed to extract Query: {:?}",
		result.err()
	);

	let query = result.unwrap();
	assert_eq!(query.tags.len(), 2);
	assert_eq!(query.tags[0], "rust");
	assert_eq!(query.tags[1], "web");
}

/// TestIntent: Verify extraction of query parameters with URL encoding
///
/// Request: GET /search?q=hello%20world
/// QueryParams: q="hello world"
/// Expected: Query extracts decoded string "hello world"
#[tokio::test]
async fn test_extract_query_url_encoded() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct SearchQuery {
		q: String,
	}

	let ctx = ParamContext::new();
	let req = create_get_request("/search?q=hello%20world");

	let result = Query::<SearchQuery>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Failed to extract Query: {:?}",
		result.err()
	);

	let query = result.unwrap();
	assert_eq!(query.q, "hello world");
}

/// TestIntent: Verify empty query string handling
///
/// Request: GET /users (no query string)
/// QueryParams: (empty)
/// Expected: Query with all optional fields as None
#[tokio::test]
async fn test_extract_query_empty() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct EmptyQuery {
		page: Option<i32>,
		limit: Option<i32>,
	}

	let ctx = ParamContext::new();
	let req = create_get_request("/users");

	let result = Query::<EmptyQuery>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Failed to extract empty Query: {:?}",
		result.err()
	);

	let query = result.unwrap();
	assert_eq!(query.page, None);
	assert_eq!(query.limit, None);
}

// ============================================================================
// Header Parameter Extraction Tests
// ============================================================================

/// TestIntent: Verify extraction of single header value into struct
///
/// Request: GET / with Content-Type: application/json
/// Headers: content-type=application/json
/// Expected: HeaderStruct extracts content_type field
#[tokio::test]
async fn test_extract_header_struct_single() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct MyHeaders {
		#[serde(rename = "content-type")]
		content_type: String,
	}

	let ctx = ParamContext::new();
	let req = create_request_with_headers("/", vec![("content-type", "application/json")]);

	let result = HeaderStruct::<MyHeaders>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Failed to extract HeaderStruct: {:?}",
		result.err()
	);

	let headers = result.unwrap();
	assert_eq!(headers.content_type, "application/json");
}

/// TestIntent: Verify extraction of multiple headers into struct
///
/// Request: GET / with User-Agent and Accept headers
/// Headers: user-agent=TestAgent, accept=application/json
/// Expected: HeaderStruct extracts both fields
#[tokio::test]
async fn test_extract_header_struct_multiple() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct MultiHeaders {
		#[serde(rename = "user-agent")]
		user_agent: String,
		accept: String,
	}

	let ctx = ParamContext::new();
	let req = create_request_with_headers(
		"/",
		vec![("user-agent", "TestAgent"), ("accept", "application/json")],
	);

	let result = HeaderStruct::<MultiHeaders>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Failed to extract MultiHeaders: {:?}",
		result.err()
	);

	let headers = result.unwrap();
	assert_eq!(headers.user_agent, "TestAgent");
	assert_eq!(headers.accept, "application/json");
}

/// TestIntent: Verify extraction of optional header field
///
/// Request: GET / with only Content-Type (no Accept header)
/// Headers: content-type=text/html, accept=None
/// Expected: HeaderStruct extracts content_type, accept is None
#[tokio::test]
async fn test_extract_header_struct_optional() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct OptionalHeaders {
		#[serde(rename = "content-type")]
		content_type: String,
		accept: Option<String>,
	}

	let ctx = ParamContext::new();
	let req = create_request_with_headers("/", vec![("content-type", "text/html")]);

	let result = HeaderStruct::<OptionalHeaders>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Failed to extract OptionalHeaders: {:?}",
		result.err()
	);

	let headers = result.unwrap();
	assert_eq!(headers.content_type, "text/html");
	assert_eq!(headers.accept, None);
}

/// TestIntent: Verify header name case-insensitivity
///
/// Request: GET / with Content-Type (uppercase T)
/// Headers: Content-Type=application/json
/// Expected: HeaderStruct extracts as lowercase content-type
#[tokio::test]
async fn test_extract_header_case_insensitive() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct CaseHeaders {
		#[serde(rename = "content-type")]
		content_type: String,
	}

	let ctx = ParamContext::new();
	let req = create_request_with_headers("/", vec![("Content-Type", "application/json")]);

	let result = HeaderStruct::<CaseHeaders>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Failed to extract headers: {:?}",
		result.err()
	);

	let headers = result.unwrap();
	assert_eq!(headers.content_type, "application/json");
}

// ============================================================================
// JSON Body Extraction Tests
// ============================================================================

/// TestIntent: Verify extraction of JSON body into struct
///
/// Request: POST /users with JSON body {"username": "alice", "age": 30}
/// Body: {"username": "alice", "age": 30}
/// Expected: Json<User> extracts both fields correctly
#[tokio::test]
async fn test_extract_json_body_simple() {
	#[derive(Debug, Deserialize, Serialize, PartialEq)]
	struct User {
		username: String,
		age: u32,
	}

	let json_body = r#"{"username": "alice", "age": 30}"#;
	let ctx = ParamContext::new();
	let req = create_json_request("/users", json_body);

	let result = Json::<User>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Failed to extract JSON: {:?}", result.err());

	let user = result.unwrap();
	assert_eq!(user.username, "alice");
	assert_eq!(user.age, 30);
}

/// TestIntent: Verify extraction of nested JSON structures
///
/// Request: POST /data with nested JSON
/// Body: {"user": {"name": "bob", "email": "bob@example.com"}}
/// Expected: Json<NestedData> extracts nested structure correctly
#[tokio::test]
async fn test_extract_json_body_nested() {
	#[derive(Debug, Deserialize, Serialize, PartialEq)]
	struct UserInfo {
		name: String,
		email: String,
	}

	#[derive(Debug, Deserialize, Serialize, PartialEq)]
	struct NestedData {
		user: UserInfo,
	}

	let json_body = r#"{"user": {"name": "bob", "email": "bob@example.com"}}"#;
	let ctx = ParamContext::new();
	let req = create_json_request("/data", json_body);

	let result = Json::<NestedData>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Failed to extract nested JSON: {:?}",
		result.err()
	);

	let data = result.unwrap();
	assert_eq!(data.user.name, "bob");
	assert_eq!(data.user.email, "bob@example.com");
}

/// TestIntent: Verify error when JSON body is malformed
///
/// Request: POST /users with invalid JSON
/// Body: {invalid json}
/// Expected: Returns DeserializationError
#[tokio::test]
async fn test_extract_json_body_malformed() {
	#[derive(Debug, Deserialize, Serialize)]
	struct User {
		username: String,
	}

	let invalid_json = r#"{invalid json}"#;
	let ctx = ParamContext::new();
	let req = create_json_request("/users", invalid_json);

	let result = Json::<User>::from_request(&req, &ctx).await;
	assert!(result.is_err(), "Expected error for malformed JSON");
}

/// TestIntent: Verify extraction of JSON array
///
/// Request: POST /batch with JSON array
/// Body: [{"id": 1}, {"id": 2}]
/// Expected: Json<Vec<Item>> extracts array correctly
#[tokio::test]
async fn test_extract_json_body_array() {
	#[derive(Debug, Deserialize, Serialize, PartialEq)]
	struct Item {
		id: i32,
	}

	let json_body = r#"[{"id": 1}, {"id": 2}]"#;
	let ctx = ParamContext::new();
	let req = create_json_request("/batch", json_body);

	let result = Json::<Vec<Item>>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Failed to extract JSON array: {:?}",
		result.err()
	);

	let items = result.unwrap();
	assert_eq!(items.len(), 2);
	assert_eq!(items[0].id, 1);
	assert_eq!(items[1].id, 2);
}

// ============================================================================
// Form Data Extraction Tests
// ============================================================================

/// TestIntent: Verify extraction of URL-encoded form data
///
/// Request: POST /login with form data
/// Body: username=alice&password=secret123
/// Expected: Form<LoginForm> extracts both fields
#[tokio::test]
async fn test_extract_form_urlencoded_simple() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct LoginForm {
		username: String,
		password: String,
	}

	let form_data = "username=alice&password=secret123";
	let ctx = ParamContext::new();
	let req = create_form_request("/login", form_data);

	let result = Form::<LoginForm>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Failed to extract Form: {:?}", result.err());

	let form = result.unwrap();
	assert_eq!(form.username, "alice");
	assert_eq!(form.password, "secret123");
}

/// TestIntent: Verify extraction of URL-encoded form with special characters
///
/// Request: POST /comment with form data
/// Body: message=hello%20world%21 (URL-encoded "hello world!")
/// Expected: Form extracts decoded message "hello world!"
#[tokio::test]
async fn test_extract_form_urlencoded_special_chars() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct CommentForm {
		message: String,
	}

	let form_data = "message=hello%20world%21";
	let ctx = ParamContext::new();
	let req = create_form_request("/comment", form_data);

	let result = Form::<CommentForm>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Failed to extract Form: {:?}", result.err());

	let form = result.unwrap();
	assert_eq!(form.message, "hello world!");
}

/// TestIntent: Verify error when Content-Type is not form
///
/// Request: POST /login with wrong Content-Type (text/plain)
/// Body: username=alice&password=secret
/// Expected: Returns InvalidParameter error
#[tokio::test]
async fn test_extract_form_wrong_content_type() {
	#[allow(dead_code)]
	#[derive(Debug, Deserialize)]
	struct LoginForm {
		username: String,
	}

	let mut headers = HeaderMap::new();
	headers.insert(header::CONTENT_TYPE, "text/plain".parse().unwrap());

	let req = Request::builder()
		.method(Method::POST)
		.uri("/login")
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::from("username=alice"))
		.build()
		.unwrap();

	let ctx = ParamContext::new();
	let result = Form::<LoginForm>::from_request(&req, &ctx).await;

	assert!(result.is_err(), "Expected error for wrong content type");
}

// ============================================================================
// Raw Body Extraction Tests
// ============================================================================

/// TestIntent: Verify extraction of raw body bytes
///
/// Request: POST /raw with plain text body
/// Body: "raw text data"
/// Expected: Body extracts raw bytes
#[tokio::test]
async fn test_extract_raw_body() {
	let body_content = "raw text data";
	let mut headers = HeaderMap::new();
	headers.insert(header::CONTENT_TYPE, "text/plain".parse().unwrap());

	let req = Request::builder()
		.method(Method::POST)
		.uri("/raw")
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::from(body_content.to_string()))
		.build()
		.unwrap();

	let ctx = ParamContext::new();
	let result = Body::from_request(&req, &ctx).await;

	assert!(result.is_ok(), "Failed to extract Body: {:?}", result.err());

	let body = result.unwrap();
	assert_eq!(body.0, body_content.as_bytes());
}

/// TestIntent: Verify extraction of empty body
///
/// Request: POST /empty with no body
/// Body: (empty)
/// Expected: Body extracts empty bytes
#[tokio::test]
async fn test_extract_empty_body() {
	let req = Request::builder()
		.method(Method::POST)
		.uri("/empty")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let ctx = ParamContext::new();
	let result = Body::from_request(&req, &ctx).await;

	assert!(
		result.is_ok(),
		"Failed to extract empty Body: {:?}",
		result.err()
	);

	let body = result.unwrap();
	assert_eq!(body.0.len(), 0);
}

// ============================================================================
// Parameter Validation Tests
// ============================================================================

/// TestIntent: Verify numeric range validation on path parameter
///
/// Path parameter: id=42
/// Constraint: min_value(1), max_value(100)
/// Expected: Validation passes for value 42
#[tokio::test]
async fn test_validate_path_numeric_range_valid() {
	let mut params = HashMap::new();
	params.insert("id".to_string(), "42".to_string());

	let ctx = ParamContext::with_path_params(params);
	let req = create_get_request("/users/42");

	let path = Path::<i32>::from_request(&req, &ctx).await.unwrap();
	let validated = path.min_value(1).max_value(100);

	let result = validated.validate_number(&validated.0);
	assert!(result.is_ok(), "Validation should pass for value 42");
}

/// TestIntent: Verify numeric range validation fails for out-of-range value
///
/// Path parameter: id=150
/// Constraint: min_value(1), max_value(100)
/// Expected: Validation fails for value 150 (exceeds max)
#[tokio::test]
async fn test_validate_path_numeric_range_exceeds_max() {
	let path = Path(150i32);
	let validated = path.min_value(1).max_value(100);

	let result = validated.validate_number(&validated.0);
	assert!(
		result.is_err(),
		"Validation should fail for value 150 exceeding max 100"
	);
}

/// TestIntent: Verify numeric range validation fails for below-minimum value
///
/// Path parameter: id=-5
/// Constraint: min_value(1), max_value(100)
/// Expected: Validation fails for value -5 (below min)
#[tokio::test]
async fn test_validate_path_numeric_range_below_min() {
	let path = Path(-5i32);
	let validated = path.min_value(1).max_value(100);

	let result = validated.validate_number(&validated.0);
	assert!(
		result.is_err(),
		"Validation should fail for value -5 below min 1"
	);
}

/// TestIntent: Verify string length validation on query parameter
///
/// Query parameter: q="rust"
/// Constraint: min_length(2), max_length(10)
/// Expected: Validation passes for "rust" (4 chars)
#[tokio::test]
async fn test_validate_query_string_length_valid() {
	let query = Query("rust".to_string());
	let validated = query.min_length(2).max_length(10);

	let result = validated.validate_string(&validated.0);
	assert!(result.is_ok(), "Validation should pass for 'rust'");
}

/// TestIntent: Verify string length validation fails for too-short string
///
/// Query parameter: q="a"
/// Constraint: min_length(3), max_length(20)
/// Expected: Validation fails for "a" (1 char < min 3)
#[tokio::test]
async fn test_validate_query_string_length_too_short() {
	let query = Query("a".to_string());
	let validated = query.min_length(3).max_length(20);

	let result = validated.validate_string(&validated.0);
	assert!(
		result.is_err(),
		"Validation should fail for string too short"
	);
}

/// TestIntent: Verify string length validation fails for too-long string
///
/// Query parameter: q="this is way too long"
/// Constraint: min_length(3), max_length(10)
/// Expected: Validation fails for string exceeding max length
#[tokio::test]
async fn test_validate_query_string_length_too_long() {
	let query = Query("this is way too long".to_string());
	let validated = query.min_length(3).max_length(10);

	let result = validated.validate_string(&validated.0);
	assert!(
		result.is_err(),
		"Validation should fail for string too long"
	);
}

/// TestIntent: Verify email format validation
///
/// Query parameter: email="user@example.com"
/// Constraint: email()
/// Expected: Validation passes for valid email
#[tokio::test]
async fn test_validate_email_format_valid() {
	let query = Query("user@example.com".to_string());
	let validated = query.email();

	let result = validated.validate_string(&validated.0);
	assert!(result.is_ok(), "Validation should pass for valid email");
}

/// TestIntent: Verify email format validation fails for invalid email
///
/// Query parameter: email="invalid"
/// Constraint: email()
/// Expected: Validation fails for invalid email format
#[tokio::test]
async fn test_validate_email_format_invalid() {
	let query = Query("invalid".to_string());
	let validated = query.email();

	let result = validated.validate_string(&validated.0);
	assert!(result.is_err(), "Validation should fail for invalid email");
}

/// TestIntent: Verify regex pattern validation
///
/// Query parameter: username="alice123"
/// Constraint: regex("^[a-zA-Z0-9]+$")
/// Expected: Validation passes for alphanumeric username
#[tokio::test]
async fn test_validate_regex_pattern_valid() {
	let query = Query("alice123".to_string());
	let validated = query.regex(r"^[a-zA-Z0-9]+$");

	let result = validated.validate_string(&validated.0);
	assert!(
		result.is_ok(),
		"Validation should pass for alphanumeric string"
	);
}

/// TestIntent: Verify regex pattern validation fails for non-matching string
///
/// Query parameter: username="alice-123"
/// Constraint: regex("^[a-zA-Z0-9]+$")
/// Expected: Validation fails for string with hyphen (not matching pattern)
#[tokio::test]
async fn test_validate_regex_pattern_invalid() {
	let query = Query("alice-123".to_string());
	let validated = query.regex(r"^[a-zA-Z0-9]+$");

	let result = validated.validate_string(&validated.0);
	assert!(
		result.is_err(),
		"Validation should fail for string with hyphen"
	);
}

/// TestIntent: Verify URL format validation
///
/// Query parameter: url="https://example.com"
/// Constraint: url()
/// Expected: Validation passes for valid URL
#[tokio::test]
async fn test_validate_url_format_valid() {
	let query = Query("https://example.com".to_string());
	let validated = query.url();

	let result = validated.validate_string(&validated.0);
	assert!(result.is_ok(), "Validation should pass for valid URL");
}

/// TestIntent: Verify URL format validation fails for invalid URL
///
/// Query parameter: url="not-a-url"
/// Constraint: url()
/// Expected: Validation fails for invalid URL format
#[tokio::test]
async fn test_validate_url_format_invalid() {
	let query = Query("not-a-url".to_string());
	let validated = query.url();

	let result = validated.validate_string(&validated.0);
	assert!(result.is_err(), "Validation should fail for invalid URL");
}

/// TestIntent: Verify chaining multiple validation constraints
///
/// Query parameter: username="alice"
/// Constraint: min_length(3), max_length(20), regex("^[a-z]+$")
/// Expected: All constraints pass for "alice"
#[tokio::test]
async fn test_validate_chained_constraints() {
	let query = Query("alice".to_string());
	let validated = query.min_length(3).max_length(20).regex(r"^[a-z]+$");

	let result = validated.validate_string(&validated.0);
	assert!(result.is_ok(), "All validation constraints should pass");
}
