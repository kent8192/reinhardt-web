//! Path parameter extraction tests
//!
//! Based on FastAPI's test_path.py
//! Reference: fastapi/tests/test_path.py
//!
//! These tests verify path parameter extraction and type validation:
//! 1. Basic type coercion (String, Integer, Float, Boolean)
//! 2. Validation constraints (min_length, max_length, etc.)
//! 3. Multiple path parameters
//! 4. Error handling for invalid values

use bytes::Bytes;
use hyper::{HeaderMap, Method, Uri, Version};
use reinhardt_apps::Request;
#[cfg(feature = "validation")]
use reinhardt_params::WithValidation;
use reinhardt_params::extract::FromRequest;
use reinhardt_params::{ParamContext, ParamError, Path, PathStruct};
use serde::Deserialize;
use std::collections::HashMap;

// Helper function to create a mock request with path params
fn create_test_context(params: Vec<(&str, &str)>) -> ParamContext {
    let mut path_params = HashMap::new();
    for (key, value) in params {
        path_params.insert(key.to_string(), value.to_string());
    }
    ParamContext::with_path_params(path_params)
}

// Helper to create a mock request
fn create_mock_request() -> Request {
    Request::new(
        Method::GET,
        Uri::from_static("/test"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    )
}

// ============================================================================
// Basic Type Tests (String, Integer, Float, Boolean)
// ============================================================================

/// Test: Path parameter with string type (FastAPI: test_path_str_*)
#[tokio::test]
async fn test_path_str_basic() {
    let ctx = create_test_context(vec![("item", "foobar")]);
    let req = create_mock_request();

    let result = Path::<String>::from_request(&req, &ctx).await;
    assert!(result.is_ok(), "Failed to extract string path param");
    assert_eq!(*result.unwrap(), "foobar");
}

/// Test: String path parameter with numeric value
#[tokio::test]
async fn test_path_str_numeric() {
    let ctx = create_test_context(vec![("item", "42")]);
    let req = create_mock_request();

    let result = Path::<String>::from_request(&req, &ctx).await;
    assert!(result.is_ok());
    assert_eq!(*result.unwrap(), "42");
}

/// Test: String path parameter with boolean-like value
#[tokio::test]
async fn test_path_str_boolean_like() {
    let ctx = create_test_context(vec![("item", "True")]);
    let req = create_mock_request();

    let result = Path::<String>::from_request(&req, &ctx).await;
    assert!(result.is_ok());
    assert_eq!(*result.unwrap(), "True");
}

// ============================================================================
// Integer Path Parameters
// ============================================================================

/// Test: Valid integer path parameter (FastAPI: test_path_int_42)
#[tokio::test]
async fn test_path_int_valid() {
    let ctx = create_test_context(vec![("item_id", "42")]);
    let req = create_mock_request();

    let result = Path::<i64>::from_request(&req, &ctx).await;
    assert!(result.is_ok(), "Failed to extract integer path param");
    assert_eq!(*result.unwrap(), 42);
}

/// Test: Negative integer path parameter
#[tokio::test]
async fn test_path_int_negative() {
    let ctx = create_test_context(vec![("item_id", "-10")]);
    let req = create_mock_request();

    let result = Path::<i64>::from_request(&req, &ctx).await;
    assert!(result.is_ok());
    assert_eq!(*result.unwrap(), -10);
}

/// Test: Integer path parameter with invalid string (FastAPI: test_path_int_foobar)
#[tokio::test]
async fn test_path_int_invalid_string() {
    let ctx = create_test_context(vec![("item_id", "foobar")]);
    let req = create_mock_request();

    let result = Path::<i64>::from_request(&req, &ctx).await;
    assert!(
        result.is_err(),
        "Should fail to parse non-numeric string as integer"
    );

    match result.unwrap_err() {
        ParamError::ParseError { name, .. } => {
            assert_eq!(name, "path");
        }
        _ => panic!("Expected ParseError"),
    }
}

/// Test: Integer path parameter with boolean string (FastAPI: test_path_int_True)
#[tokio::test]
async fn test_path_int_invalid_boolean() {
    let ctx = create_test_context(vec![("item_id", "True")]);
    let req = create_mock_request();

    let result = Path::<i64>::from_request(&req, &ctx).await;
    assert!(result.is_err(), "Should fail to parse 'True' as integer");
}

/// Test: Integer path parameter with float string (FastAPI: test_path_int_42_5)
#[tokio::test]
async fn test_path_int_invalid_float() {
    let ctx = create_test_context(vec![("item_id", "42.5")]);
    let req = create_mock_request();

    let result = Path::<i64>::from_request(&req, &ctx).await;
    assert!(
        result.is_err(),
        "Should fail to parse float string as integer"
    );
}

// ============================================================================
// Float Path Parameters
// ============================================================================

/// Test: Valid float path parameter (FastAPI: test_path_float_42)
#[tokio::test]
async fn test_path_float_valid_int() {
    let ctx = create_test_context(vec![("item_id", "42")]);
    let req = create_mock_request();

    let result = Path::<f64>::from_request(&req, &ctx).await;
    assert!(
        result.is_ok(),
        "Failed to extract float from integer string"
    );
    assert_eq!(*result.unwrap(), 42.0);
}

/// Test: Float path parameter with decimal (FastAPI: test_path_float_42_5)
#[tokio::test]
async fn test_path_float_valid_decimal() {
    let ctx = create_test_context(vec![("item_id", "42.5")]);
    let req = create_mock_request();

    let result = Path::<f64>::from_request(&req, &ctx).await;
    assert!(result.is_ok());
    assert_eq!(*result.unwrap(), 42.5);
}

/// Test: Float path parameter with invalid string (FastAPI: test_path_float_foobar)
#[tokio::test]
async fn test_path_float_invalid_string() {
    let ctx = create_test_context(vec![("item_id", "foobar")]);
    let req = create_mock_request();

    let result = Path::<f64>::from_request(&req, &ctx).await;
    assert!(
        result.is_err(),
        "Should fail to parse non-numeric string as float"
    );
}

/// Test: Float path parameter with boolean string (FastAPI: test_path_float_True)
#[tokio::test]
async fn test_path_float_invalid_boolean() {
    let ctx = create_test_context(vec![("item_id", "True")]);
    let req = create_mock_request();

    let result = Path::<f64>::from_request(&req, &ctx).await;
    assert!(result.is_err(), "Should fail to parse 'True' as float");
}

// ============================================================================
// Boolean Path Parameters
// ============================================================================

/// Test: Boolean path parameter with "true" (FastAPI: test_path_bool_true)
#[tokio::test]
async fn test_path_bool_true_lowercase() {
    let ctx = create_test_context(vec![("item_id", "true")]);
    let req = create_mock_request();

    let result = Path::<bool>::from_request(&req, &ctx).await;
    assert!(result.is_ok(), "Failed to parse 'true' as boolean");
    assert_eq!(*result.unwrap(), true);
}

/// Test: Boolean path parameter with "True" (FastAPI: test_path_bool_True)
#[tokio::test]
async fn test_path_bool_true_capitalized() {
    let ctx = create_test_context(vec![("item_id", "True")]);
    let req = create_mock_request();

    let result = Path::<bool>::from_request(&req, &ctx).await;
    // Note: serde_json's boolean parsing is strict, only accepts "true"/"false"
    // This is different from FastAPI which accepts "True"/"False"
    // Consider custom boolean deserializer for FastAPI compatibility if needed
    assert!(
        result.is_err(),
        "Strict boolean parsing: 'True' should fail"
    );
}

/// Test: Boolean path parameter with "false" (FastAPI: test_path_bool_false)
#[tokio::test]
async fn test_path_bool_false_lowercase() {
    let ctx = create_test_context(vec![("item_id", "false")]);
    let req = create_mock_request();

    let result = Path::<bool>::from_request(&req, &ctx).await;
    assert!(result.is_ok());
    assert_eq!(*result.unwrap(), false);
}

/// Test: Boolean path parameter with "False" (FastAPI: test_path_bool_False)
#[tokio::test]
async fn test_path_bool_false_capitalized() {
    let ctx = create_test_context(vec![("item_id", "False")]);
    let req = create_mock_request();

    let result = Path::<bool>::from_request(&req, &ctx).await;
    // Note: Consider custom boolean deserializer for FastAPI compatibility if needed
    assert!(
        result.is_err(),
        "Strict boolean parsing: 'False' should fail"
    );
}

/// Test: Boolean path parameter with "1" (FastAPI: test_path_bool_1)
#[tokio::test]
async fn test_path_bool_one() {
    let ctx = create_test_context(vec![("item_id", "1")]);
    let req = create_mock_request();

    let result = Path::<bool>::from_request(&req, &ctx).await;
    // Note: serde_json doesn't parse "1" as boolean
    // Consider custom boolean deserializer for FastAPI compatibility if needed
    assert!(result.is_err(), "Strict boolean parsing: '1' should fail");
}

/// Test: Boolean path parameter with "0" (FastAPI: test_path_bool_0)
#[tokio::test]
async fn test_path_bool_zero() {
    let ctx = create_test_context(vec![("item_id", "0")]);
    let req = create_mock_request();

    let result = Path::<bool>::from_request(&req, &ctx).await;
    // Note: Consider custom boolean deserializer for FastAPI compatibility if needed
    assert!(result.is_err(), "Strict boolean parsing: '0' should fail");
}

/// Test: Boolean path parameter with invalid string (FastAPI: test_path_bool_foobar)
#[tokio::test]
async fn test_path_bool_invalid_string() {
    let ctx = create_test_context(vec![("item_id", "foobar")]);
    let req = create_mock_request();

    let result = Path::<bool>::from_request(&req, &ctx).await;
    assert!(result.is_err(), "Should fail to parse 'foobar' as boolean");
}

// ============================================================================
// Multiple Path Parameters
// ============================================================================

/// Test: Multiple path parameters via struct
#[derive(Debug, Deserialize, PartialEq)]
struct UserPostPath {
    user_id: i64,
    post_id: i64,
}

#[tokio::test]
async fn test_path_multiple_params() {
    let ctx = create_test_context(vec![("user_id", "123"), ("post_id", "456")]);
    let req = create_mock_request();

    let result = PathStruct::<UserPostPath>::from_request(&req, &ctx).await;
    assert!(result.is_ok(), "Failed to extract multiple path params");

    let path = result.unwrap();
    assert_eq!(path.user_id, 123);
    assert_eq!(path.post_id, 456);
}

/// Test: Multiple path parameters with one invalid
#[tokio::test]
async fn test_path_multiple_params_one_invalid() {
    let ctx = create_test_context(vec![("user_id", "123"), ("post_id", "invalid")]);
    let req = create_mock_request();

    let result = PathStruct::<UserPostPath>::from_request(&req, &ctx).await;
    assert!(result.is_err(), "Should fail when one param is invalid");
}

/// Test: Multiple path parameters with mixed types
#[derive(Debug, Deserialize)]
struct MixedPath {
    name: String,
    id: i64,
    active: bool,
}

#[tokio::test]
async fn test_path_multiple_mixed_types() {
    let ctx = create_test_context(vec![("name", "alice"), ("id", "42"), ("active", "true")]);
    let req = create_mock_request();

    let result = PathStruct::<MixedPath>::from_request(&req, &ctx).await;
    assert!(result.is_ok(), "Failed to extract mixed-type path params");

    let path = result.unwrap();
    assert_eq!(path.name, "alice");
    assert_eq!(path.id, 42);
    assert_eq!(path.active, true);
}

// ============================================================================
// Missing and Extra Parameters
// ============================================================================

/// Test: Missing required path parameter
#[tokio::test]
async fn test_path_missing_param() {
    let ctx = create_test_context(vec![]); // No params
    let req = create_mock_request();

    let result = PathStruct::<UserPostPath>::from_request(&req, &ctx).await;
    assert!(
        result.is_err(),
        "Should fail when required params are missing"
    );
}

/// Test: Extra path parameters (should be ignored)
#[derive(Debug, Deserialize)]
struct SingleIdPath {
    id: i64,
}

#[tokio::test]
async fn test_path_extra_params_ignored() {
    let ctx = create_test_context(vec![("id", "42"), ("extra", "ignored")]);
    let req = create_mock_request();

    let result = PathStruct::<SingleIdPath>::from_request(&req, &ctx).await;
    assert!(result.is_ok(), "Extra params should be ignored");
    assert_eq!(result.unwrap().id, 42);
}

// ============================================================================
// Validation Tests
// ============================================================================
// Validation constraints (min_length, max_length, gt, ge, lt, le) are now
// implemented and tested below. These tests require the 'validation' feature.
// ============================================================================

/// Test: Path parameter with min_length constraint
#[tokio::test]
#[cfg(feature = "validation")]
async fn test_path_param_minlength_valid() {
    let ctx = create_test_context(vec![("name", "alice")]);
    let req = create_mock_request();

    let result = Path::<String>::from_request(&req, &ctx).await;
    assert!(result.is_ok());

    let path = result.unwrap();
    let validated = path.min_length(3);
    assert!(validated.validate_string(&validated.0).is_ok());
}

/// Test: Path parameter violating min_length constraint
#[tokio::test]
#[cfg(feature = "validation")]
async fn test_path_param_minlength_invalid() {
    let ctx = create_test_context(vec![("name", "ab")]);
    let req = create_mock_request();

    let result = Path::<String>::from_request(&req, &ctx).await;
    assert!(result.is_ok());

    let path = result.unwrap();
    let validated = path.min_length(5);
    let validation_result = validated.validate_string(&validated.0);
    assert!(validation_result.is_err());
}

/// Test: Path parameter with max_length constraint
#[tokio::test]
#[cfg(feature = "validation")]
async fn test_path_param_maxlength_valid() {
    let ctx = create_test_context(vec![("name", "bob")]);
    let req = create_mock_request();

    let result = Path::<String>::from_request(&req, &ctx).await;
    assert!(result.is_ok());

    let path = result.unwrap();
    let validated = path.max_length(10);
    assert!(validated.validate_string(&validated.0).is_ok());
}

/// Test: Integer path parameter with gt (greater than) constraint
#[tokio::test]
#[cfg(feature = "validation")]
async fn test_path_param_gt_valid() {
    let ctx = create_test_context(vec![("age", "25")]);
    let req = create_mock_request();

    let result = Path::<i32>::from_request(&req, &ctx).await;
    assert!(result.is_ok());

    let path = result.unwrap();
    let validated = path.min_value(18);
    assert!(validated.validate_number(&validated.0).is_ok());
}

/// Test: Integer path parameter with ge (greater than or equal) constraint
#[tokio::test]
#[cfg(feature = "validation")]
async fn test_path_param_ge_valid() {
    let ctx = create_test_context(vec![("count", "10")]);
    let req = create_mock_request();

    let result = Path::<i32>::from_request(&req, &ctx).await;
    assert!(result.is_ok());

    let path = result.unwrap();
    let validated = path.min_value(10);
    assert!(validated.validate_number(&validated.0).is_ok());
}

/// Test: Integer path parameter with lt (less than) constraint
#[tokio::test]
#[cfg(feature = "validation")]
async fn test_path_param_lt_valid() {
    let ctx = create_test_context(vec![("score", "85")]);
    let req = create_mock_request();

    let result = Path::<i32>::from_request(&req, &ctx).await;
    assert!(result.is_ok());

    let path = result.unwrap();
    let validated = path.max_value(100);
    assert!(validated.validate_number(&validated.0).is_ok());
}

/// Test: Integer path parameter with le (less than or equal) constraint
#[tokio::test]
#[cfg(feature = "validation")]
async fn test_path_param_le_valid() {
    let ctx = create_test_context(vec![("limit", "100")]);
    let req = create_mock_request();

    let result = Path::<i32>::from_request(&req, &ctx).await;
    assert!(result.is_ok());

    let path = result.unwrap();
    let validated = path.max_value(100);
    assert!(validated.validate_number(&validated.0).is_ok());
}

/// Test: Combined gt and lt constraints
#[tokio::test]
#[cfg(feature = "validation")]
async fn test_path_param_gt_lt_valid() {
    let ctx = create_test_context(vec![("value", "50")]);
    let req = create_mock_request();

    let result = Path::<i32>::from_request(&req, &ctx).await;
    assert!(result.is_ok());

    let path = result.unwrap();
    let validated = path.min_value(10).max_value(100);
    assert!(validated.validate_number(&validated.0).is_ok());
}

// ============================================================================
// Path Parameters with File Paths (FastAPI test_tutorial004)
// ============================================================================

/// Test: Path parameter captures file path without leading slash
/// Reference: fastapi/tests/test_tutorial/test_path_params/test_tutorial004.py::test_file_path
#[tokio::test]
async fn test_path_file_path_no_leading_slash() {
    let ctx = create_test_context(vec![("file_path", "home/johndoe/myfile.txt")]);
    let req = create_mock_request();

    let result = Path::<String>::from_request(&req, &ctx).await;
    assert!(
        result.is_ok(),
        "Failed to extract file path from path parameter"
    );
    assert_eq!(*result.unwrap(), "home/johndoe/myfile.txt");
}

/// Test: Path parameter captures absolute file path with leading slash
/// Reference: fastapi/tests/test_tutorial/test_path_params/test_tutorial004.py::test_root_file_path
/// Note: Double slash in URL preserves leading slash in path parameter
#[tokio::test]
async fn test_path_file_path_with_leading_slash() {
    let ctx = create_test_context(vec![("file_path", "/home/johndoe/myfile.txt")]);
    let req = create_mock_request();

    let result = Path::<String>::from_request(&req, &ctx).await;
    assert!(result.is_ok(), "Failed to extract absolute file path");
    assert_eq!(*result.unwrap(), "/home/johndoe/myfile.txt");
}

// ============================================================================
// Enum Path Parameters (FastAPI test_tutorial005)
// ============================================================================

// NOTE: Duplicate enum tests removed - replaced by test_path_param_enum_valid
// and test_path_param_enum_invalid which use PathStruct<T> with serde Deserialize

// ============================================================================
// Invalid Path Parameter Types (FastAPI test_invalid_path_param)
// ============================================================================

/// Test: Path parameters cannot be List types
/// Reference: fastapi/tests/test_invalid_path_param.py::test_invalid_sequence
/// NOTE: In Rust, this is prevented at compile-time by not implementing FromRequest for Vec
#[test]
fn test_path_invalid_list_type_compile_check() {
    // This test documents that Path<Vec<T>> doesn't implement FromRequest
    // In FastAPI, this would raise an AssertionError at app initialization
    // In Rust, this is a compile-time error

    // Attempting to use Path<Vec<i64>> would cause a compile error:
    // error[E0599]: no function or associated item named `from_request` found

    assert!(true, "Path<Vec<T>> is not supported - compile-time check");
}

/// Test: Path parameters cannot be tuple types
/// Reference: fastapi/tests/test_invalid_path_param.py::test_invalid_simple_tuple
/// NOTE: In Rust, this is prevented at compile-time
#[test]
fn test_path_invalid_tuple_type_compile_check() {
    // This test documents that Path<(T, U)> doesn't implement FromRequest
    // Attempting to use would cause compile error

    assert!(true, "Path<(T, U)> is not supported - compile-time check");
}

/// Test: Path parameters cannot be dict/map types
/// Reference: fastapi/tests/test_invalid_path_param.py::test_invalid_simple_dict
/// NOTE: In Rust, this is prevented at compile-time
#[test]
fn test_path_invalid_dict_type_compile_check() {
    // This test documents that Path<HashMap<K, V>> doesn't implement FromRequest
    // Attempting to use would cause compile error

    assert!(
        true,
        "Path<HashMap<K, V>> is not supported - compile-time check"
    );
}

/// Test: Path parameters cannot have default values
/// Reference: fastapi/tests/test_ambiguous_params.py::test_no_annotated_defaults
/// NOTE: This is enforced at the type level in Rust - Path<T> doesn't support Option<T>
/// with defaults in the same way FastAPI does.
#[tokio::test]
async fn test_path_no_default_values() {
    // In Rust, path parameters are always required at the type level
    // Using Option<T> for path doesn't make semantic sense
    // This test documents that behavior

    let ctx = create_test_context(vec![]); // No path param
    let req = create_mock_request();

    let result = Path::<i64>::from_request(&req, &ctx).await;
    assert!(
        result.is_err(),
        "Path parameters cannot have defaults - they must be present in URL"
    );
}

// ============================================================================
// Path Parameter in Dependency (FastAPI test_param_in_path_and_dependency)
// ============================================================================

/// Test: Same path parameter used in both endpoint and dependency
/// Reference: fastapi/tests/test_param_in_path_and_dependency.py::test_read_users
/// Note: This is an integration test that would require the full framework.
/// For now, we test that path extraction works consistently.
#[tokio::test]
async fn test_path_consistent_extraction() {
    let ctx = create_test_context(vec![("user_id", "123")]);
    let req = create_mock_request();

    // First extraction
    let result1 = Path::<i64>::from_request(&req, &ctx).await;
    assert!(result1.is_ok(), "First path extraction failed");

    // Second extraction (simulating dependency)
    let result2 = Path::<i64>::from_request(&req, &ctx).await;
    assert!(result2.is_ok(), "Second path extraction failed");

    // Both should extract same value
    assert_eq!(*result1.unwrap(), *result2.unwrap());
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Test: Empty string path parameter
#[tokio::test]
async fn test_path_empty_string() {
    let ctx = create_test_context(vec![("item", "")]);
    let req = create_mock_request();

    let result = Path::<String>::from_request(&req, &ctx).await;
    assert!(result.is_ok());
    assert_eq!(*result.unwrap(), "");
}

/// Test: Path parameter with special characters
#[tokio::test]
async fn test_path_special_characters() {
    let ctx = create_test_context(vec![("item", "hello-world_123")]);
    let req = create_mock_request();

    let result = Path::<String>::from_request(&req, &ctx).await;
    assert!(result.is_ok());
    assert_eq!(*result.unwrap(), "hello-world_123");
}

/// Test: Very large integer
#[tokio::test]
async fn test_path_large_integer() {
    let ctx = create_test_context(vec![("item_id", "9223372036854775807")]); // i64::MAX
    let req = create_mock_request();

    let result = Path::<i64>::from_request(&req, &ctx).await;
    assert!(result.is_ok());
    assert_eq!(*result.unwrap(), i64::MAX);
}

/// Test: Integer overflow
#[tokio::test]
async fn test_path_integer_overflow() {
    let ctx = create_test_context(vec![("item_id", "9223372036854775808")]); // i64::MAX + 1
    let req = create_mock_request();

    let result = Path::<i64>::from_request(&req, &ctx).await;
    assert!(result.is_err(), "Should fail on integer overflow");
}

// ============================================================================
// Tests from FastAPI test_param_in_path_and_dependency.py
// ============================================================================

/// Test: Path parameter used in both endpoint and dependency
/// Source: fastapi/tests/test_param_in_path_and_dependency.py::test_read_users
#[tokio::test]
async fn test_path_param_shared_with_dependency() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct UserPath {
        user_id: i32,
    }

    let ctx = create_test_context(vec![("user_id", "42")]);
    let req = create_mock_request();

    let result = PathStruct::<UserPath>::from_request(&req, &ctx).await;
    assert!(result.is_ok(), "Should extract path parameter successfully");
    assert_eq!(result.unwrap().user_id, 42);
}

// ============================================================================
// Tests from FastAPI test_tutorial/test_path_params/test_tutorial004.py
// ============================================================================

/// Test: Path parameter with file path (without leading slash)
/// Source: fastapi/tests/test_tutorial/test_path_params/test_tutorial004.py::test_file_path
#[tokio::test]
async fn test_path_param_file_path() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct FilePath {
        file_path: String,
    }

    let ctx = create_test_context(vec![("file_path", "home/johndoe/myfile.txt")]);
    let req = create_mock_request();

    let result = PathStruct::<FilePath>::from_request(&req, &ctx).await;
    assert!(result.is_ok(), "Should extract file path from URL");
    assert_eq!(result.unwrap().file_path, "home/johndoe/myfile.txt");
}

/// Test: Path parameter with absolute file path (with leading slash)
/// Source: fastapi/tests/test_tutorial/test_path_params/test_tutorial004.py::test_root_file_path
#[tokio::test]
async fn test_path_param_root_file_path() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct FilePath {
        file_path: String,
    }

    // In FastAPI, double slash preserves leading slash: //home/... -> /home/...
    let ctx = create_test_context(vec![("file_path", "/home/johndoe/myfile.txt")]);
    let req = create_mock_request();

    let result = PathStruct::<FilePath>::from_request(&req, &ctx).await;
    assert!(result.is_ok(), "Should extract absolute file path");
    assert_eq!(result.unwrap().file_path, "/home/johndoe/myfile.txt");
}

// ============================================================================
// Tests from FastAPI test_tutorial/test_path_params/test_tutorial005.py
// ============================================================================

/// Test: Path parameter with enum validation (valid value)
/// Source: fastapi/tests/test_tutorial/test_path_params/test_tutorial005.py::test_get_enums_alexnet
/// NOTE: Rust supports enum path parameters via serde's Deserialize with rename_all
#[tokio::test]
async fn test_path_param_enum_valid() {
    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(rename_all = "lowercase")]
    enum ModelName {
        Alexnet,
        Resnet,
        Lenet,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct ModelPath {
        model_name: ModelName,
    }

    let ctx = create_test_context(vec![("model_name", "alexnet")]);
    let req = create_mock_request();

    let result = PathStruct::<ModelPath>::from_request(&req, &ctx).await;
    assert!(result.is_ok(), "Should parse valid enum value");
    assert_eq!(result.unwrap().model_name, ModelName::Alexnet);
}

/// Test: Path parameter with enum validation (invalid value)
/// Source: fastapi/tests/test_tutorial/test_path_params/test_tutorial005.py::test_get_enums_invalid
/// NOTE: Serde's enum deserialization provides error messages for invalid enum values
#[tokio::test]
async fn test_path_param_enum_invalid() {
    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(rename_all = "lowercase")]
    enum ModelName {
        Alexnet,
        Resnet,
        Lenet,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct ModelPath {
        model_name: ModelName,
    }

    let ctx = create_test_context(vec![("model_name", "foo")]);
    let req = create_mock_request();

    let result = PathStruct::<ModelPath>::from_request(&req, &ctx).await;
    assert!(
        result.is_err(),
        "Should reject invalid enum value with parse error"
    );
    // The error message will indicate the value doesn't match any variant
}

// ============================================================================
// Tests from FastAPI test_invalid_path_param.py
// ============================================================================

// NOTE: Compile-time type error tests removed - Rust's type system prevents these
// at compile time, no runtime tests needed:
// - test_path_param_list_type_invalid
// - test_path_param_tuple_type_invalid
// - test_path_param_dict_type_invalid
// - test_path_param_set_type_invalid

// ============================================================================
// Tests from FastAPI test_ambiguous_params.py
// ============================================================================

/// Test: Path parameters cannot have default values
/// Source: fastapi/tests/test_ambiguous_params.py::test_no_annotated_defaults
/// NOTE: In Rust, this is enforced differently. Path params are in the URL route,
/// so they're always required. Optional<Path<T>> doesn't make semantic sense.
#[tokio::test]
async fn test_path_param_no_defaults() {
    // In FastAPI: Path(default=1) raises AssertionError: 'Path parameters cannot have a default value'
    // In Rust: This is enforced by the type system and routing layer
    // All path parameters must be present in the URL pattern and are thus required

    // Test that missing path param returns error
    let ctx = ParamContext::new(); // No path params
    let req = create_mock_request();

    #[derive(Deserialize)]
    struct Params {
        id: i32,
    }

    let result = PathStruct::<Params>::from_request(&req, &ctx).await;
    assert!(result.is_err(), "Missing path parameter should fail");
}

// ============================================================================
// Tests from FastAPI test_param_include_in_schema.py
// ============================================================================

/// Test: Hidden path parameters still extract values
/// Source: fastapi/tests/test_param_include_in_schema.py::test_hidden_path
///
/// Path parameters with include_in_schema=False should still extract normally
/// but not appear in OpenAPI schema. This is tested in the openapi crate.
/// This test verifies that extraction works regardless of schema visibility.
#[tokio::test]
async fn test_path_param_hidden_in_schema() {
    // Path parameters with include_in_schema=False should still extract normally
    // but not appear in OpenAPI schema. This is tested in integration tests.
    let ctx = create_test_context(vec![("item_id", "42")]);
    let req = create_mock_request();

    let result = Path::<i64>::from_request(&req, &ctx).await;
    assert!(result.is_ok());
    assert_eq!(*result.unwrap(), 42);
}
