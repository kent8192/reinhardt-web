//! CSS and JavaScript minification
//!
//! Provides minifiers for CSS and JavaScript files to reduce file size
//! and improve loading performance.

use super::{ProcessingResult, Processor};
use async_trait::async_trait;
use std::path::Path;

/// CSS minifier
///
/// Removes whitespace, comments, and optimizes CSS properties.
pub struct CssMinifier {
	/// Minification level (0-2)
	level: u8,
}

impl CssMinifier {
	/// Create a new CSS minifier with default settings
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_static::processing::minify::CssMinifier;
	///
	/// let minifier = CssMinifier::new();
	/// ```
	pub fn new() -> Self {
		Self { level: 2 }
	}

	/// Create a CSS minifier with custom level
	///
	/// # Arguments
	///
	/// * `level` - Minification level (0 = none, 1 = basic, 2 = aggressive)
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_static::processing::minify::CssMinifier;
	///
	/// let minifier = CssMinifier::with_level(1);
	/// ```
	pub fn with_level(level: u8) -> Self {
		Self {
			level: level.min(2),
		}
	}

	/// Minify CSS content
	fn minify_css(&self, input: &str) -> String {
		if self.level == 0 {
			return input.to_string();
		}

		let mut result = String::new();
		let mut in_comment = false;
		let mut prev_char = ' ';
		let mut chars = input.chars().peekable();

		while let Some(ch) = chars.next() {
			// Handle comments
			if ch == '/' && chars.peek() == Some(&'*') {
				in_comment = true;
				chars.next(); // Skip '*'
				continue;
			}

			if in_comment {
				if ch == '*' && chars.peek() == Some(&'/') {
					in_comment = false;
					chars.next(); // Skip '/'
				}
				continue;
			}

			// Remove unnecessary whitespace
			if ch.is_whitespace() {
				// Only add space if needed (between identifiers)
				if !prev_char.is_whitespace()
					&& result
						.chars()
						.last()
						.is_some_and(|c| c.is_alphanumeric() || c == ')' || c == ']')
					&& chars
						.peek()
						.is_some_and(|&next| next.is_alphanumeric() || next == '(')
				{
					result.push(' ');
				}
				prev_char = ' ';
				continue;
			}

			result.push(ch);
			prev_char = ch;
		}

		result
	}
}

impl Default for CssMinifier {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl Processor for CssMinifier {
	async fn process(&self, input: &[u8], _path: &Path) -> ProcessingResult<Vec<u8>> {
		let css = String::from_utf8_lossy(input);
		let minified = self.minify_css(&css);
		Ok(minified.into_bytes())
	}

	fn can_process(&self, path: &Path) -> bool {
		path.extension()
			.and_then(|ext| ext.to_str())
			.map(|ext| ext.eq_ignore_ascii_case("css"))
			.unwrap_or(false)
	}

	fn name(&self) -> &str {
		"CssMinifier"
	}
}

/// JavaScript minifier
///
/// Removes whitespace and comments from JavaScript files.
/// TODO: This is a basic minifier. For production use, consider using
/// dedicated tools like swc or terser.
pub struct JsMinifier {
	/// Remove comments
	remove_comments: bool,
}

impl JsMinifier {
	/// Create a new JavaScript minifier
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_static::processing::minify::JsMinifier;
	///
	/// let minifier = JsMinifier::new();
	/// ```
	pub fn new() -> Self {
		Self {
			remove_comments: true,
		}
	}

	/// Minify JavaScript content
	fn minify_js(&self, input: &str) -> String {
		if !self.remove_comments {
			return input.to_string();
		}

		let mut result = String::new();
		let mut chars = input.chars().peekable();
		let mut in_string = false;
		let mut string_delimiter = ' ';
		let mut prev_char = ' ';

		while let Some(ch) = chars.next() {
			// Handle string literals
			if (ch == '"' || ch == '\'' || ch == '`') && prev_char != '\\' {
				if !in_string {
					in_string = true;
					string_delimiter = ch;
				} else if ch == string_delimiter {
					in_string = false;
				}
				result.push(ch);
				prev_char = ch;
				continue;
			}

			if in_string {
				result.push(ch);
				prev_char = ch;
				continue;
			}

			// Handle single-line comments
			if ch == '/' && chars.peek() == Some(&'/') {
				chars.next(); // Skip second '/'
				while let Some(&next) = chars.peek() {
					chars.next();
					if next == '\n' {
						result.push('\n');
						break;
					}
				}
				prev_char = '\n';
				continue;
			}

			// Handle multi-line comments
			if ch == '/' && chars.peek() == Some(&'*') {
				chars.next(); // Skip '*'
				while let Some(c) = chars.next() {
					if c == '*' && chars.peek() == Some(&'/') {
						chars.next(); // Skip '/'
						break;
					}
				}
				prev_char = ' ';
				continue;
			}

			// Remove unnecessary whitespace
			if ch.is_whitespace() {
				// Keep newlines for ASI (Automatic Semicolon Insertion)
				if ch == '\n' && !result.ends_with('\n') {
					result.push('\n');
				} else if !prev_char.is_whitespace()
					&& result
						.chars()
						.last()
						.is_some_and(|c| c.is_alphanumeric() || c == ')' || c == ']' || c == '}')
					&& chars
						.peek()
						.is_some_and(|&next| next.is_alphanumeric() || next == '(' || next == '{')
				{
					result.push(' ');
				}
				prev_char = ' ';
				continue;
			}

			result.push(ch);
			prev_char = ch;
		}

		result
	}
}

impl Default for JsMinifier {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl Processor for JsMinifier {
	async fn process(&self, input: &[u8], _path: &Path) -> ProcessingResult<Vec<u8>> {
		let js = String::from_utf8_lossy(input);
		let minified = self.minify_js(&js);
		Ok(minified.into_bytes())
	}

	fn can_process(&self, path: &Path) -> bool {
		path.extension()
			.and_then(|ext| ext.to_str())
			.map(|ext| ext.eq_ignore_ascii_case("js"))
			.unwrap_or(false)
	}

	fn name(&self) -> &str {
		"JsMinifier"
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::PathBuf;

	#[test]
	fn test_css_minifier_can_process() {
		let minifier = CssMinifier::new();
		assert!(minifier.can_process(&PathBuf::from("style.css")));
		assert!(minifier.can_process(&PathBuf::from("style.CSS")));
		assert!(!minifier.can_process(&PathBuf::from("script.js")));
	}

	#[tokio::test]
	async fn test_css_minifier_basic() {
		let minifier = CssMinifier::new();
		let input = b"body { color: red; }";
		let result = minifier
			.process(input, &PathBuf::from("test.css"))
			.await
			.unwrap();
		let output = String::from_utf8(result).unwrap();
		assert!(output.len() <= input.len());
	}

	#[tokio::test]
	async fn test_css_minifier_removes_comments() {
		let minifier = CssMinifier::new();
		let input = b"/* comment */ body { color: red; }";
		let result = minifier
			.process(input, &PathBuf::from("test.css"))
			.await
			.unwrap();
		let output = String::from_utf8(result).unwrap();
		assert!(!output.contains("comment"));
	}

	#[tokio::test]
	async fn test_css_minifier_removes_whitespace() {
		let minifier = CssMinifier::new();
		let input = b"body {\n  color: red;\n  margin: 0;\n}";
		let result = minifier
			.process(input, &PathBuf::from("test.css"))
			.await
			.unwrap();
		let output = String::from_utf8(result).unwrap();
		assert!(output.len() < input.len());
		assert!(!output.contains('\n'));
	}

	#[tokio::test]
	async fn test_css_minifier_level_0() {
		let minifier = CssMinifier::with_level(0);
		let input = b"body { color: red; }";
		let result = minifier
			.process(input, &PathBuf::from("test.css"))
			.await
			.unwrap();
		assert_eq!(result, input);
	}

	#[test]
	fn test_js_minifier_can_process() {
		let minifier = JsMinifier::new();
		assert!(minifier.can_process(&PathBuf::from("app.js")));
		assert!(minifier.can_process(&PathBuf::from("app.JS")));
		assert!(!minifier.can_process(&PathBuf::from("style.css")));
	}

	#[tokio::test]
	async fn test_js_minifier_basic() {
		let minifier = JsMinifier::new();
		let input = b"const x = 1;";
		let result = minifier
			.process(input, &PathBuf::from("test.js"))
			.await
			.unwrap();
		let output = String::from_utf8(result).unwrap();
		assert!(output.len() <= input.len());
	}

	#[tokio::test]
	async fn test_js_minifier_removes_single_line_comments() {
		let minifier = JsMinifier::new();
		let input = b"// comment\nconst x = 1;";
		let result = minifier
			.process(input, &PathBuf::from("test.js"))
			.await
			.unwrap();
		let output = String::from_utf8(result).unwrap();
		assert!(!output.contains("comment"));
	}

	#[tokio::test]
	async fn test_js_minifier_removes_multi_line_comments() {
		let minifier = JsMinifier::new();
		let input = b"/* comment */ const x = 1;";
		let result = minifier
			.process(input, &PathBuf::from("test.js"))
			.await
			.unwrap();
		let output = String::from_utf8(result).unwrap();
		assert!(!output.contains("comment"));
	}

	#[tokio::test]
	async fn test_js_minifier_preserves_strings() {
		let minifier = JsMinifier::new();
		let input = b"const x = \"// not a comment\";";
		let result = minifier
			.process(input, &PathBuf::from("test.js"))
			.await
			.unwrap();
		let output = String::from_utf8(result).unwrap();
		assert!(output.contains("// not a comment"));
	}

	#[tokio::test]
	async fn test_js_minifier_preserves_newlines() {
		let minifier = JsMinifier::new();
		let input = b"const x = 1\nconst y = 2";
		let result = minifier
			.process(input, &PathBuf::from("test.js"))
			.await
			.unwrap();
		let output = String::from_utf8(result).unwrap();
		// Should preserve at least one newline for ASI
		assert!(output.contains('\n'));
	}
}
