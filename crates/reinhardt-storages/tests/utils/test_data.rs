//! Test data generation utilities.

use rand::Rng;
use std::path::Path;

/// Test file structure.
#[derive(Debug, Clone)]
pub struct TestFile {
	pub name: String,
	pub content: Vec<u8>,
	pub size: usize,
}

impl TestFile {
	/// Create a new test file.
	pub fn new(name: String, content: Vec<u8>) -> Self {
		let size = content.len();
		Self { name, content, size }
	}

	/// Get the file extension.
	pub fn extension(&self) -> Option<&str> {
		Path::new(&self.name).extension().and_then(|e| e.to_str())
	}
}

/// Generate random bytes of specified size.
pub fn generate_random_bytes(size: usize) -> Vec<u8> {
	let mut bytes = vec![0u8; size];
	rand::thread_rng().fill(&mut bytes[..]);
	bytes
}

/// Generate text content with specified number of lines.
pub fn generate_text_content(lines: usize) -> String {
	(0..lines)
		.map(|i| format!("Line {}: {}", i, "test content".repeat(10)))
		.collect::<Vec<_>>()
		.join("\n")
}

/// Generate binary content containing all byte values.
pub fn generate_binary_content() -> Vec<u8> {
	(0u8..=255).collect::<Vec<_>>()
}

/// Generate unique file name with prefix.
pub fn generate_unique_name(prefix: &str) -> String {
	format!("{}-{}", prefix, uuid::Uuid::new_v4())
}

/// Generate nested path with specified depth.
pub fn generate_nested_path(depth: usize, file_name: &str) -> String {
	let parts: Vec<String> = (0..depth).map(|i| format!("level{}", i)).collect();
	format!("{}/{}", parts.join("/"), file_name)
}

/// Create test file with random content.
pub fn create_test_file(name: String, size: usize) -> TestFile {
	let content = generate_random_bytes(size);
	TestFile::new(name, content)
}

/// Create test file with text content.
pub fn create_text_file(name: String, lines: usize) -> TestFile {
	let content = generate_text_content(lines);
	let size = content.len();
	TestFile::new(name, content.into_bytes())
}

/// Create test file with binary content.
pub fn create_binary_file(name: String) -> TestFile {
	let content = generate_binary_content();
	let size = content.len();
	TestFile::new(name, content)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_generate_random_bytes() {
		let bytes = generate_random_bytes(100);
		assert_eq!(bytes.len(), 100);
	}

	#[test]
	fn test_generate_text_content() {
		let content = generate_text_content(5);
		let lines: Vec<&str> = content.lines().collect();
		assert_eq!(lines.len(), 5);
	}

	#[test]
	fn test_generate_binary_content() {
		let content = generate_binary_content();
		assert_eq!(content.len(), 256);
		assert_eq!(content[0], 0);
		assert_eq!(content[255], 255);
	}

	#[test]
	fn test_generate_unique_name() {
		let name1 = generate_unique_name("test");
		let name2 = generate_unique_name("test");
		assert_ne!(name1, name2);
		assert!(name1.starts_with("test-"));
		assert!(name2.starts_with("test-"));
	}

	#[test]
	fn test_generate_nested_path() {
		let path = generate_nested_path(3, "file.txt");
		assert_eq!(path, "level0/level1/level2/file.txt");
	}

	#[test]
	fn test_test_file_extension() {
		let file = TestFile::new("test.txt".to_string(), vec![]);
		assert_eq!(file.extension(), Some("txt"));

		let file2 = TestFile::new("test".to_string(), vec![]);
		assert_eq!(file2.extension(), None);

		let file3 = TestFile::new("path/to/file.json".to_string(), vec![]);
		assert_eq!(file3.extension(), Some("json"));
	}

	#[test]
	fn test_create_test_file() {
		let file = create_test_file("test.bin".to_string(), 1000);
		assert_eq!(file.name, "test.bin");
		assert_eq!(file.size, 1000);
		assert_eq!(file.content.len(), 1000);
	}

	#[test]
	fn test_create_text_file() {
		let file = create_text_file("test.txt".to_string(), 10);
		assert_eq!(file.name, "test.txt");
		assert!(file.size > 0);
		let content = String::from_utf8(file.content).unwrap();
		assert!(content.lines().count() >= 10);
	}

	#[test]
	fn test_create_binary_file() {
		let file = create_binary_file("binary.bin".to_string());
		assert_eq!(file.name, "binary.bin");
		assert_eq!(file.size, 256);
	}
}
