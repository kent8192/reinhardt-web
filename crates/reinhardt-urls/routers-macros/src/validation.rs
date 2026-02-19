//! URL path validation logic for compile-time checks.

/// Errors that can occur during path validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PathValidationError {
	/// Path does not start with '/'
	MustStartWithSlash,
	/// Unmatched opening brace '{'
	UnmatchedOpenBrace(usize),
	/// Unmatched closing brace '}'
	UnmatchedCloseBrace(usize),
	/// Parameter name is empty
	EmptyParameterName(usize),
	/// Parameter name is not valid snake_case
	InvalidParameterName { name: String, position: usize },
	/// Double slash '//' found
	DoubleSlash(usize),
	/// Invalid character in path
	InvalidCharacter { ch: char, position: usize },
	/// Nested parameters (e.g., {{inner}})
	NestedParameters(usize),
	/// Path traversal sequence '..' found
	PathTraversal(usize),
	/// Duplicate parameter name in path
	DuplicateParameterName { name: String, position: usize },
}

/// Validates URL path syntax
pub(crate) fn validate_path_syntax(path: &str) -> Result<(), PathValidationError> {
	// Check if path starts with '/'
	if !path.starts_with('/') {
		return Err(PathValidationError::MustStartWithSlash);
	}

	// Check for path traversal sequences before character-level validation
	if let Some(pos) = find_path_traversal(path) {
		return Err(PathValidationError::PathTraversal(pos));
	}

	// Track parameter brace depth and position
	let mut brace_depth = 0;
	let mut param_start: Option<usize> = None;
	let mut prev_char: Option<char> = None;
	let mut seen_params: Vec<String> = Vec::new();

	for (i, ch) in path.char_indices() {
		// Check for double slashes
		if ch == '/' && prev_char == Some('/') {
			return Err(PathValidationError::DoubleSlash(i));
		}

		match ch {
			'{' => {
				if brace_depth > 0 {
					return Err(PathValidationError::NestedParameters(i));
				}
				brace_depth += 1;
				param_start = Some(i);
			}
			'}' => {
				if brace_depth == 0 {
					return Err(PathValidationError::UnmatchedCloseBrace(i));
				}
				brace_depth -= 1;

				// Extract and validate parameter name
				if let Some(start) = param_start {
					let param_name = &path[start + 1..i];
					if param_name.is_empty() {
						return Err(PathValidationError::EmptyParameterName(start));
					}
					validate_parameter_name(param_name, start)?;

					// Check for duplicate parameter names
					if seen_params.iter().any(|p| p == param_name) {
						return Err(PathValidationError::DuplicateParameterName {
							name: param_name.to_string(),
							position: start,
						});
					}
					seen_params.push(param_name.to_string());
				}
				param_start = None;
			}
			c if brace_depth == 0 => {
				// Outside of parameter: validate path character
				validate_path_character(c, i)?;
			}
			_ => {
				// Inside parameter: will be validated when closing brace is found
			}
		}

		prev_char = Some(ch);
	}

	// Check for unclosed braces
	if brace_depth > 0
		&& let Some(start) = param_start
	{
		return Err(PathValidationError::UnmatchedOpenBrace(start));
	}

	Ok(())
}

/// Finds a path traversal sequence ('..') in the path, returning its position if found.
///
/// Only detects '..' sequences that appear as path segment components,
/// i.e., preceded and followed by '/' or at string boundaries.
fn find_path_traversal(path: &str) -> Option<usize> {
	let bytes = path.as_bytes();
	let len = bytes.len();

	for i in 0..len.saturating_sub(1) {
		if bytes[i] == b'.' && bytes[i + 1] == b'.' {
			// Check that this is a standalone '..' segment, not part of '...'
			let before_ok = i == 0 || bytes[i - 1] == b'/';
			let after_idx = i + 2;
			let after_ok = after_idx >= len || bytes[after_idx] == b'/';
			if before_ok && after_ok {
				return Some(i);
			}
		}
	}

	None
}

/// Validates a single path character (outside of parameters)
fn validate_path_character(ch: char, position: usize) -> Result<(), PathValidationError> {
	match ch {
		'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '/' | '.' | '*' => Ok(()),
		_ => Err(PathValidationError::InvalidCharacter { ch, position }),
	}
}

/// Validates parameter name is valid snake_case identifier
fn validate_parameter_name(name: &str, position: usize) -> Result<(), PathValidationError> {
	if name.is_empty() {
		return Err(PathValidationError::EmptyParameterName(position));
	}

	let mut chars = name.chars();

	// First character must be lowercase letter or underscore
	if let Some(first) = chars.next()
		&& !matches!(first, 'a'..='z' | '_')
	{
		return Err(PathValidationError::InvalidParameterName {
			name: name.to_string(),
			position,
		});
	}

	// Remaining characters must be lowercase letter, digit, or underscore
	for ch in chars {
		if !matches!(ch, 'a'..='z' | '0'..='9' | '_') {
			return Err(PathValidationError::InvalidParameterName {
				name: name.to_string(),
				position,
			});
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_valid_simple_path() {
		// Arrange & Act & Assert
		assert!(validate_path_syntax("/users/").is_ok());
		assert!(validate_path_syntax("/items/").is_ok());
		assert!(validate_path_syntax("/api/v1/users/").is_ok());
	}

	#[rstest]
	fn test_valid_path_with_parameters() {
		// Arrange & Act & Assert
		assert!(validate_path_syntax("/users/{id}/").is_ok());
		assert!(validate_path_syntax("/users/{user_id}/").is_ok());
		assert!(validate_path_syntax("/users/{user_id}/posts/{post_id}/").is_ok());
		assert!(validate_path_syntax("/items/{item_id}/details/").is_ok());
	}

	#[rstest]
	fn test_must_start_with_slash() {
		// Arrange & Act
		let result = validate_path_syntax("users/");

		// Assert
		assert_eq!(result, Err(PathValidationError::MustStartWithSlash));
	}

	#[rstest]
	fn test_unmatched_open_brace() {
		// Arrange & Act
		let result = validate_path_syntax("/users/{id/");

		// Assert
		assert!(matches!(
			result,
			Err(PathValidationError::UnmatchedOpenBrace(_))
		));
	}

	#[rstest]
	fn test_unmatched_close_brace() {
		// Arrange & Act
		let result = validate_path_syntax("/users/id}/");

		// Assert
		assert!(matches!(
			result,
			Err(PathValidationError::UnmatchedCloseBrace(_))
		));
	}

	#[rstest]
	fn test_empty_parameter_name() {
		// Arrange & Act
		let result = validate_path_syntax("/users/{}/");

		// Assert
		assert!(matches!(
			result,
			Err(PathValidationError::EmptyParameterName(_))
		));
	}

	#[rstest]
	fn test_invalid_parameter_name_uppercase() {
		// Arrange & Act
		let result = validate_path_syntax("/users/{userId}/");

		// Assert
		assert!(matches!(
			result,
			Err(PathValidationError::InvalidParameterName { .. })
		));
	}

	#[rstest]
	fn test_invalid_parameter_name_hyphen() {
		// Arrange & Act
		let result = validate_path_syntax("/users/{user-id}/");

		// Assert
		assert!(matches!(
			result,
			Err(PathValidationError::InvalidParameterName { .. })
		));
	}

	#[rstest]
	fn test_double_slash() {
		// Arrange & Act
		let result = validate_path_syntax("/users//posts/");

		// Assert
		assert!(matches!(result, Err(PathValidationError::DoubleSlash(_))));
	}

	#[rstest]
	fn test_nested_parameters() {
		// Arrange & Act
		let result = validate_path_syntax("/users/{{id}}/");

		// Assert
		assert!(matches!(
			result,
			Err(PathValidationError::NestedParameters(_))
		));
	}

	#[rstest]
	fn test_valid_parameter_with_underscore() {
		// Arrange & Act & Assert
		assert!(validate_path_syntax("/users/{_id}/").is_ok());
		assert!(validate_path_syntax("/users/{_}/").is_ok());
		assert!(validate_path_syntax("/users/{user_id_123}/").is_ok());
	}

	#[rstest]
	fn test_valid_path_with_hyphens() {
		// Arrange & Act & Assert
		assert!(validate_path_syntax("/user-profiles/").is_ok());
		assert!(validate_path_syntax("/api-v1/user-data/").is_ok());
	}

	#[rstest]
	fn test_valid_path_with_dots() {
		// Arrange & Act & Assert
		assert!(validate_path_syntax("/files/document.pdf").is_ok());
		assert!(validate_path_syntax("/api/v1.0/users/").is_ok());
	}

	#[rstest]
	fn test_path_traversal_rejected() {
		// Arrange & Act
		let result = validate_path_syntax("/files/../etc/passwd");

		// Assert
		assert_eq!(result, Err(PathValidationError::PathTraversal(7)));
	}

	#[rstest]
	fn test_path_traversal_multiple_rejected() {
		// Arrange & Act
		let result = validate_path_syntax("/api/../../secret");

		// Assert
		assert!(matches!(result, Err(PathValidationError::PathTraversal(_))));
	}

	#[rstest]
	fn test_path_traversal_at_root_rejected() {
		// Arrange & Act
		let result = validate_path_syntax("/../");

		// Assert
		assert!(matches!(result, Err(PathValidationError::PathTraversal(_))));
	}

	#[rstest]
	fn test_single_dot_allowed() {
		// Arrange & Act & Assert
		// Single dots should still be allowed (file extensions, versioning)
		assert!(validate_path_syntax("/files/test.txt").is_ok());
		assert!(validate_path_syntax("/api/v1.0/").is_ok());
	}

	#[rstest]
	fn test_triple_dots_allowed() {
		// Arrange & Act & Assert
		// Triple dots are not path traversal
		assert!(validate_path_syntax("/files/test.../").is_ok());
	}

	#[rstest]
	fn test_duplicate_parameter_name_rejected() {
		// Arrange & Act
		let result = validate_path_syntax("/users/{id}/posts/{id}/");

		// Assert
		assert!(matches!(
			result,
			Err(PathValidationError::DuplicateParameterName { name, .. }) if name == "id"
		));
	}

	#[rstest]
	fn test_different_parameter_names_allowed() {
		// Arrange & Act & Assert
		assert!(validate_path_syntax("/users/{user_id}/posts/{post_id}/").is_ok());
	}

	#[rstest]
	fn test_duplicate_parameter_name_different_segments() {
		// Arrange & Act
		let result = validate_path_syntax("/a/{name}/b/{name}/");

		// Assert
		assert!(matches!(
			result,
			Err(PathValidationError::DuplicateParameterName { name, .. }) if name == "name"
		));
	}
}
