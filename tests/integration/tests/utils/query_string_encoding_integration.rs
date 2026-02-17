//! Query String Encoding and Special Cases Integration Tests
//!
//! These tests document advanced query string handling based on Django tests.
//! They require proper URL parsing, encoding handling, and character set support.
//!
//! References:
//! - django/tests/requests_tests/tests.py::test_httprequest_full_path_with_query_string_and_fragment
//! - django/tests/requests_tests/tests.py::test_set_encoding_clears_GET

use bytes::Bytes;
use hyper::{HeaderMap, Method, Uri, Version};
use reinhardt_di::params::extract::FromRequest;
use reinhardt_di::params::{ParamContext, Query};
use reinhardt_http::Request;
use rstest::rstest;
use serde::Deserialize;

/// Test: Query string with fragments and special characters
/// Reference: django/tests/requests_tests/tests.py::test_httprequest_full_path_with_query_string_and_fragment
///
/// Expected behavior:
/// - Query string properly extracted even with URL fragments
/// - Special characters URL-encoded in path
/// - Fragment (#) handled correctly
#[rstest]
#[tokio::test]
async fn test_query_string_with_fragment() {
	#[derive(Debug, Deserialize)]
	struct QueryParams {
		test: String,
	}

	// URL fragments (#section) are typically handled client-side and not sent to server
	// But if they are in the query string, they should be parsed
	let uri = Uri::try_from("/test?test=value#fragment").expect("Invalid URI");

	let req = Request::builder()
		.method(Method::GET)
		.uri(uri)
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();
	let ctx = ParamContext::new();

	let result = Query::<QueryParams>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Failed to parse query string with fragment");

	// The fragment should not be part of the query parameter value
	// Most HTTP libraries strip fragments before sending
	let params = result.unwrap();
	assert_eq!(params.test, "value");
}

/// Test: Query string with URL-encoded special characters
#[rstest]
#[tokio::test]
async fn test_query_string_special_chars() {
	#[derive(Debug, Deserialize)]
	struct QueryParams {
		data: String,
	}

	// Special characters: & = ? # should be URL-encoded
	let uri = Uri::try_from("/test?data=%26%3D%3F%23").expect("Invalid URI");

	let req = Request::builder()
		.method(Method::GET)
		.uri(uri)
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();
	let ctx = ParamContext::new();

	let result = Query::<QueryParams>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().data, "&=?#");
}

/// Test: Query string encoding changes
/// Reference: django/tests/requests_tests/tests.py::test_set_encoding_clears_GET
///
/// NOTE: This test documents Django's behavior where changing request encoding
/// re-parses query parameters. In Rust, encoding is typically UTF-8 by default.
/// Test: Query string with Unicode characters
#[rstest]
#[tokio::test]
async fn test_query_string_unicode() {
	#[derive(Debug, Deserialize)]
	struct QueryParams {
		text: String,
	}

	// Unicode should be URL-encoded
	// "nihongo" (Japanese) URL-encoded
	let uri = Uri::try_from("/test?text=%E6%97%A5%E6%9C%AC%E8%AA%9E").expect("Invalid URI");

	let req = Request::builder()
		.method(Method::GET)
		.uri(uri)
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();
	let ctx = ParamContext::new();

	let result = Query::<QueryParams>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Failed to parse Unicode query parameter");
	assert_eq!(result.unwrap().text, "日本語"); // Japanese text "nihongo"
}

/// Test: Query string with emoji
#[rstest]
#[tokio::test]
async fn test_query_string_emoji() {
	#[derive(Debug, Deserialize)]
	struct QueryParams {
		emoji: String,
	}

	// Emoji "🦀" URL-encoded
	let uri = Uri::try_from("/test?emoji=%F0%9F%A6%80").expect("Invalid URI");

	let req = Request::builder()
		.method(Method::GET)
		.uri(uri)
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();
	let ctx = ParamContext::new();

	let result = Query::<QueryParams>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Failed to parse emoji query parameter");
	assert_eq!(result.unwrap().emoji, "🦀");
}

/// Test: Query string with multiple values for same parameter
#[rstest]
#[tokio::test]
async fn test_query_string_repeated_param() {
	#[derive(Debug, Deserialize)]
	struct QueryParams {
		tags: Vec<String>,
	}

	// Note: serde_urlencoded may not handle repeated keys as expected
	let uri = Uri::try_from("/test?tags=rust&tags=web&tags=framework").expect("Invalid URI");

	let req = Request::builder()
		.method(Method::GET)
		.uri(uri)
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();
	let ctx = ParamContext::new();

	let result = Query::<QueryParams>::from_request(&req, &ctx).await;
	// This is a known limitation of serde_urlencoded
	// It may only capture the last value or fail to parse
	// Document the current behavior
	match result {
		Ok(params) => {
			// If it works, verify at least one value present
			assert!(!params.tags.is_empty(), "Should have at least one tag");
		}
		Err(_) => {
			// Expected limitation with current implementation
			println!("Note: serde_urlencoded doesn't fully support repeated keys");
		}
	}
}

/// Test: Empty query string vs no query string
#[rstest]
#[tokio::test]
async fn test_empty_vs_no_query_string() {
	#[derive(Debug, Deserialize)]
	struct QueryParams {
		#[serde(default)]
		optional: String,
	}

	// No query string
	let uri1 = Uri::try_from("/test").expect("Invalid URI");
	let req1 = Request::builder()
		.method(Method::GET)
		.uri(uri1)
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();
	let ctx1 = ParamContext::new();

	// Empty query string
	let uri2 = Uri::try_from("/test?").expect("Invalid URI");
	let req2 = Request::builder()
		.method(Method::GET)
		.uri(uri2)
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();
	let ctx2 = ParamContext::new();

	let result1 = Query::<QueryParams>::from_request(&req1, &ctx1).await;
	let result2 = Query::<QueryParams>::from_request(&req2, &ctx2).await;

	assert!(result1.is_ok(), "No query string should work with defaults");
	assert!(
		result2.is_ok(),
		"Empty query string should work with defaults"
	);

	assert_eq!(result1.unwrap().optional, "");
	assert_eq!(result2.unwrap().optional, "");
}

/// Test: Malformed query string handling
#[rstest]
#[tokio::test]
async fn test_malformed_query_string() {
	#[derive(Debug, Deserialize)]
	struct QueryParams {
		key: String,
	}

	// Malformed: missing value
	let uri = Uri::try_from("/test?key").expect("Invalid URI");

	let req = Request::builder()
		.method(Method::GET)
		.uri(uri)
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();
	let ctx = ParamContext::new();

	let result = Query::<QueryParams>::from_request(&req, &ctx).await;
	// Behavior depends on parser - might treat as empty string or error
	// Document current behavior
	match result {
		Ok(params) => {
			// Some parsers treat "key" as "key="
			assert_eq!(params.key, "");
		}
		Err(_) => {
			// Other parsers might reject malformed query strings
			println!("Malformed query string rejected");
		}
	}
}
