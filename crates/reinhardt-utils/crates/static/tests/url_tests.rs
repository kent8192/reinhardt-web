use reinhardt_utils::r#static::storage::{FileSystemStorage, MemoryStorage, Storage};
use tempfile::TempDir;

#[test]
fn test_url_with_querystring() {
	let temp_dir = TempDir::new().unwrap();
	let storage = FileSystemStorage::new(temp_dir.path(), "/static/");

	// URL generation should handle file names with query strings
	let url = storage.url("test.css?version=123");
	assert_eq!(url, "/static/test.css?version=123");
}

#[test]
fn test_url_with_fragment() {
	let temp_dir = TempDir::new().unwrap();
	let storage = FileSystemStorage::new(temp_dir.path(), "/static/");

	// URL generation should handle file names with fragments
	let url = storage.url("test.css#section");
	assert_eq!(url, "/static/test.css#section");
}

#[test]
fn test_url_with_querystring_and_fragment() {
	let temp_dir = TempDir::new().unwrap();
	let storage = FileSystemStorage::new(temp_dir.path(), "/static/");

	// URL generation should handle file names with both query strings and fragments
	let url = storage.url("test.css?version=123#section");
	assert_eq!(url, "/static/test.css?version=123#section");
}

#[test]
fn test_url_special_characters() {
	let temp_dir = TempDir::new().unwrap();
	let storage = FileSystemStorage::new(temp_dir.path(), "/static/");

	// URL generation should preserve special characters
	let url = storage.url("special?chars&quoted.html");
	assert_eq!(url, "/static/special?chars&quoted.html");
}

#[test]
fn test_url_absolute_path() {
	let temp_dir = TempDir::new().unwrap();
	let storage = FileSystemStorage::new(temp_dir.path(), "/static/");

	// URL generation should handle absolute paths
	let url = storage.url("/absolute/path.txt");
	assert_eq!(url, "/static/absolute/path.txt");
}

#[test]
fn test_url_nested_path() {
	let temp_dir = TempDir::new().unwrap();
	let storage = FileSystemStorage::new(temp_dir.path(), "/static/");

	// URL generation should handle nested paths
	let url = storage.url("nested/deep/path/file.js");
	assert_eq!(url, "/static/nested/deep/path/file.js");
}

#[test]
fn test_url_with_different_base_urls() {
	let temp_dir = TempDir::new().unwrap();

	// Test with different base URL formats
	let storage1 = FileSystemStorage::new(temp_dir.path(), "/static/");
	assert_eq!(storage1.url("test.txt"), "/static/test.txt");

	let storage2 = FileSystemStorage::new(temp_dir.path(), "/static");
	assert_eq!(storage2.url("test.txt"), "/static/test.txt");

	let storage3 = FileSystemStorage::new(temp_dir.path(), "static/");
	assert_eq!(storage3.url("test.txt"), "static/test.txt");

	let storage4 = FileSystemStorage::new(temp_dir.path(), "static");
	assert_eq!(storage4.url("test.txt"), "static/test.txt");
}

#[test]
fn test_url_empty_filename() {
	let temp_dir = TempDir::new().unwrap();
	let storage = FileSystemStorage::new(temp_dir.path(), "/static/");

	// URL generation with empty filename
	let url = storage.url("");
	assert_eq!(url, "/static/");
}

#[test]
fn test_url_root_path() {
	let temp_dir = TempDir::new().unwrap();
	let storage = FileSystemStorage::new(temp_dir.path(), "/static/");

	// URL generation with root path
	let url = storage.url("/");
	assert_eq!(url, "/static/");
}

#[test]
fn test_memory_storage_url_with_querystring() {
	let storage = MemoryStorage::new("/static/");

	let url = storage.url("test.css?version=123");
	assert_eq!(url, "/static/test.css?version=123");
}

#[test]
fn test_memory_storage_url_with_fragment() {
	let storage = MemoryStorage::new("/static/");

	let url = storage.url("test.css#section");
	assert_eq!(url, "/static/test.css#section");
}

#[test]
fn test_memory_storage_url_special_characters() {
	let storage = MemoryStorage::new("/static/");

	let url = storage.url("special?chars&quoted.html");
	assert_eq!(url, "/static/special?chars&quoted.html");
}

#[test]
fn test_url_consistency_between_storages() {
	let temp_dir = TempDir::new().unwrap();
	let fs_storage = FileSystemStorage::new(temp_dir.path(), "/static/");
	let mem_storage = MemoryStorage::new("/static/");

	let test_names = vec![
		"test.txt",
		"/test.txt",
		"nested/path.css",
		"file.js?version=1",
		"file.js#anchor",
		"file.js?v=1#anchor",
	];

	for name in test_names {
		assert_eq!(
			fs_storage.url(name),
			mem_storage.url(name),
			"URL mismatch for: {}",
			name
		);
	}
}
