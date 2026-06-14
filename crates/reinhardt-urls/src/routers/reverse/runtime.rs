//! Runtime URL reversal helpers (free functions and shared result aliases).
//!
//! These string-based reversal routines are the building blocks used by
//! both the runtime `UrlReverser` and the typed-pattern reversal helpers.

use super::super::pattern::validate_reverse_param;
use aho_corasick::AhoCorasick;
use reinhardt_core::exception::{Error, Result};
use std::collections::HashMap;

/// Error type for URL reverse resolution failures.
pub type ReverseError = Error;
/// Result type for URL reverse resolution operations.
pub type ReverseResult<T> = Result<T>;

/// Extract parameter names from a URL pattern
///
/// # Examples
///
/// ```
/// use reinhardt_urls::routers::reverse::extract_param_names;
///
/// let names = extract_param_names("/users/{id}/posts/{post_id}/");
/// assert_eq!(names, vec!["id", "post_id"]);
/// ```
pub fn extract_param_names(pattern: &str) -> Vec<String> {
	let mut names = Vec::new();
	let mut chars = pattern.chars().peekable();

	while let Some(ch) = chars.next() {
		if ch == '{' {
			let name: String = chars.by_ref().take_while(|&c| c != '}').collect();
			if !name.is_empty() {
				names.push(name);
			}
		}
	}

	names
}

/// Fallible URL parameter substitution using Aho-Corasick algorithm.
///
/// Returns `Err(ReverseError::Validation(..))` instead of panicking when any
/// parameter value is rejected by `validate_reverse_param` (path separators,
/// query delimiters, or encoded sequences). Behavior is otherwise identical to
/// the panicking variant.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use reinhardt_urls::routers::reverse::try_reverse_with_aho_corasick;
///
/// let mut params = HashMap::new();
/// params.insert("id".to_string(), "123".to_string());
///
/// let url = try_reverse_with_aho_corasick("/users/{id}/", &params).unwrap();
/// assert_eq!(url, "/users/123/");
/// ```
pub fn try_reverse_with_aho_corasick(
	pattern: &str,
	params: &HashMap<String, String>,
) -> ReverseResult<String> {
	let param_names = extract_param_names(pattern);

	if param_names.is_empty() {
		return Ok(pattern.to_string());
	}

	// Validate parameter values against injection attacks.
	for (name, value) in params {
		if !validate_reverse_param(value) {
			return Err(Error::Validation(format!(
				"Invalid parameter value for '{}': contains dangerous characters (path separators, query delimiters, or encoded sequences)",
				name
			)));
		}
	}

	let placeholders: Vec<String> = param_names
		.iter()
		.map(|name| format!("{{{}}}", name))
		.collect();

	let ac = match AhoCorasick::new(&placeholders) {
		Ok(ac) => ac,
		Err(_) => {
			// Fallback to the single-pass implementation if AC construction fails.
			return try_reverse_single_pass(pattern, params);
		}
	};

	let mut replacements = Vec::new();
	for mat in ac.find_iter(pattern) {
		let param_name = &param_names[mat.pattern()];
		if let Some(value) = params.get(param_name) {
			replacements.push((mat.start(), mat.end(), value.clone()));
		} else {
			replacements.push((mat.start(), mat.end(), format!("{{{}}}", param_name)));
		}
	}

	let mut result = pattern.to_string();
	for (start, end, value) in replacements.into_iter().rev() {
		result.replace_range(start..end, &value);
	}

	Ok(result)
}

/// Fallible variant of `reverse_single_pass`.
///
/// Returns `Err(ReverseError::Validation(..))` instead of panicking when any
/// parameter value is rejected by `validate_reverse_param`. Behavior is
/// otherwise identical to the panicking variant.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use reinhardt_urls::routers::reverse::try_reverse_single_pass;
///
/// let mut params = HashMap::new();
/// params.insert("id".to_string(), "123".to_string());
///
/// let url = try_reverse_single_pass("/users/{id}/", &params).unwrap();
/// assert_eq!(url, "/users/123/");
/// ```
pub fn try_reverse_single_pass(
	pattern: &str,
	params: &HashMap<String, String>,
) -> ReverseResult<String> {
	// Validate parameter values against injection attacks.
	for (name, value) in params {
		if !validate_reverse_param(value) {
			return Err(Error::Validation(format!(
				"Invalid parameter value for '{}': contains dangerous characters (path separators, query delimiters, or encoded sequences)",
				name
			)));
		}
	}

	let mut result = String::with_capacity(pattern.len());
	let mut chars = pattern.chars().peekable();

	while let Some(ch) = chars.next() {
		if ch == '{' {
			let param_name: String = chars.by_ref().take_while(|&c| c != '}').collect();

			if let Some(value) = params.get(&param_name) {
				result.push_str(value);
			} else {
				// Parameter not found - preserve placeholder.
				result.push('{');
				result.push_str(&param_name);
				result.push('}');
			}
		} else {
			result.push(ch);
		}
	}

	Ok(result)
}
