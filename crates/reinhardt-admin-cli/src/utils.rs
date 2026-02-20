//! Utility functions for the admin CLI.
//!
//! Provides rollback helpers used by the formatting commands to restore
//! files when an error occurs mid-operation.

use std::collections::HashMap;
use std::path::PathBuf;

/// Restore a set of files to their original contents, logging any errors.
///
/// Only files present in `original_contents` are restored. All failures
/// are collected and returned together so that a single write error does
/// not prevent the remaining files from being rolled back.
///
/// # Returns
///
/// A list of `(path, error)` pairs for any writes that failed.
pub(crate) fn rollback_files(
	modified_files: &[PathBuf],
	original_contents: &HashMap<PathBuf, String>,
) -> Vec<(PathBuf, std::io::Error)> {
	let mut errors = Vec::new();
	for file_path in modified_files {
		if let Some(original) = original_contents.get(file_path)
			&& let Err(e) = std::fs::write(file_path, original)
		{
			eprintln!("Warning: failed to rollback {}: {}", file_path.display(), e);
			errors.push((file_path.clone(), e));
		}
	}
	errors
}

/// Log rollback errors in a user-visible way.
///
/// If `errors` is non-empty, prints a summary of files that could not
/// be rolled back.
pub(crate) fn report_rollback_errors(errors: &[(PathBuf, std::io::Error)]) {
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

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

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
}
