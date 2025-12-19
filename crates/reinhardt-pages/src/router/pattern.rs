//! Path Pattern Matching for URL routing.
//!
//! This module provides Django-style path pattern matching compatible
//! with reinhardt-urls patterns.

use std::collections::HashMap;

/// A path parameter extracted from a URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathParam {
	/// The parameter name.
	pub name: String,
	/// The extracted value.
	pub value: String,
}

/// Represents a compiled path pattern.
///
/// Supports Django-style patterns like:
/// - `/users/` - Exact match
/// - `/users/{id}/` - Single path parameter
/// - `/users/{id}/posts/{post_id}/` - Multiple parameters
/// - `/static/{path:*}/` - Wildcard matching (rest of path)
#[derive(Debug, Clone)]
pub struct PathPattern {
	/// The original pattern string.
	pattern: String,
	/// Compiled regex pattern.
	regex: regex::Regex,
	/// Parameter names in order.
	param_names: Vec<String>,
	/// Whether this is an exact match pattern.
	is_exact: bool,
}

impl PathPattern {
	/// Creates a new path pattern from a Django-style pattern string.
	///
	/// # Pattern Syntax
	///
	/// - `{name}` - Captures a path segment (excludes `/`)
	/// - `{name:*}` - Captures the rest of the path (includes `/`)
	/// - Literal text is matched exactly
	///
	/// # Examples
	///
	/// ```ignore
	/// let pattern = PathPattern::new("/users/{id}/");
	/// assert!(pattern.matches("/users/42/").is_some());
	/// assert!(pattern.matches("/users/abc/").is_some());
	/// assert!(pattern.matches("/users/").is_none());
	/// ```
	pub fn new(pattern: &str) -> Self {
		let (regex_str, param_names) = Self::compile_pattern(pattern);
		let regex = regex::Regex::new(&regex_str).expect("Invalid pattern");

		Self {
			pattern: pattern.to_string(),
			regex,
			param_names,
			is_exact: !pattern.contains('{'),
		}
	}

	/// Compiles a pattern string into a regex and extracts parameter names.
	fn compile_pattern(pattern: &str) -> (String, Vec<String>) {
		let mut regex_str = String::from("^");
		let mut param_names = Vec::new();
		let mut chars = pattern.chars().peekable();

		while let Some(c) = chars.next() {
			match c {
				'{' => {
					// Start of a parameter
					let mut param = String::new();
					let mut is_wildcard = false;

					while let Some(&next) = chars.peek() {
						if next == '}' {
							chars.next(); // consume '}'
							break;
						}
						if next == ':' {
							chars.next(); // consume ':'
							if chars.peek() == Some(&'*') {
								chars.next(); // consume '*'
								is_wildcard = true;
							}
							continue;
						}
						param.push(chars.next().unwrap());
					}

					param_names.push(param.clone());

					if is_wildcard {
						// Wildcard: match anything including slashes
						regex_str.push_str(&format!("(?P<{}>.*)", param));
					} else {
						// Normal: match anything except slashes
						regex_str.push_str(&format!("(?P<{}>[^/]+)", param));
					}
				}
				'/' | '.' | '+' | '*' | '?' | '(' | ')' | '[' | ']' | '^' | '$' | '|' | '\\' => {
					// Escape regex special characters
					regex_str.push('\\');
					regex_str.push(c);
				}
				_ => {
					regex_str.push(c);
				}
			}
		}

		regex_str.push('$');
		(regex_str, param_names)
	}

	/// Returns the original pattern string.
	pub fn pattern(&self) -> &str {
		&self.pattern
	}

	/// Returns the parameter names.
	pub fn param_names(&self) -> &[String] {
		&self.param_names
	}

	/// Attempts to match a path against this pattern.
	///
	/// Returns `Some(params)` if the path matches, where `params` is a map
	/// of parameter names to their extracted values.
	pub fn matches(&self, path: &str) -> Option<HashMap<String, String>> {
		self.regex.captures(path).map(|caps| {
			self.param_names
				.iter()
				.filter_map(|name| {
					caps.name(name)
						.map(|m| (name.clone(), m.as_str().to_string()))
				})
				.collect()
		})
	}

	/// Generates a path from this pattern with the given parameters.
	///
	/// # Examples
	///
	/// ```ignore
	/// let pattern = PathPattern::new("/users/{id}/posts/{post_id}/");
	/// let params = [("id", "42"), ("post_id", "123")].into();
	/// assert_eq!(pattern.reverse(&params), Some("/users/42/posts/123/".to_string()));
	/// ```
	pub fn reverse(&self, params: &HashMap<String, String>) -> Option<String> {
		let mut result = self.pattern.clone();

		for name in &self.param_names {
			let value = params.get(name)?;
			// Replace {name} or {name:*} with the value
			let placeholder = format!("{{{}}}", name);
			let wildcard_placeholder = format!("{{{}:*}}", name);

			if result.contains(&placeholder) {
				result = result.replace(&placeholder, value);
			} else if result.contains(&wildcard_placeholder) {
				result = result.replace(&wildcard_placeholder, value);
			} else {
				return None;
			}
		}

		Some(result)
	}

	/// Checks if this pattern would match the given path.
	pub fn is_match(&self, path: &str) -> bool {
		self.regex.is_match(path)
	}

	/// Returns whether this is an exact match pattern (no parameters).
	pub fn is_exact(&self) -> bool {
		self.is_exact
	}
}

impl PartialEq for PathPattern {
	fn eq(&self, other: &Self) -> bool {
		self.pattern == other.pattern
	}
}

impl Eq for PathPattern {}

impl std::fmt::Display for PathPattern {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.pattern)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_exact_pattern() {
		let pattern = PathPattern::new("/users/");
		assert!(pattern.is_exact());
		assert!(pattern.is_match("/users/"));
		assert!(!pattern.is_match("/users/123/"));
	}

	#[test]
	fn test_single_param() {
		let pattern = PathPattern::new("/users/{id}/");
		assert!(!pattern.is_exact());
		assert!(pattern.is_match("/users/42/"));
		assert!(pattern.is_match("/users/abc/"));
		assert!(!pattern.is_match("/users/"));

		let params = pattern.matches("/users/42/").unwrap();
		assert_eq!(params.get("id"), Some(&"42".to_string()));
	}

	#[test]
	fn test_multiple_params() {
		let pattern = PathPattern::new("/users/{user_id}/posts/{post_id}/");
		let params = pattern.matches("/users/42/posts/123/").unwrap();

		assert_eq!(params.get("user_id"), Some(&"42".to_string()));
		assert_eq!(params.get("post_id"), Some(&"123".to_string()));
	}

	#[test]
	fn test_wildcard_param() {
		let pattern = PathPattern::new("/static/{path:*}");
		let params = pattern.matches("/static/css/styles/main.css").unwrap();

		assert_eq!(params.get("path"), Some(&"css/styles/main.css".to_string()));
	}

	#[test]
	fn test_reverse_simple() {
		let pattern = PathPattern::new("/users/{id}/");
		let mut params = HashMap::new();
		params.insert("id".to_string(), "42".to_string());

		assert_eq!(pattern.reverse(&params), Some("/users/42/".to_string()));
	}

	#[test]
	fn test_reverse_multiple_params() {
		let pattern = PathPattern::new("/users/{user_id}/posts/{post_id}/");
		let mut params = HashMap::new();
		params.insert("user_id".to_string(), "42".to_string());
		params.insert("post_id".to_string(), "123".to_string());

		assert_eq!(
			pattern.reverse(&params),
			Some("/users/42/posts/123/".to_string())
		);
	}

	#[test]
	fn test_reverse_missing_param() {
		let pattern = PathPattern::new("/users/{id}/");
		let params = HashMap::new();

		assert_eq!(pattern.reverse(&params), None);
	}

	#[test]
	fn test_param_names() {
		let pattern = PathPattern::new("/a/{x}/b/{y}/c/{z}/");
		assert_eq!(pattern.param_names(), &["x", "y", "z"]);
	}

	#[test]
	fn test_special_chars_escaped() {
		let pattern = PathPattern::new("/api/v1.0/");
		assert!(pattern.is_match("/api/v1.0/"));
		assert!(!pattern.is_match("/api/v1X0/"));
	}

	#[test]
	fn test_pattern_display() {
		let pattern = PathPattern::new("/users/{id}/");
		assert_eq!(format!("{}", pattern), "/users/{id}/");
	}

	#[test]
	fn test_pattern_equality() {
		let p1 = PathPattern::new("/users/{id}/");
		let p2 = PathPattern::new("/users/{id}/");
		let p3 = PathPattern::new("/users/{user_id}/");

		assert_eq!(p1, p2);
		assert_ne!(p1, p3);
	}
}
