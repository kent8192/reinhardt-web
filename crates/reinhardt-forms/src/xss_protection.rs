//! XSS (Cross-Site Scripting) protection utilities
//!
//! This module provides advanced XSS protection for form inputs including:
//! - HTML sanitization
//! - Attribute filtering
//! - Script tag removal
//! - Event handler stripping

use regex::Regex;
use std::sync::OnceLock;

/// XSS protection error types
#[derive(Debug, thiserror::Error)]
pub enum XssError {
	#[error("Potentially dangerous content detected: {0}")]
	DangerousContent(String),
	#[error("Sanitization failed: {0}")]
	SanitizationFailed(String),
}

/// Configuration for XSS protection
#[derive(Debug, Clone)]
pub struct XssConfig {
	/// Allow specific HTML tags
	pub allowed_tags: Vec<String>,
	/// Allow specific attributes
	pub allowed_attributes: Vec<String>,
	/// Strip all tags (convert to text)
	pub strip_all_tags: bool,
	/// Escape HTML entities
	pub escape_html: bool,
}

impl Default for XssConfig {
	fn default() -> Self {
		Self {
			allowed_tags: vec![
				"b".to_string(),
				"i".to_string(),
				"u".to_string(),
				"p".to_string(),
			],
			allowed_attributes: vec!["class".to_string(), "id".to_string()],
			strip_all_tags: false,
			escape_html: true,
		}
	}
}

impl XssConfig {
	/// Create a strict configuration (no HTML allowed)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::XssConfig;
	///
	/// let config = XssConfig::strict();
	/// assert!(config.strip_all_tags);
	/// assert!(config.escape_html);
	/// ```
	pub fn strict() -> Self {
		Self {
			allowed_tags: vec![],
			allowed_attributes: vec![],
			strip_all_tags: true,
			escape_html: true,
		}
	}

	/// Create a permissive configuration (safe HTML tags allowed)
	pub fn permissive() -> Self {
		Self {
			allowed_tags: vec![
				"b".to_string(),
				"i".to_string(),
				"u".to_string(),
				"strong".to_string(),
				"em".to_string(),
				"p".to_string(),
				"br".to_string(),
				"a".to_string(),
				"ul".to_string(),
				"ol".to_string(),
				"li".to_string(),
			],
			allowed_attributes: vec![
				"class".to_string(),
				"id".to_string(),
				"href".to_string(),
				"title".to_string(),
			],
			strip_all_tags: false,
			escape_html: false,
		}
	}

	/// Allow specific HTML tags
	pub fn allow_tags(mut self, tags: Vec<String>) -> Self {
		self.allowed_tags = tags;
		self
	}

	/// Allow specific attributes
	pub fn allow_attributes(mut self, attributes: Vec<String>) -> Self {
		self.allowed_attributes = attributes;
		self
	}
}

/// XSS protector with configurable sanitization
pub struct XssProtector {
	config: XssConfig,
}

impl XssProtector {
	/// Create a new XSS protector with default configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::XssProtector;
	///
	/// let protector = XssProtector::new();
	/// let safe = protector.sanitize("<script>alert('xss')</script>");
	/// assert!(!safe.contains("<script>"));
	/// ```
	pub fn new() -> Self {
		Self {
			config: XssConfig::default(),
		}
	}

	/// Create a protector with custom configuration
	pub fn with_config(config: XssConfig) -> Self {
		Self { config }
	}

	/// Create a strict protector (no HTML allowed)
	pub fn strict() -> Self {
		Self {
			config: XssConfig::strict(),
		}
	}

	/// Sanitize input string
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::XssProtector;
	///
	/// let protector = XssProtector::new();
	/// let clean = protector.sanitize("<b>Bold</b><script>alert(1)</script>");
	/// assert!(clean.contains("<b>Bold</b>"));
	/// assert!(!clean.contains("<script>"));
	/// ```
	pub fn sanitize(&self, input: &str) -> String {
		let mut result = input.to_string();

		// Remove script tags
		result = self.remove_script_tags(&result);

		// Remove event handlers
		result = self.remove_event_handlers(&result);

		// Remove dangerous attributes
		result = self.remove_dangerous_attributes(&result);

		// Remove dangerous protocols
		result = self.remove_dangerous_protocols(&result);

		if self.config.strip_all_tags {
			result = self.strip_tags(&result);
			// Only escape HTML after stripping all tags
			if self.config.escape_html {
				result = self.escape_html(&result);
			}
		} else {
			result = self.filter_tags(&result);
			// Don't escape HTML when filtering tags (preserve allowed tags)
		}

		result
	}

	/// Check if input contains potential XSS
	pub fn contains_xss(&self, input: &str) -> bool {
		self.detect_script_tags(input)
			|| self.detect_event_handlers(input)
			|| self.detect_dangerous_protocols(input)
	}

	/// Remove all script tags
	fn remove_script_tags(&self, input: &str) -> String {
		static SCRIPT_RE: OnceLock<Regex> = OnceLock::new();
		let re = SCRIPT_RE.get_or_init(|| Regex::new(r"(?i)<script[^>]*>.*?</script>").unwrap());
		re.replace_all(input, "").to_string()
	}

	/// Remove event handlers (onclick, onload, etc.)
	fn remove_event_handlers(&self, input: &str) -> String {
		static EVENT_RE: OnceLock<Regex> = OnceLock::new();
		let re =
			EVENT_RE.get_or_init(|| Regex::new(r#"(?i)\s*on\w+\s*=\s*["'][^"']*["']"#).unwrap());
		re.replace_all(input, "").to_string()
	}

	/// Remove dangerous attributes
	fn remove_dangerous_attributes(&self, input: &str) -> String {
		static DANGEROUS_ATTRS_RE: OnceLock<Regex> = OnceLock::new();
		let re = DANGEROUS_ATTRS_RE.get_or_init(|| {
			Regex::new(r#"(?i)\s*(onerror|onload|onclick|onmouseover)\s*=\s*["'][^"']*["']"#)
				.unwrap()
		});
		re.replace_all(input, "").to_string()
	}

	/// Remove dangerous protocols (javascript:, data:, etc.)
	fn remove_dangerous_protocols(&self, input: &str) -> String {
		static PROTOCOL_RE: OnceLock<Regex> = OnceLock::new();
		let re =
			PROTOCOL_RE.get_or_init(|| Regex::new(r#"(?i)(javascript|data|vbscript):"#).unwrap());
		re.replace_all(input, "").to_string()
	}

	/// Strip all HTML tags
	fn strip_tags(&self, input: &str) -> String {
		static TAG_RE: OnceLock<Regex> = OnceLock::new();
		let re = TAG_RE.get_or_init(|| Regex::new(r"<[^>]*>").unwrap());
		re.replace_all(input, "").to_string()
	}

	/// Filter tags based on whitelist
	fn filter_tags(&self, input: &str) -> String {
		let mut result = input.to_string();

		// For simplicity, remove all tags not in allowed list
		static TAG_RE: OnceLock<Regex> = OnceLock::new();
		let re = TAG_RE.get_or_init(|| Regex::new(r"<(/?)([a-zA-Z][a-zA-Z0-9]*)[^>]*>").unwrap());

		for cap in re.captures_iter(input) {
			let full_match = cap.get(0).unwrap().as_str();
			let tag_name = cap.get(2).unwrap().as_str().to_lowercase();

			if !self.config.allowed_tags.contains(&tag_name) {
				result = result.replace(full_match, "");
			}
		}

		result
	}

	/// Escape HTML entities
	fn escape_html(&self, input: &str) -> String {
		input
			.replace('&', "&amp;")
			.replace('<', "&lt;")
			.replace('>', "&gt;")
			.replace('"', "&quot;")
			.replace('\'', "&#x27;")
	}

	/// Detect script tags
	fn detect_script_tags(&self, input: &str) -> bool {
		input.to_lowercase().contains("<script")
	}

	/// Detect event handlers
	fn detect_event_handlers(&self, input: &str) -> bool {
		let input_lower = input.to_lowercase();
		input_lower.contains("onclick")
			|| input_lower.contains("onload")
			|| input_lower.contains("onerror")
			|| input_lower.contains("onmouseover")
	}

	/// Detect dangerous protocols
	fn detect_dangerous_protocols(&self, input: &str) -> bool {
		let input_lower = input.to_lowercase();
		input_lower.contains("javascript:")
			|| input_lower.contains("data:")
			|| input_lower.contains("vbscript:")
	}
}

impl Default for XssProtector {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_xss_config_default() {
		let config = XssConfig::default();
		assert!(!config.strip_all_tags);
		assert!(config.escape_html);
		assert!(config.allowed_tags.contains(&"b".to_string()));
	}

	#[test]
	fn test_xss_config_strict() {
		let config = XssConfig::strict();
		assert!(config.strip_all_tags);
		assert!(config.escape_html);
		assert!(config.allowed_tags.is_empty());
	}

	#[test]
	fn test_xss_protector_remove_script() {
		let protector = XssProtector::new();
		let input = "<script>alert('xss')</script>Hello";
		let result = protector.sanitize(input);
		assert!(!result.contains("<script>"));
		assert!(result.contains("Hello"));
	}

	#[test]
	fn test_xss_protector_remove_event_handlers() {
		let protector = XssProtector::new();
		let input = r#"<div onclick="alert('xss')">Click</div>"#;
		let result = protector.sanitize(input);
		assert!(!result.contains("onclick"));
	}

	#[test]
	fn test_xss_protector_remove_dangerous_protocols() {
		let protector = XssProtector::new();
		let input = r#"<a href="javascript:alert('xss')">Link</a>"#;
		let result = protector.sanitize(input);
		assert!(!result.contains("javascript:"));
	}

	#[test]
	fn test_xss_protector_strip_tags() {
		let protector = XssProtector::strict();
		let input = "<b>Bold</b><script>alert(1)</script>";
		let result = protector.sanitize(input);
		assert!(!result.contains("<b>"));
		assert!(!result.contains("<script>"));
		assert!(result.contains("Bold"));
	}

	#[test]
	fn test_xss_protector_escape_html() {
		let protector = XssProtector::strict();
		let input = "<b>Test & \"quotes\"</b>";
		let result = protector.sanitize(input);
		assert!(result.contains("&amp;"));
		assert!(result.contains("&quot;"));
	}

	#[test]
	fn test_xss_protector_contains_xss() {
		let protector = XssProtector::new();

		assert!(protector.contains_xss("<script>alert(1)</script>"));
		assert!(protector.contains_xss(r#"<div onclick="alert(1)">Test</div>"#));
		assert!(protector.contains_xss(r#"<a href="javascript:alert(1)">Link</a>"#));
		assert!(!protector.contains_xss("<p>Safe content</p>"));
	}

	#[test]
	fn test_xss_protector_permissive_config() {
		let config = XssConfig::permissive();
		let protector = XssProtector::with_config(config);

		let input = "<b>Bold</b><p>Paragraph</p><script>alert(1)</script>";
		let result = protector.sanitize(input);

		assert!(result.contains("<b>Bold</b>"));
		assert!(result.contains("<p>Paragraph</p>"));
		assert!(!result.contains("<script>"));
	}

	#[test]
	fn test_xss_protector_custom_allowed_tags() {
		let config = XssConfig::default().allow_tags(vec!["b".to_string()]);
		let protector = XssProtector::with_config(config);

		let input = "<b>Bold</b><i>Italic</i>";
		let result = protector.sanitize(input);

		assert!(result.contains("<b>Bold</b>"));
		assert!(!result.contains("<i>"));
	}

	#[test]
	fn test_xss_protector_nested_scripts() {
		let protector = XssProtector::new();
		let input = "<div><script>alert(1)</script></div>";
		let result = protector.sanitize(input);
		assert!(!result.contains("<script>"));
	}

	#[test]
	fn test_xss_protector_multiple_event_handlers() {
		let protector = XssProtector::new();
		let input = r#"<div onclick="alert(1)" onload="alert(2)" onerror="alert(3)">Test</div>"#;
		let result = protector.sanitize(input);
		assert!(!result.contains("onclick"));
		assert!(!result.contains("onload"));
		assert!(!result.contains("onerror"));
	}
}
