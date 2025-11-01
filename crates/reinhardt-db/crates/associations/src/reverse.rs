//! Automatic reverse relationship accessor generation
//!
//! This module provides utilities for automatically generating reverse relationship
//! accessor names when `related_name` is not explicitly provided.

/// Generate a reverse accessor name from a model name
///
/// Converts a model name to a plural form suitable for use as a reverse accessor.
/// This follows Django's convention of adding "_set" suffix for reverse relationships.
///
/// # Arguments
///
/// * `model_name` - The name of the model class
///
/// # Returns
///
/// A string representing the suggested reverse accessor name
///
/// # Examples
///
/// ```
/// use reinhardt_associations::generate_reverse_accessor;
///
/// assert_eq!(generate_reverse_accessor("Post"), "post_set");
/// assert_eq!(generate_reverse_accessor("Comment"), "comment_set");
/// assert_eq!(generate_reverse_accessor("UserProfile"), "user_profile_set");
/// ```
pub fn generate_reverse_accessor(model_name: &str) -> String {
	// Convert CamelCase to snake_case and add _set suffix
	let snake_case = to_snake_case(model_name);
	format!("{}_set", snake_case)
}

/// Generate a reverse accessor name for one-to-one relationships
///
/// For one-to-one relationships, the reverse accessor should be singular,
/// not plural with "_set" suffix.
///
/// # Arguments
///
/// * `model_name` - The name of the model class
///
/// # Returns
///
/// A string representing the suggested reverse accessor name
///
/// # Examples
///
/// ```
/// use reinhardt_associations::generate_reverse_accessor_singular;
///
/// assert_eq!(generate_reverse_accessor_singular("UserProfile"), "user_profile");
/// assert_eq!(generate_reverse_accessor_singular("Address"), "address");
/// ```
pub fn generate_reverse_accessor_singular(model_name: &str) -> String {
	to_snake_case(model_name)
}

/// Convert CamelCase to snake_case
///
/// # Arguments
///
/// * `s` - The CamelCase string to convert
///
/// # Returns
///
/// The snake_case version of the string
///
/// # Examples
///
/// ```
/// use reinhardt_associations::to_snake_case;
///
/// assert_eq!(to_snake_case("UserProfile"), "user_profile");
/// assert_eq!(to_snake_case("Post"), "post");
/// assert_eq!(to_snake_case("APIKey"), "api_key");
/// ```
pub fn to_snake_case(s: &str) -> String {
	let mut result = String::new();
	let chars: Vec<char> = s.chars().collect();

	for (i, &ch) in chars.iter().enumerate() {
		if ch.is_uppercase() {
			// Add underscore before uppercase letter if:
			// 1. Not at the beginning
			// 2. Previous character is lowercase OR next character is lowercase
			if i > 0 {
				let prev_is_lower = chars[i - 1].is_lowercase();
				let next_is_lower = i + 1 < chars.len() && chars[i + 1].is_lowercase();
				if prev_is_lower || next_is_lower {
					result.push('_');
				}
			}
			result.push(ch.to_ascii_lowercase());
		} else {
			result.push(ch);
		}
	}

	result
}

/// Trait for types that can have reverse relationships
///
/// This trait allows relationship types to automatically generate
/// reverse accessor names when not explicitly provided.
pub trait ReverseRelationship {
	/// Get the reverse accessor name, generating one if not explicitly set
	///
	/// # Arguments
	///
	/// * `model_name` - The name of the source model
	///
	/// # Returns
	///
	/// The reverse accessor name (either explicitly set or auto-generated)
	fn get_or_generate_reverse_name(&self, model_name: &str) -> String;

	/// Get the explicitly set reverse accessor name
	fn explicit_reverse_name(&self) -> Option<&str>;
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_generate_reverse_accessor() {
		assert_eq!(generate_reverse_accessor("Post"), "post_set");
		assert_eq!(generate_reverse_accessor("Comment"), "comment_set");
		assert_eq!(generate_reverse_accessor("UserProfile"), "user_profile_set");
		assert_eq!(generate_reverse_accessor("BlogPost"), "blog_post_set");
	}

	#[test]
	fn test_generate_reverse_accessor_singular() {
		assert_eq!(
			generate_reverse_accessor_singular("UserProfile"),
			"user_profile"
		);
		assert_eq!(generate_reverse_accessor_singular("Address"), "address");
		assert_eq!(generate_reverse_accessor_singular("Profile"), "profile");
	}

	#[test]
	fn test_to_snake_case() {
		assert_eq!(to_snake_case("UserProfile"), "user_profile");
		assert_eq!(to_snake_case("Post"), "post");
		assert_eq!(to_snake_case("BlogPost"), "blog_post");
		assert_eq!(to_snake_case("APIKey"), "api_key");
		assert_eq!(to_snake_case("HTTPRequest"), "http_request");
	}

	#[test]
	fn test_to_snake_case_single_char() {
		assert_eq!(to_snake_case("A"), "a");
		assert_eq!(to_snake_case("B"), "b");
	}

	#[test]
	fn test_to_snake_case_already_lowercase() {
		assert_eq!(to_snake_case("post"), "post");
		assert_eq!(to_snake_case("user"), "user");
	}

	#[test]
	fn test_to_snake_case_mixed() {
		assert_eq!(to_snake_case("XMLHttpRequest"), "xml_http_request");
		assert_eq!(to_snake_case("IOError"), "io_error");
	}

	#[test]
	fn test_to_snake_case_consecutive_uppercase() {
		assert_eq!(to_snake_case("HTTPSConnection"), "https_connection");
		assert_eq!(to_snake_case("URLPattern"), "url_pattern");
	}
}
