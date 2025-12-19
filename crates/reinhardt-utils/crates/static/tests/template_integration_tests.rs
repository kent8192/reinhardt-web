//! Integration tests for template static configuration

use reinhardt_static::storage::{FileSystemStorage, Storage};
use reinhardt_static::{ManifestStaticFilesStorage, TemplateStaticConfig};
use tempfile::TempDir;

#[tokio::test]
async fn test_template_config_from_storage_with_manifest() {
	let temp_dir = TempDir::new().unwrap();
	let static_root = temp_dir.path().to_path_buf();

	// Create manifest file
	let manifest_content = r#"{
  "css/style.css": "css/style.abc123.css",
  "js/app.js": "js/app.def456.js"
}"#;

	std::fs::write(static_root.join("staticfiles.json"), manifest_content).unwrap();

	let storage = ManifestStaticFilesStorage::new(static_root, "/static/");
	let config = TemplateStaticConfig::from_storage(&storage).await.unwrap();

	// Test URL resolution with manifest
	assert_eq!(
		config.resolve_url("css/style.css"),
		"/static/css/style.abc123.css"
	);
	assert_eq!(config.resolve_url("js/app.js"), "/static/js/app.def456.js");
}

#[tokio::test]
async fn test_template_config_from_storage_without_manifest() {
	let temp_dir = TempDir::new().unwrap();
	let static_root = temp_dir.path().to_path_buf();

	// No manifest file
	let storage = ManifestStaticFilesStorage::new(static_root, "/static/");
	let config = TemplateStaticConfig::from_storage(&storage).await.unwrap();

	// Should fallback to basic URL generation
	assert!(!config.use_manifest);
	assert_eq!(config.resolve_url("css/style.css"), "/static/css/style.css");
}

#[tokio::test]
async fn test_template_config_resolve_url_with_storage_consistency() {
	let temp_dir = TempDir::new().unwrap();
	let static_root = temp_dir.path().to_path_buf();

	// Create manifest
	let manifest_content = r#"{
  "test.css": "test.abc123.css"
}"#;

	std::fs::write(static_root.join("staticfiles.json"), manifest_content).unwrap();

	let storage = ManifestStaticFilesStorage::new(static_root.clone(), "/static/");
	let config = TemplateStaticConfig::from_storage(&storage).await.unwrap();

	// TemplateStaticConfig should resolve to hashed filename from manifest
	assert_eq!(config.resolve_url("test.css"), "/static/test.abc123.css");

	// Verify manifest was properly loaded
	assert!(config.use_manifest);
	assert_eq!(
		config.manifest.get("test.css"),
		Some(&"test.abc123.css".to_string())
	);
}

#[test]
fn test_template_config_resolve_url_consistency_with_storage_trait() {
	let temp_dir = TempDir::new().unwrap();
	let storage = FileSystemStorage::new(temp_dir.path(), "/static/");
	let config = TemplateStaticConfig::new("/static/".to_string());

	let test_paths = vec![
		"test.txt",
		"css/style.css",
		"js/app.js?v=1",
		"file.css#section",
	];

	for path in test_paths {
		// Without manifest, TemplateStaticConfig should match Storage::url()
		assert_eq!(
			config.resolve_url(path),
			storage.url(path),
			"Mismatch for path: {}",
			path
		);
	}
}
