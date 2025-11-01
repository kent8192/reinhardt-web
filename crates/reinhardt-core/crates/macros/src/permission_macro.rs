//! Permission string validation using nom parser combinators
//!
//! This module provides compile-time validation for Django-style permission strings.
//! Permission strings follow the format "app_label.permission_codename" where:
//! - app_label: The application label (e.g., "auth", "contenttypes", "myapp")
//! - permission_codename: The permission code (e.g., "view_user", "add_post", "delete_comment")
//!
//! Both parts must be valid Python/Django identifiers (start with letter or underscore,
//! followed by letters, numbers, or underscores).

use nom::{
	IResult, Parser,
	branch::alt,
	bytes::complete::tag,
	character::complete::{alpha1, alphanumeric1},
	combinator::{map, recognize},
	multi::many0_count,
	sequence::{pair, separated_pair},
};

// ============================================================================
// AST Definitions
// ============================================================================

/// Abstract Syntax Tree for permission strings
#[derive(Debug, Clone, PartialEq)]
pub struct PermissionAst {
	/// Application label (e.g., "auth", "myapp")
	pub app_label: String,
	/// Permission codename (e.g., "view_user", "add_post")
	pub permission_codename: String,
}

impl PermissionAst {
	/// Get the full permission string in "app.permission" format
	#[allow(dead_code)]
	pub fn to_string(&self) -> String {
		format!("{}.{}", self.app_label, self.permission_codename)
	}
}

// ============================================================================
// Nom Parsers
// ============================================================================

/// Parse a valid Python/Django identifier
/// Must start with letter or underscore, followed by alphanumeric or underscore
fn identifier(input: &str) -> IResult<&str, &str> {
	recognize(pair(
		alt((alpha1, tag("_"))),
		many0_count(alt((alphanumeric1, tag("_")))),
	))
	.parse(input)
}

/// Parse a permission string in "app.permission" format
fn permission_string(input: &str) -> IResult<&str, PermissionAst> {
	map(
		separated_pair(identifier, tag("."), identifier),
		|(app_label, permission_codename)| PermissionAst {
			app_label: app_label.to_string(),
			permission_codename: permission_codename.to_string(),
		},
	)
	.parse(input)
}

// ============================================================================
// Validation
// ============================================================================
/// Parse and validate a permission string
///
/// Permission strings must follow the format "app_label.permission_codename" where:
/// - `app_label`: The application label (e.g., "auth", "contenttypes")
/// - `permission_codename`: The permission code (e.g., "view_user", "add_post")
///
/// Both parts must be valid Python/Django identifiers (start with letter or underscore,
/// followed by letters, numbers, or underscores).
///
/// Returns a `PermissionAst` if valid, or an error message describing the problem.
pub fn parse_and_validate(permission: &str) -> std::result::Result<PermissionAst, String> {
	// Pre-validation: check for common errors
	if permission.trim().is_empty() {
		return Err("Permission string cannot be empty".to_string());
	}

	if !permission.contains('.') {
		return Err(
			"Permission string must be in 'app.permission' format (e.g., 'auth.view_user')"
				.to_string(),
		);
	}

	let dot_count = permission.chars().filter(|&c| c == '.').count();
	if dot_count > 1 {
		return Err(format!(
			"Permission string must contain exactly one dot, found {}",
			dot_count
		));
	}

	// Check for spaces
	if permission.contains(' ') {
		return Err("Permission string cannot contain spaces".to_string());
	}

	// Check for special characters that are not allowed
	let invalid_chars: Vec<char> = permission
		.chars()
		.filter(|c| !c.is_alphanumeric() && *c != '.' && *c != '_')
		.collect();

	if !invalid_chars.is_empty() {
		return Err(format!(
			"Permission string contains invalid characters: {:?}",
			invalid_chars
		));
	}

	// Parse with nom
	match permission_string(permission) {
		Ok((remaining, ast)) => {
			if remaining.is_empty() {
				Ok(ast)
			} else {
				Err(format!(
					"Unexpected characters after permission string: '{}'",
					remaining
				))
			}
		}
		Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
			let remaining = e.input;
			let error_pos = permission.len() - remaining.len();

			// Provide helpful error messages
			if remaining.starts_with('.') {
				Err("App label cannot be empty. Use format 'app.permission'".to_string())
			} else if error_pos == permission.len() {
				Err("Permission codename cannot be empty after the dot".to_string())
			} else if remaining.chars().next().map_or(false, |c| c.is_numeric()) {
				Err(format!(
					"Invalid identifier at position {}: cannot start with a number",
					error_pos
				))
			} else {
				Err(format!(
					"Invalid permission string at position {}: '{}'",
					error_pos, remaining
				))
			}
		}
		Err(nom::Err::Incomplete(_)) => Err("Incomplete permission string".to_string()),
	}
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_valid_simple_permission() {
		let result = parse_and_validate("auth.view_user");
		assert!(result.is_ok());
		let ast = result.unwrap();
		assert_eq!(ast.app_label, "auth");
		assert_eq!(ast.permission_codename, "view_user");
	}

	#[test]
	fn test_valid_permission_with_underscores() {
		let result = parse_and_validate("my_app.add_blog_post");
		assert!(result.is_ok());
		let ast = result.unwrap();
		assert_eq!(ast.app_label, "my_app");
		assert_eq!(ast.permission_codename, "add_blog_post");
	}

	#[test]
	fn test_valid_permission_starting_with_underscore() {
		let result = parse_and_validate("_app._permission");
		assert!(result.is_ok());
		let ast = result.unwrap();
		assert_eq!(ast.app_label, "_app");
		assert_eq!(ast.permission_codename, "_permission");
	}

	#[test]
	fn test_invalid_empty_string() {
		let result = parse_and_validate("");
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("cannot be empty"));
	}

	#[test]
	fn test_invalid_no_dot() {
		let result = parse_and_validate("authviewuser");
		assert!(result.is_err());
		assert!(
			result
				.unwrap_err()
				.contains("must be in 'app.permission' format")
		);
	}

	#[test]
	fn test_invalid_multiple_dots() {
		let result = parse_and_validate("auth.contrib.view_user");
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("exactly one dot"));
	}

	#[test]
	fn test_invalid_empty_app_label() {
		let result = parse_and_validate(".view_user");
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("App label cannot be empty"));
	}

	#[test]
	fn test_invalid_empty_permission() {
		let result = parse_and_validate("auth.");
		assert!(result.is_err());
		assert!(
			result
				.unwrap_err()
				.contains("Permission codename cannot be empty")
		);
	}

	#[test]
	fn test_invalid_starts_with_number() {
		let result = parse_and_validate("123app.view_user");
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("cannot start with a number"));
	}

	#[test]
	fn test_invalid_contains_space() {
		let result = parse_and_validate("auth.view user");
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("cannot contain spaces"));
	}

	#[test]
	fn test_invalid_special_characters() {
		let result = parse_and_validate("auth.view-user");
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("invalid characters"));
	}

	#[test]
	fn test_valid_all_lowercase() {
		let result = parse_and_validate("contenttypes.delete_contenttype");
		assert!(result.is_ok());
	}

	#[test]
	fn test_valid_mixed_case() {
		let result = parse_and_validate("MyApp.ViewMyModel");
		assert!(result.is_ok());
		let ast = result.unwrap();
		assert_eq!(ast.app_label, "MyApp");
		assert_eq!(ast.permission_codename, "ViewMyModel");
	}
}
