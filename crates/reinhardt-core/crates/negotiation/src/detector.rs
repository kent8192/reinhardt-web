//! Content-Type detection from request body

use crate::media_type::MediaType;

/// Content-Type detector that automatically detects media type from request body
#[derive(Debug, Clone)]
pub struct ContentTypeDetector {
	/// Fallback media type when detection fails
	default_type: MediaType,
}

impl ContentTypeDetector {
	/// Creates a new ContentTypeDetector with application/octet-stream as default
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_negotiation::detector::ContentTypeDetector;
	///
	/// let detector = ContentTypeDetector::new();
	/// ```
	pub fn new() -> Self {
		Self {
			default_type: MediaType::new("application", "octet-stream"),
		}
	}

	/// Creates a ContentTypeDetector with a custom default media type
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_negotiation::{detector::ContentTypeDetector, MediaType};
	///
	/// let detector = ContentTypeDetector::with_default(
	///     MediaType::new("text", "plain")
	/// );
	/// ```
	pub fn with_default(default_type: MediaType) -> Self {
		Self { default_type }
	}

	/// Detects content type from request body
	///
	/// Supports JSON, XML, YAML, and Form Data detection.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_negotiation::detector::ContentTypeDetector;
	///
	/// let detector = ContentTypeDetector::new();
	///
	/// // JSON detection
	/// let json_body = r#"{"name": "John", "age": 30}"#;
	/// let media_type = detector.detect(json_body.as_bytes());
	/// assert_eq!(media_type.type_, "application");
	/// assert_eq!(media_type.subtype, "json");
	///
	/// // XML detection
	/// let xml_body = r#"<?xml version="1.0"?><root><name>John</name></root>"#;
	/// let media_type = detector.detect(xml_body.as_bytes());
	/// assert_eq!(media_type.subtype, "xml");
	///
	/// // YAML detection
	/// let yaml_body = "name: John\nage: 30";
	/// let media_type = detector.detect(yaml_body.as_bytes());
	/// assert_eq!(media_type.subtype, "yaml");
	///
	/// // Form data detection
	/// let form_body = "name=John&age=30";
	/// let media_type = detector.detect(form_body.as_bytes());
	/// assert_eq!(media_type.subtype, "x-www-form-urlencoded");
	/// ```
	pub fn detect(&self, body: &[u8]) -> MediaType {
		if body.is_empty() {
			return self.default_type.clone();
		}

		// Try to convert to string for text-based detection
		let body_str = match std::str::from_utf8(body) {
			Ok(s) => s.trim(),
			Err(_) => return self.default_type.clone(),
		};

		if body_str.is_empty() {
			return self.default_type.clone();
		}

		// JSON detection
		if self.is_json(body_str) {
			return MediaType::new("application", "json");
		}

		// XML detection
		if self.is_xml(body_str) {
			return MediaType::new("application", "xml");
		}

		// YAML detection
		if self.is_yaml(body_str) {
			return MediaType::new("application", "yaml");
		}

		// Form data detection
		if self.is_form_data(body_str) {
			return MediaType::new("application", "x-www-form-urlencoded");
		}

		self.default_type.clone()
	}

	/// Checks if the body is JSON format
	fn is_json(&self, body: &str) -> bool {
		let first_char = body.chars().next();
		matches!(first_char, Some('{') | Some('['))
	}

	/// Checks if the body is XML format
	fn is_xml(&self, body: &str) -> bool {
		body.starts_with("<?xml") || body.starts_with('<')
	}

	/// Checks if the body is YAML format
	fn is_yaml(&self, body: &str) -> bool {
		// Simple YAML detection: contains key-value pairs with colons
		// and doesn't look like JSON or XML
		if self.is_json(body) || self.is_xml(body) {
			return false;
		}

		body.lines()
			.any(|line| line.contains(':') && !line.trim_start().starts_with('#'))
	}

	/// Checks if the body is form data format
	fn is_form_data(&self, body: &str) -> bool {
		// Form data contains key=value pairs separated by &
		if body.contains('=') {
			let has_ampersand_or_single = body.contains('&') || !body.contains(' ');
			let not_json_xml = !self.is_json(body) && !self.is_xml(body);
			return has_ampersand_or_single && not_json_xml;
		}
		false
	}
}

impl Default for ContentTypeDetector {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_detect_json() {
		let detector = ContentTypeDetector::new();
		let json = r#"{"name": "test"}"#;
		let result = detector.detect(json.as_bytes());
		assert_eq!(result.subtype, "json");
	}

	#[test]
	fn test_detect_xml() {
		let detector = ContentTypeDetector::new();
		let xml = r#"<?xml version="1.0"?><root/>"#;
		let result = detector.detect(xml.as_bytes());
		assert_eq!(result.subtype, "xml");
	}

	#[test]
	fn test_detect_yaml() {
		let detector = ContentTypeDetector::new();
		let yaml = "name: test\nage: 30";
		let result = detector.detect(yaml.as_bytes());
		assert_eq!(result.subtype, "yaml");
	}

	#[test]
	fn test_detect_form_data() {
		let detector = ContentTypeDetector::new();
		let form = "name=test&age=30";
		let result = detector.detect(form.as_bytes());
		assert_eq!(result.subtype, "x-www-form-urlencoded");
	}

	#[test]
	fn test_empty_body() {
		let detector = ContentTypeDetector::new();
		let result = detector.detect(b"");
		assert_eq!(result.subtype, "octet-stream");
	}

	#[test]
	fn test_custom_default() {
		let detector = ContentTypeDetector::with_default(MediaType::new("text", "plain"));
		let result = detector.detect(b"");
		assert_eq!(result.subtype, "plain");
	}

	#[test]
	fn test_json_array() {
		let detector = ContentTypeDetector::new();
		let json = r#"[{"name": "test"}]"#;
		let result = detector.detect(json.as_bytes());
		assert_eq!(result.subtype, "json");
	}

	#[test]
	fn test_xml_without_declaration() {
		let detector = ContentTypeDetector::new();
		let xml = r#"<root><child>value</child></root>"#;
		let result = detector.detect(xml.as_bytes());
		assert_eq!(result.subtype, "xml");
	}
}
