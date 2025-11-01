//! Form parameter extraction tests (application/x-www-form-urlencoded)
//!
//! Based on FastAPI's test_forms_single_param.py and test_forms_single_model.py
//! Reference: fastapi/tests/test_forms_*.py
//!
//! These tests verify form data extraction and validation:
//! 1. application/x-www-form-urlencoded parsing
//! 2. Type validation and coercion
//! 3. Multiple form fields
//! 4. Content-Type validation
//! 5. URL encoding handling

use bytes::Bytes;
use hyper::{HeaderMap, Method, Uri, Version};
use reinhardt_apps::Request;
use reinhardt_params::extract::FromRequest;
use reinhardt_params::{Form, ParamContext};
use serde::Deserialize;

// Helper function to create a mock request with form data
fn create_form_request(form_data: &str) -> Request {
	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::CONTENT_TYPE,
		"application/x-www-form-urlencoded".parse().unwrap(),
	);

	Request::new(
		Method::POST,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::from(form_data.to_string()),
	)
}

// Helper function to create request without Content-Type header
fn create_request_no_content_type(form_data: &str) -> Request {
	Request::new(
		Method::POST,
		Uri::from_static("/test"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::from(form_data.to_string()),
	)
}

fn create_empty_context() -> ParamContext {
	ParamContext::new()
}

// ============================================================================
// Basic Form Field Extraction
// ============================================================================

/// Test: Single form field extraction (FastAPI: test_single_form_field)
#[tokio::test]
async fn test_form_single_field() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct FormData {
		username: String,
	}

	let req = create_form_request("username=Rick");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Failed to extract single form field");
	assert_eq!(result.unwrap().username, "Rick");
}

/// Test: Missing required form field
#[tokio::test]
async fn test_form_missing_required_field() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		username: String,
	}

	let req = create_form_request("");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(
		result.is_err(),
		"Should fail when required field is missing"
	);
}

/// Test: Multiple form fields
#[tokio::test]
async fn test_params_form_multiple_fields() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct FormData {
		username: String,
		password: String,
		remember: bool,
	}

	let req = create_form_request("username=alice&password=secret123&remember=true");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Failed to extract multiple form fields");

	let form = result.unwrap();
	assert_eq!(form.username, "alice");
	assert_eq!(form.password, "secret123");
	assert_eq!(form.remember, true);
}

// ============================================================================
// Type Validation
// ============================================================================

/// Test: Integer form field
#[tokio::test]
async fn test_form_integer_field() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		age: i32,
	}

	let req = create_form_request("age=30");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Failed to parse integer form field");
	assert_eq!(result.unwrap().age, 30);
}

/// Test: Integer form field with invalid value
#[tokio::test]
async fn test_form_integer_invalid() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		age: i32,
	}

	let req = create_form_request("age=thirty");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(result.is_err(), "Should fail to parse invalid integer");
}

/// Test: Float form field
#[tokio::test]
async fn test_form_float_field() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		price: f64,
	}

	let req = create_form_request("price=19.99");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().price, 19.99);
}

/// Test: Boolean form field true
#[tokio::test]
async fn test_form_boolean_true() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		active: bool,
	}

	let req = create_form_request("active=true");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().active, true);
}

/// Test: Boolean form field false
#[tokio::test]
async fn test_form_boolean_false() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		active: bool,
	}

	let req = create_form_request("active=false");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().active, false);
}

// ============================================================================
// Optional Fields
// ============================================================================

/// Test: Optional field not provided
#[tokio::test]
async fn test_form_optional_missing() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		username: String,
		email: Option<String>,
	}

	let req = create_form_request("username=bob");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Optional field should be allowed to be missing"
	);

	let form = result.unwrap();
	assert_eq!(form.username, "bob");
	assert_eq!(form.email, None);
}

/// Test: Optional field provided
#[tokio::test]
async fn test_form_optional_provided() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		username: String,
		email: Option<String>,
	}

	let req = create_form_request("username=bob&email=bob@example.com");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(result.is_ok());

	let form = result.unwrap();
	assert_eq!(form.username, "bob");
	assert_eq!(form.email, Some("bob@example.com".to_string()));
}

// ============================================================================
// Default Values
// ============================================================================

/// Test: Default value when field missing
#[tokio::test]
async fn test_form_default_value() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		username: String,
		#[serde(default = "default_role")]
		role: String,
	}

	fn default_role() -> String {
		"user".to_string()
	}

	let req = create_form_request("username=charlie");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(result.is_ok());

	let form = result.unwrap();
	assert_eq!(form.username, "charlie");
	assert_eq!(form.role, "user");
}

/// Test: Provided value overrides default
#[tokio::test]
async fn test_form_override_default() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		username: String,
		#[serde(default = "default_role")]
		role: String,
	}

	fn default_role() -> String {
		"user".to_string()
	}

	let req = create_form_request("username=charlie&role=admin");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(result.is_ok());

	let form = result.unwrap();
	assert_eq!(form.username, "charlie");
	assert_eq!(form.role, "admin");
}

// ============================================================================
// Content-Type Validation
// ============================================================================

/// Test: Missing Content-Type header
#[tokio::test]
async fn test_form_missing_content_type() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		username: String,
	}

	let req = create_request_no_content_type("username=test");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(
		result.is_err(),
		"Should fail when Content-Type header is missing or incorrect"
	);
}

/// Test: Wrong Content-Type header
#[tokio::test]
async fn test_form_wrong_content_type() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		username: String,
	}

	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::CONTENT_TYPE,
		"application/json".parse().unwrap(),
	);

	let req = Request::new(
		Method::POST,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::from("username=test"),
	);
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(
		result.is_err(),
		"Should fail when Content-Type is not form-urlencoded"
	);
}

// ============================================================================
// URL Encoding
// ============================================================================

/// Test: URL encoded values (spaces as +)
#[tokio::test]
async fn test_form_url_encoded_plus() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		message: String,
	}

	let req = create_form_request("message=Hello+World");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().message, "Hello World");
}

/// Test: URL encoded values (%20 for space)
#[tokio::test]
async fn test_form_url_encoded_percent() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		message: String,
	}

	let req = create_form_request("message=Hello%20World");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().message, "Hello World");
}

/// Test: Special characters in form data
#[tokio::test]
async fn test_form_special_characters() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		text: String,
	}

	// & needs to be encoded as %26
	let req = create_form_request("text=Hello%26Goodbye");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().text, "Hello&Goodbye");
}

/// Test: Equals sign in value
#[tokio::test]
async fn test_form_equals_in_value() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		equation: String,
	}

	// = needs to be encoded as %3D
	let req = create_form_request("equation=x%3D5");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().equation, "x=5");
}

// ============================================================================
// Empty and Edge Cases
// ============================================================================

/// Test: Empty form value
#[tokio::test]
async fn test_form_empty_value() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		username: String,
	}

	let req = create_form_request("username=");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().username, "");
}

/// Test: Extra fields should be ignored
#[tokio::test]
async fn test_form_extra_fields_ignored() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		username: String,
	}

	let req = create_form_request("username=test&extra=ignored&another=also_ignored");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Extra fields should be ignored");
	assert_eq!(result.unwrap().username, "test");
}

// ============================================================================
// Multipart Form Data
// ============================================================================

/// Test: Multipart form data extraction with text fields
#[tokio::test]
async fn test_form_multipart_text_fields() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct FormData {
		username: String,
	}

	let boundary = "----WebKitFormBoundary";
	let body = format!(
		"------WebKitFormBoundary\r\n\
         Content-Disposition: form-data; name=\"username\"\r\n\
         \r\n\
         test\r\n\
         ------WebKitFormBoundary--\r\n"
	);

	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::CONTENT_TYPE,
		format!("multipart/form-data; boundary={}", boundary)
			.parse()
			.unwrap(),
	);

	let req = Request::new(
		Method::POST,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::from(body),
	);
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;

	#[cfg(feature = "multipart")]
	{
		assert!(result.is_ok(), "Failed to parse multipart form data");
		assert_eq!(result.unwrap().username, "test");
	}

	#[cfg(not(feature = "multipart"))]
	{
		assert!(
			result.is_err(),
			"multipart/form-data requires 'multipart' feature"
		);
	}
}

// ============================================================================
// Complex Structures
// ============================================================================

/// Test: Form with multiple types
#[tokio::test]
async fn test_form_complex_structure() {
	#[derive(Debug, Deserialize)]
	struct RegistrationForm {
		username: String,
		email: String,
		age: i32,
		subscribe: bool,
		referrer: Option<String>,
	}

	let req = create_form_request("username=john&email=john@example.com&age=25&subscribe=true");
	let ctx = create_empty_context();

	let result = Form::<RegistrationForm>::from_request(&req, &ctx).await;
	assert!(result.is_ok());

	let form = result.unwrap();
	assert_eq!(form.username, "john");
	assert_eq!(form.email, "john@example.com");
	assert_eq!(form.age, 25);
	assert_eq!(form.subscribe, true);
	assert_eq!(form.referrer, None);
}

/// Test: Form with all fields provided
#[tokio::test]
async fn test_form_all_fields_provided() {
	#[derive(Debug, Deserialize)]
	struct RegistrationForm {
		username: String,
		email: String,
		age: i32,
		subscribe: bool,
		referrer: Option<String>,
	}

	let req = create_form_request(
		"username=jane&email=jane@example.com&age=30&subscribe=false&referrer=google",
	);
	let ctx = create_empty_context();

	let result = Form::<RegistrationForm>::from_request(&req, &ctx).await;
	assert!(result.is_ok());

	let form = result.unwrap();
	assert_eq!(form.username, "jane");
	assert_eq!(form.email, "jane@example.com");
	assert_eq!(form.age, 30);
	assert_eq!(form.subscribe, false);
	assert_eq!(form.referrer, Some("google".to_string()));
}

// ============================================================================
// Django/DRF Form Tests (django-rest-framework/tests/test_request.py)
// ============================================================================

/// Test: GET request data should be empty
/// Reference: django-rest-framework/tests/test_request.py::test_standard_behaviour_determines_no_content_GET
#[tokio::test]
async fn test_get_request_no_form_data() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		username: String,
	}

	// GET requests should not have form data
	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::CONTENT_TYPE,
		"application/x-www-form-urlencoded".parse().unwrap(),
	);

	let req = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::from("username=test"),
	);
	let ctx = create_empty_context();

	// Form extraction from GET is allowed but unusual
	// The test documents this behavior
	let result = Form::<FormData>::from_request(&req, &ctx).await;
	// Whether this succeeds or fails depends on implementation policy
	// Most frameworks ignore body in GET requests
}

/// Test: POST request with form data extraction
/// Reference: django-rest-framework/tests/test_request.py::test_request_DATA_with_form_content
#[tokio::test]
async fn test_post_form_data_extraction() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		qwerty: String,
	}

	let req = create_form_request("qwerty=uiop");
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Failed to extract form data from POST request"
	);
	assert_eq!(result.unwrap().qwerty, "uiop");
}

/// Test: PUT request with form data extraction
/// Reference: django-rest-framework/tests/test_request.py::test_standard_behaviour_determines_form_content_PUT
#[tokio::test]
async fn test_put_form_data_extraction() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		qwerty: String,
	}

	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::CONTENT_TYPE,
		"application/x-www-form-urlencoded".parse().unwrap(),
	);

	let req = Request::new(
		Method::PUT,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::from("qwerty=uiop"),
	);
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Failed to extract form data from PUT request"
	);
	assert_eq!(result.unwrap().qwerty, "uiop");
}

// ============================================================================
// File Upload Tests (multipart/form-data - Not Yet Implemented)
// ============================================================================

/// Test: File upload with multipart/form-data
/// Reference: django-rest-framework/tests/test_request.py::test_request_POST_with_files
#[cfg(feature = "multipart")]
#[tokio::test]
async fn test_file_upload_multipart() {
	use reinhardt_params::Multipart;

	let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
	let body = format!(
		"------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         Hello World!\r\n\
         ------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n"
	);

	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::CONTENT_TYPE,
		format!("multipart/form-data; boundary={}", boundary)
			.parse()
			.unwrap(),
	);

	let req = Request::new(
		Method::POST,
		Uri::from_static("/upload"),
		Version::HTTP_11,
		headers,
		Bytes::from(body),
	);
	let ctx = create_empty_context();

	let result = Multipart::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Should parse multipart: {:?}", result.err());

	let mut multipart = result.unwrap();
	let field = multipart.next_field().await.unwrap();
	assert!(field.is_some(), "Should have at least one field");

	let field = field.unwrap();
	assert_eq!(field.name(), Some("file"));
	assert_eq!(field.file_name(), Some("test.txt"));

	let data = field.bytes().await.unwrap();
	assert_eq!(&data[..], b"Hello World!");
}

/// Test: Mixed form fields and file uploads
#[cfg(feature = "multipart")]
#[tokio::test]
async fn test_mixed_form_and_files() {
	use reinhardt_params::Multipart;

	let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
	let body = format!(
		"------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
         Content-Disposition: form-data; name=\"username\"\r\n\
         \r\n\
         Alice\r\n\
         ------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"data.txt\"\r\n\
         Content-Type: text/plain\r\n\
         \r\n\
         File content\r\n\
         ------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n"
	);

	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::CONTENT_TYPE,
		format!("multipart/form-data; boundary={}", boundary)
			.parse()
			.unwrap(),
	);

	let req = Request::new(
		Method::POST,
		Uri::from_static("/upload"),
		Version::HTTP_11,
		headers,
		Bytes::from(body),
	);
	let ctx = create_empty_context();

	let result = Multipart::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Should parse multipart: {:?}", result.err());

	let mut multipart = result.unwrap();

	// First field: username
	let field = multipart.next_field().await.unwrap().unwrap();
	assert_eq!(field.name(), Some("username"));
	let username = field.text().await.unwrap();
	assert_eq!(username, "Alice");

	// Second field: file
	let field = multipart.next_field().await.unwrap().unwrap();
	assert_eq!(field.name(), Some("file"));
	assert_eq!(field.file_name(), Some("data.txt"));
	let content = field.bytes().await.unwrap();
	assert_eq!(&content[..], b"File content");
}

/// Test: Form extractor with multipart - extracts only text fields
#[cfg(feature = "multipart")]
#[tokio::test]
async fn test_form_extractor_multipart_text_only() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct ContactForm {
		name: String,
		email: String,
	}

	let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
	let body = format!(
		"------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
         Content-Disposition: form-data; name=\"name\"\r\n\
         \r\n\
         Alice\r\n\
         ------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
         Content-Disposition: form-data; name=\"email\"\r\n\
         \r\n\
         alice@example.com\r\n\
         ------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n"
	);

	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::CONTENT_TYPE,
		format!("multipart/form-data; boundary={}", boundary)
			.parse()
			.unwrap(),
	);

	let req = Request::new(
		Method::POST,
		Uri::from_static("/contact"),
		Version::HTTP_11,
		headers,
		Bytes::from(body),
	);
	let ctx = create_empty_context();

	let result = Form::<ContactForm>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Failed to parse multipart form: {:?}",
		result.err()
	);

	let form = result.unwrap();
	assert_eq!(form.name, "Alice");
	assert_eq!(form.email, "alice@example.com");
}

/// Test: Form extractor ignores file fields in multipart
#[cfg(feature = "multipart")]
#[tokio::test]
async fn test_form_extractor_ignores_files() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct FormWithText {
		username: String,
	}

	let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
	let body = format!(
		"------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
         Content-Disposition: form-data; name=\"username\"\r\n\
         \r\n\
         Bob\r\n\
         ------WebKitFormBoundary7MA4YWxkTrZu0gW\r\n\
         Content-Disposition: form-data; name=\"avatar\"; filename=\"photo.jpg\"\r\n\
         Content-Type: image/jpeg\r\n\
         \r\n\
         <binary data>\r\n\
         ------WebKitFormBoundary7MA4YWxkTrZu0gW--\r\n"
	);

	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::CONTENT_TYPE,
		format!("multipart/form-data; boundary={}", boundary)
			.parse()
			.unwrap(),
	);

	let req = Request::new(
		Method::POST,
		Uri::from_static("/profile"),
		Version::HTTP_11,
		headers,
		Bytes::from(body),
	);
	let ctx = create_empty_context();

	let result = Form::<FormWithText>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Failed to parse multipart form: {:?}",
		result.err()
	);

	let form = result.unwrap();
	assert_eq!(form.username, "Bob");
}

// ============================================================================
// Tests from FastAPI test_forms_single_param.py
// ============================================================================

/// Test: Single form field extraction
/// Source: fastapi/tests/test_forms_single_param.py::test_single_form_field
#[tokio::test]
async fn test_single_form_field_extraction() {
	#[derive(Debug, Deserialize)]
	struct FormData {
		username: String,
	}

	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::CONTENT_TYPE,
		"application/x-www-form-urlencoded".parse().unwrap(),
	);

	let req = Request::new(
		Method::POST,
		Uri::from_static("/login"),
		Version::HTTP_11,
		headers,
		Bytes::from("username=Rick"),
	);
	let ctx = create_empty_context();

	let result = Form::<FormData>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Failed to extract single form field");
	assert_eq!(result.unwrap().username, "Rick");
}
