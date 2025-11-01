//! Integration tests for ManifestStaticFilesStorage with dependency resolution

use reinhardt_static::ManifestStaticFilesStorage;
use std::collections::HashMap;
use tempfile::TempDir;

#[tokio::test]
async fn test_manifest_save_with_dependencies() {
	let temp_dir = TempDir::new().unwrap();
	let storage =
		ManifestStaticFilesStorage::new(temp_dir.path(), "/static/").with_manifest_strict(false);

	let mut files = HashMap::new();
	files.insert("img/logo.png".to_string(), b"fake image data".to_vec());
	files.insert(
		"styles.css".to_string(),
		b"body { background: url(img/logo.png); }".to_vec(),
	);

	let count = storage.save_with_dependencies(files).await.unwrap();
	assert_eq!(count, 2);

	// Check that files were hashed
	let css_url = storage.url("styles.css");
	assert!(css_url.contains("styles."));
	assert!(css_url.ends_with(".css"));

	let img_url = storage.url("img/logo.png");
	assert!(img_url.contains("img/logo."));
	assert!(img_url.ends_with(".png"));

	// Check that CSS references the hashed image
	let saved_css = storage.open("styles.css").await.unwrap();
	let saved_str = String::from_utf8(saved_css).unwrap();
	assert!(
		saved_str.contains("img/logo.") && saved_str.contains(".png"),
		"CSS should contain hashed image reference, got: {}",
		saved_str
	);

	// Check that manifest was created
	assert!(storage.exists("staticfiles.json"));
}

#[tokio::test]
async fn test_manifest_with_multiple_css_dependencies() {
	let temp_dir = TempDir::new().unwrap();
	let storage =
		ManifestStaticFilesStorage::new(temp_dir.path(), "/static/").with_manifest_strict(false);

	let mut files = HashMap::new();

	// Create a dependency chain: main.css -> base.css -> font.woff
	files.insert("fonts/font.woff".to_string(), b"fake font data".to_vec());
	files.insert(
		"base.css".to_string(),
		b"@font-face { src: url(fonts/font.woff); }".to_vec(),
	);
	files.insert("main.css".to_string(), b"@import url(base.css);".to_vec());

	let count = storage.save_with_dependencies(files).await.unwrap();
	assert_eq!(count, 3);

	// Check that all files are in manifest
	assert!(storage.get_hashed_path("fonts/font.woff").is_some());
	assert!(storage.get_hashed_path("base.css").is_some());
	assert!(storage.get_hashed_path("main.css").is_some());

	// Check that base.css references hashed font
	let base_css = storage.open("base.css").await.unwrap();
	let base_str = String::from_utf8(base_css).unwrap();
	assert!(
		base_str.contains("fonts/font.") && base_str.contains(".woff"),
		"base.css should reference hashed font, got: {}",
		base_str
	);

	// Check that main.css references hashed base.css
	let main_css = storage.open("main.css").await.unwrap();
	let main_str = String::from_utf8(main_css).unwrap();
	assert!(
		main_str.contains("base.") && main_str.contains(".css"),
		"main.css should reference hashed base.css, got: {}",
		main_str
	);
}

#[tokio::test]
async fn test_manifest_persistence_after_batch_processing() {
	let temp_dir = TempDir::new().unwrap();

	// First instance: save with dependencies
	{
		let storage = ManifestStaticFilesStorage::new(temp_dir.path(), "/static/")
			.with_manifest_strict(false);

		let mut files = HashMap::new();
		files.insert("app.js".to_string(), b"console.log('app');".to_vec());
		files.insert("styles.css".to_string(), b"body { color: red; }".to_vec());

		storage.save_with_dependencies(files).await.unwrap();
	}

	// Second instance: should be able to read from manifest
	{
		let storage = ManifestStaticFilesStorage::new(temp_dir.path(), "/static/")
			.with_manifest_strict(false);

		// URLs should work from manifest
		let js_url = storage.url("app.js");
		let css_url = storage.url("styles.css");

		assert!(js_url.contains("app."));
		assert!(css_url.contains("styles."));
	}
}

#[tokio::test]
async fn test_manifest_empty_files() {
	let temp_dir = TempDir::new().unwrap();
	let storage =
		ManifestStaticFilesStorage::new(temp_dir.path(), "/static/").with_manifest_strict(false);

	let files = HashMap::new();
	let count = storage.save_with_dependencies(files).await.unwrap();

	assert_eq!(count, 0);
}

#[tokio::test]
async fn test_manifest_single_file_no_dependencies() {
	let temp_dir = TempDir::new().unwrap();
	let storage =
		ManifestStaticFilesStorage::new(temp_dir.path(), "/static/").with_manifest_strict(false);

	let mut files = HashMap::new();
	files.insert("standalone.txt".to_string(), b"standalone content".to_vec());

	let count = storage.save_with_dependencies(files).await.unwrap();
	assert_eq!(count, 1);

	assert!(storage.get_hashed_path("standalone.txt").is_some());
	assert!(storage.exists("staticfiles.json"));
}

#[tokio::test]
async fn test_manifest_non_css_files() {
	let temp_dir = TempDir::new().unwrap();
	let storage =
		ManifestStaticFilesStorage::new(temp_dir.path(), "/static/").with_manifest_strict(false);

	let mut files = HashMap::new();
	files.insert("image.png".to_string(), b"fake png".to_vec());
	files.insert("data.json".to_string(), b"{}".to_vec());
	files.insert("script.js".to_string(), b"alert('hi')".to_vec());

	let count = storage.save_with_dependencies(files).await.unwrap();
	assert_eq!(count, 3);

	// All should be in manifest
	assert!(storage.get_hashed_path("image.png").is_some());
	assert!(storage.get_hashed_path("data.json").is_some());
	assert!(storage.get_hashed_path("script.js").is_some());
}

#[tokio::test]
async fn test_manifest_complex_directory_structure() {
	let temp_dir = TempDir::new().unwrap();
	let storage =
		ManifestStaticFilesStorage::new(temp_dir.path(), "/static/").with_manifest_strict(false);

	let mut files = HashMap::new();
	files.insert("assets/images/logo.png".to_string(), b"logo".to_vec());
	files.insert(
		"assets/css/theme.css".to_string(),
		b"body { background: url(../images/logo.png); }".to_vec(),
	);

	let count = storage.save_with_dependencies(files).await.unwrap();
	assert_eq!(count, 2);

	// Check paths are preserved in manifest
	let logo_path = storage.get_hashed_path("assets/images/logo.png");
	assert!(logo_path.is_some());
	assert!(logo_path.unwrap().contains("assets/images/"));

	let css_path = storage.get_hashed_path("assets/css/theme.css");
	assert!(css_path.is_some());
	assert!(css_path.unwrap().contains("assets/css/"));
}
