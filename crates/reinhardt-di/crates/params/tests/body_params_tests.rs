//! Body parameter extraction tests (JSON and raw body)
//!
//! Based on FastAPI's test_multi_body_errors.py and related body tests
//! Reference: fastapi/tests/test_multi_body_errors.py
//!
//! These tests verify body parameter extraction and validation:
//! 1. JSON deserialization from request body
//! 2. Type validation and coercion
//! 3. Nested structures and arrays
//! 4. Error handling for malformed JSON
//! 5. Raw body extraction

use bytes::Bytes;
use hyper::{HeaderMap, Method, Uri, Version};
use reinhardt_apps::Request;
use reinhardt_params::extract::FromRequest;
use reinhardt_params::{Body, Json, ParamContext};
use serde::{Deserialize, Serialize};

// Helper function to create a mock request with JSON body
fn create_json_request(json_body: &str) -> Request {
	Request::new(
		Method::POST,
		Uri::from_static("/test"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::from(json_body.to_string()),
	)
}

// Helper function to create a mock request with raw body
fn create_body_request(body_data: &[u8]) -> Request {
	Request::new(
		Method::POST,
		Uri::from_static("/test"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::copy_from_slice(body_data),
	)
}

fn create_empty_context() -> ParamContext {
	ParamContext::new()
}

// ============================================================================
// Basic JSON Deserialization
// ============================================================================

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct User {
	name: String,
	age: i32,
}

/// Test: Valid JSON body deserialization
#[tokio::test]
async fn test_json_valid_body() {
	let json = r#"{"name": "Alice", "age": 30}"#;
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<User>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Failed to deserialize valid JSON");

	let user = result.unwrap();
	assert_eq!(user.name, "Alice");
	assert_eq!(user.age, 30);
}

/// Test: Empty JSON body
#[tokio::test]
async fn test_json_empty_body() {
	let req = create_json_request("");
	let ctx = create_empty_context();

	let result = Json::<User>::from_request(&req, &ctx).await;
	assert!(result.is_err(), "Should fail on empty body");
}

/// Test: Malformed JSON
#[tokio::test]
async fn test_json_malformed() {
	let json = r#"{"name": "Alice", "age": }"#; // Missing value
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<User>::from_request(&req, &ctx).await;
	assert!(result.is_err(), "Should fail on malformed JSON");
}

/// Test: Missing required field
#[tokio::test]
async fn test_json_missing_field() {
	let json = r#"{"name": "Alice"}"#; // Missing age
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<User>::from_request(&req, &ctx).await;
	assert!(
		result.is_err(),
		"Should fail when required field is missing"
	);
}

/// Test: Extra fields should be ignored
#[tokio::test]
async fn test_json_extra_fields() {
	let json = r#"{"name": "Alice", "age": 30, "email": "alice@example.com"}"#;
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<User>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Extra fields should be ignored");

	let user = result.unwrap();
	assert_eq!(user.name, "Alice");
	assert_eq!(user.age, 30);
}

/// Test: Invalid type for field
#[tokio::test]
async fn test_json_invalid_type() {
	let json = r#"{"name": "Alice", "age": "thirty"}"#;
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<User>::from_request(&req, &ctx).await;
	assert!(result.is_err(), "Should fail when field has wrong type");
}

// ============================================================================
// Optional Fields
// ============================================================================

#[derive(Debug, Deserialize, PartialEq)]
struct UserWithOptional {
	name: String,
	age: Option<i32>,
	email: Option<String>,
}

/// Test: Optional fields missing
#[tokio::test]
async fn test_json_optional_missing() {
	let json = r#"{"name": "Bob"}"#;
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<UserWithOptional>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Optional fields should be allowed to be missing"
	);

	let user = result.unwrap();
	assert_eq!(user.name, "Bob");
	assert_eq!(user.age, None);
	assert_eq!(user.email, None);
}

/// Test: Optional fields provided
#[tokio::test]
async fn test_json_optional_provided() {
	let json = r#"{"name": "Bob", "age": 25, "email": "bob@example.com"}"#;
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<UserWithOptional>::from_request(&req, &ctx).await;
	assert!(result.is_ok());

	let user = result.unwrap();
	assert_eq!(user.name, "Bob");
	assert_eq!(user.age, Some(25));
	assert_eq!(user.email, Some("bob@example.com".to_string()));
}

// ============================================================================
// Default Values
// ============================================================================

#[derive(Debug, Deserialize, PartialEq)]
struct UserWithDefaults {
	name: String,
	#[serde(default = "default_age")]
	age: i32,
	#[serde(default = "default_active")]
	active: bool,
}

fn default_age() -> i32 {
	18
}

fn default_active() -> bool {
	true
}

/// Test: Default values used when fields missing
#[tokio::test]
async fn test_json_default_values() {
	let json = r#"{"name": "Charlie"}"#;
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<UserWithDefaults>::from_request(&req, &ctx).await;
	assert!(result.is_ok());

	let user = result.unwrap();
	assert_eq!(user.name, "Charlie");
	assert_eq!(user.age, 18);
	assert_eq!(user.active, true);
}

/// Test: Provided values override defaults
#[tokio::test]
async fn test_json_override_defaults() {
	let json = r#"{"name": "Charlie", "age": 25, "active": false}"#;
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<UserWithDefaults>::from_request(&req, &ctx).await;
	assert!(result.is_ok());

	let user = result.unwrap();
	assert_eq!(user.name, "Charlie");
	assert_eq!(user.age, 25);
	assert_eq!(user.active, false);
}

// ============================================================================
// Nested Structures
// ============================================================================

#[derive(Debug, Deserialize, PartialEq)]
struct Address {
	street: String,
	city: String,
	zip: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct UserWithAddress {
	name: String,
	address: Address,
}

/// Test: Nested structure deserialization
#[tokio::test]
async fn test_json_nested_structure() {
	let json = r#"
    {
        "name": "David",
        "address": {
            "street": "123 Main St",
            "city": "Springfield",
            "zip": "12345"
        }
    }"#;
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<UserWithAddress>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Failed to deserialize nested structure");

	let user = result.unwrap();
	assert_eq!(user.name, "David");
	assert_eq!(user.address.street, "123 Main St");
	assert_eq!(user.address.city, "Springfield");
	assert_eq!(user.address.zip, "12345");
}

/// Test: Nested structure with missing field
#[tokio::test]
async fn test_json_nested_missing_field() {
	let json = r#"
    {
        "name": "David",
        "address": {
            "street": "123 Main St",
            "city": "Springfield"
        }
    }"#;
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<UserWithAddress>::from_request(&req, &ctx).await;
	assert!(result.is_err(), "Should fail when nested field is missing");
}

// ============================================================================
// Arrays and Lists
// ============================================================================

#[derive(Debug, Deserialize, PartialEq)]
struct Item {
	name: String,
	quantity: i32,
}

/// Test: Array of objects
#[tokio::test]
async fn test_json_array_of_objects() {
	let json = r#"[
        {"name": "Item1", "quantity": 5},
        {"name": "Item2", "quantity": 10}
    ]"#;
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<Vec<Item>>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Failed to deserialize array of objects");

	let items = result.unwrap();
	assert_eq!(items.len(), 2);
	assert_eq!(items[0].name, "Item1");
	assert_eq!(items[0].quantity, 5);
	assert_eq!(items[1].name, "Item2");
	assert_eq!(items[1].quantity, 10);
}

/// Test: Empty array
#[tokio::test]
async fn test_json_empty_array() {
	let json = r#"[]"#;
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<Vec<Item>>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().len(), 0);
}

/// Test: Array with invalid element (FastAPI: test_put_incorrect_body_multiple)
#[tokio::test]
async fn test_json_array_invalid_element() {
	let json = r#"[
        {"name": "Item1", "quantity": "five"},
        {"quantity": 10}
    ]"#;
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<Vec<Item>>::from_request(&req, &ctx).await;
	assert!(result.is_err(), "Should fail when array element is invalid");
}

// ============================================================================
// Primitive Types
// ============================================================================

/// Test: Simple string body
#[tokio::test]
async fn test_json_string() {
	let json = r#""hello world""#;
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<String>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(*result.unwrap(), "hello world");
}

/// Test: Integer body
#[tokio::test]
async fn test_json_integer() {
	let json = r#"42"#;
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<i32>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(*result.unwrap(), 42);
}

/// Test: Float body
#[tokio::test]
async fn test_json_float() {
	let json = r#"3.14"#;
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<f64>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(*result.unwrap(), 3.14);
}

/// Test: Boolean body
#[tokio::test]
async fn test_json_boolean() {
	let json = r#"true"#;
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<bool>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(*result.unwrap(), true);
}

// ============================================================================
// Raw Body Extraction
// ============================================================================

/// Test: Extract raw body bytes
#[tokio::test]
async fn test_raw_body_extraction() {
	let body_data = b"Hello, World!";
	let req = create_body_request(body_data);
	let ctx = create_empty_context();

	let result = Body::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Failed to extract raw body");

	let body = result.unwrap();
	assert_eq!(body.0, body_data);
}

/// Test: Extract empty raw body
#[tokio::test]
async fn test_raw_body_empty() {
	let req = create_body_request(b"");
	let ctx = create_empty_context();

	let result = Body::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().0, Vec::<u8>::new());
}

/// Test: Extract binary body
#[tokio::test]
async fn test_raw_body_binary() {
	let body_data = &[0u8, 1, 2, 3, 255, 128, 64];
	let req = create_body_request(body_data);
	let ctx = create_empty_context();

	let result = Body::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().0, body_data);
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Test: Null value in JSON
#[tokio::test]
async fn test_json_null_value() {
	let json = r#"{"name": "Eve", "age": null}"#;
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<UserWithOptional>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Null should be accepted for Option<T>");

	let user = result.unwrap();
	assert_eq!(user.name, "Eve");
	assert_eq!(user.age, None);
}

/// Test: Unicode in JSON
#[tokio::test]
async fn test_json_unicode() {
	let json = r#"{"name": "José 日本語 🦀", "age": 30}"#;
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<User>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().name, "José 日本語 🦀");
}

/// Test: Large JSON body
#[tokio::test]
async fn test_json_large_body() {
	// Create a large array
	let items: Vec<String> = (0..1000)
		.map(|i| format!(r#"{{"name": "Item{}", "quantity": {}}}"#, i, i))
		.collect();
	let json = format!("[{}]", items.join(","));

	let req = create_json_request(&json);
	let ctx = create_empty_context();

	let result = Json::<Vec<Item>>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Should handle large JSON bodies");
	assert_eq!(result.unwrap().len(), 1000);
}

/// Test: Deeply nested structure
#[tokio::test]
async fn test_json_deeply_nested() {
	#[derive(Debug, Deserialize)]
	struct Level3 {
		value: i32,
	}

	#[derive(Debug, Deserialize)]
	struct Level2 {
		level3: Level3,
	}

	#[derive(Debug, Deserialize)]
	struct Level1 {
		level2: Level2,
	}

	let json = r#"{"level2": {"level3": {"value": 42}}}"#;
	let req = create_json_request(json);
	let ctx = create_empty_context();

	let result = Json::<Level1>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().level2.level3.value, 42);
}
