//! Tests for the codemod's directory walker.

use rstest::rstest;
use std::fs;
use tempfile::tempdir;

#[rstest]
fn walker_finds_rs_files_recursively_and_skips_target() {
	// Arrange
	let dir = tempdir().unwrap();
	fs::write(dir.path().join("a.rs"), "fn a() {}").unwrap();
	fs::create_dir_all(dir.path().join("nested")).unwrap();
	fs::write(dir.path().join("nested/b.rs"), "fn b() {}").unwrap();
	fs::create_dir_all(dir.path().join("target")).unwrap();
	fs::write(dir.path().join("target/c.rs"), "fn c() {}").unwrap();

	// Act
	let mut files = reinhardt_admin_cli::migrate_v2::walker::find_rs_files(dir.path()).unwrap();
	files.sort();

	// Assert
	assert_eq!(files.len(), 2);
	assert!(files[0].ends_with("a.rs"));
	assert!(files[1].ends_with("nested/b.rs"));
}
