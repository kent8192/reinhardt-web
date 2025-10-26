//! Integration tests for validated parameter extraction
//!
//! Tests for ValidationConstraints with Path, Query, and Form parameters

use bytes::Bytes;
use hyper::{HeaderMap, Method, Uri, Version};
use reinhardt_apps::Request;
use reinhardt_params::{Path, Query, ValidatedPath, ValidatedQuery, WithValidation};
use std::collections::HashMap;

fn create_test_request(uri: &str, query: &str) -> Request {
    let full_uri = if query.is_empty() {
        uri.to_string()
    } else {
        format!("{}?{}", uri, query)
    };

    Request::new(
        Method::GET,
        full_uri.parse().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    )
}

fn create_path_context(params: Vec<(&str, &str)>) -> reinhardt_params::ParamContext {
    let mut map = HashMap::new();
    for (k, v) in params {
        map.insert(k.to_string(), v.to_string());
    }
    reinhardt_params::ParamContext::with_path_params(map)
}

// ============================================================================
// Path Parameter Validation Tests
// ============================================================================

/// Test: ValidatedPath with numeric range constraints
#[tokio::test]
async fn test_validated_path_numeric_range() {
    let path = Path(42i32);
    let validated = path.min_value(1).max_value(100);

    // Test within range
    assert!(validated.validate_number(&42).is_ok());
    assert!(validated.validate_number(&1).is_ok());
    assert!(validated.validate_number(&100).is_ok());

    // Test outside range
    assert!(validated.validate_number(&0).is_err());
    assert!(validated.validate_number(&101).is_err());
    assert!(validated.validate_number(&-1).is_err());
}

/// Test: ValidatedPath with string length constraints
#[tokio::test]
async fn test_validated_path_string_length() {
    let path = Path("hello".to_string());
    let validated = path.min_length(3).max_length(10);

    // Test within range
    assert!(validated.validate_string("hello").is_ok());
    assert!(validated.validate_string("abc").is_ok());
    assert!(validated.validate_string("0123456789").is_ok());

    // Test outside range
    assert!(validated.validate_string("ab").is_err()); // too short
    assert!(validated.validate_string("12345678901").is_err()); // too long
}

/// Test: ValidatedPath with email validation
#[tokio::test]
async fn test_validated_path_email() {
    let path = Path("test@example.com".to_string());
    let validated = path.email();

    // Valid emails
    assert!(validated.validate_string("test@example.com").is_ok());
    assert!(validated.validate_string("user@domain.co.jp").is_ok());
    assert!(validated.validate_string("admin+tag@company.org").is_ok());

    // Invalid emails
    assert!(validated.validate_string("not-an-email").is_err());
    assert!(validated.validate_string("@example.com").is_err());
    assert!(validated.validate_string("test@").is_err());
}

/// Test: ValidatedPath with URL validation
#[tokio::test]
async fn test_validated_path_url() {
    let path = Path("https://example.com".to_string());
    let validated = path.url();

    // Valid URLs
    assert!(validated.validate_string("https://example.com").is_ok());
    assert!(validated.validate_string("http://localhost:8000").is_ok());
    assert!(validated
        .validate_string("https://example.com/path?query=value")
        .is_ok());

    // Invalid URLs
    assert!(validated.validate_string("not-a-url").is_err());
    assert!(validated.validate_string("ftp://example.com").is_err());
}

/// Test: ValidatedPath with regex validation
#[tokio::test]
async fn test_validated_path_regex() {
    let path = Path("abc123".to_string());
    let validated = path.regex(r"^[a-z]+\d+$");

    // Valid patterns
    assert!(validated.validate_string("abc123").is_ok());
    assert!(validated.validate_string("test456").is_ok());

    // Invalid patterns
    assert!(validated.validate_string("ABC123").is_err()); // uppercase
    assert!(validated.validate_string("123abc").is_err()); // numbers first
    assert!(validated.validate_string("abc").is_err()); // no numbers
}

/// Test: ValidatedPath with combined constraints
#[tokio::test]
async fn test_validated_path_combined() {
    let path = Path("test@example.com".to_string());
    let validated = path.min_length(10).max_length(50).email();

    // Valid: meets all constraints
    assert!(validated.validate_string("test@example.com").is_ok());

    // Invalid: too short (even if valid email)
    assert!(validated.validate_string("a@b.com").is_err());

    // Invalid: too long
    assert!(validated
        .validate_string("verylongemailaddressthatshouldexceedfiftycharacters@example.com")
        .is_err());

    // Invalid: not an email
    assert!(validated.validate_string("not-an-email-string").is_err());
}

// ============================================================================
// Query Parameter Validation Tests
// ============================================================================

/// Test: ValidatedQuery with numeric constraints
#[tokio::test]
async fn test_validated_query_numeric() {
    let query = Query(25i32);
    let validated = query.min_value(0).max_value(100);

    assert!(validated.validate_number(&25).is_ok());
    assert!(validated.validate_number(&0).is_ok());
    assert!(validated.validate_number(&100).is_ok());
    assert!(validated.validate_number(&-1).is_err());
    assert!(validated.validate_number(&101).is_err());
}

/// Test: ValidatedQuery with string constraints
#[tokio::test]
async fn test_validated_query_string() {
    let query = Query("search".to_string());
    let validated = query.min_length(2).max_length(20);

    assert!(validated.validate_string("search").is_ok());
    assert!(validated.validate_string("ab").is_ok());
    assert!(validated.validate_string("12345678901234567890").is_ok());
    assert!(validated.validate_string("a").is_err());
    assert!(validated.validate_string("123456789012345678901").is_err());
}

// ============================================================================
// Chaining and Builder Pattern Tests
// ============================================================================

/// Test: Builder pattern chaining
#[tokio::test]
async fn test_validation_chaining() {
    let path = Path("https://example.com".to_string());
    let validated = path.min_length(10).max_length(100).url();

    // Valid
    assert!(validated.validate_string("https://example.com").is_ok());

    // Invalid: too short
    assert!(validated.validate_string("http://a").is_err());

    // Invalid: not a URL
    assert!(validated.validate_string("not-a-url").is_err());
}

/// Test: Deref trait for ValidationConstraints
#[tokio::test]
async fn test_validation_deref() {
    let path = Path(42i32);
    let validated = path.min_value(0).max_value(100);

    // Can access inner value through Deref
    assert_eq!(validated.0, 42);
}

/// Test: into_inner method
#[tokio::test]
async fn test_validation_into_inner() {
    let path = Path("test".to_string());
    let validated = path.min_length(2).max_length(10);

    let inner = validated.into_inner();
    assert_eq!(inner.0, "test");
}

// ============================================================================
// Error Message Tests
// ============================================================================

/// Test: Validation error messages are descriptive
#[tokio::test]
async fn test_validation_error_messages() {
    let path = Path(150i32);
    let validated = path.min_value(0).max_value(100);

    let result = validated.validate_number(&150);
    assert!(result.is_err());

    if let Err(e) = result {
        let msg = e.to_string();
        assert!(msg.contains("150")); // Should mention the value
        assert!(msg.contains("100")); // Should mention the max
    }
}

/// Test: String length error messages
#[tokio::test]
async fn test_string_validation_error_messages() {
    let path = Path("ab".to_string());
    let validated = path.min_length(5).max_length(10);

    let result = validated.validate_string("ab");
    assert!(result.is_err());

    if let Err(e) = result {
        let msg = e.to_string();
        assert!(msg.contains("2")); // Should mention actual length
        assert!(msg.contains("5")); // Should mention minimum length
    }
}

// ============================================================================
// Type Alias Tests
// ============================================================================

/// Test: ValidatedPath type alias works correctly
#[test]
fn test_validated_path_type_alias() {
    let _validated: ValidatedPath<i32> = Path(42).min_value(0).max_value(100);
    // Type check passes
}

/// Test: ValidatedQuery type alias works correctly
#[test]
fn test_validated_query_type_alias() {
    let _validated: ValidatedQuery<String> = Query("test".to_string()).min_length(1);
    // Type check passes
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Test: Empty string validation
#[tokio::test]
async fn test_empty_string_validation() {
    let path = Path("".to_string());
    let validated = path.min_length(1);

    assert!(validated.validate_string("").is_err());
    assert!(validated.validate_string("a").is_ok());
}

/// Test: Boundary values
#[tokio::test]
async fn test_boundary_values() {
    let path = Path(10i32);
    let validated = path.min_value(10).max_value(20);

    // Boundary values should be valid
    assert!(validated.validate_number(&10).is_ok());
    assert!(validated.validate_number(&20).is_ok());

    // Just outside boundaries should be invalid
    assert!(validated.validate_number(&9).is_err());
    assert!(validated.validate_number(&21).is_err());
}

/// Test: Float validation with decimal precision
#[tokio::test]
async fn test_float_validation() {
    let path = Path(3.14f64);
    let validated = path.min_value(0.0).max_value(10.0);

    assert!(validated.validate_number(&3.14).is_ok());
    assert!(validated.validate_number(&0.0).is_ok());
    assert!(validated.validate_number(&10.0).is_ok());
    assert!(validated.validate_number(&-0.1).is_err());
    assert!(validated.validate_number(&10.1).is_err());
}
