//! Integration tests for Advanced Parser Functionality
//!
//! These tests verify that reinhardt-parsers work correctly with complex data.

use bytes::Bytes;
use reinhardt_parsers::{JSONParser, MediaType, ParseError, Parser};
use serde_json::json;

// ============================================================================
// JSON Parser Tests
// ============================================================================

#[tokio::test]
async fn test_json_parser_nested_objects() {
	let parser = JSONParser::new();

	let json_data = json!({
		"user": {
			"name": "Alice",
			"address": {
				"street": "123 Main St",
				"city": "Springfield",
				"location": {
					"lat": 42.123,
					"lon": -71.456
				}
			},
			"tags": ["admin", "verified", "premium"]
		},
		"meta": {
			"version": 2,
			"timestamp": "2024-01-01T00:00:00Z"
		}
	});

	let body = Bytes::from(serde_json::to_vec(&json_data).unwrap());
	let result = parser.parse(Some("application/json"), body).await.unwrap();

	// Verify nested structure is preserved
	match result {
		reinhardt_parsers::parser::ParsedData::Json(value) => {
			assert_eq!(value["user"]["name"], "Alice");
			assert_eq!(value["user"]["address"]["city"], "Springfield");
			assert_eq!(value["user"]["address"]["location"]["lat"], 42.123);
			assert!(value["user"]["tags"].is_array());
			assert_eq!(value["user"]["tags"][0], "admin");
		}
		_ => panic!("Expected JSON data"),
	}
}

#[tokio::test]
async fn test_json_parser_error_handling() {
	let parser = JSONParser::new();

	// Invalid JSON - missing closing brace
	let invalid_json = b"{\"name\": \"Alice\"";
	let body = Bytes::from(&invalid_json[..]);

	let result = parser.parse(Some("application/json"), body).await;

	assert!(result.is_err());
	match result {
		Err(ParseError::ParseError(msg)) => {
			assert!(msg.contains("Invalid JSON"));
		}
		_ => panic!("Expected ParseError"),
	}
}

#[tokio::test]
async fn test_json_parser_empty_body() {
	let parser = JSONParser::new();

	let body = Bytes::new();
	let result = parser.parse(Some("application/json"), body).await;

	// Should error on empty body by default
	assert!(result.is_err());
}

#[tokio::test]
async fn test_json_parser_empty_body_allowed() {
	let parser = JSONParser::new().allow_empty(true);

	let body = Bytes::new();
	let result = parser.parse(Some("application/json"), body).await;

	// Should return null when empty is allowed
	assert!(result.is_ok());
}

// ============================================================================
// Media Type Tests
// ============================================================================

#[test]
fn test_media_type_basic() {
	let media_type = MediaType::new("application", "json");

	assert_eq!(media_type.main_type, "application");
	assert_eq!(media_type.sub_type, "json");
}

#[test]
fn test_media_type_with_parameters() {
	let media_type = MediaType::new("text", "html")
		.with_param("charset", "utf-8")
		.with_param("boundary", "----WebKitFormBoundary");

	assert_eq!(
		media_type.parameters.get("charset"),
		Some(&"utf-8".to_string())
	);
	assert_eq!(
		media_type.parameters.get("boundary"),
		Some(&"----WebKitFormBoundary".to_string())
	);
}

#[test]
fn test_media_type_parse() {
	let result = MediaType::parse("application/json; charset=utf-8").unwrap();

	assert_eq!(result.main_type, "application");
	assert_eq!(result.sub_type, "json");
	assert_eq!(result.parameters.get("charset"), Some(&"utf-8".to_string()));
}

#[test]
fn test_media_type_parse_complex() {
	let result =
		MediaType::parse("multipart/form-data; boundary=----WebKitFormBoundary; charset=utf-8")
			.unwrap();

	assert_eq!(result.main_type, "multipart");
	assert_eq!(result.sub_type, "form-data");
	assert_eq!(result.parameters.len(), 2);
}

// ============================================================================
// Parser Content Type Support Tests
// ============================================================================

#[test]
fn test_json_parser_media_types() {
	let parser = JSONParser::new();
	let media_types = parser.media_types();

	assert_eq!(
		media_types,
		vec!["application/json".to_string()],
		"JSONParser should support exactly 'application/json' media type"
	);
}

#[tokio::test]
async fn test_json_parser_array_data() {
	let parser = JSONParser::new();

	let json_array = json!([
		{"id": 1, "name": "Item 1"},
		{"id": 2, "name": "Item 2"},
		{"id": 3, "name": "Item 3"}
	]);

	let body = Bytes::from(serde_json::to_vec(&json_array).unwrap());
	let result = parser.parse(Some("application/json"), body).await.unwrap();

	match result {
		reinhardt_parsers::parser::ParsedData::Json(value) => {
			assert!(value.is_array());
			let arr = value.as_array().unwrap();
			assert_eq!(arr.len(), 3);
			assert_eq!(arr[0]["name"], "Item 1");
		}
		_ => panic!("Expected JSON array"),
	}
}

#[tokio::test]
async fn test_json_parser_unicode() {
	let parser = JSONParser::new();

	let json_data = json!({
		"message": "こんにちは世界",
		"emoji": "🎉🎊",
		"mixed": "Hello 世界 👋"
	});

	let body = Bytes::from(serde_json::to_vec(&json_data).unwrap());
	let result = parser.parse(Some("application/json"), body).await.unwrap();

	match result {
		reinhardt_parsers::parser::ParsedData::Json(value) => {
			assert_eq!(value["message"], "こんにちは世界");
			assert_eq!(value["emoji"], "🎉🎊");
			assert_eq!(value["mixed"], "Hello 世界 👋");
		}
		_ => panic!("Expected JSON data"),
	}
}
