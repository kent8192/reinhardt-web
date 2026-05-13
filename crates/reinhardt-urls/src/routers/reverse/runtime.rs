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

/// Optimized URL parameter substitution using Aho-Corasick algorithm
///
/// This function uses Aho-Corasick for multi-pattern matching, allowing
/// simultaneous detection of all placeholders in a single pass.
///
/// # Algorithm
///
/// 1. Extract all placeholder names from the pattern
/// 2. Build Aho-Corasick automaton for all placeholders (one-time construction)
/// 3. Find all placeholder positions in O(n+z) where z is number of matches
/// 4. Replace placeholders from right to left to avoid position shifts
///
/// # Performance
///
/// - Time complexity: O(n+m+z) where:
///   - n: pattern length
///   - m: total parameter values length
///   - z: number of placeholder matches
/// - Expected improvement: 3-5x for patterns with 10+ parameters
///
/// # Arguments
///
/// * `pattern` - URL pattern with placeholders like "/users/{id}/posts/{post_id}/"
/// * `params` - HashMap of parameter names to values
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use reinhardt_urls::routers::reverse::reverse_with_aho_corasick;
///
/// let mut params = HashMap::new();
/// params.insert("id".to_string(), "123".to_string());
/// params.insert("post_id".to_string(), "456".to_string());
///
/// let url = reverse_with_aho_corasick("/users/{id}/posts/{post_id}/", &params);
/// assert_eq!(url, "/users/123/posts/456/");
/// ```
#[deprecated(
	since = "0.1.0-rc.29",
	note = "use `try_reverse_with_aho_corasick`; this variant panics on invalid params and will be removed in a future release"
)]
pub fn reverse_with_aho_corasick(pattern: &str, params: &HashMap<String, String>) -> String {
	// Extract all placeholder names
	let param_names = extract_param_names(pattern);

	if param_names.is_empty() {
		return pattern.to_string();
	}

	// Validate parameter values against injection attacks
	for (name, value) in params {
		if !validate_reverse_param(value) {
			panic!(
				"Invalid parameter value for '{}': contains dangerous characters (path separators, query delimiters, or encoded sequences)",
				name
			);
		}
	}

	// Build patterns for Aho-Corasick: ["{id}", "{post_id}", ...]
	let placeholders: Vec<String> = param_names
		.iter()
		.map(|name| format!("{{{}}}", name))
		.collect();

	// Build Aho-Corasick automaton
	let ac = match AhoCorasick::new(&placeholders) {
		Ok(ac) => ac,
		Err(_) => {
			// Fallback to original implementation if AC construction fails
			#[allow(deprecated, reason = "internal fallback during deprecation cycle")]
			return reverse_single_pass(pattern, params);
		}
	};

	// Find all matches
	let mut replacements = Vec::new();
	for mat in ac.find_iter(pattern) {
		let param_name = &param_names[mat.pattern()];
		if let Some(value) = params.get(param_name) {
			replacements.push((mat.start(), mat.end(), value.clone()));
		} else {
			// Keep placeholder if parameter not found
			replacements.push((mat.start(), mat.end(), format!("{{{}}}", param_name)));
		}
	}

	// Apply replacements from right to left to avoid position shifts
	let mut result = pattern.to_string();
	for (start, end, value) in replacements.into_iter().rev() {
		result.replace_range(start..end, &value);
	}

	result
}

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

/// Single-pass URL parameter substitution algorithm
///
/// This function performs placeholder substitution in O(n+m) time complexity,
/// where n is the length of the pattern and m is the total length of parameter values.
///
/// # Algorithm
///
/// 1. Iterate through pattern characters once (O(n))
/// 2. When encountering '{', extract parameter name until '}'
/// 3. Lookup parameter value in HashMap (O(1) amortized)
/// 4. Append value to result string
///
/// # Performance
///
/// - Old algorithm: O(n×m×p) where p is number of parameters
/// - New algorithm: O(n+m) where m is total length of parameter values
/// - Expected improvement: 10-50x for patterns with multiple parameters
///
/// # Arguments
///
/// * `pattern` - URL pattern with placeholders like "/users/{id}/posts/{post_id}/"
/// * `params` - HashMap of parameter names to values
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use reinhardt_urls::routers::reverse::reverse_single_pass;
///
/// let mut params = HashMap::new();
/// params.insert("id".to_string(), "123".to_string());
/// params.insert("post_id".to_string(), "456".to_string());
///
/// let url = reverse_single_pass("/users/{id}/posts/{post_id}/", &params);
/// assert_eq!(url, "/users/123/posts/456/");
/// ```
#[deprecated(
	since = "0.1.0-rc.29",
	note = "use `try_reverse_single_pass`; this variant panics on invalid params and will be removed in a future release"
)]
pub fn reverse_single_pass(pattern: &str, params: &HashMap<String, String>) -> String {
	// Validate parameter values against injection attacks
	for (name, value) in params {
		if !validate_reverse_param(value) {
			panic!(
				"Invalid parameter value for '{}': contains dangerous characters (path separators, query delimiters, or encoded sequences)",
				name
			);
		}
	}

	let mut result = String::with_capacity(pattern.len());
	let mut chars = pattern.chars().peekable();

	while let Some(ch) = chars.next() {
		if ch == '{' {
			// Extract parameter name until '}'
			let param_name: String = chars.by_ref().take_while(|&c| c != '}').collect();

			// Lookup parameter value (O(1) amortized)
			if let Some(value) = params.get(&param_name) {
				result.push_str(value);
			} else {
				// Parameter not found - preserve placeholder
				// This should not happen if validation was done beforehand
				result.push('{');
				result.push_str(&param_name);
				result.push('}');
			}
		} else {
			result.push(ch);
		}
	}

	result
}

/// Fallible variant of [`reverse_with_aho_corasick`].
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

/// Fallible variant of [`reverse_single_pass`].
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
