use super::validation::{
	MAX_PATH_SEGMENTS, MAX_PATTERN_LENGTH, MAX_REGEX_SIZE, type_spec_to_regex, validate_path_param,
	validate_reverse_param,
};
use aho_corasick::AhoCorasick;
use regex::Regex;
use std::collections::{HashMap, HashSet};

/// Path pattern for URL matching
/// Similar to Django's URL patterns but using composition
#[derive(Clone, Debug)]
pub struct PathPattern {
	/// Original pattern string (may contain type specifiers)
	pattern: String,
	/// Pattern normalized to `{name}` format for URL reversal
	normalized_pattern: String,
	pub(super) regex: Regex,
	pub(super) param_names: Vec<String>,
	/// Parameter names that use the `path` type specifier.
	/// These require post-match validation to reject directory traversal.
	pub(super) path_type_params: HashSet<String>,
	/// Pre-built Aho-Corasick automaton for efficient URL reversal
	/// This is constructed once during pattern creation for O(n+m+z) reversal
	aho_corasick: Option<AhoCorasick>,
}

/// Parse result containing regex, param names, and normalized pattern for URL reversal
struct ParsePatternResult {
	regex_str: String,
	param_names: Vec<String>,
	/// Parameter names that use the `path` type specifier
	path_type_params: HashSet<String>,
	/// Pattern normalized to `{name}` format for URL reversal
	/// e.g., "/users/{<int:id>}/" -> "/users/{id}/"
	normalized_pattern: String,
}

impl PathPattern {
	/// Create a new path pattern
	/// Patterns like "/users/{id}/" are converted to regex
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::{PathPattern, path};
	///
	/// // Create a simple pattern without parameters
	/// let pattern = PathPattern::new(path!("/users/")).unwrap();
	/// assert_eq!(pattern.pattern(), "/users/");
	///
	/// // Create a pattern with a parameter
	/// let pattern = PathPattern::new(path!("/users/{id}/")).unwrap();
	/// assert_eq!(pattern.param_names(), &["id"]);
	/// ```
	pub fn new(pattern: impl Into<String>) -> Result<Self, String> {
		let pattern = pattern.into();

		// Reject patterns exceeding the maximum length to prevent ReDoS
		if pattern.len() > MAX_PATTERN_LENGTH {
			return Err(format!(
				"Pattern length {} exceeds maximum allowed length of {} bytes",
				pattern.len(),
				MAX_PATTERN_LENGTH
			));
		}

		// Reject patterns with excessive path segments to prevent resource exhaustion
		let segment_count = pattern.split('/').count();
		if segment_count > MAX_PATH_SEGMENTS {
			return Err(format!(
				"Pattern has {} path segments, exceeding maximum of {}",
				segment_count, MAX_PATH_SEGMENTS
			));
		}

		let parse_result = Self::parse_pattern(&pattern)?;

		// Use RegexBuilder with size limits to prevent memory exhaustion
		let regex = regex::RegexBuilder::new(&parse_result.regex_str)
			.size_limit(MAX_REGEX_SIZE)
			.build()
			.map_err(|e| format!("Failed to compile pattern regex: {}", e))?;

		// Build Aho-Corasick automaton for URL reversal if there are parameters
		let aho_corasick = if !parse_result.param_names.is_empty() {
			let placeholders: Vec<String> = parse_result
				.param_names
				.iter()
				.map(|name| format!("{{{}}}", name))
				.collect();

			AhoCorasick::new(&placeholders)
				.map(Some)
				.map_err(|e| format!("Failed to build Aho-Corasick automaton: {}", e))?
		} else {
			None
		};

		Ok(Self {
			pattern,
			normalized_pattern: parse_result.normalized_pattern,
			regex,
			param_names: parse_result.param_names,
			path_type_params: parse_result.path_type_params,
			aho_corasick,
		})
	}

	fn parse_pattern(pattern: &str) -> Result<ParsePatternResult, String> {
		let mut regex_str = String::from("^");
		let mut param_names = Vec::new();
		let mut path_type_params = HashSet::new();
		let mut normalized_pattern = String::new();
		let mut chars = pattern.chars().peekable();

		while let Some(ch) = chars.next() {
			match ch {
				'{' => {
					// Extract parameter content (everything between { and })
					let mut param_content = String::new();
					while let Some(&next_ch) = chars.peek() {
						if next_ch == '}' {
							chars.next(); // consume '}'
							break;
						}
						param_content.push(chars.next().unwrap());
					}

					if param_content.is_empty() {
						return Err("Empty parameter name".to_string());
					}

					// Check for typed parameter syntax: {<type:name>}
					let (param_name, regex_pattern) =
						if param_content.starts_with('<') && param_content.ends_with('>') {
							// Parse {<type:name>}
							let inner = &param_content[1..param_content.len() - 1]; // Remove < >
							if let Some(colon_pos) = inner.find(':') {
								let type_spec = &inner[..colon_pos];
								let name = &inner[colon_pos + 1..];
								if name.is_empty() {
									return Err(format!(
										"Empty parameter name in typed parameter: {{<{}:>}}",
										type_spec
									));
								}
								if type_spec == "path" {
									path_type_params.insert(name.to_string());
								}
								(name.to_string(), type_spec_to_regex(type_spec))
							} else {
								return Err(format!(
									"Invalid typed parameter syntax: {{<{}>}}. Expected {{<type:name>}}",
									inner
								));
							}
						} else {
							// Simple {name} parameter - use default [^/]+
							(param_content, "[^/]+")
						};

					param_names.push(param_name.clone());
					regex_str.push_str(&format!("(?P<{}>{})", param_name, regex_pattern));
					// Write normalized placeholder for URL reversal
					normalized_pattern.push_str(&format!("{{{}}}", param_name));
				}
				_ => {
					// Escape special regex characters
					if ".*+?^${}()|[]\\".contains(ch) {
						regex_str.push('\\');
					}
					regex_str.push(ch);
					// Copy literal characters to normalized pattern
					normalized_pattern.push(ch);
				}
			}
		}

		regex_str.push('$');
		Ok(ParsePatternResult {
			regex_str,
			param_names,
			path_type_params,
			normalized_pattern,
		})
	}
	/// Get the original pattern string
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::{PathPattern, path};
	///
	/// let pattern = PathPattern::new(path!("/users/{id}/")).unwrap();
	/// assert_eq!(pattern.pattern(), "/users/{id}/");
	/// ```
	pub fn pattern(&self) -> &str {
		&self.pattern
	}

	/// Convert pattern to matchit-compatible format
	///
	/// Transforms path-type parameters from `{<path:name>}` to `{*name}`
	/// for use with the matchit radix router. Non-path parameters remain
	/// as `{name}`.
	pub(crate) fn to_matchit_pattern(&self) -> String {
		let mut result = String::new();
		let mut chars = self.pattern.chars().peekable();

		while let Some(ch) = chars.next() {
			if ch == '{' {
				let mut param_content = String::new();
				while let Some(&next_ch) = chars.peek() {
					if next_ch == '}' {
						chars.next();
						break;
					}
					param_content.push(chars.next().unwrap());
				}

				// Check for typed parameter: {<type:name>}
				if param_content.starts_with('<') && param_content.ends_with('>') {
					let inner = &param_content[1..param_content.len() - 1];
					if let Some(colon_pos) = inner.find(':') {
						let type_spec = &inner[..colon_pos];
						let name = &inner[colon_pos + 1..];
						if type_spec == "path" {
							// Convert path type to matchit catch-all: {*name}
							result.push_str(&format!("{{*{}}}", name));
						} else {
							// Other typed params use simple {name}
							result.push_str(&format!("{{{}}}", name));
						}
					} else {
						result.push_str(&format!("{{{}}}", param_content));
					}
				} else {
					// Simple {name} parameter
					result.push_str(&format!("{{{}}}", param_content));
				}
			} else {
				result.push(ch);
			}
		}

		result
	}
	/// Get the list of parameter names in the pattern
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::{PathPattern, path};
	///
	/// let pattern = PathPattern::new(path!("/users/{user_id}/posts/{post_id}/")).unwrap();
	/// assert_eq!(pattern.param_names(), &["user_id", "post_id"]);
	/// ```
	pub fn param_names(&self) -> &[String] {
		&self.param_names
	}

	/// Test if the pattern matches a given path
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::{PathPattern, path};
	///
	/// let pattern = PathPattern::new(path!("/users/{id}/")).unwrap();
	/// assert!(pattern.is_match("/users/123/"));
	/// assert!(!pattern.is_match("/users/"));
	/// ```
	pub fn is_match(&self, path: &str) -> bool {
		self.regex.is_match(path)
	}

	/// Match a path and extract parameters
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::{PathPattern, path};
	///
	/// let pattern = PathPattern::new(path!("/users/{id}/")).unwrap();
	/// let params = pattern.extract_params("/users/123/").unwrap();
	/// assert_eq!(params.get("id"), Some(&"123".to_string()));
	/// ```
	pub fn extract_params(&self, path: &str) -> Option<HashMap<String, String>> {
		self.regex.captures(path).and_then(|captures| {
			let mut params = HashMap::new();
			for name in self.param_names() {
				if let Some(value) = captures.name(name) {
					let val = value.as_str();
					// Validate path-type parameters against directory traversal
					if self.path_type_params.contains(name) && !validate_path_param(val) {
						return None;
					}
					params.insert(name.clone(), val.to_string());
				}
			}
			Some(params)
		})
	}

	/// Reverse URL pattern with parameters using Aho-Corasick algorithm
	///
	/// This method uses pre-built Aho-Corasick automaton for efficient
	/// multi-pattern matching with O(n+m+z) complexity where:
	/// - n: pattern length
	/// - m: total parameter values length
	/// - z: number of placeholder matches
	///
	/// # Arguments
	///
	/// * `params` - HashMap of parameter names to values
	///
	/// # Returns
	///
	/// * `Ok(String)` - Reversed URL with parameters substituted
	/// * `Err(String)` - If required parameters are missing
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::{PathPattern, path};
	/// use std::collections::HashMap;
	///
	/// let pattern = PathPattern::new(path!("/users/{id}/posts/{post_id}/")).unwrap();
	///
	/// let mut params = HashMap::new();
	/// params.insert("id".to_string(), "123".to_string());
	/// params.insert("post_id".to_string(), "456".to_string());
	///
	/// let url = pattern.reverse(&params).unwrap();
	/// assert_eq!(url, "/users/123/posts/456/");
	/// ```
	pub fn reverse(&self, params: &HashMap<String, String>) -> Result<String, String> {
		// Validate all required parameters are present
		for param_name in &self.param_names {
			if !params.contains_key(param_name) {
				return Err(format!("Missing required parameter: {}", param_name));
			}
		}

		// Validate parameter values against injection attacks
		for (name, value) in params {
			if !validate_reverse_param(value) {
				return Err(format!(
					"Invalid parameter value for '{}': contains dangerous characters",
					name
				));
			}
		}

		// If no parameters, return normalized pattern as-is
		if self.param_names.is_empty() {
			return Ok(self.normalized_pattern.clone());
		}

		// Use Aho-Corasick if available, otherwise fallback to single-pass
		match &self.aho_corasick {
			Some(ac) => {
				// Find all matches using Aho-Corasick on normalized pattern
				let mut replacements = Vec::new();
				for mat in ac.find_iter(&self.normalized_pattern) {
					let param_name = &self.param_names[mat.pattern()];
					// We already validated all params exist above
					let value = params.get(param_name).unwrap();
					replacements.push((mat.start(), mat.end(), value.clone()));
				}

				// Apply replacements from right to left to avoid position shifts
				let mut result = self.normalized_pattern.clone();
				for (start, end, value) in replacements.into_iter().rev() {
					result.replace_range(start..end, &value);
				}

				Ok(result)
			}
			None => {
				// Fallback: no parameters, just return normalized pattern
				Ok(self.normalized_pattern.clone())
			}
		}
	}
}
