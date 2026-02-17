//! File collection utilities for the page! macro formatter.
//!
//! This module provides utilities for collecting Rust source files from directories.

use std::path::PathBuf;

use walkdir::WalkDir;

/// Collect all Rust files from a path.
///
/// If the path is a file with `.rs` extension, returns a vector with just that file.
/// If the path is a directory, recursively collects all `.rs` files, excluding
/// `target/` and `.git/` directories.
///
/// # Errors
///
/// Returns an error if the path does not exist.
pub(crate) fn collect_rust_files(path: &PathBuf) -> Result<Vec<PathBuf>, String> {
	let mut files = Vec::new();

	if path.is_file() {
		if path.extension().is_some_and(|ext| ext == "rs") {
			files.push(path.clone());
		}
	} else if path.is_dir() {
		for entry in WalkDir::new(path)
			.follow_links(true)
			.into_iter()
			.filter_map(|e| e.ok())
		{
			let entry_path = entry.path();
			if entry_path.is_file()
				&& entry_path.extension().is_some_and(|ext| ext == "rs")
				&& !entry_path
					.components()
					.any(|c| c.as_os_str() == "target" || c.as_os_str() == ".git")
			{
				files.push(entry_path.to_path_buf());
			}
		}
	} else {
		return Err(format!("Path does not exist: {}", path.display()));
	}

	Ok(files)
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_collect_rust_files_nonexistent() {
		let path = PathBuf::from("/nonexistent/path/that/does/not/exist");
		let result = collect_rust_files(&path);

		assert!(result.is_err());
		assert!(result.unwrap_err().contains("does not exist"));
	}

	#[rstest]
	fn test_collect_rust_files_single_file() {
		use std::io::Write;

		let temp_dir = std::env::temp_dir();
		let test_file = temp_dir.join("test_collect_rust.rs");

		// Create a temporary Rust file
		{
			let mut file = std::fs::File::create(&test_file).unwrap();
			writeln!(file, "fn main() {{}}").unwrap();
		}

		let result = collect_rust_files(&test_file).unwrap();
		assert_eq!(result.len(), 1);
		assert_eq!(result[0], test_file);

		// Cleanup
		std::fs::remove_file(&test_file).ok();
	}

	#[rstest]
	fn test_collect_rust_files_non_rust() {
		use std::io::Write;

		let temp_dir = std::env::temp_dir();
		let test_file = temp_dir.join("test_collect.txt");

		// Create a temporary non-Rust file
		{
			let mut file = std::fs::File::create(&test_file).unwrap();
			writeln!(file, "not rust").unwrap();
		}

		let result = collect_rust_files(&test_file).unwrap();
		assert!(result.is_empty());

		// Cleanup
		std::fs::remove_file(&test_file).ok();
	}
}
