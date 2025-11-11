//! YAML parser for parsing YAML request bodies
//!
//! Provides parsing of YAML content into a JSON-like structure using serde_yaml.

use crate::parser::{ParseResult, ParsedData, Parser};
use async_trait::async_trait;
use bytes::Bytes;
use http::HeaderMap;
use reinhardt_exception::Error;
use serde_json::Value;

/// YAML parser for application/x-yaml and application/yaml content types
///
/// # Examples
///
/// ```
/// use reinhardt_parsers::yaml::YamlParser;
/// use reinhardt_parsers::parser::Parser;
/// use bytes::Bytes;
/// use http::HeaderMap;
///
/// # tokio_test::block_on(async {
/// let parser = YamlParser::new();
/// let yaml = Bytes::from("name: John\nage: 30\n");
/// let headers = HeaderMap::new();
/// let result = parser.parse(Some("application/yaml"), yaml, &headers).await.unwrap();
/// # });
/// ```
#[derive(Debug, Clone, Default)]
pub struct YamlParser {
	/// Whether to allow empty bodies (returns null)
	pub allow_empty: bool,
}

impl YamlParser {
	/// Create a new YamlParser with default settings (empty not allowed).
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_parsers::yaml::YamlParser;
	///
	/// let parser = YamlParser::new();
	/// assert!(!parser.allow_empty);
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Configure whether to allow empty request bodies.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_parsers::yaml::YamlParser;
	///
	/// let parser = YamlParser::new().allow_empty(true);
	/// assert!(parser.allow_empty);
	/// ```
	pub fn allow_empty(mut self, allow: bool) -> Self {
		self.allow_empty = allow;
		self
	}
}

#[async_trait]
impl Parser for YamlParser {
	fn media_types(&self) -> Vec<String> {
		vec![
			"application/yaml".to_string(),
			"application/x-yaml".to_string(),
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
				return Ok(ParsedData::Yaml(Value::Null));
			} else {
				return Err(Error::ParseError("Empty request body".to_string()));
			}
		}

		match serde_yaml::from_slice::<Value>(&body) {
			Ok(value) => Ok(ParsedData::Yaml(value)),
			Err(e) => Err(Error::ParseError(format!("Invalid YAML: {}", e))),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_yaml_parser_simple() {
		let parser = YamlParser::new();
		let yaml = Bytes::from("name: John\nage: 30\n");

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/yaml"), yaml, &headers)
			.await
			.unwrap();

		match result {
			ParsedData::Yaml(value) => {
				assert_eq!(value["name"], "John");
				assert_eq!(value["age"], 30);
			}
			_ => panic!("Expected YAML data"),
		}
	}

	#[tokio::test]
	async fn test_yaml_parser_nested() {
		let parser = YamlParser::new();
		let yaml = Bytes::from("person:\n  name: John\n  address:\n    city: NYC\n");

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/yaml"), yaml, &headers)
			.await
			.unwrap();

		match result {
			ParsedData::Yaml(value) => {
				assert_eq!(value["person"]["name"], "John");
				assert_eq!(value["person"]["address"]["city"], "NYC");
			}
			_ => panic!("Expected YAML data"),
		}
	}

	#[tokio::test]
	async fn test_yaml_parser_array() {
		let parser = YamlParser::new();
		let yaml = Bytes::from("items:\n  - apple\n  - banana\n  - orange\n");

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/yaml"), yaml, &headers)
			.await
			.unwrap();

		match result {
			ParsedData::Yaml(value) => {
				let items = value["items"].as_array().unwrap();
				assert_eq!(items.len(), 3);
				assert_eq!(items[0], "apple");
				assert_eq!(items[1], "banana");
				assert_eq!(items[2], "orange");
			}
			_ => panic!("Expected YAML data"),
		}
	}

	#[tokio::test]
	async fn test_yaml_parser_invalid() {
		let parser = YamlParser::new();
		let yaml = Bytes::from("invalid: [unclosed array");

		let headers = HeaderMap::new();

		let result = parser.parse(Some("application/yaml"), yaml, &headers).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_yaml_parser_empty_not_allowed() {
		let parser = YamlParser::new();
		let body = Bytes::new();

		let headers = HeaderMap::new();

		let result = parser.parse(Some("application/yaml"), body, &headers).await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_yaml_parser_empty_allowed() {
		let parser = YamlParser::new().allow_empty(true);
		let body = Bytes::new();

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/yaml"), body, &headers)
			.await
			.unwrap();

		match result {
			ParsedData::Yaml(Value::Null) => {}
			_ => panic!("Expected null YAML value"),
		}
	}

	#[test]
	fn test_yaml_parser_media_types() {
		let parser = YamlParser::new();
		let media_types = parser.media_types();

		assert!(media_types.contains(&"application/yaml".to_string()));
		assert!(media_types.contains(&"application/x-yaml".to_string()));
	}

	#[tokio::test]
	async fn test_yaml_parser_can_parse() {
		let parser = YamlParser::new();

		assert!(parser.can_parse(Some("application/yaml")));
		assert!(parser.can_parse(Some("application/x-yaml")));
		assert!(!parser.can_parse(Some("application/json")));
		assert!(!parser.can_parse(None));
	}

	#[tokio::test]
	async fn test_yaml_parser_boolean_values() {
		let parser = YamlParser::new();
		let yaml = Bytes::from("active: true\ndisabled: false\n");

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/yaml"), yaml, &headers)
			.await
			.unwrap();

		match result {
			ParsedData::Yaml(value) => {
				assert_eq!(value["active"], true);
				assert_eq!(value["disabled"], false);
			}
			_ => panic!("Expected YAML data"),
		}
	}

	#[tokio::test]
	async fn test_yaml_parser_number_types() {
		let parser = YamlParser::new();
		let yaml = Bytes::from("integer: 42\nfloat: 3.14\n");

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/yaml"), yaml, &headers)
			.await
			.unwrap();

		match result {
			ParsedData::Yaml(value) => {
				assert_eq!(value["integer"], 42);
				assert_eq!(value["float"], 3.14);
			}
			_ => panic!("Expected YAML data"),
		}
	}

	#[tokio::test]
	async fn test_yaml_parser_null_value() {
		let parser = YamlParser::new();
		let yaml = Bytes::from("value: null\n");

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/yaml"), yaml, &headers)
			.await
			.unwrap();

		match result {
			ParsedData::Yaml(value) => {
				assert_eq!(value["value"], Value::Null);
			}
			_ => panic!("Expected YAML data"),
		}
	}

	#[tokio::test]
	async fn test_yaml_parser_multiline_string() {
		let parser = YamlParser::new();
		let yaml = Bytes::from("description: |\n  This is a\n  multiline string\n");

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/yaml"), yaml, &headers)
			.await
			.unwrap();

		match result {
			ParsedData::Yaml(value) => {
				let desc = value["description"].as_str().unwrap();
				assert!(desc.contains("This is a"));
				assert!(desc.contains("multiline string"));
			}
			_ => panic!("Expected YAML data"),
		}
	}

	#[tokio::test]
	async fn test_yaml_parser_complex_structure() {
		let parser = YamlParser::new();
		let yaml = Bytes::from(
			r#"
user:
  name: John Doe
  email: john@example.com
  roles:
    - admin
    - developer
  settings:
    theme: dark
    notifications: true
"#,
		);

		let headers = HeaderMap::new();

		let result = parser
			.parse(Some("application/yaml"), yaml, &headers)
			.await
			.unwrap();

		match result {
			ParsedData::Yaml(value) => {
				assert_eq!(value["user"]["name"], "John Doe");
				assert_eq!(value["user"]["email"], "john@example.com");
				let roles = value["user"]["roles"].as_array().unwrap();
				assert_eq!(roles.len(), 2);
				assert_eq!(value["user"]["settings"]["theme"], "dark");
				assert_eq!(value["user"]["settings"]["notifications"], true);
			}
			_ => panic!("Expected YAML data"),
		}
	}
}
