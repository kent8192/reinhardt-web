//! File collection utilities for the Reinhardt DSL formatter.
//!
//! This module provides utilities for collecting Rust source files from directories.
//!
//! # Security
//!
//! Symlink following is disabled by default to prevent arbitrary file read/write
//! attacks where a malicious symlink in the project directory could point to
//! sensitive files outside the project tree.

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// Collect all Rust files from a path.
///
/// If the path is a file with `.rs` extension, returns a vector with just that file.
/// If the path is a directory, recursively collects all `.rs` files, excluding
/// `target/` and `.git/` directories.
///
/// # Security
///
/// - Symlinks are not followed to prevent symlink-based attacks
/// - Resolved paths are validated to stay within the base directory
///
/// # Errors
///
/// Returns an error if the path does not exist.
pub(crate) fn collect_rust_files(path: &PathBuf) -> Result<Vec<PathBuf>, String> {
	let mut files = Vec::new();

	if path.is_file() {
		// Verify the file is not a symlink before processing
		if path.is_symlink() {
			return Err(format!(
				"Refusing to process symlink: {}",
				sanitize_path_for_error(path)
			));
		}
		if path.extension().is_some_and(|ext| ext == "rs") {
			files.push(path.clone());
		}
	} else if path.is_dir() {
		// Canonicalize the base directory to use as boundary for path validation
		let base_dir = path
			.canonicalize()
			.map_err(|e| format!("Failed to resolve base path: {}", e.kind()))?;

		for entry in WalkDir::new(path)
			.follow_links(false) // Do not follow symlinks to prevent symlink attacks
			.into_iter()
			.filter_map(|result| match result {
				Ok(entry) => Some(entry),
				Err(e) => {
					eprintln!(
						"Warning: skipping directory entry due to error: {}",
						e
					);
					None
				}
			}) {
			let entry_path = entry.path();

			// Skip symlinks entirely
			if entry.path_is_symlink() {
				continue;
			}

			if entry_path.is_file()
				&& entry_path.extension().is_some_and(|ext| ext == "rs")
				&& !entry_path
					.components()
					.any(|c| c.as_os_str() == "target" || c.as_os_str() == ".git")
			{
				// Validate that the resolved path stays within the base directory
				if is_within_directory(entry_path, &base_dir) {
					files.push(entry_path.to_path_buf());
				}
			}
		}
	} else {
		return Err(format!(
			"Path does not exist: {}",
			sanitize_path_for_error(path)
		));
	}

	Ok(files)
}

/// Find separate (nested) cargo workspace roots beneath `project_root`.
///
/// A directory qualifies as a nested workspace root when it contains a
/// `Cargo.toml` that declares its own `[workspace]` table and is not
/// `project_root` itself, for example `examples/` and the trybuild fixture
/// crates.
///
/// The `fmt-all` command runs `cargo fmt --all` only over the root workspace,
/// so these independent workspaces never receive the rustfmt pass. Formatting
/// only their DSL macros (and skipping rustfmt) diverges from the output of
/// their own `fmt`/`cargo fmt` tasks (such as `fmt-check-examples`), which
/// yields contradictory results between `fmt-all` and per-workspace formatting.
/// Callers rely on this list to exclude those files so that each sub-workspace
/// is formatted solely by its dedicated task.
pub(crate) fn nested_workspace_roots(project_root: &Path) -> Vec<PathBuf> {
	let mut roots = Vec::new();

	for entry in WalkDir::new(project_root)
		.follow_links(false)
		.into_iter()
		.filter_map(Result::ok)
	{
		// Symlinked entries are ignored, mirroring `collect_rust_files`.
		if entry.path_is_symlink() {
			continue;
		}

		let path = entry.path();
		let is_manifest =
			path.is_file() && path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml");
		if !is_manifest {
			continue;
		}

		// Build output and VCS metadata never hold workspace roots of interest.
		if path
			.components()
			.any(|component| component.as_os_str() == "target" || component.as_os_str() == ".git")
		{
			continue;
		}

		let Some(dir) = path.parent() else {
			continue;
		};

		// The root workspace itself is not a nested workspace.
		if dir == project_root {
			continue;
		}

		if manifest_declares_workspace(path) {
			roots.push(dir.to_path_buf());
		}
	}

	roots
}

/// Returns `true` when the given `Cargo.toml` declares a `[workspace]` table.
///
/// Only workspace root manifests contain a `[workspace]` (or `[workspace.*]`)
/// table; member crates reference the workspace through `workspace = true`
/// keys without declaring the table. This distinction reliably separates a
/// nested workspace root from an ordinary member crate.
fn manifest_declares_workspace(manifest: &Path) -> bool {
	let Ok(content) = std::fs::read_to_string(manifest) else {
		return false;
	};

	content.lines().any(|line| {
		let trimmed = line.trim_start();
		trimmed == "[workspace]" || trimmed.starts_with("[workspace.")
	})
}

/// Sanitize a path for error messages to prevent information leakage.
///
/// Returns only the filename to avoid exposing full file system paths.
fn sanitize_path_for_error(path: &Path) -> String {
	path.file_name()
		.map(|name| format!("<...>/{}", name.to_string_lossy()))
		.unwrap_or_else(|| "<path>".to_string())
}

/// Check if a path is within the given base directory.
///
/// Uses canonicalization to resolve any `.` or `..` components before checking
/// the prefix. Returns `false` if canonicalization fails (e.g., the path doesn't exist).
fn is_within_directory(path: &Path, base_dir: &Path) -> bool {
	path.canonicalize()
		.map(|resolved| resolved.starts_with(base_dir))
		.unwrap_or(false)
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn collect_rust_files_rejects_nonexistent_path() {
		// Arrange
		let path = PathBuf::from("/nonexistent/path/that/does/not/exist");

		// Act
		let result = collect_rust_files(&path);

		// Assert
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("does not exist"));
	}

	#[rstest]
	fn collect_rust_files_accepts_single_file() {
		// Arrange
		use std::io::Write;
		let temp_dir = std::env::temp_dir();
		let test_file = temp_dir.join("test_collect_rust.rs");
		{
			let mut file = std::fs::File::create(&test_file).unwrap();
			writeln!(file, "fn main() {{}}").unwrap();
		}

		// Act
		let result = collect_rust_files(&test_file).unwrap();

		// Assert
		assert_eq!(result.len(), 1);
		assert_eq!(result[0], test_file);

		// Cleanup
		std::fs::remove_file(&test_file).ok();
	}

	#[rstest]
	fn collect_rust_files_skips_non_rust_files() {
		// Arrange
		use std::io::Write;
		let temp_dir = std::env::temp_dir();
		let test_file = temp_dir.join("test_collect.txt");
		{
			let mut file = std::fs::File::create(&test_file).unwrap();
			writeln!(file, "not rust").unwrap();
		}

		// Act
		let result = collect_rust_files(&test_file).unwrap();

		// Assert
		assert!(result.is_empty());

		// Cleanup
		std::fs::remove_file(&test_file).ok();
	}

	#[cfg(unix)]
	#[rstest]
	fn collect_rust_files_skips_symlinks() {
		// Arrange
		use std::io::Write;
		let temp_dir = tempfile::TempDir::new().unwrap();
		let real_file = temp_dir.path().join("real.rs");
		{
			let mut file = std::fs::File::create(&real_file).unwrap();
			writeln!(file, "fn main() {{}}").unwrap();
		}
		let symlink_file = temp_dir.path().join("symlink.rs");
		std::os::unix::fs::symlink(&real_file, &symlink_file).unwrap();

		// Act
		let result = collect_rust_files(&temp_dir.path().to_path_buf()).unwrap();

		// Assert - should only contain the real file, not the symlink
		assert_eq!(result.len(), 1);
		let real_canonical = real_file.canonicalize().unwrap();
		let result_canonical = result[0].canonicalize().unwrap();
		assert_eq!(result_canonical, real_canonical);
	}

	#[cfg(unix)]
	#[rstest]
	fn collect_rust_files_rejects_symlink_as_single_file() {
		// Arrange
		use std::io::Write;
		let temp_dir = tempfile::TempDir::new().unwrap();
		let real_file = temp_dir.path().join("real.rs");
		{
			let mut file = std::fs::File::create(&real_file).unwrap();
			writeln!(file, "fn main() {{}}").unwrap();
		}
		let symlink_file = temp_dir.path().join("symlink.rs");
		std::os::unix::fs::symlink(&real_file, &symlink_file).unwrap();

		// Act
		let result = collect_rust_files(&symlink_file);

		// Assert
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("symlink"));
	}

	#[rstest]
	fn nested_workspace_roots_flags_only_separate_workspaces() {
		// Arrange
		use std::io::Write;
		let temp_dir = tempfile::TempDir::new().unwrap();
		let root = temp_dir.path();

		// Root workspace manifest (must never be reported as nested).
		{
			let mut file = std::fs::File::create(root.join("Cargo.toml")).unwrap();
			writeln!(file, "[workspace]\nmembers = [\"member-crate\"]").unwrap();
		}

		// An ordinary member crate (no `[workspace]` table) must not be flagged.
		let member = root.join("member-crate");
		std::fs::create_dir_all(&member).unwrap();
		{
			let mut file = std::fs::File::create(member.join("Cargo.toml")).unwrap();
			writeln!(file, "[package]\nname = \"member-crate\"").unwrap();
		}

		// A separate nested workspace (examples-like) must be flagged.
		let examples = root.join("examples");
		std::fs::create_dir_all(&examples).unwrap();
		{
			let mut file = std::fs::File::create(examples.join("Cargo.toml")).unwrap();
			writeln!(file, "[workspace]\nmembers = [\"demo\"]").unwrap();
		}

		// A workspace root declared only via `[workspace.dependencies]`.
		let tooling = root.join("tooling");
		std::fs::create_dir_all(&tooling).unwrap();
		{
			let mut file = std::fs::File::create(tooling.join("Cargo.toml")).unwrap();
			writeln!(file, "[workspace.dependencies]\nserde = \"1\"").unwrap();
		}

		// Act
		let mut roots = nested_workspace_roots(root);
		roots.sort();

		// Assert
		let mut expected = vec![examples, tooling];
		expected.sort();
		assert_eq!(roots, expected);
	}
}
