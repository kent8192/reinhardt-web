//! XSS prevention tests
//!
//! Tests for cross-site scripting prevention utilities

use reinhardt_security::{escape_html, sanitize_html};

#[test]
fn test_escape_html_script_tags() {
	// Test: Script tags are escaped
	let input = "<script>alert('xss')</script>";
	let escaped = escape_html(input);
	assert_eq!(
		escaped,
		"&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
	);
	assert!(!escaped.contains("<script"));
	assert!(!escaped.contains("</script>"));
}

#[test]
fn test_escape_html_img_tag() {
	// Test: Image tags with onerror are escaped
	let input = r#"<img src="x" onerror="alert('xss')">"#;
	let escaped = escape_html(input);
	assert!(!escaped.contains("<img"));
	assert!(escaped.contains("&lt;img"));
}

#[test]
fn test_escape_html_ampersand() {
	// Test: Ampersands are escaped
	let input = "Tom & Jerry";
	let escaped = escape_html(input);
	assert_eq!(escaped, "Tom &amp; Jerry");
}

#[test]
fn test_escape_html_less_than() {
	// Test: Less than signs are escaped
	let input = "5 < 10";
	let escaped = escape_html(input);
	assert_eq!(escaped, "5 &lt; 10");
}

#[test]
fn test_escape_html_greater_than() {
	// Test: Greater than signs are escaped
	let input = "10 > 5";
	let escaped = escape_html(input);
	assert_eq!(escaped, "10 &gt; 5");
}

#[test]
fn test_escape_html_double_quotes() {
	// Test: Double quotes are escaped
	let input = r#"He said "Hello""#;
	let escaped = escape_html(input);
	assert_eq!(escaped, "He said &quot;Hello&quot;");
}

#[test]
fn test_escape_html_single_quotes() {
	// Test: Single quotes are escaped
	let input = "It's a test";
	let escaped = escape_html(input);
	assert_eq!(escaped, "It&#x27;s a test");
}

#[test]
fn test_escape_html_all_special_chars() {
	// Test: All special characters are escaped
	let input = r#"<>&"'"#;
	let escaped = escape_html(input);
	assert_eq!(escaped, "&lt;&gt;&amp;&quot;&#x27;");
}

#[test]
fn test_escape_html_normal_text() {
	// Test: Normal text is unchanged
	let input = "Hello, World!";
	let escaped = escape_html(input);
	assert_eq!(escaped, "Hello, World!");
}

#[test]
fn test_escape_html_empty_string() {
	// Test: Empty string returns empty string
	let escaped = escape_html("");
	assert_eq!(escaped, "");
}

#[test]
fn test_escape_html_numbers() {
	// Test: Numbers are unchanged
	let input = "12345";
	let escaped = escape_html(input);
	assert_eq!(escaped, "12345");
}

#[test]
fn test_escape_html_unicode() {
	// Test: Unicode characters are preserved
	let input = "Hello 世界 🌍";
	let escaped = escape_html(input);
	assert_eq!(escaped, "Hello 世界 🌍");
}

#[test]
fn test_escape_html_multiple_tags() {
	// Test: Multiple tags are escaped
	let input = "<div><span>test</span></div>";
	let escaped = escape_html(input);
	assert!(!escaped.contains("<div"));
	assert!(!escaped.contains("</div>"));
	assert!(!escaped.contains("<span"));
	assert!(!escaped.contains("</span>"));
}

#[test]
fn test_escape_html_javascript_protocol() {
	// Test: JavaScript protocol in HTML tags is neutralized by escaping the tags
	let input = r#"<a href="javascript:alert('xss')">click</a>"#;
	let escaped = escape_html(input);
	assert!(!escaped.contains("<a"));
	// The tags are escaped, making the javascript: protocol harmless
}

#[test]
fn test_escape_html_data_protocol() {
	// Test: Data protocol is neutralized by escaping tags
	let input = r#"<img src="data:text/html,<script>alert('xss')</script>">"#;
	let escaped = escape_html(input);
	assert!(!escaped.contains("<img"));
	assert!(!escaped.contains("<script"));
}

#[test]
fn test_escape_html_event_handlers() {
	// Test: Event handlers are neutralized by escaping the HTML tags
	let input = r#"<div onclick="alert('xss')">click me</div>"#;
	let escaped = escape_html(input);
	assert!(!escaped.contains("<div"));
	// The div tag is escaped, making the onclick handler harmless
}

#[test]
fn test_escape_html_nested_quotes() {
	// Test: Nested quotes are handled
	let input = r#"<a href="javascript:alert("xss")">link</a>"#;
	let escaped = escape_html(input);
	assert!(!escaped.contains("<a"));
	assert!(!escaped.contains(r#"href="javascript"#));
}

#[test]
fn test_escape_html_style_attribute() {
	// Test: Style attributes are neutralized by escaping the HTML tags
	let input = r#"<div style="background:url('javascript:alert(1)')">test</div>"#;
	let escaped = escape_html(input);
	assert!(!escaped.contains("<div"));
	// The div tag is escaped, making the style attribute harmless
}

#[test]
fn test_sanitize_html_basic() {
	// Test: sanitize_html escapes dangerous content
	let input = "<script>alert('xss')</script>";
	let sanitized = sanitize_html(input);
	assert!(!sanitized.contains("<script"));
	assert!(!sanitized.contains("</script>"));
}

#[test]
fn test_sanitize_html_same_as_escape() {
	// Test: sanitize_html currently behaves same as escape_html
	let input = "<div>test</div>";
	let escaped = escape_html(input);
	let sanitized = sanitize_html(input);
	assert_eq!(escaped, sanitized);
}

#[test]
fn test_escape_html_sql_injection_attempt() {
	// Test: SQL injection patterns are escaped (treating as text)
	let input = "'; DROP TABLE users; --";
	let escaped = escape_html(input);
	assert_eq!(escaped, "&#x27;; DROP TABLE users; --");
}

#[test]
fn test_escape_html_comment_injection() {
	// Test: HTML comments are escaped
	let input = "<!-- <script>alert('xss')</script> -->";
	let escaped = escape_html(input);
	assert!(!escaped.contains("<!--"));
	assert!(!escaped.contains("-->"));
}

#[test]
fn test_escape_html_null_byte() {
	// Test: Null bytes are preserved (escaped as-is in Rust strings)
	let input = "test\0null";
	let escaped = escape_html(input);
	assert_eq!(escaped, "test\0null");
}

#[test]
fn test_escape_html_newlines() {
	// Test: Newlines are preserved
	let input = "line1\nline2\r\nline3";
	let escaped = escape_html(input);
	assert_eq!(escaped, "line1\nline2\r\nline3");
}

#[test]
fn test_escape_html_tabs() {
	// Test: Tabs are preserved
	let input = "col1\tcol2\tcol3";
	let escaped = escape_html(input);
	assert_eq!(escaped, "col1\tcol2\tcol3");
}

#[test]
fn test_escape_html_mixed_content() {
	// Test: Mixed safe and unsafe content
	let input = "Safe text <script>unsafe</script> more safe";
	let escaped = escape_html(input);
	assert!(!escaped.contains("<script"));
	assert!(escaped.contains("Safe text"));
	assert!(escaped.contains("more safe"));
}

#[test]
fn test_escape_html_repeated_escaping() {
	// Test: Repeated escaping is safe (idempotent-like for already escaped)
	let input = "<script>";
	let escaped1 = escape_html(input);
	let escaped2 = escape_html(&escaped1);
	// Second escaping will escape the & in &lt;
	assert_ne!(escaped1, escaped2);
	assert_eq!(escaped2, "&amp;lt;script&amp;gt;");
}

#[test]
fn test_escape_html_long_string() {
	// Test: Long strings are handled correctly
	let input = "<script>".repeat(1000);
	let escaped = escape_html(&input);
	assert!(!escaped.contains("<script"));
	assert_eq!(escaped, "&lt;script&gt;".repeat(1000));
}
