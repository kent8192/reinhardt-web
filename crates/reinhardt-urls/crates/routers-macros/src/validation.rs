//! URL path validation logic for compile-time checks.

/// Errors that can occur during path validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathValidationError {
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
}

/// Validates URL path syntax
pub fn validate_path_syntax(path: &str) -> Result<(), PathValidationError> {
	// Check if path starts with '/'
	if !path.starts_with('/') {
		return Err(PathValidationError::MustStartWithSlash);
	}

	// Track parameter brace depth and position
	let mut brace_depth = 0;
	let mut param_start: Option<usize> = None;
	let mut prev_char: Option<char> = None;

	for (i, ch) in path.chars().enumerate() {
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
		&& let Some(start) = param_start {
			return Err(PathValidationError::UnmatchedOpenBrace(start));
		}

	Ok(())
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
		&& !matches!(first, 'a'..='z' | '_') {
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

	#[test]
	fn test_valid_simple_path() {
		assert!(validate_path_syntax("/users/").is_ok());
		assert!(validate_path_syntax("/items/").is_ok());
		assert!(validate_path_syntax("/api/v1/users/").is_ok());
	}

	#[test]
	fn test_valid_path_with_parameters() {
		assert!(validate_path_syntax("/users/{id}/").is_ok());
		assert!(validate_path_syntax("/users/{user_id}/").is_ok());
		assert!(validate_path_syntax("/users/{user_id}/posts/{post_id}/").is_ok());
		assert!(validate_path_syntax("/items/{item_id}/details/").is_ok());
	}

	#[test]
	fn test_must_start_with_slash() {
		let result = validate_path_syntax("users/");
		assert_eq!(result, Err(PathValidationError::MustStartWithSlash));
	}

	#[test]
	fn test_unmatched_open_brace() {
		let result = validate_path_syntax("/users/{id/");
		assert!(matches!(
			result,
			Err(PathValidationError::UnmatchedOpenBrace(_))
		));
	}

	#[test]
	fn test_unmatched_close_brace() {
		let result = validate_path_syntax("/users/id}/");
		assert!(matches!(
			result,
			Err(PathValidationError::UnmatchedCloseBrace(_))
		));
	}

	#[test]
	fn test_empty_parameter_name() {
		let result = validate_path_syntax("/users/{}/");
		assert!(matches!(
			result,
			Err(PathValidationError::EmptyParameterName(_))
		));
	}

	#[test]
	fn test_invalid_parameter_name_uppercase() {
		let result = validate_path_syntax("/users/{userId}/");
		assert!(matches!(
			result,
			Err(PathValidationError::InvalidParameterName { .. })
		));
	}

	#[test]
	fn test_invalid_parameter_name_hyphen() {
		let result = validate_path_syntax("/users/{user-id}/");
		assert!(matches!(
			result,
			Err(PathValidationError::InvalidParameterName { .. })
		));
	}

	#[test]
	fn test_double_slash() {
		let result = validate_path_syntax("/users//posts/");
		assert!(matches!(result, Err(PathValidationError::DoubleSlash(_))));
	}

	#[test]
	fn test_nested_parameters() {
		let result = validate_path_syntax("/users/{{id}}/");
		assert!(matches!(
			result,
			Err(PathValidationError::NestedParameters(_))
		));
	}

	#[test]
	fn test_valid_parameter_with_underscore() {
		assert!(validate_path_syntax("/users/{_id}/").is_ok());
		assert!(validate_path_syntax("/users/{_}/").is_ok());
		assert!(validate_path_syntax("/users/{user_id_123}/").is_ok());
	}

	#[test]
	fn test_valid_path_with_hyphens() {
		assert!(validate_path_syntax("/user-profiles/").is_ok());
		assert!(validate_path_syntax("/api-v1/user-data/").is_ok());
	}

	#[test]
	fn test_valid_path_with_dots() {
		assert!(validate_path_syntax("/files/document.pdf").is_ok());
		assert!(validate_path_syntax("/api/v1.0/users/").is_ok());
	}
}
