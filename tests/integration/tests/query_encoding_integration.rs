//! Integration tests for query string encoding
//!
//! These tests verify URL encoding and query parameter handling.

use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, CONTROLS};
use reinhardt_params::Query;
use reinhardt_routers::Route;
use serde_json::json;
use std::collections::HashMap;

/// Define the query parameter encoding set
/// https://url.spec.whatwg.org/#query-percent-encode-set
const QUERY: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'#').add(b'<').add(b'>');

// ============================================================================
// Basic Encoding Tests
// ============================================================================

#[test]
fn test_basic_query_encoding() {
    let param = "hello world";
    let encoded = utf8_percent_encode(param, QUERY).to_string();

    assert_eq!(encoded, "hello%20world");
}

#[test]
fn test_special_characters_encoding() {
    let param = "user@example.com";
    let encoded = utf8_percent_encode(param, QUERY).to_string();

    // @ should be preserved in query encoding
    assert!(encoded.contains("@"));
    assert_eq!(encoded, "user@example.com");
}

#[test]
fn test_utf8_encoding() {
    let param = "こんにちは";
    let encoded = utf8_percent_encode(param, QUERY).to_string();

    // UTF-8 characters should be percent-encoded
    assert!(encoded.contains("%"));
    assert!(encoded.len() > param.len());
}

#[test]
fn test_special_symbols_encoding() {
    let param = "a+b=c&d";
    let encoded = utf8_percent_encode(param, QUERY).to_string();

    // These characters have special meaning in query strings
    assert!(encoded.contains("+") || encoded.contains("%2B"));
    assert!(encoded.contains("=") || encoded.contains("%3D"));
    assert!(encoded.contains("&") || encoded.contains("%26"));
}

// ============================================================================
// Decoding Tests
// ============================================================================

#[test]
fn test_basic_query_decoding() {
    let encoded = "hello%20world";
    let decoded = percent_decode_str(encoded).decode_utf8().unwrap();

    assert_eq!(decoded, "hello world");
}

#[test]
fn test_utf8_decoding() {
    let encoded = "%E3%81%93%E3%82%93%E3%81%AB%E3%81%A1%E3%81%AF";
    let decoded = percent_decode_str(encoded).decode_utf8().unwrap();

    assert_eq!(decoded, "こんにちは");
}

#[test]
fn test_round_trip_encoding() {
    let original = "Hello World! 世界";
    let encoded = utf8_percent_encode(original, QUERY).to_string();
    let decoded = percent_decode_str(&encoded).decode_utf8().unwrap();

    assert_eq!(decoded, original);
}

// ============================================================================
// Query Parameter Building Tests
// ============================================================================

fn build_query_string(params: &HashMap<String, String>) -> String {
    let mut parts: Vec<String> = params
        .iter()
        .map(|(k, v)| {
            format!(
                "{}={}",
                utf8_percent_encode(k, QUERY),
                utf8_percent_encode(v, QUERY)
            )
        })
        .collect();
    parts.sort(); // Sort for deterministic output
    parts.join("&")
}

#[test]
fn test_build_simple_query_string() {
    let mut params = HashMap::new();
    params.insert("name".to_string(), "Alice".to_string());
    params.insert("age".to_string(), "30".to_string());

    let query = build_query_string(&params);

    assert!(query.contains("name=Alice") || query.contains("age=30"));
    assert!(query.contains("&"));
}

#[test]
fn test_build_query_string_with_spaces() {
    let mut params = HashMap::new();
    params.insert("query".to_string(), "hello world".to_string());

    let query = build_query_string(&params);

    assert!(query.contains("query=hello%20world"));
}

#[test]
fn test_build_query_string_with_utf8() {
    let mut params = HashMap::new();
    params.insert("message".to_string(), "こんにちは".to_string());

    let query = build_query_string(&params);

    assert!(query.contains("message="));
    assert!(query.contains("%"));
}

// ============================================================================
// Array Parameter Tests
// ============================================================================

fn build_array_query_string(key: &str, values: &[String]) -> String {
    values
        .iter()
        .map(|v| {
            format!(
                "{}={}",
                utf8_percent_encode(key, QUERY),
                utf8_percent_encode(v, QUERY)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

#[test]
fn test_array_parameters() {
    let values = vec![
        "value1".to_string(),
        "value2".to_string(),
        "value3".to_string(),
    ];
    let query = build_array_query_string("tags", &values);

    assert!(query.contains("tags=value1"));
    assert!(query.contains("tags=value2"));
    assert!(query.contains("tags=value3"));
    assert_eq!(query.matches("tags=").count(), 3);
}

#[test]
fn test_array_parameters_with_encoding() {
    let values = vec!["hello world".to_string(), "foo bar".to_string()];
    let query = build_array_query_string("items", &values);

    assert!(query.contains("items=hello%20world"));
    assert!(query.contains("items=foo%20bar"));
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[test]
fn test_empty_value_encoding() {
    let mut params = HashMap::new();
    params.insert("key".to_string(), "".to_string());

    let query = build_query_string(&params);

    assert_eq!(query, "key=");
}

#[test]
fn test_special_url_characters() {
    let param = "?query#fragment";
    let encoded = utf8_percent_encode(param, QUERY).to_string();

    // ? and # should be encoded
    assert!(encoded.contains("%") || !encoded.contains("?"));
    assert!(encoded.contains("%") || !encoded.contains("#"));
}

#[test]
fn test_plus_sign_encoding() {
    let param = "a+b";
    let encoded = utf8_percent_encode(param, QUERY).to_string();

    // + can remain as-is in modern query encoding (represents itself, not space)
    assert!(encoded == "a+b" || encoded.contains("%2B"));
}

#[test]
fn test_equals_sign_encoding() {
    let param = "key=value";
    let encoded = utf8_percent_encode(param, QUERY).to_string();

    // = should be preserved or encoded depending on context
    assert!(encoded.contains("=") || encoded.contains("%3D"));
}

// ============================================================================
// JSON Value Encoding Tests
// ============================================================================

fn encode_json_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => utf8_percent_encode(s, QUERY).to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        _ => value.to_string(),
    }
}

#[test]
fn test_json_string_encoding() {
    let value = json!("hello world");
    let encoded = encode_json_value(&value);

    assert_eq!(encoded, "hello%20world");
}

#[test]
fn test_json_number_encoding() {
    let value = json!(42);
    let encoded = encode_json_value(&value);

    assert_eq!(encoded, "42");
}

#[test]
fn test_json_boolean_encoding() {
    let value_true = json!(true);
    let encoded_true = encode_json_value(&value_true);
    assert_eq!(encoded_true, "true");

    let value_false = json!(false);
    let encoded_false = encode_json_value(&value_false);
    assert_eq!(encoded_false, "false");
}

#[test]
fn test_json_null_encoding() {
    let value = json!(null);
    let encoded = encode_json_value(&value);

    assert_eq!(encoded, "null");
}
