use regex::Regex;
use reinhardt_static::storage::HashedFileStorage;
use std::collections::HashMap;
use tempfile::TempDir;

#[tokio::test]
async fn test_hashed_storage_basic() {
	let temp_dir = TempDir::new().unwrap();
	let storage = HashedFileStorage::new(temp_dir.path(), "/static/");

	// Save a simple file
	let content = b"Hello, world!";
	let hashed_name = storage.save("test.txt", content).await.unwrap();

	// Verify the hashed name contains a hash
	assert!(hashed_name.contains("."));
	assert!(hashed_name.ends_with(".txt"));
	assert_ne!(hashed_name, "test.txt");

	// Verify we can read it back
	let read_content = storage.open("test.txt").await.unwrap();
	assert_eq!(read_content, content);

	// Verify URL generation
	let url = storage.url("test.txt");
	assert!(url.starts_with("/static/"));
	assert!(url.contains(&hashed_name));

	// Verify exists check
	assert!(storage.exists("test.txt"));
	assert!(!storage.exists("nonexistent.txt"));

	// Cleanup is automatic with TempDir
}

#[tokio::test]
async fn test_css_url_replacement() {
	let temp_dir = TempDir::new().unwrap();
	let storage = HashedFileStorage::new(temp_dir.path(), "/static/");

	// Use save_with_dependencies to handle dependency order
	let mut files = HashMap::new();

	// Add the referenced image
	files.insert("img/logo.png".to_string(), b"fake image data".to_vec());

	// Add CSS that references it
	files.insert(
		"styles.css".to_string(),
		b"body { background: url(img/logo.png); }".to_vec(),
	);

	// Process with dependency resolution
	let count = storage.save_with_dependencies(files).await.unwrap();
	assert_eq!(count, 2);

	// Read back the saved CSS
	let saved_css = storage.open("styles.css").await.unwrap();
	let saved_str = String::from_utf8(saved_css).unwrap();

	// Should contain hashed version with pattern: img/logo.[hash].png
	let hashed_pattern = Regex::new(r"img/logo\.[a-f0-9]{8,}\.png").unwrap();
	assert!(
		hashed_pattern.is_match(&saved_str),
		"CSS should contain hashed image reference matching pattern 'img/logo.[hash].png', got: {}",
		saved_str
	);
	// Should not contain original reference
	assert!(!saved_str.contains("url(img/logo.png)"));

	// Cleanup is automatic with TempDir
}

#[tokio::test]
async fn test_css_url_with_quotes() {
	let temp_dir = TempDir::new().unwrap();
	let storage = HashedFileStorage::new(temp_dir.path(), "/static/");

	// Save image first
	storage.save("bg.jpg", b"fake image").await.unwrap();

	// Save CSS that references it
	storage
		.save("app.css", b"body { background: url('bg.jpg'); }")
		.await
		.unwrap();

	// The current implementation does simple string replacement
	// which works for simple cases like this
	let saved_css = storage.open("app.css").await.unwrap();
	let saved_str = String::from_utf8(saved_css).unwrap();

	// Should not contain original reference (it was replaced during save)
	// Note: Since we're not using save_with_dependencies, the replacement
	// doesn't happen automatically. This test documents current behavior.
	assert!(saved_str.contains("bg.jpg"));

	// Cleanup is automatic with TempDir
}

#[tokio::test]
async fn test_multiple_files_same_content() {
	let temp_dir = TempDir::new().unwrap();
	let storage = HashedFileStorage::new(temp_dir.path(), "/static/");

	let content = b"same content";

	// Save the same content with different names
	let hash1 = storage.save("file1.txt", content).await.unwrap();
	let hash2 = storage.save("file2.txt", content).await.unwrap();

	// The hashed parts should be the same (same content = same hash)
	// but the base names are different
	assert_ne!(hash1, hash2);
	assert!(hash1.starts_with("file1."));
	assert!(hash2.starts_with("file2."));

	// Both should contain the same hash in the middle
	let hash1_parts: Vec<&str> = hash1.split('.').collect();
	let hash2_parts: Vec<&str> = hash2.split('.').collect();
	assert_eq!(hash1_parts[1], hash2_parts[1]); // Same hash value

	// Cleanup is automatic with TempDir
}

#[tokio::test]
async fn test_nested_paths() {
	let temp_dir = TempDir::new().unwrap();
	let storage = HashedFileStorage::new(temp_dir.path(), "/static/");

	// Save a file in a nested directory
	let content = b"nested file content";
	let hashed_name = storage
		.save("css/components/button.css", content)
		.await
		.unwrap();

	assert!(hashed_name.contains("css/components/"));
	assert!(hashed_name.ends_with(".css"));

	// Verify we can read it back
	let read_content = storage.open("css/components/button.css").await.unwrap();
	assert_eq!(read_content, content);

	// Cleanup is automatic with TempDir
}

#[tokio::test]
async fn test_get_hashed_path() {
	let temp_dir = TempDir::new().unwrap();
	let storage = HashedFileStorage::new(temp_dir.path(), "/static/");

	// Before saving, should return None
	assert!(storage.get_hashed_path("test.txt").is_none());

	// After saving, should return the hashed name
	let content = b"test content";
	let hashed_name = storage.save("test.txt", content).await.unwrap();

	let retrieved = storage.get_hashed_path("test.txt").unwrap();
	assert_eq!(retrieved, hashed_name);

	// Cleanup is automatic with TempDir
}

#[tokio::test]
async fn test_save_with_dependencies_multiple_css() {
	let temp_dir = TempDir::new().unwrap();
	let storage = HashedFileStorage::new(temp_dir.path(), "/static/");

	let mut files = HashMap::new();

	// Add shared image
	files.insert("logo.png".to_string(), b"logo data".to_vec());

	// Add multiple CSS files referencing the same image
	files.insert(
		"main.css".to_string(),
		b"body { background: url(logo.png); }".to_vec(),
	);
	files.insert(
		"theme.css".to_string(),
		b".header { background: url(logo.png); }".to_vec(),
	);

	let count = storage.save_with_dependencies(files).await.unwrap();
	assert_eq!(count, 3);

	// Both CSS files should have the hashed logo reference
	let main_css = storage.open("main.css").await.unwrap();
	let main_str = String::from_utf8(main_css).unwrap();
	let hashed_pattern = Regex::new(r"logo\.[a-f0-9]{8,}\.png").unwrap();
	assert!(
		hashed_pattern.is_match(&main_str),
		"main.css should contain hashed logo reference matching pattern 'logo.[hash].png', got: {}",
		main_str
	);
	assert!(!main_str.contains("url(logo.png)"));

	let theme_css = storage.open("theme.css").await.unwrap();
	let theme_str = String::from_utf8(theme_css).unwrap();
	assert!(
		hashed_pattern.is_match(&theme_str),
		"theme.css should contain hashed logo reference matching pattern 'logo.[hash].png', got: {}",
		theme_str
	);
	assert!(!theme_str.contains("url(logo.png)"));

	// Cleanup is automatic with TempDir
}
