use async_trait::async_trait;
use bytes::Bytes;
use http::HeaderMap;
use std::collections::HashMap;

use super::parser::{ParseError, ParseResult, ParsedData, Parser};

/// Form parser for application/x-www-form-urlencoded content type
#[derive(Debug, Clone, Default)]
pub struct FormParser;

impl FormParser {
	/// Create a new FormParser.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::parsers::form::FormParser;
	///
	/// let parser = FormParser::new();
	/// ```
	pub fn new() -> Self {
		Self
	}
}

#[async_trait]
impl Parser for FormParser {
	fn media_types(&self) -> Vec<String> {
		vec!["application/x-www-form-urlencoded".to_string()]
	}

	async fn parse(
		&self,
		_content_type: Option<&str>,
		body: Bytes,
		_headers: &HeaderMap,
	) -> ParseResult<ParsedData> {
		if body.is_empty() {
			return Ok(ParsedData::Form(HashMap::new()));
		}

		match serde_urlencoded::from_bytes::<HashMap<String, String>>(&body) {
			Ok(form_data) => Ok(ParsedData::Form(form_data)),
			Err(e) => Err(ParseError::ParseError(format!("Invalid form data: {}", e))),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	#[tokio::test]
	async fn test_form_parser_valid() {
		let parser = FormParser::new();
		let body = Bytes::from("name=test&value=123&active=true");
		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/x-www-form-urlencoded"), body, &headers)
			.await
			.unwrap();

		match result {
			ParsedData::Form(form) => {
				assert_eq!(form.get("name"), Some(&"test".to_string()));
				assert_eq!(form.get("value"), Some(&"123".to_string()));
				assert_eq!(form.get("active"), Some(&"true".to_string()));
			}
			_ => panic!("Expected form data"),
		}
	}

	#[rstest]
	#[tokio::test]
	async fn test_form_parser_empty() {
		let parser = FormParser::new();
		let body = Bytes::new();
		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/x-www-form-urlencoded"), body, &headers)
			.await
			.unwrap();

		match result {
			ParsedData::Form(form) => {
				assert!(form.is_empty());
			}
			_ => panic!("Expected form data"),
		}
	}

	#[rstest]
	#[tokio::test]
	async fn test_form_parser_url_encoded() {
		let parser = FormParser::new();
		let body = Bytes::from("message=Hello%20World&symbol=%26");
		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/x-www-form-urlencoded"), body, &headers)
			.await
			.unwrap();

		match result {
			ParsedData::Form(form) => {
				assert_eq!(form.get("message"), Some(&"Hello World".to_string()));
				assert_eq!(form.get("symbol"), Some(&"&".to_string()));
			}
			_ => panic!("Expected form data"),
		}
	}

	#[rstest]
	fn test_form_parser_media_types() {
		let parser = FormParser::new();
		let media_types = parser.media_types();

		assert!(media_types.contains(&"application/x-www-form-urlencoded".to_string()));
	}

	// Tests from Django REST Framework

	#[rstest]
	#[tokio::test]
	async fn test_form_parse_drf() {
		// DRF test: Make sure the form parsing works correctly
		let parser = FormParser::new();
		let body = Bytes::from("field1=abc&field2=defghijk");
		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/x-www-form-urlencoded"), body, &headers)
			.await
			.unwrap();

		match result {
			ParsedData::Form(form) => {
				// Validate the form data can be used for validation
				assert_eq!(form.get("field1"), Some(&"abc".to_string()));
				assert_eq!(form.get("field2"), Some(&"defghijk".to_string()));

				// Simulate form validation (field1 max_length=3, field2 any length)
				let field1_valid = form.get("field1").map(|v| v.len() <= 3).unwrap_or(false);
				assert!(field1_valid);
			}
			_ => panic!("Expected form data"),
		}
	}
}
