//! Utility functions for the admin CLI.
//!
//! Provides input validation, display formatting, boolean coercion,
//! signal handling, and error-aware rollback helpers used across
//! the admin CLI commands.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Input validation (#629)
// ---------------------------------------------------------------------------

/// Minimum allowed username length.
const MIN_USERNAME_LENGTH: usize = 3;

/// Maximum allowed username length.
const MAX_USERNAME_LENGTH: usize = 150;

/// Minimum allowed password length.
const MIN_PASSWORD_LENGTH: usize = 8;

/// Validate a username for the `createsuperuser` command.
///
/// A valid username must:
/// - Be between 3 and 150 characters long.
/// - Contain only ASCII alphanumeric characters, underscores, hyphens, or dots.
/// - Start with an alphanumeric character.
///
/// # Errors
///
/// Returns a human-readable error description on invalid input.
pub fn validate_username(username: &str) -> Result<(), String> {
	if username.is_empty() {
		return Err("username must not be empty".into());
	}
	if username.len() < MIN_USERNAME_LENGTH {
		return Err(format!(
			"username must be at least {} characters",
			MIN_USERNAME_LENGTH
		));
	}
	if username.len() > MAX_USERNAME_LENGTH {
		return Err(format!(
			"username must be at most {} characters",
			MAX_USERNAME_LENGTH
		));
	}
	if !username
		.chars()
		.next()
		.is_some_and(|c| c.is_ascii_alphanumeric())
	{
		return Err("username must start with an alphanumeric character".into());
	}
	if !username
		.chars()
		.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
	{
		return Err(
			"username must only contain alphanumeric characters, underscores, hyphens, or dots"
				.into(),
		);
	}
	Ok(())
}

/// Validate an email address for the `createsuperuser` command.
///
/// Checks for the presence of exactly one `@` symbol, a non-empty local
/// part, and a domain that contains at least one `.`.
///
/// # Errors
///
/// Returns a human-readable error description on invalid input.
pub fn validate_email(email: &str) -> Result<(), String> {
	if email.is_empty() {
		return Err("email must not be empty".into());
	}
	let parts: Vec<&str> = email.splitn(2, '@').collect();
	if parts.len() != 2 {
		return Err("email must contain exactly one '@' symbol".into());
	}
	let local = parts[0];
	let domain = parts[1];
	if local.is_empty() {
		return Err("email local part must not be empty".into());
	}
	if domain.is_empty() {
		return Err("email domain must not be empty".into());
	}
	if !domain.contains('.') {
		return Err("email domain must contain at least one '.'".into());
	}
	Ok(())
}

/// Validate a password for the `createsuperuser` command.
///
/// A valid password must be at least 8 characters long.
///
/// # Errors
///
/// Returns a human-readable error description on invalid input.
pub fn validate_password(password: &str) -> Result<(), String> {
	if password.len() < MIN_PASSWORD_LENGTH {
		return Err(format!(
			"password must be at least {} characters",
			MIN_PASSWORD_LENGTH
		));
	}
	Ok(())
}

// ---------------------------------------------------------------------------
// Display formatting (#666)
// ---------------------------------------------------------------------------

/// Default maximum display length before truncation.
pub const DEFAULT_DISPLAY_MAX_LENGTH: usize = 50;

/// Truncation indicator appended to long strings.
const TRUNCATION_MARKER: &str = "...";

/// Truncate a string for display, appending a marker when truncated.
///
/// If `value` is longer than `max_length`, it is truncated and
/// `"..."` is appended. The returned string (including the marker)
/// will never exceed `max_length + marker.len()`.
///
/// # Examples
///
/// ```rust,ignore
/// assert_eq!(truncate_for_display("short", 50), "short");
/// assert_eq!(truncate_for_display("a]".repeat(30).as_str(), 10), "aaaaaaaaaa...");
/// ```
pub fn truncate_for_display(value: &str, max_length: usize) -> String {
	if value.len() <= max_length {
		value.to_string()
	} else {
		let truncated: String = value.chars().take(max_length).collect();
		format!("{}{}", truncated, TRUNCATION_MARKER)
	}
}

// ---------------------------------------------------------------------------
// Boolean coercion (#668)
// ---------------------------------------------------------------------------

/// Parse a string into a boolean using a strict set of recognised values.
///
/// Accepted values (case-insensitive):
/// - `true`: `"true"`, `"yes"`, `"1"`
/// - `false`: `"false"`, `"no"`, `"0"`
///
/// # Errors
///
/// Returns an error for any value not in the recognised set.
pub fn parse_bool_strict(value: &str) -> Result<bool, String> {
	match value.to_lowercase().as_str() {
		"true" | "yes" | "1" => Ok(true),
		"false" | "no" | "0" => Ok(false),
		other => Err(format!(
			"invalid boolean value '{}': accepted values are true/false, yes/no, 1/0",
			other
		)),
	}
}

// ---------------------------------------------------------------------------
// Rollback helper (#608)
// ---------------------------------------------------------------------------

/// Restore a set of files to their original contents, logging any errors.
///
/// Only files present in `modified_files` are restored.  All failures
/// are collected and returned together so that a single write error does
/// not prevent the remaining files from being rolled back.
///
/// # Returns
///
/// A list of `(path, error)` pairs for any writes that failed.
pub fn rollback_files(
	modified_files: &[PathBuf],
	original_contents: &HashMap<PathBuf, String>,
) -> Vec<(PathBuf, std::io::Error)> {
	let mut errors = Vec::new();
	for file_path in modified_files {
		if let Some(original) = original_contents.get(file_path) {
			if let Err(e) = std::fs::write(file_path, original) {
				eprintln!("Warning: failed to rollback {}: {}", file_path.display(), e);
				errors.push((file_path.clone(), e));
			}
		}
	}
	errors
}

/// Log rollback errors in a user-visible way.
///
/// If `errors` is non-empty, prints a summary of files that could not
/// be rolled back.
pub fn report_rollback_errors(errors: &[(PathBuf, std::io::Error)]) {
	if errors.is_empty() {
		return;
	}
	eprintln!(
		"Warning: {} file(s) could not be rolled back:",
		errors.len()
	);
	for (path, err) in errors {
		eprintln!("  - {}: {}", path.display(), err);
	}
}

// ---------------------------------------------------------------------------
// Signal handling helper (#633)
// ---------------------------------------------------------------------------

/// Install a Ctrl+C (SIGINT) handler that sets a shared flag.
///
/// Returns a closure that can be polled to check whether a shutdown
/// has been requested.  This is intended for use in the interactive
/// shell loop so that it can exit cleanly instead of terminating
/// abruptly.
///
/// # Errors
///
/// Returns an error if the signal handler could not be registered.
pub fn install_ctrlc_handler() -> Result<std::sync::Arc<std::sync::atomic::AtomicBool>, String> {
	let shutdown_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
	let flag_clone = shutdown_flag.clone();
	ctrlc::set_handler(move || {
		flag_clone.store(true, std::sync::atomic::Ordering::SeqCst);
	})
	.map_err(|e| format!("failed to install Ctrl+C handler: {}", e))?;
	Ok(shutdown_flag)
}

// ---------------------------------------------------------------------------
// Path utilities (shared)
// ---------------------------------------------------------------------------

/// Mask a path for safe display in error messages.
///
/// Only shows the file name, preventing full path disclosure.
pub fn mask_path(path: &Path) -> String {
	path.file_name()
		.map(|name| format!("<...>/{}", name.to_string_lossy()))
		.unwrap_or_else(|| "<file>".to_string())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	// ---- validate_username -------------------------------------------------

	#[rstest]
	#[case("alice", true)]
	#[case("bob_jones", true)]
	#[case("user-123", true)]
	#[case("user.name", true)]
	#[case("A1", false)] // too short
	#[case("", false)] // empty
	#[case("_start", false)] // starts with underscore
	#[case("-start", false)] // starts with hyphen
	#[case(".start", false)] // starts with dot
	#[case("has space", false)] // contains space
	fn test_validate_username(#[case] input: &str, #[case] should_pass: bool) {
		// Arrange - inputs provided by rstest

		// Act
		let result = validate_username(input);

		// Assert
		assert_eq!(result.is_ok(), should_pass, "input: {:?}", input);
	}

	#[rstest]
	fn test_validate_username_max_length() {
		// Arrange
		let long_name = "a".repeat(MAX_USERNAME_LENGTH + 1);

		// Act
		let result = validate_username(&long_name);

		// Assert
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("at most"));
	}

	#[rstest]
	fn test_validate_username_exact_min_length() {
		// Arrange
		let name = "a".repeat(MIN_USERNAME_LENGTH);

		// Act
		let result = validate_username(&name);

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_username_exact_max_length() {
		// Arrange
		let name = "a".repeat(MAX_USERNAME_LENGTH);

		// Act
		let result = validate_username(&name);

		// Assert
		assert!(result.is_ok());
	}

	// ---- validate_email ----------------------------------------------------

	#[rstest]
	#[case("user@example.com", true)]
	#[case("a@b.c", true)]
	#[case("user@domain", false)] // no dot in domain
	#[case("@example.com", false)] // empty local
	#[case("user@", false)] // empty domain
	#[case("nodomain", false)] // no @
	#[case("", false)] // empty
	fn test_validate_email(#[case] input: &str, #[case] should_pass: bool) {
		// Arrange - inputs provided by rstest

		// Act
		let result = validate_email(input);

		// Assert
		assert_eq!(result.is_ok(), should_pass, "input: {:?}", input);
	}

	// ---- validate_password -------------------------------------------------

	#[rstest]
	#[case("longpass1", true)]
	#[case("12345678", true)]
	#[case("short", false)]
	#[case("1234567", false)] // 7 chars
	#[case("", false)]
	fn test_validate_password(#[case] input: &str, #[case] should_pass: bool) {
		// Arrange - inputs provided by rstest

		// Act
		let result = validate_password(input);

		// Assert
		assert_eq!(result.is_ok(), should_pass, "input: {:?}", input);
	}

	// ---- truncate_for_display (#666) ---------------------------------------

	#[rstest]
	#[case("short", 50, "short")]
	#[case("exact", 5, "exact")]
	#[case("hello world", 5, "hello...")]
	#[case("abcdefghij", 3, "abc...")]
	#[case("", 10, "")]
	fn test_truncate_for_display(
		#[case] input: &str,
		#[case] max_len: usize,
		#[case] expected: &str,
	) {
		// Arrange - inputs provided by rstest

		// Act
		let result = truncate_for_display(input, max_len);

		// Assert
		assert_eq!(result, expected);
	}

	#[rstest]
	fn test_truncate_preserves_full_string_at_boundary() {
		// Arrange
		let input = "a".repeat(DEFAULT_DISPLAY_MAX_LENGTH);

		// Act
		let result = truncate_for_display(&input, DEFAULT_DISPLAY_MAX_LENGTH);

		// Assert
		assert_eq!(result, input);
		assert!(!result.contains("..."));
	}

	#[rstest]
	fn test_truncate_adds_marker_beyond_boundary() {
		// Arrange
		let input = "a".repeat(DEFAULT_DISPLAY_MAX_LENGTH + 1);

		// Act
		let result = truncate_for_display(&input, DEFAULT_DISPLAY_MAX_LENGTH);

		// Assert
		assert!(result.ends_with("..."));
		assert_eq!(
			result.len(),
			DEFAULT_DISPLAY_MAX_LENGTH + TRUNCATION_MARKER.len()
		);
	}

	// ---- parse_bool_strict (#668) ------------------------------------------

	#[rstest]
	#[case("true", Ok(true))]
	#[case("True", Ok(true))]
	#[case("TRUE", Ok(true))]
	#[case("yes", Ok(true))]
	#[case("Yes", Ok(true))]
	#[case("YES", Ok(true))]
	#[case("1", Ok(true))]
	#[case("false", Ok(false))]
	#[case("False", Ok(false))]
	#[case("FALSE", Ok(false))]
	#[case("no", Ok(false))]
	#[case("No", Ok(false))]
	#[case("NO", Ok(false))]
	#[case("0", Ok(false))]
	fn test_parse_bool_strict_valid(#[case] input: &str, #[case] expected: Result<bool, String>) {
		// Arrange - inputs provided by rstest

		// Act
		let result = parse_bool_strict(input);

		// Assert
		assert_eq!(result, expected);
	}

	#[rstest]
	#[case("")]
	#[case("y")]
	#[case("n")]
	#[case("on")]
	#[case("off")]
	#[case("maybe")]
	#[case("2")]
	#[case("tru")]
	#[case("fals")]
	fn test_parse_bool_strict_rejects_arbitrary(#[case] input: &str) {
		// Arrange - inputs provided by rstest

		// Act
		let result = parse_bool_strict(input);

		// Assert
		assert!(result.is_err(), "expected error for input: {:?}", input);
		let err = result.unwrap_err();
		assert!(
			err.contains("invalid boolean value"),
			"error message should describe the issue: {}",
			err
		);
	}

	// ---- rollback_files (#608) ---------------------------------------------

	#[rstest]
	fn test_rollback_files_restores_modified() {
		// Arrange
		let dir = tempfile::tempdir().expect("failed to create temp dir");
		let file1 = dir.path().join("file1.rs");
		let file2 = dir.path().join("file2.rs");

		std::fs::write(&file1, "original1").expect("write file1");
		std::fs::write(&file2, "original2").expect("write file2");

		let mut originals = HashMap::new();
		originals.insert(file1.clone(), "original1".to_string());
		originals.insert(file2.clone(), "original2".to_string());

		// Simulate modification
		std::fs::write(&file1, "modified1").expect("modify file1");

		// Act
		let errors = rollback_files(&[file1.clone()], &originals);

		// Assert
		assert!(errors.is_empty(), "rollback should succeed");
		assert_eq!(
			std::fs::read_to_string(&file1).unwrap(),
			"original1",
			"file1 should be restored"
		);
		assert_eq!(
			std::fs::read_to_string(&file2).unwrap(),
			"original2",
			"file2 should remain unchanged since it was not in modified list"
		);
	}

	#[rstest]
	fn test_rollback_files_skips_untracked() {
		// Arrange
		let originals = HashMap::new();
		let nonexistent = PathBuf::from("/tmp/reinhardt-test-nonexistent-file.rs");

		// Act
		let errors = rollback_files(&[nonexistent], &originals);

		// Assert
		assert!(errors.is_empty(), "should skip files not in originals map");
	}

	// ---- mask_path ---------------------------------------------------------

	#[rstest]
	#[case("/home/user/project/src/main.rs", "<...>/main.rs")]
	#[case("relative/path/file.rs", "<...>/file.rs")]
	fn test_mask_path(#[case] input: &str, #[case] expected: &str) {
		// Arrange
		let path = Path::new(input);

		// Act
		let result = mask_path(path);

		// Assert
		assert_eq!(result, expected);
	}
}
