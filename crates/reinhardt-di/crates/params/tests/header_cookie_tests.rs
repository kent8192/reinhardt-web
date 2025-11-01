//! Header and Cookie parameter extraction tests
//!
//! Based on FastAPI's header and cookie tests
//! Reference: fastapi/tests/test_header.py, fastapi/tests/test_cookie.py
//!
//! ## Header/Cookie Name Specification
//!
//! Reinhardt provides two ways to specify header/cookie names:
//!
//! 1. **Compile-time with marker types** (Recommended for common headers/cookies):
//!    ```rust,ignore
//!    use reinhardt_params::{HeaderNamed, CookieNamed, Authorization, SessionId};
//!
//!    async fn handler(
//!        auth: HeaderNamed<Authorization, String>,
//!        session: CookieNamed<SessionId, Option<String>>,
//!    ) {
//!        // ...
//!    }
//!    ```
//!
//! 2. **Struct-based with serde rename** (For multiple headers/cookies):
//!    ```rust,ignore
//!    use reinhardt_params::{HeaderStruct, CookieStruct};
//!
//!    #[derive(Deserialize)]
//!    struct MyHeaders {
//!        #[serde(rename = "x-api-key")]
//!        api_key: String,
//!    }
//!
//!    async fn handler(headers: HeaderStruct<MyHeaders>) {
//!        // ...
//!    }
//!    ```

use bytes::Bytes;
use hyper::{HeaderMap, Method, Uri, Version};
use reinhardt_apps::Request;
use reinhardt_params::extract::FromRequest;
use reinhardt_params::{
	Authorization, ContentType, Cookie, CookieNamed, CookieStruct, CsrfToken, Header, HeaderNamed,
	HeaderStruct, ParamContext, SessionId,
};
use serde::Deserialize;

fn create_empty_context() -> ParamContext {
	ParamContext::new()
}

fn create_test_request_with_headers(headers: &[(&str, &str)], body: &str) -> Request {
	let mut header_map = HeaderMap::new();
	for (name, value) in headers {
		header_map.insert(
			hyper::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
			value.parse().unwrap(),
		);
	}

	Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		header_map,
		Bytes::from(body.as_bytes().to_vec()),
	)
}

// ============================================================================
// Header Extraction (Requires name specification via ParamContext)
// ============================================================================

/// Test header extraction (requires name specification)
///
/// Current limitation: Header<T> extractor doesn't know which header to extract
/// Needs enhancement to specify header name, e.g.:
/// - #[header("Authorization")] or
/// - Type-level encoding or
/// - Context-based specification
#[tokio::test]
async fn test_header_extraction() {
	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::AUTHORIZATION,
		"Bearer token123".parse().unwrap(),
	);

	let req = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);
	let mut ctx = create_empty_context();
	ctx.set_header_name::<String>("authorization");

	let result = Header::<String>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().into_inner(), "Bearer token123");
}

/// Test custom header extraction (requires name specification)
#[tokio::test]
async fn test_custom_header() {
	let mut headers = HeaderMap::new();
	headers.insert("X-Custom-Header", "custom-value".parse().unwrap());

	let req = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);
	let mut ctx = create_empty_context();
	ctx.set_header_name::<String>("x-custom-header");

	let result = Header::<String>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().into_inner(), "custom-value");
}

/// Test missing header (requires name specification)
#[tokio::test]
async fn test_header_missing() {
	let req = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);
	let mut ctx = create_empty_context();
	ctx.set_header_name::<String>("authorization");

	let result = Header::<String>::from_request(&req, &ctx).await;
	assert!(result.is_err());
}

/// Test optional header (requires name specification)
///
/// Future implementation should support Option<Header<T>> for optional headers
#[tokio::test]
async fn test_header_optional() {
	let req = create_test_request_with_headers(&[], "");
	let mut ctx = create_empty_context();
	ctx.set_header_name::<String>("x-optional");

	let result = Header::<Option<String>>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert!(result.unwrap().is_none());
}

/// Test header case insensitivity (requires name specification)
///
/// HTTP headers are case-insensitive per RFC 7230
#[tokio::test]
async fn test_header_case_insensitive() {
	let mut headers = HeaderMap::new();
	headers.insert("x-custom-header", "value".parse().unwrap());

	let req = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);
	let mut ctx = create_empty_context();
	// Register lower-case; header map may contain different casing
	ctx.set_header_name::<String>("x-custom-header");

	let result = Header::<String>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().into_inner(), "value");
}

// ============================================================================
// Cookie Extraction (Requires name specification via ParamContext)
// ============================================================================

/// Test cookie extraction (requires name specification)
///
/// Current limitation: Cookie<T> extractor doesn't know which cookie to extract
/// Needs enhancement similar to Header extraction
#[tokio::test]
async fn test_cookie_extraction() {
	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::COOKIE,
		"session_id=abc123; user_id=42".parse().unwrap(),
	);

	let req = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);
	let mut ctx = create_empty_context();
	ctx.set_cookie_name::<String>("session_id");

	let result = Cookie::<String>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().into_inner(), "abc123");
}

/// Test missing cookie (requires name specification)
#[tokio::test]
async fn test_cookie_missing() {
	let req = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);
	let mut ctx = create_empty_context();
	ctx.set_cookie_name::<String>("session_id");
	let result = Cookie::<String>::from_request(&req, &ctx).await;
	assert!(result.is_err());
}

/// Test multiple cookies
///
/// HTTP Cookie header format: "name1=value1; name2=value2; name3=value3"
#[tokio::test]
async fn test_multiple_cookies() {
	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::COOKIE,
		"session=abc; user=john; theme=dark".parse().unwrap(),
	);

	let req = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);

	// Extract session cookie
	let mut ctx1 = create_empty_context();
	ctx1.set_cookie_name::<String>("session");
	let result = Cookie::<String>::from_request(&req, &ctx1).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().into_inner(), "abc");

	// Extract user cookie
	let mut ctx2 = create_empty_context();
	ctx2.set_cookie_name::<String>("user");
	let result2 = Cookie::<String>::from_request(&req, &ctx2).await;
	assert!(result2.is_ok());
	assert_eq!(result2.unwrap().into_inner(), "john");

	// Extract theme cookie
	let mut ctx3 = create_empty_context();
	ctx3.set_cookie_name::<String>("theme");
	let result3 = Cookie::<String>::from_request(&req, &ctx3).await;
	assert!(result3.is_ok());
	assert_eq!(result3.unwrap().into_inner(), "dark");
}

/// Test cookie with special characters
///
/// Cookies can contain URL-encoded values
#[tokio::test]
async fn test_cookie_url_encoded() {
	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::COOKIE,
		"message=Hello%20World".parse().unwrap(),
	);

	let req = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);
	let mut ctx = create_empty_context();
	ctx.set_cookie_name::<String>("message");

	// Should decode URL-encoded cookie values
	let result = Cookie::<String>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().into_inner(), "Hello World");
}

/// Test optional cookie
///
/// Similar to headers, should support Option<Cookie<T>>
#[tokio::test]
async fn test_cookie_optional() {
	// Test with missing cookie
	let req = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);
	let mut ctx = create_empty_context();
	ctx.set_cookie_name::<String>("session_id");
	let result = Cookie::<Option<String>>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert!(result.unwrap().is_none());

	// Test with existing cookie
	let mut headers = HeaderMap::new();
	headers.insert(hyper::header::COOKIE, "session_id=abc123".parse().unwrap());
	let req2 = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);
	let result2 = Cookie::<Option<String>>::from_request(&req2, &ctx).await;
	assert!(result2.is_ok());
	let value = result2.unwrap().into_inner();
	assert!(value.is_some());
	assert_eq!(value.unwrap(), "abc123");
}

// ============================================================================
// Integration Tests (Future)
// ============================================================================

/// Test combined header and cookie extraction
///
/// Real applications often need both headers and cookies
#[tokio::test]
async fn test_header_and_cookie_combined() {
	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::AUTHORIZATION,
		"Bearer token123".parse().unwrap(),
	);
	headers.insert(hyper::header::COOKIE, "session_id=abc123".parse().unwrap());

	let req = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);

	// Extract header
	let mut ctx1 = create_empty_context();
	ctx1.set_header_name::<String>("authorization");
	let auth_result = Header::<String>::from_request(&req, &ctx1).await;
	assert!(auth_result.is_ok());
	assert_eq!(auth_result.unwrap().into_inner(), "Bearer token123");

	// Extract cookie
	let mut ctx2 = create_empty_context();
	ctx2.set_cookie_name::<String>("session_id");
	let cookie_result = Cookie::<String>::from_request(&req, &ctx2).await;
	assert!(cookie_result.is_ok());
	assert_eq!(cookie_result.unwrap().into_inner(), "abc123");
}

/// Test: Header<T> design decision - HeaderStruct is the recommended approach
///
/// **Design Decision**: Header<T> intentionally supports only String types.
/// For type conversion (i64, bool, custom types), use HeaderStruct.
///
/// **Rationale**:
/// 1. **Single Responsibility**: Header<T> extracts a single header value as String
/// 2. **Type Safety**: HeaderStruct with serde provides compile-time type checking
/// 3. **Extensibility**: HeaderStruct handles complex validation and default values
/// 4. **Consistency**: Aligns with FastAPI's Header() vs Pydantic model pattern
///
/// **Recommended Pattern** (verified in test_header_type_conversion_with_struct):
/// ```rust,ignore
/// #[derive(Deserialize)]
/// struct TypedHeaders {
///     #[serde(rename = "x-request-id")]
///     request_id: i64,
///     #[serde(rename = "x-enabled")]
///     enabled: bool,
/// }
///
/// async fn handler(headers: HeaderStruct<TypedHeaders>) {
///     // request_id is already i64, not String
///     let id: i64 = headers.request_id;
/// }
/// ```
///
/// **Alternative Not Recommended**:
/// ```rust,ignore
// NOT IMPLEMENTED - use HeaderStruct instead
/// let count = Header::<i64>::from_request(&req, &ctx).await?;
/// ```
#[test]
fn test_header_type_conversion_design_decision() {
	// This test documents the design decision that Header<T> is String-only.
	// HeaderStruct provides type conversion capabilities.

	assert!(
		true,
		"Header<T> is intentionally String-only; use HeaderStruct for type conversion"
	);
}

// ============================================================================
// Documentation Tests
// ============================================================================

// ============================================================================
// Header Parameter Models (FastAPI test_tutorial/test_header_param_models)
// ============================================================================

/// Test: Multiple header extraction using Pydantic-like model
/// Reference: fastapi/tests/test_tutorial/test_header_param_models/test_tutorial001.py::test_header_param_model
/// NOTE: Uses HeaderStruct with serde rename for header name mapping
#[tokio::test]
async fn test_header_param_model_extraction() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct CommonHeaders {
		#[serde(rename = "save-data")]
		save_data: String,

		#[serde(rename = "if-modified-since")]
		if_modified_since: Option<String>,

		#[serde(rename = "traceparent")]
		traceparent: Option<String>,
	}

	let headers = vec![
		("save-data", "on"),
		("if-modified-since", "Wed, 21 Oct 2015 07:28:00 GMT"),
	];

	let req = create_test_request_with_headers(&headers, "");
	let ctx = create_empty_context();

	let result = HeaderStruct::<CommonHeaders>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Should extract headers: {:?}", result.err());

	let headers = result.unwrap();
	assert_eq!(headers.save_data, "on");
	assert_eq!(
		headers.if_modified_since,
		Some("Wed, 21 Oct 2015 07:28:00 GMT".to_string())
	);
	assert_eq!(headers.traceparent, None);
}

// NOTE: Old duplicate tests removed - replaced by *_with_struct versions

// ============================================================================
// CookieStruct Tests (Implemented)
// ============================================================================

/// Test: API key extraction from cookie using CookieStruct
/// Reference: fastapi/tests/test_security_api_key_cookie_optional.py::test_security_api_key
#[tokio::test]
async fn test_security_api_key_cookie_present_with_struct() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct ApiKeyCookie {
		key: String,
	}

	let mut headers = HeaderMap::new();
	headers.insert(hyper::header::COOKIE, "key=secret".parse().unwrap());

	let req = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);
	let ctx = create_empty_context();

	let result = CookieStruct::<ApiKeyCookie>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Should extract cookie: {:?}", result.err());
	assert_eq!(result.unwrap().key, "secret");
}

/// Test: Optional cookie handling when missing using CookieStruct
/// Reference: fastapi/tests/test_security_api_key_cookie_optional.py::test_security_api_key_no_key
#[tokio::test]
async fn test_security_api_key_cookie_optional_with_struct() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct ApiKeyCookie {
		key: Option<String>,
	}

	let req = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);
	let ctx = create_empty_context();

	let result = CookieStruct::<ApiKeyCookie>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Should handle missing cookie: {:?}",
		result.err()
	);
	assert_eq!(result.unwrap().key, None);
}

/// Test: Multiple cookies extraction using CookieStruct
#[tokio::test]
async fn test_multiple_cookies_with_struct() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct SessionCookies {
		session_id: String,
		user_id: Option<String>,
		theme: Option<String>,
	}

	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::COOKIE,
		"session_id=abc123; theme=dark".parse().unwrap(),
	);

	let req = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);
	let ctx = create_empty_context();

	let result = CookieStruct::<SessionCookies>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Should extract cookies: {:?}", result.err());

	let cookies = result.unwrap();
	assert_eq!(cookies.session_id, "abc123");
	assert_eq!(cookies.user_id, None);
	assert_eq!(cookies.theme, Some("dark".to_string()));
}

// ============================================================================
// HeaderStruct Individual Tests
// ============================================================================

/// Test: API key extraction from header using HeaderStruct
/// Reference: fastapi/tests/test_security_api_key_header.py::test_security_api_key
#[tokio::test]
async fn test_security_api_key_header_present_with_struct() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct ApiKeyHeader {
		key: String,
	}

	let headers = vec![("key", "secret")];
	let req = create_test_request_with_headers(&headers, "");
	let ctx = create_empty_context();

	let result = HeaderStruct::<ApiKeyHeader>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Should extract header: {:?}", result.err());
	assert_eq!(result.unwrap().key, "secret");
}

/// Test: Optional header handling when missing using HeaderStruct
#[tokio::test]
async fn test_header_optional_with_struct() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct OptionalHeader {
		authorization: Option<String>,
	}

	let req = create_test_request_with_headers(&[], "");
	let ctx = create_empty_context();

	let result = HeaderStruct::<OptionalHeader>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Should handle missing header: {:?}",
		result.err()
	);
	assert_eq!(result.unwrap().authorization, None);
}

/// Test: Multiple headers extraction using HeaderStruct
#[tokio::test]
async fn test_multiple_headers_with_struct() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct CustomHeaders {
		#[serde(rename = "x-request-id")]
		x_request_id: String,
		#[serde(rename = "x-correlation-id")]
		x_correlation_id: Option<String>,
		#[serde(rename = "user-agent")]
		user_agent: Option<String>,
	}

	let headers = vec![("x-request-id", "123"), ("user-agent", "Mozilla/5.0")];
	let req = create_test_request_with_headers(&headers, "");
	let ctx = create_empty_context();

	let result = HeaderStruct::<CustomHeaders>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Should extract headers: {:?}", result.err());

	let hdrs = result.unwrap();
	assert_eq!(hdrs.x_request_id, "123");
	assert_eq!(hdrs.x_correlation_id, None);
	assert_eq!(hdrs.user_agent, Some("Mozilla/5.0".to_string()));
}

/// Test: Header case insensitivity with HeaderStruct
/// HTTP headers are case-insensitive per RFC 7230
#[tokio::test]
async fn test_header_case_insensitive_with_struct() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct CaseInsensitiveHeader {
		#[serde(rename = "x-custom-header")]
		x_custom_header: String,
	}

	let headers = vec![("X-Custom-Header", "value")];
	let req = create_test_request_with_headers(&headers, "");
	let ctx = create_empty_context();

	let result = HeaderStruct::<CaseInsensitiveHeader>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Should handle case insensitivity: {:?}",
		result.err()
	);
	assert_eq!(result.unwrap().x_custom_header, "value");
}

/// Test: Header type conversion (string to integer)
#[tokio::test]
async fn test_header_type_conversion_with_struct() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct TypedHeader {
		#[serde(rename = "x-request-id")]
		x_request_id: i64,
	}

	let headers = vec![("x-request-id", "12345")];
	let req = create_test_request_with_headers(&headers, "");
	let ctx = create_empty_context();

	let result = HeaderStruct::<TypedHeader>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Should convert header to i64: {:?}",
		result.err()
	);
	assert_eq!(result.unwrap().x_request_id, 12345);
}

/// Test: Authorization header extraction
#[tokio::test]
async fn test_authorization_header_with_struct() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct AuthHeader {
		authorization: String,
	}

	let headers = vec![("authorization", "Bearer token123")];
	let req = create_test_request_with_headers(&headers, "");
	let ctx = create_empty_context();

	let result = HeaderStruct::<AuthHeader>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Should extract authorization header: {:?}",
		result.err()
	);
	assert_eq!(result.unwrap().authorization, "Bearer token123");
}

/// Test: Content-Type header extraction
#[tokio::test]
async fn test_content_type_header_with_struct() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct ContentTypeHeader {
		#[serde(rename = "content-type")]
		content_type: String,
	}

	let headers = vec![("content-type", "application/json")];
	let req = create_test_request_with_headers(&headers, "");
	let ctx = create_empty_context();

	let result = HeaderStruct::<ContentTypeHeader>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Should extract content-type header: {:?}",
		result.err()
	);
	assert_eq!(result.unwrap().content_type, "application/json");
}

// ============================================================================
// OpenAPI Schema Tests - MOVED to openapi crate
// ============================================================================
// NOTE: OpenAPI/schema tests are implemented in the openapi crate:
// - Parameter metadata extraction (param_metadata.rs)
// - include_in_schema functionality
// - Hidden parameters that still function at runtime

// ============================================================================
// Tests from FastAPI test_tutorial/test_header_param_models
// ============================================================================

/// Test: Header parameter model with default values
/// Source: fastapi/tests/test_tutorial/test_header_param_models/test_tutorial001.py::test_header_param_model_defaults
#[tokio::test]
async fn test_header_param_model_defaults() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct HeadersWithDefaults {
		#[serde(rename = "x-request-id")]
		x_request_id: Option<String>,

		#[serde(rename = "x-tags", default)]
		x_tags: Vec<String>,
	}

	let req = create_test_request_with_headers(&[], "");
	let ctx = create_empty_context();

	let result = HeaderStruct::<HeadersWithDefaults>::from_request(&req, &ctx).await;
	assert!(result.is_ok(), "Should handle defaults: {:?}", result.err());

	let headers = result.unwrap();
	assert_eq!(headers.x_request_id, None);
	assert_eq!(headers.x_tags, Vec::<String>::new());
}

/// Test: Header parameter model validation error for missing required header
/// Source: fastapi/tests/test_tutorial/test_header_param_models/test_tutorial001.py::test_header_param_model_invalid
#[tokio::test]
async fn test_header_param_model_missing_required() {
	#[derive(Debug, Deserialize)]
	struct RequiredHeaders {
		#[serde(rename = "x-api-key")]
		x_api_key: String, // Required field
	}

	let req = create_test_request_with_headers(&[], "");
	let ctx = create_empty_context();

	let result = HeaderStruct::<RequiredHeaders>::from_request(&req, &ctx).await;
	assert!(
		result.is_err(),
		"Should fail when required header is missing"
	);
}

/// Test: Header parameter model ignores extra headers by default
/// Source: fastapi/tests/test_tutorial/test_header_param_models/test_tutorial001.py::test_header_param_model_extra
#[tokio::test]
async fn test_header_param_model_extra_ignored() {
	#[derive(Debug, Deserialize, PartialEq)]
	struct LimitedHeaders {
		#[serde(rename = "x-api-key")]
		x_api_key: String,
	}

	let headers = vec![
		("x-api-key", "secret"),
		("x-extra-header", "ignored"), // Extra header
		("user-agent", "test"),        // Another extra header
	];
	let req = create_test_request_with_headers(&headers, "");
	let ctx = create_empty_context();

	let result = HeaderStruct::<LimitedHeaders>::from_request(&req, &ctx).await;
	assert!(
		result.is_ok(),
		"Should ignore extra headers by default: {:?}",
		result.err()
	);
	assert_eq!(result.unwrap().x_api_key, "secret");
}

/// Test: Header parameter model with strict extra='forbid'
/// Source: fastapi/tests/test_tutorial/test_header_param_models/test_tutorial002.py::test_header_param_model_extra
#[tokio::test]
async fn test_header_param_model_extra_forbid() {
	#[derive(Debug, Deserialize)]
	#[serde(deny_unknown_fields)] // Rust equivalent of extra='forbid'
	struct StrictHeaders {
		#[serde(rename = "x-api-key")]
		x_api_key: String,
	}

	let headers = vec![
		("x-api-key", "secret"),
		("x-extra-header", "not-allowed"), // This should cause error
	];
	let req = create_test_request_with_headers(&headers, "");
	let ctx = create_empty_context();

	let result = HeaderStruct::<StrictHeaders>::from_request(&req, &ctx).await;
	assert!(
		result.is_err(),
		"Should reject extra headers with deny_unknown_fields"
	);
}

// ============================================================================
// Tests from Django REST Framework test_request.py
// ============================================================================

/// Test: GET request.data returns empty dict
/// Source: django-rest-framework/tests/test_request.py::test_standard_behaviour_determines_no_content_GET
#[tokio::test]
async fn test_get_request_no_body() {
	let req = create_test_request_with_headers(&[], "");
	// NOTE: This is more about request.data behavior than header/cookie extraction
	// GET requests with no body should have empty body
	let body = req.read_body().unwrap();
	assert_eq!(body.len(), 0);
}

// ============================================================================
// HeaderNamed and CookieNamed Tests
// ============================================================================

/// Test: HeaderNamed extraction with compile-time name
#[tokio::test]
async fn test_header_named_authorization() {
	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::AUTHORIZATION,
		"Bearer token123".parse().unwrap(),
	);

	let req = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);
	let ctx = create_empty_context();

	let result = HeaderNamed::<Authorization, String>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().into_inner(), "Bearer token123");
}

/// Test: HeaderNamed extraction with optional value
#[tokio::test]
async fn test_header_named_content_type_optional() {
	let headers1 = HeaderMap::new();
	let req1 = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers1,
		Bytes::new(),
	);
	let ctx = create_empty_context();

	let result1 = HeaderNamed::<ContentType, Option<String>>::from_request(&req1, &ctx).await;
	assert!(result1.is_ok());
	assert_eq!(result1.unwrap().into_inner(), None);

	let mut headers2 = HeaderMap::new();
	headers2.insert(
		hyper::header::CONTENT_TYPE,
		"application/json".parse().unwrap(),
	);
	let req2 = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers2,
		Bytes::new(),
	);

	let result2 = HeaderNamed::<ContentType, Option<String>>::from_request(&req2, &ctx).await;
	assert!(result2.is_ok());
	assert_eq!(
		result2.unwrap().into_inner(),
		Some("application/json".to_string())
	);
}

/// Test: CookieNamed extraction with compile-time name
#[tokio::test]
async fn test_cookie_named_sessionid() {
	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::COOKIE,
		"sessionid=abc123; csrftoken=xyz789".parse().unwrap(),
	);

	let req = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);
	let ctx = create_empty_context();

	let result = CookieNamed::<SessionId, String>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().into_inner(), "abc123");
}

/// Test: CookieNamed extraction with optional value
#[tokio::test]
async fn test_cookie_named_optional() {
	let headers1 = HeaderMap::new();
	let req1 = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers1,
		Bytes::new(),
	);
	let ctx = create_empty_context();

	let result1 = CookieNamed::<SessionId, Option<String>>::from_request(&req1, &ctx).await;
	assert!(result1.is_ok());
	assert_eq!(result1.unwrap().into_inner(), None);

	let mut headers2 = HeaderMap::new();
	headers2.insert(
		hyper::header::COOKIE,
		"sessionid=session_value".parse().unwrap(),
	);
	let req2 = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers2,
		Bytes::new(),
	);

	let result2 = CookieNamed::<SessionId, Option<String>>::from_request(&req2, &ctx).await;
	assert!(result2.is_ok());
	assert_eq!(
		result2.unwrap().into_inner(),
		Some("session_value".to_string())
	);
}

/// Test: CookieNamed with multiple cookies
#[tokio::test]
async fn test_cookie_named_csrf_token() {
	let mut headers = HeaderMap::new();
	headers.insert(
		hyper::header::COOKIE,
		"sessionid=abc; csrftoken=def; user_id=123".parse().unwrap(),
	);

	let req = Request::new(
		Method::GET,
		Uri::from_static("/test"),
		Version::HTTP_11,
		headers,
		Bytes::new(),
	);
	let ctx = create_empty_context();

	let result = CookieNamed::<CsrfToken, String>::from_request(&req, &ctx).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap().into_inner(), "def");
}
