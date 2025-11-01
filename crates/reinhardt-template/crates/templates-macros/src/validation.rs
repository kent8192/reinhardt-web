//! Template path validation logic for compile-time checks.

use std::path::Path;

/// Errors that can occur during template path validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateValidationError {
	/// Path contains parent directory reference (..)
	ContainsParentDirectory,
	/// Path contains backslash (only forward slashes allowed)
	ContainsBackslash,
	/// Path is absolute (must be relative)
	IsAbsolutePath,
	/// Path has no file extension
	NoFileExtension,
	/// Path has invalid file extension
	InvalidFileExtension { ext: String },
	/// Path is empty
	EmptyPath,
	/// Path contains null bytes
	ContainsNullByte,
	/// Path contains invalid characters
	InvalidCharacter { ch: char, position: usize },
}

/// Valid template file extensions
const VALID_EXTENSIONS: &[&str] = &[
	"html", "htm", "txt", "md", "xml", "json", "css", "js", "svg", "rst",
];

/// Validates template path syntax
pub fn validate_template_path(path: &str) -> Result<(), TemplateValidationError> {
	// Check for empty path
	if path.is_empty() {
		return Err(TemplateValidationError::EmptyPath);
	}

	// Check for null bytes
	if path.contains('\0') {
		return Err(TemplateValidationError::ContainsNullByte);
	}

	// Check for backslashes (only forward slashes allowed)
	if path.contains('\\') {
		return Err(TemplateValidationError::ContainsBackslash);
	}

	// Check if path is absolute (starts with /)
	if path.starts_with('/') {
		return Err(TemplateValidationError::IsAbsolutePath);
	}

	// Check for parent directory references (..)
	if path.contains("..") {
		return Err(TemplateValidationError::ContainsParentDirectory);
	}

	// Validate characters (allow alphanumeric, -, _, /, .)
	for (i, ch) in path.chars().enumerate() {
		match ch {
			'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '/' | '.' => {}
			_ => return Err(TemplateValidationError::InvalidCharacter { ch, position: i }),
		}
	}

	// Check file extension
	let path_obj = Path::new(path);
	let extension = path_obj
		.extension()
		.and_then(|ext| ext.to_str())
		.ok_or(TemplateValidationError::NoFileExtension)?;

	if !VALID_EXTENSIONS.contains(&extension) {
		return Err(TemplateValidationError::InvalidFileExtension {
			ext: extension.to_string(),
		});
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_valid_paths() {
		assert!(validate_template_path("emails/welcome.html").is_ok());
		assert!(validate_template_path("blog/post_detail.html").is_ok());
		assert!(validate_template_path("admin/user-list.html").is_ok());
		assert!(validate_template_path("docs/readme.md").is_ok());
		assert!(validate_template_path("config/settings.json").is_ok());
		assert!(validate_template_path("styles/main.css").is_ok());
		assert!(validate_template_path("base.html").is_ok());
	}

	#[test]
	fn test_valid_nested_paths() {
		assert!(validate_template_path("apps/blog/templates/post.html").is_ok());
		assert!(validate_template_path("frontend/components/header.html").is_ok());
	}

	#[test]
	fn test_parent_directory_rejected() {
		let result = validate_template_path("../etc/passwd");
		assert_eq!(
			result,
			Err(TemplateValidationError::ContainsParentDirectory)
		);

		let result = validate_template_path("templates/../config.html");
		assert_eq!(
			result,
			Err(TemplateValidationError::ContainsParentDirectory)
		);
	}

	#[test]
	fn test_backslash_rejected() {
		let result = validate_template_path("emails\\welcome.html");
		assert_eq!(result, Err(TemplateValidationError::ContainsBackslash));

		let result = validate_template_path("blog\\posts\\detail.html");
		assert_eq!(result, Err(TemplateValidationError::ContainsBackslash));
	}

	#[test]
	fn test_absolute_path_rejected() {
		let result = validate_template_path("/etc/passwd");
		assert_eq!(result, Err(TemplateValidationError::IsAbsolutePath));

		let result = validate_template_path("/templates/base.html");
		assert_eq!(result, Err(TemplateValidationError::IsAbsolutePath));
	}

	#[test]
	fn test_no_extension_rejected() {
		let result = validate_template_path("templates/base");
		assert_eq!(result, Err(TemplateValidationError::NoFileExtension));
	}

	#[test]
	fn test_invalid_extension_rejected() {
		let result = validate_template_path("script.py");
		assert!(matches!(
			result,
			Err(TemplateValidationError::InvalidFileExtension { .. })
		));

		let result = validate_template_path("binary.exe");
		assert!(matches!(
			result,
			Err(TemplateValidationError::InvalidFileExtension { .. })
		));
	}

	#[test]
	fn test_empty_path_rejected() {
		let result = validate_template_path("");
		assert_eq!(result, Err(TemplateValidationError::EmptyPath));
	}

	#[test]
	fn test_null_byte_rejected() {
		let result = validate_template_path("template\0.html");
		assert_eq!(result, Err(TemplateValidationError::ContainsNullByte));
	}

	#[test]
	fn test_invalid_characters_rejected() {
		let result = validate_template_path("template*.html");
		assert!(matches!(
			result,
			Err(TemplateValidationError::InvalidCharacter { .. })
		));

		let result = validate_template_path("template?.html");
		assert!(matches!(
			result,
			Err(TemplateValidationError::InvalidCharacter { .. })
		));

		let result = validate_template_path("template|file.html");
		assert!(matches!(
			result,
			Err(TemplateValidationError::InvalidCharacter { .. })
		));
	}

	#[test]
	fn test_all_valid_extensions() {
		for ext in VALID_EXTENSIONS {
			let path = format!("template.{}", ext);
			assert!(
				validate_template_path(&path).is_ok(),
				"Extension .{} should be valid",
				ext
			);
		}
	}

	#[test]
	fn test_hyphen_and_underscore_allowed() {
		assert!(validate_template_path("user-profile_detail.html").is_ok());
		assert!(validate_template_path("post_list-view.html").is_ok());
		assert!(validate_template_path("admin/user-list_view.html").is_ok());
	}
}
