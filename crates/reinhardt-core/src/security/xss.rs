//! XSS prevention utilities

use regex::Regex;
use std::sync::OnceLock;

/// Escape HTML special characters
///
/// # Examples
///
/// ```
/// use reinhardt_core::security::escape_html;
///
/// let input = "<script>alert('XSS')</script>";
/// let escaped = escape_html(input);
/// assert_eq!(escaped, "&lt;script&gt;alert(&#x27;XSS&#x27;)&lt;/script&gt;");
///
/// let with_quotes = r#"<a href="javascript:alert('xss')">Click</a>"#;
/// let escaped_quotes = escape_html(with_quotes);
/// assert_eq!(escaped_quotes, "&lt;a href=&quot;javascript:alert(&#x27;xss&#x27;)&quot;&gt;Click&lt;/a&gt;");
/// ```
pub fn escape_html(input: &str) -> String {
	input
		.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&#x27;")
}

/// Escape HTML attributes
///
/// # Examples
///
/// ```
/// use reinhardt_core::security::xss::escape_html_attr;
///
/// let attr = r#"value" onload="alert('xss')"#;
/// let escaped = escape_html_attr(attr);
/// // The onload itself remains, but the quotes are escaped to neutralize it
/// assert!(escaped.contains("&quot;"));
/// assert!(escaped.contains("&#x27;"));
/// ```
pub fn escape_html_attr(input: &str) -> String {
	input
		.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&#x27;")
		.replace('\n', "&#10;")
		.replace('\r', "&#13;")
}

/// Escape for JavaScript context
///
/// # Examples
///
/// ```
/// use reinhardt_core::security::xss::escape_javascript;
///
/// let script = "'; alert('xss'); var x='";
/// let escaped = escape_javascript(script);
/// // Verify that single quotes are escaped
/// assert!(escaped.contains("\\'"));
/// // Verify the format after escaping
/// assert_eq!(escaped, "\\'; alert(\\'xss\\'); var x=\\'");
/// ```
pub fn escape_javascript(input: &str) -> String {
	input
		.replace('\\', "\\\\")
		.replace('\'', "\\'")
		.replace('"', "\\\"")
		.replace('\n', "\\n")
		.replace('\r', "\\r")
		.replace('\t', "\\t")
		.replace('<', "\\x3C")
		.replace('>', "\\x3E")
		.replace('/', "\\/")
}

/// Escape for URLs
///
/// # Examples
///
/// ```
/// use reinhardt_core::security::xss::escape_url;
///
/// let url = "javascript:alert('xss')";
/// let escaped = escape_url(url);
/// assert!(escaped.contains("%3A"));
/// ```
pub fn escape_url(input: &str) -> String {
	urlencoding::encode(input).to_string()
}

static DANGEROUS_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

fn get_dangerous_patterns() -> &'static Vec<Regex> {
	DANGEROUS_PATTERNS.get_or_init(|| {
		vec![
			// JavaScript protocol
			Regex::new(r"(?i)javascript:").unwrap(),
			// Data URI
			Regex::new(r"(?i)data:text/html").unwrap(),
			// VBScript (IE)
			Regex::new(r"(?i)vbscript:").unwrap(),
			// Event handlers
			Regex::new(r"(?i)on\w+\s*=").unwrap(),
			// Dangerous tags like iframe/embed
			Regex::new(r"(?i)<(iframe|embed|object|applet|meta|link|base)").unwrap(),
			// script tag
			Regex::new(r"(?i)<script").unwrap(),
		]
	})
}

/// Detect dangerous patterns
///
/// # Examples
///
/// ```
/// use reinhardt_core::security::xss::detect_xss_patterns;
///
/// assert!(detect_xss_patterns("<script>alert(1)</script>"));
/// assert!(detect_xss_patterns(r#"<img src=x onerror="alert(1)">"#));
/// assert!(detect_xss_patterns("javascript:alert(1)"));
/// assert!(!detect_xss_patterns("Safe text"));
/// ```
pub fn detect_xss_patterns(input: &str) -> bool {
	get_dangerous_patterns()
		.iter()
		.any(|pattern| pattern.is_match(input))
}

/// Sanitize HTML (enhanced implementation)
///
/// # Examples
///
/// ```
/// use reinhardt_core::security::sanitize_html;
///
/// let dangerous = "<script>alert('XSS')</script><b>Bold text</b>";
/// let sanitized = sanitize_html(dangerous);
/// assert_eq!(sanitized, "&lt;script&gt;alert(&#x27;XSS&#x27;)&lt;/script&gt;&lt;b&gt;Bold text&lt;/b&gt;");
///
/// let user_input = "User's comment with <img src=x onerror=alert(1)>";
/// let safe_output = sanitize_html(user_input);
/// assert!(safe_output.contains("&lt;img"));
/// ```
pub fn sanitize_html(input: &str) -> String {
	escape_html(input)
}

/// Validate URLs and allow only safe protocols
///
/// # Examples
///
/// ```
/// use reinhardt_core::security::xss::is_safe_url;
///
/// assert!(is_safe_url("https://example.com"));
/// assert!(is_safe_url("http://example.com"));
/// assert!(is_safe_url("/path/to/page"));
/// assert!(is_safe_url("mailto:user@example.com"));
/// assert!(!is_safe_url("javascript:alert(1)"));
/// assert!(!is_safe_url("data:text/html,<script>alert(1)</script>"));
/// ```
pub fn is_safe_url(url: &str) -> bool {
	let url_lower = url.to_lowercase();

	// Allow relative URLs
	if url.starts_with('/') || url.starts_with("./") || url.starts_with("../") {
		return true;
	}

	// Allow only safe protocols
	let safe_protocols = ["http://", "https://", "mailto:", "ftp://", "ftps://"];

	safe_protocols
		.iter()
		.any(|protocol| url_lower.starts_with(protocol))
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_escape_html() {
		assert_eq!(
			escape_html("<script>alert('xss')</script>"),
			"&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
		);
	}

	#[rstest]
	fn test_escape_html_attr() {
		let attr = r#"value" onload="alert('xss')"#;
		let escaped = escape_html_attr(attr);
		assert!(escaped.contains("&quot;"));
		assert!(escaped.contains("&#x27;"));
	}

	#[rstest]
	fn test_escape_javascript() {
		let script = "'; alert('xss'); var x='";
		let escaped = escape_javascript(script);
		// Verify that single quotes are escaped
		assert!(escaped.contains("\\'"));
		// Verify the actual output
		// Input: '; alert('xss'); var x='
		// Output: \\'; alert(\\'xss\\'); var x=\\'
		assert_eq!(escaped, "\\'; alert(\\'xss\\'); var x=\\'");
	}

	#[rstest]
	fn test_escape_url() {
		let url = "javascript:alert('xss')";
		let escaped = escape_url(url);
		assert!(escaped.contains("%3A"));
	}

	#[rstest]
	fn test_detect_xss_patterns() {
		assert!(detect_xss_patterns("<script>alert(1)</script>"));
		assert!(detect_xss_patterns(r#"<img src=x onerror="alert(1)">"#));
		assert!(detect_xss_patterns("javascript:alert(1)"));
		assert!(detect_xss_patterns("<iframe src='evil.com'>"));
		assert!(!detect_xss_patterns("Safe text"));
		assert!(!detect_xss_patterns("Normal <b>HTML</b>"));
	}

	#[rstest]
	fn test_is_safe_url() {
		assert!(is_safe_url("https://example.com"));
		assert!(is_safe_url("http://example.com"));
		assert!(is_safe_url("/path/to/page"));
		assert!(is_safe_url("./relative/path"));
		assert!(is_safe_url("../parent/path"));
		assert!(is_safe_url("mailto:user@example.com"));
		assert!(!is_safe_url("javascript:alert(1)"));
		assert!(!is_safe_url("data:text/html,<script>alert(1)</script>"));
		assert!(!is_safe_url("vbscript:alert(1)"));
	}

	#[rstest]
	fn test_sanitize_html() {
		let dangerous = "<script>alert('XSS')</script><b>Bold text</b>";
		let sanitized = sanitize_html(dangerous);
		assert_eq!(
			sanitized,
			"&lt;script&gt;alert(&#x27;XSS&#x27;)&lt;/script&gt;&lt;b&gt;Bold text&lt;/b&gt;"
		);
	}
}
