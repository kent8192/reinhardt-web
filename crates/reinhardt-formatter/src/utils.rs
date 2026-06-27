//! Utility functions for the admin CLI.
//!
//! Provides rollback helpers used by the formatting commands to restore
//! files when an error occurs mid-operation.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

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

/// Write content to a file atomically by writing to a temporary file first, then renaming.
///
/// This prevents data corruption if the write is interrupted (e.g., by a signal or power
/// failure). The rename operation is atomic on the same filesystem.
///
/// Original file permissions are preserved after the write.
pub(crate) fn atomic_write(path: &Path, content: &str) -> std::io::Result<()> {
	// Preserve original permissions before overwrite (if the file exists)
	let original_perms = std::fs::metadata(path).ok().map(|m| m.permissions());
	let parent = path.parent().unwrap_or_else(|| Path::new("."));
	let mut tmp_file = None;
	let mut tmp_path = PathBuf::new();

	for _ in 0..100 {
		tmp_path = unique_sibling_path(path, "tmp");
		match std::fs::OpenOptions::new()
			.write(true)
			.create_new(true)
			.open(&tmp_path)
		{
			Ok(file) => {
				tmp_file = Some(file);
				break;
			}
			Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
			Err(e) => return Err(e),
		}
	}

	let mut tmp_file = tmp_file.ok_or_else(|| {
		std::io::Error::new(
			std::io::ErrorKind::AlreadyExists,
			format!(
				"could not create a unique temporary file in {}",
				parent.display()
			),
		)
	})?;

	if let Some(perms) = original_perms.as_ref() {
		tmp_file.set_permissions(perms.clone())?;
	}

	if let Err(e) = tmp_file.write_all(content.as_bytes()) {
		let _ = std::fs::remove_file(&tmp_path);
		return Err(e);
	}
	drop(tmp_file);

	if let Err(e) = std::fs::rename(&tmp_path, path) {
		let _ = std::fs::remove_file(&tmp_path);
		return Err(e);
	}

	Ok(())
}

/// Build a unique temporary path next to a source path.
///
/// The caller must still create the returned path with `create_new(true)` so
/// a pre-created file or symlink cannot be followed.
pub(crate) fn unique_sibling_path(path: &Path, suffix: &str) -> PathBuf {
	let parent = path.parent().unwrap_or_else(|| Path::new("."));
	let file_name = path
		.file_name()
		.unwrap_or_else(|| std::ffi::OsStr::new("unknown"))
		.to_string_lossy();
	let timestamp = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|duration| duration.as_nanos())
		.unwrap_or(0);
	let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
	parent.join(format!(
		".{file_name}.{pid}.{timestamp}.{counter}.{suffix}",
		pid = std::process::id()
	))
}

/// RAII guard that cleans up a backup file if the operation is not committed.
///
/// Call `commit()` to indicate the operation succeeded and the backup should be kept.
/// If dropped without committing, the backup file is automatically deleted to prevent
/// orphaned backup files on failure.
pub(crate) struct BackupGuard {
	backup_path: PathBuf,
	committed: bool,
}

impl BackupGuard {
	/// Create a new backup guard. The backup file at `backup_path` must already exist.
	pub(crate) fn new(backup_path: PathBuf) -> Self {
		Self {
			backup_path,
			committed: false,
		}
	}

	/// Mark the backup as committed, preventing automatic cleanup on drop.
	pub(crate) fn commit(&mut self) {
		self.committed = true;
	}
}

impl Drop for BackupGuard {
	fn drop(&mut self) {
		if !self.committed {
			let _ = std::fs::remove_file(&self.backup_path);
		}
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
		let errors = rollback_files(std::slice::from_ref(&file1), &originals);

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

	#[cfg(unix)]
	#[rstest]
	fn test_atomic_write_does_not_follow_predictable_tmp_symlink() {
		// Arrange
		let dir = tempfile::tempdir().expect("failed to create temp dir");
		let source = dir.path().join("lib.rs");
		let victim = dir.path().join("victim.txt");
		let predictable_tmp = dir.path().join("lib.tmp");

		std::fs::write(&source, "original").expect("write source");
		std::fs::write(&victim, "victim").expect("write victim");
		std::os::unix::fs::symlink(&victim, &predictable_tmp).expect("create symlink");

		// Act
		atomic_write(&source, "formatted").expect("atomic write");

		// Assert
		assert_eq!(
			std::fs::read_to_string(&source).expect("read source"),
			"formatted",
			"source should receive formatted content"
		);
		assert_eq!(
			std::fs::read_to_string(&victim).expect("read victim"),
			"victim",
			"predictable sibling symlink target must not be overwritten"
		);
		assert!(
			std::fs::symlink_metadata(&predictable_tmp)
				.expect("stat predictable symlink")
				.file_type()
				.is_symlink(),
			"preexisting predictable symlink should not be renamed over the source"
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
