//! Common test fixtures for storage tests.

use crate::utils::{TestFile, create_binary_file, create_text_file};
use rstest::fixture;

/// Empty file fixture (0 bytes).
#[fixture]
pub fn empty_file() -> TestFile {
	TestFile::new("empty.txt".to_string(), vec![])
}

/// Small file fixture (< 1KB).
#[fixture]
pub fn small_file() -> TestFile {
	create_text_file("small.txt".to_string(), 10)
}

/// Medium file fixture (around 1KB).
#[fixture]
pub fn medium_file() -> TestFile {
	create_text_file("medium.txt".to_string(), 100)
}

/// Large file fixture (around 100KB).
#[fixture]
pub fn large_file() -> TestFile {
	create_text_file("large.txt".to_string(), 10000)
}

/// Binary file fixture with all byte values.
#[fixture]
pub fn binary_file() -> TestFile {
	create_binary_file("binary.bin".to_string())
}

/// Collection of test files with varying sizes.
#[fixture]
pub fn test_files() -> Vec<TestFile> {
	vec![
		TestFile::new("empty.txt".to_string(), vec![]),
		create_text_file("small.txt".to_string(), 10),
		create_text_file("medium.txt".to_string(), 100),
		create_binary_file("binary.bin".to_string()),
	]
}

/// Unique file name fixture.
#[fixture]
pub fn unique_file_name() -> String {
	crate::utils::generate_unique_name("test")
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_empty_file_fixture() {
		let file = empty_file();
		assert_eq!(file.name, "empty.txt");
		assert_eq!(file.size, 0);
		assert!(file.content.is_empty());
	}

	#[test]
	fn test_small_file_fixture() {
		let file = small_file();
		assert_eq!(file.name, "small.txt");
		assert!(file.size > 0);
		assert!(file.size < 1024);
	}

	#[test]
	fn test_medium_file_fixture() {
		let file = medium_file();
		assert_eq!(file.name, "medium.txt");
		assert!(file.size > 100);
	}

	#[test]
	fn test_large_file_fixture() {
		let file = large_file();
		assert_eq!(file.name, "large.txt");
		assert!(file.size > 10000);
	}

	#[test]
	fn test_binary_file_fixture() {
		let file = binary_file();
		assert_eq!(file.name, "binary.bin");
		assert_eq!(file.size, 256); // All byte values
	}

	#[test]
	fn test_test_files_fixture() {
		let files = test_files();
		assert_eq!(files.len(), 4);
		assert_eq!(files[0].name, "empty.txt");
		assert_eq!(files[1].name, "small.txt");
		assert_eq!(files[2].name, "medium.txt");
		assert_eq!(files[3].name, "binary.bin");
	}

	#[test]
	fn test_unique_file_name_fixture() {
		let name1 = unique_file_name();
		let name2 = unique_file_name();
		assert_ne!(name1, name2);
		assert!(name1.starts_with("test-"));
	}
}
