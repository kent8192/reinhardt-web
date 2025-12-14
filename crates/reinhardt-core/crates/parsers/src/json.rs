use async_trait::async_trait;
use bytes::Bytes;
use http::HeaderMap;
use serde_json::Value;

use crate::parser::{ParseError, ParseResult, ParsedData, Parser};

/// JSON parser for application/json content type
///
/// ## Strict Mode
///
/// The `strict` flag controls validation of parsed JSON values:
/// - `strict = true`: Rejects non-finite float values (Infinity, -Infinity, NaN)
/// - `strict = false`: Allows any valid JSON (but still rejects invalid JSON)
///
/// ## Limitations
///
/// Due to serde_json's adherence to JSON RFC 8259, literal `Infinity`, `-Infinity`,
/// and `NaN` values are **not valid JSON** and will be rejected during parsing
/// regardless of the `strict` flag. The strict validation only affects **post-parse**
/// validation of float values.
///
/// In other words:
/// - Invalid JSON input (e.g., raw `Infinity` literal) → Always rejected by serde_json
/// - Valid JSON with non-finite values → Rejected only if `strict = true`
#[derive(Debug, Clone)]
pub struct JSONParser {
	/// Whether to allow empty bodies (returns null)
	pub allow_empty: bool,
	/// Whether to enforce strict JSON (reject Infinity, -Infinity, NaN)
	pub strict: bool,
}

impl Default for JSONParser {
	fn default() -> Self {
		Self {
			allow_empty: false,
			strict: true, // Default to strict mode like DRF
		}
	}
}

impl JSONParser {
	/// Create a new JSONParser with default settings (strict mode, empty not allowed).
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_parsers::json::JSONParser;
	///
	/// let parser = JSONParser::new();
	/// assert!(!parser.allow_empty);
	/// assert!(parser.strict);
	/// ```
	pub fn new() -> Self {
		Self::default()
	}
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_parsers::json::JSONParser;
	///
	/// let parser = JSONParser::new().allow_empty(true);
	/// assert!(parser.allow_empty);
	/// ```
	pub fn allow_empty(mut self, allow: bool) -> Self {
		self.allow_empty = allow;
		self
	}
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_parsers::json::JSONParser;
	///
	/// let parser = JSONParser::new().strict(false);
	/// assert!(!parser.strict);
	/// ```
	pub fn strict(mut self, strict: bool) -> Self {
		self.strict = strict;
		self
	}
}

#[async_trait]
impl Parser for JSONParser {
	fn media_types(&self) -> Vec<String> {
		vec![
			"application/json".to_string(),
			"application/*+json".to_string(),
		]
	}

	async fn parse(
		&self,
		_content_type: Option<&str>,
		body: Bytes,
		_headers: &HeaderMap,
	) -> ParseResult<ParsedData> {
		if body.is_empty() {
			if self.allow_empty {
				return Ok(ParsedData::Json(Value::Null));
			} else {
				return Err(ParseError::ParseError("Empty request body".to_string()));
			}
		}

		match serde_json::from_slice::<Value>(&body) {
			Ok(value) => {
				// Check for non-finite floats in strict mode
				if self.strict {
					Self::validate_strict_json(&value)?;
				}
				Ok(ParsedData::Json(value))
			}
			Err(e) => Err(ParseError::ParseError(format!("Invalid JSON: {}", e))),
		}
	}
}

impl JSONParser {
	/// Validate that JSON doesn't contain non-finite float values (Infinity, -Infinity, NaN)
	///
	/// # Note
	///
	/// This validation is performed **after** serde_json parsing. Due to serde_json's
	/// strict RFC 8259 compliance, literal `Infinity`/`NaN` strings are already rejected
	/// during parsing. This method validates the **parsed** numeric values.
	fn validate_strict_json(value: &Value) -> ParseResult<()> {
		match value {
			Value::Number(n) => {
				if let Some(f) = n.as_f64()
					&& !f.is_finite()
				{
					return Err(ParseError::ParseError(
                            "Non-finite float values (Infinity, -Infinity, NaN) are not allowed in strict mode".to_string()
                        ));
				}
			}
			Value::Array(arr) => {
				for item in arr {
					Self::validate_strict_json(item)?;
				}
			}
			Value::Object(obj) => {
				for value in obj.values() {
					Self::validate_strict_json(value)?;
				}
			}
			_ => {}
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_json_parser_valid() {
		let parser = JSONParser::new();
		let body = Bytes::from(r#"{"name": "test", "value": 123}"#);
		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/json"), body, &headers)
			.await
			.unwrap();

		match result {
			ParsedData::Json(value) => {
				assert_eq!(value["name"], "test");
				assert_eq!(value["value"], 123);
			}
			_ => panic!("Expected JSON data"),
		}
	}

	#[tokio::test]
	async fn test_json_parser_invalid() {
		let parser = JSONParser::new();
		let body = Bytes::from(r#"{"invalid": json}"#);
		let headers = HeaderMap::new();

		let result = parser.parse(Some("application/json"), body, &headers).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_json_parser_empty_not_allowed() {
		let parser = JSONParser::new();
		let body = Bytes::new();
		let headers = HeaderMap::new();

		let result = parser.parse(Some("application/json"), body, &headers).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_json_parser_empty_allowed() {
		let parser = JSONParser::new().allow_empty(true);
		let body = Bytes::new();
		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/json"), body, &headers)
			.await
			.unwrap();

		match result {
			ParsedData::Json(Value::Null) => {}
			_ => panic!("Expected null JSON value"),
		}
	}

	#[test]
	fn test_json_parser_media_types() {
		let parser = JSONParser::new();
		let media_types = parser.media_types();

		assert!(media_types.contains(&"application/json".to_string()));
		assert!(media_types.contains(&"application/*+json".to_string()));
	}

	// Tests from Django REST Framework

	#[tokio::test]
	async fn test_json_float_strictness() {
		// DRF test: Test Infinity, -Infinity, NaN handling with strict mode
		let parser = JSONParser::new(); // Default strict = true
		let headers = HeaderMap::new();

		// In strict mode, these should fail
		// Note: These fail during serde_json parsing (not strict validation)
		// because Infinity/NaN literals are not valid JSON per RFC 8259
		for value in ["Infinity", "-Infinity", "NaN"] {
			let body = Bytes::from(value);
			let result = parser.parse(Some("application/json"), body, &headers).await;
			assert!(
				result.is_err(),
				"Expected error for {} (invalid JSON literal)",
				value
			);
		}

		// In non-strict mode, valid JSON should still parse
		// (strict only affects post-parse validation of float values)
		let parser_non_strict = JSONParser::new().strict(false);
		let valid_json = Bytes::from(r#"{"value": 1.0}"#);
		let result = parser_non_strict
			.parse(Some("application/json"), valid_json, &headers)
			.await;
		assert!(result.is_ok(), "Valid JSON should parse in non-strict mode");
	}

	#[tokio::test]
	async fn test_json_edge_case_large_numbers() {
		// Test extremely large numbers near floating-point limits
		let parser = JSONParser::new();
		let headers = HeaderMap::new();

		// Maximum finite f64 value (approximately 1.7976931348623157e308)
		let large_number = Bytes::from(r#"{"value": 1e308}"#);
		let result = parser
			.parse(Some("application/json"), large_number, &headers)
			.await;
		assert!(result.is_ok(), "Should parse very large finite numbers");

		// Negative large number
		let large_negative = Bytes::from(r#"{"value": -1e308}"#);
		let result = parser
			.parse(Some("application/json"), large_negative, &headers)
			.await;
		assert!(result.is_ok(), "Should parse very large negative numbers");
	}

	#[tokio::test]
	async fn test_json_edge_case_small_numbers() {
		// Test extremely small numbers near zero
		let parser = JSONParser::new();
		let headers = HeaderMap::new();

		// Minimum positive normalized f64 value (approximately 2.2250738585072014e-308)
		let small_number = Bytes::from(r#"{"value": 2.2250738585072014e-308}"#);
		let result = parser
			.parse(Some("application/json"), small_number, &headers)
			.await;
		assert!(result.is_ok(), "Should parse very small finite numbers");

		// Very small negative number
		let small_negative = Bytes::from(r#"{"value": -2.2250738585072014e-308}"#);
		let result = parser
			.parse(Some("application/json"), small_negative, &headers)
			.await;
		assert!(result.is_ok(), "Should parse very small negative numbers");
	}

	#[tokio::test]
	async fn test_json_scientific_notation() {
		// Test various scientific notation formats
		let parser = JSONParser::new();
		let headers = HeaderMap::new();

		let test_cases = vec![
			r#"{"value": 1.5e10}"#,      // Basic scientific notation
			r#"{"value": 1.5E10}"#,      // Uppercase E
			r#"{"value": 1.5e+10}"#,     // Explicit positive exponent
			r#"{"value": 1.5e-10}"#,     // Negative exponent
			r#"{"array": [1e5, 2e-5]}"#, // Multiple scientific notations
		];

		for test_case in test_cases {
			let body = Bytes::from(test_case);
			let result = parser.parse(Some("application/json"), body, &headers).await;
			assert!(
				result.is_ok(),
				"Should parse scientific notation: {}",
				test_case
			);
		}
	}

	#[tokio::test]
	async fn test_json_nested_float_validation() {
		// Test strict validation in nested structures
		let parser_strict = JSONParser::new(); // strict = true by default
		let headers = HeaderMap::new();

		// This will be rejected by serde_json (not by strict validation)
		// because Infinity is not valid JSON
		let nested_infinity = Bytes::from(r#"{"outer": {"inner": Infinity}}"#);
		let result = parser_strict
			.parse(Some("application/json"), nested_infinity, &headers)
			.await;
		assert!(
			result.is_err(),
			"Nested Infinity literal should be rejected by serde_json"
		);

		// Valid nested structure with finite floats
		let nested_valid = Bytes::from(r#"{"outer": {"inner": 123.456}}"#);
		let result = parser_strict
			.parse(Some("application/json"), nested_valid, &headers)
			.await;
		assert!(result.is_ok(), "Valid nested floats should be accepted");
	}

	#[tokio::test]
	async fn test_json_array_float_validation() {
		// Test strict validation in arrays
		let parser_strict = JSONParser::new();
		let parser_non_strict = JSONParser::new().strict(false);
		let headers = HeaderMap::new();

		// Valid array with finite floats
		let valid_array = Bytes::from(r#"[1.0, 2.5, 3.14159, 100.0]"#);
		let result = parser_strict
			.parse(Some("application/json"), valid_array.clone(), &headers)
			.await;
		assert!(result.is_ok(), "Valid float arrays should be accepted");

		let result = parser_non_strict
			.parse(Some("application/json"), valid_array, &headers)
			.await;
		assert!(
			result.is_ok(),
			"Valid float arrays should be accepted in non-strict mode"
		);

		// Invalid: Array with Infinity literal (rejected by serde_json)
		let invalid_array = Bytes::from(r#"[1.0, Infinity, 3.0]"#);
		let result = parser_strict
			.parse(Some("application/json"), invalid_array.clone(), &headers)
			.await;
		assert!(
			result.is_err(),
			"Infinity literal in array should be rejected"
		);

		let result = parser_non_strict
			.parse(Some("application/json"), invalid_array, &headers)
			.await;
		assert!(
			result.is_err(),
			"Infinity literal in array should be rejected even in non-strict mode"
		);
	}
}
