//! Advanced tests for collectstatic command
//!
//! Tests edge cases, error handling, symlinks, and performance scenarios

use reinhardt_commands::collectstatic::{CollectStaticCommand, CollectStaticOptions};
use reinhardt_static::storage::StaticFilesConfig;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn create_test_config(source_dir: &Path, dest_dir: &Path) -> StaticFilesConfig {
	StaticFilesConfig {
		static_root: dest_dir.to_path_buf(),
		static_url: "/static/".to_string(),
		staticfiles_dirs: vec![source_dir.to_path_buf()],
		media_url: None,
	}
}

#[test]
#[cfg(unix)]
fn test_symlink_collection() {
	use std::os::unix::fs::symlink;

	let source_dir = TempDir::new().unwrap();
	let dest_dir = TempDir::new().unwrap();

	// Create a file and a symlink pointing to it
	fs::write(source_dir.path().join("original.txt"), b"content").unwrap();

	let config = create_test_config(source_dir.path(), dest_dir.path());
	let options = CollectStaticOptions {
		link: true,
		interactive: false,
		verbosity: 0,
		..Default::default()
	};

	let mut command = CollectStaticCommand::new(config, options);
	let stats = command.execute().unwrap();

	assert_eq!(stats.copied, 1);

	// Check that symlink was created
	let dest_file = dest_dir.path().join("original.txt");
	assert!(dest_file.exists());
	assert!(dest_file.symlink_metadata().unwrap().is_symlink());
}

#[test]
fn test_multiple_source_directories() {
	let source_dir1 = TempDir::new().unwrap();
	let source_dir2 = TempDir::new().unwrap();
	let dest_dir = TempDir::new().unwrap();

	// Create files in different source directories
	fs::write(source_dir1.path().join("file1.css"), b"content1").unwrap();
	fs::write(source_dir2.path().join("file2.js"), b"content2").unwrap();

	let config = StaticFilesConfig {
		static_root: dest_dir.path().to_path_buf(),
		static_url: "/static/".to_string(),
		staticfiles_dirs: vec![
			source_dir1.path().to_path_buf(),
			source_dir2.path().to_path_buf(),
		],
		media_url: None,
	};

	let options = CollectStaticOptions {
		interactive: false,
		verbosity: 0,
		..Default::default()
	};

	let mut command = CollectStaticCommand::new(config, options);
	let stats = command.execute().unwrap();

	assert_eq!(stats.copied, 2);
	assert!(dest_dir.path().join("file1.css").exists());
	assert!(dest_dir.path().join("file2.js").exists());
}

#[test]
fn test_file_overwrite_from_later_source() {
	let source_dir1 = TempDir::new().unwrap();
	let source_dir2 = TempDir::new().unwrap();
	let dest_dir = TempDir::new().unwrap();

	// Create same filename in both directories with different content
	fs::write(source_dir1.path().join("common.txt"), b"first").unwrap();
	fs::write(source_dir2.path().join("common.txt"), b"second").unwrap();

	let config = StaticFilesConfig {
		static_root: dest_dir.path().to_path_buf(),
		static_url: "/static/".to_string(),
		staticfiles_dirs: vec![
			source_dir1.path().to_path_buf(),
			source_dir2.path().to_path_buf(),
		],
		media_url: None,
	};

	let options = CollectStaticOptions {
		interactive: false,
		verbosity: 0,
		..Default::default()
	};

	let mut command = CollectStaticCommand::new(config, options);
	command.execute().unwrap();

	// The second source should overwrite the first
	let content = fs::read_to_string(dest_dir.path().join("common.txt")).unwrap();
	assert_eq!(content, "second");
}

#[test]
fn test_clear_removes_existing_files() {
	let source_dir = TempDir::new().unwrap();
	let dest_dir = TempDir::new().unwrap();

	// Create an existing file in destination
	fs::write(dest_dir.path().join("old_file.txt"), b"old").unwrap();

	// Create new file in source
	fs::write(source_dir.path().join("new_file.txt"), b"new").unwrap();

	let config = create_test_config(source_dir.path(), dest_dir.path());
	let options = CollectStaticOptions {
		clear: true,
		interactive: false,
		verbosity: 0,
		..Default::default()
	};

	let mut command = CollectStaticCommand::new(config, options);
	command.execute().unwrap();

	// Old file should be removed
	assert!(!dest_dir.path().join("old_file.txt").exists());
	// New file should be present
	assert!(dest_dir.path().join("new_file.txt").exists());
}

#[test]
fn test_empty_source_directory() {
	let source_dir = TempDir::new().unwrap();
	let dest_dir = TempDir::new().unwrap();

	let config = create_test_config(source_dir.path(), dest_dir.path());
	let options = CollectStaticOptions {
		interactive: false,
		verbosity: 0,
		..Default::default()
	};

	let mut command = CollectStaticCommand::new(config, options);
	let stats = command.execute().unwrap();

	assert_eq!(stats.copied, 0);
	assert_eq!(stats.unmodified, 0);
}

#[test]
fn test_nonexistent_source_directory() {
	let dest_dir = TempDir::new().unwrap();
	let nonexistent = PathBuf::from("/nonexistent/path/that/does/not/exist");

	let config = StaticFilesConfig {
		static_root: dest_dir.path().to_path_buf(),
		static_url: "/static/".to_string(),
		staticfiles_dirs: vec![nonexistent],
		media_url: None,
	};

	let options = CollectStaticOptions {
		interactive: false,
		verbosity: 0,
		..Default::default()
	};

	let mut command = CollectStaticCommand::new(config, options);
	// Should not panic, just skip the directory
	let stats = command.execute().unwrap();

	assert_eq!(stats.copied, 0);
}

#[test]
fn test_deeply_nested_structure() {
	let source_dir = TempDir::new().unwrap();
	let dest_dir = TempDir::new().unwrap();

	// Create deeply nested directory structure
	let deep_path = source_dir
		.path()
		.join("level1")
		.join("level2")
		.join("level3")
		.join("level4");
	fs::create_dir_all(&deep_path).unwrap();
	fs::write(deep_path.join("deep_file.txt"), b"deep content").unwrap();

	let config = create_test_config(source_dir.path(), dest_dir.path());
	let options = CollectStaticOptions {
		interactive: false,
		verbosity: 0,
		..Default::default()
	};

	let mut command = CollectStaticCommand::new(config, options);
	let stats = command.execute().unwrap();

	assert_eq!(stats.copied, 1);
	assert!(
		dest_dir
			.path()
			.join("level1/level2/level3/level4/deep_file.txt")
			.exists()
	);
}

#[test]
fn test_special_characters_in_filenames() {
	let source_dir = TempDir::new().unwrap();
	let dest_dir = TempDir::new().unwrap();

	// Create files with special characters
	let special_names = vec![
		"file with spaces.txt",
		"file-with-dashes.txt",
		"file_with_underscores.txt",
		"file.multiple.dots.txt",
	];

	for name in &special_names {
		fs::write(source_dir.path().join(name), b"content").unwrap();
	}

	let config = create_test_config(source_dir.path(), dest_dir.path());
	let options = CollectStaticOptions {
		interactive: false,
		verbosity: 0,
		..Default::default()
	};

	let mut command = CollectStaticCommand::new(config, options);
	let stats = command.execute().unwrap();

	assert_eq!(stats.copied, special_names.len());
	for name in &special_names {
		assert!(dest_dir.path().join(name).exists());
	}
}

#[test]
fn test_large_number_of_files() {
	let source_dir = TempDir::new().unwrap();
	let dest_dir = TempDir::new().unwrap();

	// Create 100 files
	let file_count = 100;
	for i in 0..file_count {
		fs::write(
			source_dir.path().join(format!("file_{:03}.txt", i)),
			format!("content {}", i).as_bytes(),
		)
		.unwrap();
	}

	let config = create_test_config(source_dir.path(), dest_dir.path());
	let options = CollectStaticOptions {
		interactive: false,
		verbosity: 0,
		..Default::default()
	};

	let mut command = CollectStaticCommand::new(config, options);
	let stats = command.execute().unwrap();

	assert_eq!(stats.copied, file_count);
}

#[test]
fn test_binary_file_collection() {
	let source_dir = TempDir::new().unwrap();
	let dest_dir = TempDir::new().unwrap();

	// Create a binary file with non-UTF8 bytes
	let binary_data: Vec<u8> = (0..=255).collect();
	fs::write(source_dir.path().join("binary.dat"), &binary_data).unwrap();

	let config = create_test_config(source_dir.path(), dest_dir.path());
	let options = CollectStaticOptions {
		interactive: false,
		verbosity: 0,
		..Default::default()
	};

	let mut command = CollectStaticCommand::new(config, options);
	let stats = command.execute().unwrap();

	assert_eq!(stats.copied, 1);

	let copied_data = fs::read(dest_dir.path().join("binary.dat")).unwrap();
	assert_eq!(copied_data, binary_data);
}

#[test]
fn test_empty_file_collection() {
	let source_dir = TempDir::new().unwrap();
	let dest_dir = TempDir::new().unwrap();

	// Create empty files
	fs::write(source_dir.path().join("empty1.txt"), b"").unwrap();
	fs::write(source_dir.path().join("empty2.css"), b"").unwrap();

	let config = create_test_config(source_dir.path(), dest_dir.path());
	let options = CollectStaticOptions {
		interactive: false,
		verbosity: 0,
		..Default::default()
	};

	let mut command = CollectStaticCommand::new(config, options);
	let stats = command.execute().unwrap();

	assert_eq!(stats.copied, 2);
	assert_eq!(
		fs::read(dest_dir.path().join("empty1.txt")).unwrap().len(),
		0
	);
	assert_eq!(
		fs::read(dest_dir.path().join("empty2.css")).unwrap().len(),
		0
	);
}

#[test]
fn test_ignore_hidden_files_by_default() {
	let source_dir = TempDir::new().unwrap();
	let dest_dir = TempDir::new().unwrap();

	// Create hidden files (starting with dot)
	fs::write(source_dir.path().join(".gitignore"), b"ignore").unwrap();
	fs::write(source_dir.path().join(".DS_Store"), b"store").unwrap();

	// Create normal file
	fs::write(source_dir.path().join("app.js"), b"app").unwrap();

	let config = create_test_config(source_dir.path(), dest_dir.path());
	let options = CollectStaticOptions {
		interactive: false,
		verbosity: 0,
		..Default::default()
	};

	let mut command = CollectStaticCommand::new(config, options);
	let stats = command.execute().unwrap();

	// Only app.js should be copied, hidden files should be ignored
	assert_eq!(stats.copied, 1);
	assert!(dest_dir.path().join("app.js").exists());
	assert!(!dest_dir.path().join(".gitignore").exists());
	assert!(!dest_dir.path().join(".DS_Store").exists());
}

#[test]
fn test_wildcard_ignore_pattern() {
	let source_dir = TempDir::new().unwrap();
	let dest_dir = TempDir::new().unwrap();

	// Create files with various extensions
	fs::write(source_dir.path().join("keep.txt"), b"keep").unwrap();
	fs::write(source_dir.path().join("remove.bak"), b"remove").unwrap();
	fs::write(source_dir.path().join("also_remove.bak"), b"remove").unwrap();

	let config = create_test_config(source_dir.path(), dest_dir.path());
	let options = CollectStaticOptions {
		ignore_patterns: vec!["*.bak".to_string()],
		interactive: false,
		verbosity: 0,
		..Default::default()
	};

	let mut command = CollectStaticCommand::new(config, options);
	let stats = command.execute().unwrap();

	assert_eq!(stats.copied, 1);
	assert!(dest_dir.path().join("keep.txt").exists());
	assert!(!dest_dir.path().join("remove.bak").exists());
	assert!(!dest_dir.path().join("also_remove.bak").exists());
}

#[test]
fn test_modified_file_is_recopied() {
	use std::thread;
	use std::time::Duration;

	let source_dir = TempDir::new().unwrap();
	let dest_dir = TempDir::new().unwrap();

	// Create initial file
	fs::write(source_dir.path().join("file.txt"), b"v1").unwrap();

	let config = create_test_config(source_dir.path(), dest_dir.path());
	let options = CollectStaticOptions {
		interactive: false,
		verbosity: 0,
		..Default::default()
	};

	// First collection
	let mut command = CollectStaticCommand::new(config.clone(), options.clone());
	let stats1 = command.execute().unwrap();
	assert_eq!(stats1.copied, 1);

	// Wait a bit to ensure modification time changes
	thread::sleep(Duration::from_millis(10));

	// Modify the file
	fs::write(source_dir.path().join("file.txt"), b"v2").unwrap();

	// Second collection
	let mut command = CollectStaticCommand::new(config, options);
	let stats2 = command.execute().unwrap();

	// File should be recopied
	assert_eq!(stats2.copied, 1);
	assert_eq!(stats2.unmodified, 0);

	let content = fs::read_to_string(dest_dir.path().join("file.txt")).unwrap();
	assert_eq!(content, "v2");
}
