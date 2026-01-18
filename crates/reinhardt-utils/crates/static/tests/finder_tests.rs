use reinhardt_utils::r#static::storage::{StaticFilesConfig, StaticFilesFinder};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test helper to create test directories and files
fn setup_test_files() -> (TempDir, TempDir, TempDir) {
	let temp_dir1 = TempDir::new().unwrap();
	let temp_dir2 = TempDir::new().unwrap();
	let temp_dir3 = TempDir::new().unwrap();

	// Create test files in first directory
	let file1_path = temp_dir1.path().join("test").join("file.txt");
	fs::create_dir_all(file1_path.parent().unwrap()).unwrap();
	fs::write(&file1_path, b"Test content 1").unwrap();

	// Create test files in second directory
	let file2_path = temp_dir2.path().join("test").join("file.txt");
	fs::create_dir_all(file2_path.parent().unwrap()).unwrap();
	fs::write(&file2_path, b"Test content 2").unwrap();

	let file3_path = temp_dir2.path().join("other").join("file.txt");
	fs::create_dir_all(file3_path.parent().unwrap()).unwrap();
	fs::write(&file3_path, b"Test content 3").unwrap();

	// Create media file in third directory
	let media_file_path = temp_dir3.path().join("media-file.txt");
	fs::write(&media_file_path, b"Media content").unwrap();

	(temp_dir1, temp_dir2, temp_dir3)
}

#[test]
fn test_find_first() {
	let (temp_dir1, temp_dir2, _temp_dir3) = setup_test_files();

	let config = StaticFilesConfig {
		static_root: PathBuf::from("static"),
		static_url: "/static/".to_string(),
		staticfiles_dirs: vec![
			temp_dir1.path().to_path_buf(),
			temp_dir2.path().to_path_buf(),
		],
		media_url: None,
	};

	let finder = StaticFilesFinder::new(config.staticfiles_dirs);

	// Should find the first occurrence
	let found = finder.find("test/file.txt");
	let found_path = found.unwrap();
	assert_eq!(
		found_path.canonicalize().unwrap(),
		temp_dir1
			.path()
			.join("test/file.txt")
			.canonicalize()
			.unwrap()
	);
}

#[test]
fn test_find_nonexistent() {
	let (temp_dir1, _temp_dir2, _temp_dir3) = setup_test_files();

	let config = StaticFilesConfig {
		static_root: PathBuf::from("static"),
		static_url: "/static/".to_string(),
		staticfiles_dirs: vec![temp_dir1.path().to_path_buf()],
		media_url: None,
	};

	let finder = StaticFilesFinder::new(config.staticfiles_dirs);

	// Should return Err for non-existent file
	let found = finder.find("does/not/exist.txt");
	assert!(found.is_err());
}

#[test]
fn test_find_in_multiple_dirs() {
	let (temp_dir1, temp_dir2, _temp_dir3) = setup_test_files();

	let config = StaticFilesConfig {
		static_root: PathBuf::from("static"),
		static_url: "/static/".to_string(),
		staticfiles_dirs: vec![
			temp_dir1.path().to_path_buf(),
			temp_dir2.path().to_path_buf(),
		],
		media_url: None,
	};

	let finder = StaticFilesFinder::new(config.staticfiles_dirs);

	// Should find file in second directory
	let found = finder.find("other/file.txt");
	let found_path = found.unwrap();
	assert_eq!(
		found_path.canonicalize().unwrap(),
		temp_dir2
			.path()
			.join("other/file.txt")
			.canonicalize()
			.unwrap()
	);
}

#[test]
fn test_find_all_files() {
	let temp_dir = TempDir::new().unwrap();

	// Create files in root directory
	fs::write(temp_dir.path().join("file1.txt"), b"Content 1").unwrap();
	fs::write(temp_dir.path().join("file2.txt"), b"Content 2").unwrap();
	fs::write(temp_dir.path().join("file3.css"), b"Content 3").unwrap();

	let config = StaticFilesConfig {
		static_root: PathBuf::from("static"),
		static_url: "/static/".to_string(),
		staticfiles_dirs: vec![temp_dir.path().to_path_buf()],
		media_url: None,
	};

	let finder = StaticFilesFinder::new(config.staticfiles_dirs);

	let files = finder.find_all();
	assert_eq!(files.len(), 3);

	// Verify files can be found individually
	assert!(finder.find("file1.txt").is_ok());
	assert!(finder.find("file2.txt").is_ok());
	assert!(finder.find("file3.css").is_ok());
}

#[test]
fn test_empty_staticfiles_dirs() {
	let config = StaticFilesConfig {
		static_root: PathBuf::from("static"),
		static_url: "/static/".to_string(),
		staticfiles_dirs: vec![],
		media_url: None,
	};

	let finder = StaticFilesFinder::new(config.staticfiles_dirs);

	// Should return Err when no directories configured
	let found = finder.find("test/file.txt");
	assert!(found.is_err());

	let files = finder.find_all();
	assert!(files.is_empty());
}

#[test]
fn test_nonexistent_staticfiles_dirs() {
	let config = StaticFilesConfig {
		static_root: PathBuf::from("static"),
		static_url: "/static/".to_string(),
		staticfiles_dirs: vec![PathBuf::from("/nonexistent/directory")],
		media_url: None,
	};

	let finder = StaticFilesFinder::new(config.staticfiles_dirs);

	// Should handle non-existent directories gracefully
	let found = finder.find("test/file.txt");
	assert!(found.is_err());

	let files = finder.find_all();
	assert!(files.is_empty());
}
