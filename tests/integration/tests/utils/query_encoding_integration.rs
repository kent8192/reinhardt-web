//! Integration tests for query parameter handling in reinhardt_http
//!
//! These tests verify the `Request::decoded_query_params()` and `Request::query_params`
//! functionality for URL-encoded query parameter processing.

use hyper::Method;
use reinhardt_http::Request;

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a Request with the given query string
fn create_request_with_query(query: &str) -> Request {
	let uri = if query.is_empty() {
		"/test".to_string()
	} else {
		format!("/test?{}", query)
	};
	Request::builder()
		.method(Method::GET)
		.uri(&uri)
		.build()
		.unwrap()
}

// ============================================================================
// Basic Query Parameter Tests
// ============================================================================

#[test]
fn test_basic_query_parameter_parsing() {
	let request = create_request_with_query("name=Alice&age=30");

	assert_eq!(request.query_params.get("name"), Some(&"Alice".to_string()));
	assert_eq!(request.query_params.get("age"), Some(&"30".to_string()));
}

#[test]
fn test_encoded_space_in_query_params() {
	// %20 is the URL-encoded space
	let request = create_request_with_query("greeting=hello%20world");

	// query_params returns the raw (encoded) value
	assert_eq!(
		request.query_params.get("greeting"),
		Some(&"hello%20world".to_string())
	);

	// decoded_query_params returns the decoded value
	let decoded = request.decoded_query_params();
	assert_eq!(decoded.get("greeting"), Some(&"hello world".to_string()));
}

#[test]
fn test_encoded_special_characters() {
	// @ symbol is preserved, but & needs encoding as %26
	let request = create_request_with_query("email=user%40example.com");

	let decoded = request.decoded_query_params();
	assert_eq!(decoded.get("email"), Some(&"user@example.com".to_string()));
}

#[test]
fn test_utf8_encoded_query_params() {
	// Japanese "こんにちは" URL-encoded
	let request =
		create_request_with_query("message=%E3%81%93%E3%82%93%E3%81%AB%E3%81%A1%E3%81%AF");

	let decoded = request.decoded_query_params();
	assert_eq!(decoded.get("message"), Some(&"こんにちは".to_string()));
}

// ============================================================================
// Decoding Tests
// ============================================================================

#[test]
fn test_decoded_query_params_basic() {
	let request = create_request_with_query("name=John%20Doe&city=New%20York");

	let decoded = request.decoded_query_params();

	assert_eq!(decoded.get("name"), Some(&"John Doe".to_string()));
	assert_eq!(decoded.get("city"), Some(&"New York".to_string()));
}

#[test]
fn test_decoded_query_params_preserves_unencoded() {
	let request = create_request_with_query("plain=value&number=123");

	let decoded = request.decoded_query_params();

	assert_eq!(decoded.get("plain"), Some(&"value".to_string()));
	assert_eq!(decoded.get("number"), Some(&"123".to_string()));
}

#[test]
fn test_decoded_utf8_round_trip() {
	// Test that encoded UTF-8 strings decode correctly
	// "東京" (Tokyo) URL-encoded
	let request = create_request_with_query("city=%E6%9D%B1%E4%BA%AC");

	let decoded = request.decoded_query_params();
	assert_eq!(decoded.get("city"), Some(&"東京".to_string()));
}

#[test]
fn test_mixed_encoded_and_plain_values() {
	// "Hello World! 世界" partially encoded
	let request = create_request_with_query("greeting=Hello%20World%21%20%E4%B8%96%E7%95%8C");

	let decoded = request.decoded_query_params();
	assert_eq!(
		decoded.get("greeting"),
		Some(&"Hello World! 世界".to_string())
	);
}

// ============================================================================
// Multiple Query Parameters Tests
// ============================================================================

#[test]
fn test_multiple_query_parameters() {
	let request = create_request_with_query("a=1&b=2&c=3");

	assert_eq!(request.query_params.len(), 3);
	assert_eq!(request.query_params.get("a"), Some(&"1".to_string()));
	assert_eq!(request.query_params.get("b"), Some(&"2".to_string()));
	assert_eq!(request.query_params.get("c"), Some(&"3".to_string()));
}

#[test]
fn test_query_params_with_encoded_ampersand_in_value() {
	// Value contains an ampersand that's encoded as %26
	let request = create_request_with_query("company=Smith%26Jones");

	let decoded = request.decoded_query_params();
	assert_eq!(decoded.get("company"), Some(&"Smith&Jones".to_string()));
}

#[test]
fn test_query_params_with_encoded_equals_in_value() {
	// Value contains an equals sign that's encoded as %3D
	let request = create_request_with_query("equation=1%2B1%3D2");

	let decoded = request.decoded_query_params();
	assert_eq!(decoded.get("equation"), Some(&"1+1=2".to_string()));
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[test]
fn test_empty_query_value() {
	let request = create_request_with_query("key=");

	assert_eq!(request.query_params.get("key"), Some(&"".to_string()));
}

#[test]
fn test_empty_query_string() {
	let request = create_request_with_query("");

	assert!(request.query_params.is_empty());
}

#[test]
fn test_query_param_without_value() {
	let request = create_request_with_query("flag");

	// The parser will treat this as key with empty value
	assert!(request.query_params.contains_key("flag"));
}

#[test]
fn test_special_url_characters_encoded() {
	// Hash (#) and question mark (?) encoded
	let request = create_request_with_query("fragment=%23section&query=%3Fq%3Dtest");

	let decoded = request.decoded_query_params();
	assert_eq!(decoded.get("fragment"), Some(&"#section".to_string()));
	assert_eq!(decoded.get("query"), Some(&"?q=test".to_string()));
}

#[test]
fn test_plus_sign_in_query() {
	// Plus sign can represent space in some encodings (form data)
	// But in URL query strings, it should be preserved as-is or encoded as %2B
	let request = create_request_with_query("math=a%2Bb");

	let decoded = request.decoded_query_params();
	assert_eq!(decoded.get("math"), Some(&"a+b".to_string()));
}

#[test]
fn test_numeric_query_values() {
	let request = create_request_with_query("int=42&float=3.14&negative=-10");

	assert_eq!(request.query_params.get("int"), Some(&"42".to_string()));
	assert_eq!(request.query_params.get("float"), Some(&"3.14".to_string()));
	assert_eq!(
		request.query_params.get("negative"),
		Some(&"-10".to_string())
	);
}

#[test]
fn test_boolean_query_values() {
	let request = create_request_with_query("active=true&deleted=false");

	assert_eq!(
		request.query_params.get("active"),
		Some(&"true".to_string())
	);
	assert_eq!(
		request.query_params.get("deleted"),
		Some(&"false".to_string())
	);
}

// ============================================================================
// JSON-like Query Parameter Tests
// ============================================================================

#[test]
fn test_json_string_as_query_value() {
	// JSON string value URL-encoded
	let request = create_request_with_query("data=%22hello%20world%22");

	let decoded = request.decoded_query_params();
	assert_eq!(decoded.get("data"), Some(&"\"hello world\"".to_string()));
}

#[test]
fn test_json_object_as_query_value() {
	// Simple JSON object URL-encoded: {"key":"value"}
	let request = create_request_with_query("json=%7B%22key%22%3A%22value%22%7D");

	let decoded = request.decoded_query_params();
	let json_value = decoded.get("json").unwrap();
	assert!(json_value.contains("key"));
	assert!(json_value.contains("value"));
}

#[test]
fn test_json_array_as_query_value() {
	// JSON array URL-encoded: [1,2,3]
	let request = create_request_with_query("items=%5B1%2C2%2C3%5D");

	let decoded = request.decoded_query_params();
	assert_eq!(decoded.get("items"), Some(&"[1,2,3]".to_string()));
}

// ============================================================================
// Request Building Integration Tests
// ============================================================================

#[test]
fn test_request_builder_with_complex_uri() {
	let request = Request::builder()
		.method(Method::GET)
		.uri("/api/search?q=rust%20programming&page=1&limit=10")
		.build()
		.unwrap();

	let decoded = request.decoded_query_params();

	assert_eq!(decoded.get("q"), Some(&"rust programming".to_string()));
	assert_eq!(decoded.get("page"), Some(&"1".to_string()));
	assert_eq!(decoded.get("limit"), Some(&"10".to_string()));
}

#[test]
fn test_request_path_separate_from_query() {
	let request = create_request_with_query("key=value");

	assert_eq!(request.path(), "/test");
	assert_eq!(request.query_params.get("key"), Some(&"value".to_string()));
}

#[test]
fn test_request_with_unicode_path_and_query() {
	let request = Request::builder()
		.method(Method::GET)
		.uri("/api/search?city=%E6%9D%B1%E4%BA%AC&country=%E6%97%A5%E6%9C%AC")
		.build()
		.unwrap();

	let decoded = request.decoded_query_params();

	assert_eq!(decoded.get("city"), Some(&"東京".to_string()));
	assert_eq!(decoded.get("country"), Some(&"日本".to_string()));
}

// ============================================================================
// Duplicate Key Handling Tests
// ============================================================================

#[test]
fn test_duplicate_query_keys() {
	// When duplicate keys exist, the current implementation keeps one
	// (behavior depends on HashMap insertion order)
	let request = create_request_with_query("tag=first&tag=second");

	// HashMap will contain one of the values
	assert!(request.query_params.contains_key("tag"));
	let tag_value = request.query_params.get("tag").unwrap();
	assert!(tag_value == "first" || tag_value == "second");
}

// ============================================================================
// Large Query String Tests
// ============================================================================

#[test]
fn test_many_query_parameters() {
	let params: Vec<String> = (0..50).map(|i| format!("key{}=value{}", i, i)).collect();
	let query_string = params.join("&");

	let request = create_request_with_query(&query_string);

	assert_eq!(request.query_params.len(), 50);
	assert_eq!(
		request.query_params.get("key0"),
		Some(&"value0".to_string())
	);
	assert_eq!(
		request.query_params.get("key49"),
		Some(&"value49".to_string())
	);
}

#[test]
fn test_long_query_value() {
	let long_value = "x".repeat(1000);
	let query = format!("data={}", long_value);

	let request = create_request_with_query(&query);

	assert_eq!(request.query_params.get("data"), Some(&long_value));
}

// ============================================================================
// Whitespace Handling Tests
// ============================================================================

#[test]
fn test_leading_trailing_whitespace_encoded() {
	// Leading and trailing spaces encoded as %20
	let request = create_request_with_query("text=%20hello%20world%20");

	let decoded = request.decoded_query_params();
	assert_eq!(decoded.get("text"), Some(&" hello world ".to_string()));
}

#[test]
fn test_tab_character_encoded() {
	// Tab character encoded as %09
	let request = create_request_with_query("text=hello%09world");

	let decoded = request.decoded_query_params();
	assert_eq!(decoded.get("text"), Some(&"hello\tworld".to_string()));
}

#[test]
fn test_newline_character_encoded() {
	// Newline encoded as %0A
	let request = create_request_with_query("text=line1%0Aline2");

	let decoded = request.decoded_query_params();
	assert_eq!(decoded.get("text"), Some(&"line1\nline2".to_string()));
}
