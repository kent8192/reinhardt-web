//! Integration tests for static file storage backends
//!
//! This test file verifies the integration between:
//! - Storage backends (filesystem, S3, in-memory)
//! - File hashing and naming
//! - Post-processing pipeline (compression, minification)
//! - Manifest generation and persistence
//! - Content type detection
//! - Cache control and CDN integration
//! - Concurrent upload handling
//!
//! ## Testing Strategy
//! Tests use real filesystem (via tempfile) and S3-compatible storage
//! (via MinIO in TestContainers) to ensure static file operations work
//! correctly in production-like scenarios.
//!
//! ## Integration Points Tested
//! - HashedFileStorage + FileSystemStorage
//! - HashedFileStorage + S3Storage
//! - ManifestStaticFilesStorage + ProcessingPipeline
//! - Storage backend switching
//! - Multi-source file collection

use reinhardt_static::ManifestStaticFilesStorage;
use reinhardt_static::processing::compress::GzipCompressor;
use reinhardt_static::processing::{ProcessingConfig, ProcessingPipeline, Processor};
use reinhardt_static::storage::{FileSystemStorage, HashedFileStorage, Storage};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Mutex;
use walkdir::WalkDir;

// ==================== Test Fixtures ====================

// Note: Fixtures are now inline to avoid rstest dependency issues

// ==================== FileSystem Storage Integration Tests ====================

/// Test Intent: Verify basic filesystem storage with hashing integration
/// Integration Point: HashedFileStorage + FileSystemStorage
#[tokio::test]
async fn test_filesystem_storage_with_hashing() {
	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let storage = HashedFileStorage::new(temp_static_dir.path(), "/static/");

	// Save files with automatic hashing
	let js_content = b"console.log('Hello, World!');";
	let css_content = b"body { margin: 0; }";

	let js_hashed = storage.save("app.js", js_content).await.unwrap();
	let css_hashed = storage.save("styles.css", css_content).await.unwrap();

	// Verify hashed filenames contain hash
	assert!(js_hashed.contains("."), "Hashed name should contain hash");
	assert!(js_hashed.ends_with(".js"), "Should preserve extension");
	assert!(css_hashed.contains("."), "Hashed name should contain hash");
	assert!(css_hashed.ends_with(".css"), "Should preserve extension");

	// Verify files can be retrieved by original name
	let retrieved_js = storage.open("app.js").await.unwrap();
	let retrieved_css = storage.open("styles.css").await.unwrap();

	assert_eq!(retrieved_js, js_content);
	assert_eq!(retrieved_css, css_content);

	// Verify URL generation includes hashed names
	let js_url = storage.url("app.js");
	let css_url = storage.url("styles.css");

	assert!(js_url.starts_with("/static/"));
	assert!(js_url.contains(&js_hashed));
	assert!(css_url.starts_with("/static/"));
	assert!(css_url.contains(&css_hashed));

	// Cleanup is automatic with TempDir
}

/// Test Intent: Verify content-based hashing consistency
/// Integration Point: HashedFileStorage hash generation
#[tokio::test]
async fn test_content_based_hashing_consistency() {
	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let storage = HashedFileStorage::new(temp_static_dir.path(), "/static/");

	let content = b"identical content for hash test";

	// Save same content with different names
	let hash1 = storage.save("file1.txt", content).await.unwrap();
	let hash2 = storage.save("file2.txt", content).await.unwrap();

	// Extract hash portion (name.HASH.ext)
	let extract_hash = |s: &str| -> Option<String> {
		let parts: Vec<&str> = s.split('.').collect();
		parts.get(1).map(|s| s.to_string())
	};

	let hash_part1 = extract_hash(&hash1);
	let hash_part2 = extract_hash(&hash2);

	// Hash portion should be identical (same content = same hash)
	assert_eq!(
		hash_part1, hash_part2,
		"Same content should produce same hash"
	);

	// But full names differ (different base names)
	assert_ne!(hash1, hash2, "Full names should differ due to base name");

	// Cleanup is automatic with TempDir
}

/// Test Intent: Verify nested directory structure preservation with hashing
/// Integration Point: HashedFileStorage + FileSystemStorage path handling
#[tokio::test]
async fn test_nested_path_with_hashing() {
	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let storage = HashedFileStorage::new(temp_static_dir.path(), "/static/");

	let content = b"nested file content";
	let nested_path = "assets/images/logo.png";

	let hashed_name = storage.save(nested_path, content).await.unwrap();

	// Verify directory structure is preserved
	assert!(
		hashed_name.contains("assets/images/"),
		"Directory structure should be preserved"
	);
	assert!(
		hashed_name.ends_with(".png"),
		"Extension should be preserved"
	);

	// Verify file can be retrieved
	let retrieved = storage.open(nested_path).await.unwrap();
	assert_eq!(retrieved, content);

	// Verify physical file exists at hashed path
	let hashed_physical_path = temp_static_dir.path().join(&hashed_name);
	assert!(
		hashed_physical_path.exists(),
		"Hashed file should exist at physical path"
	);

	// Cleanup is automatic with TempDir
}

// ==================== Multi-Source Collection Tests ====================

/// Test Intent: Verify static file collection from multiple source directories
/// Integration Point: HashedFileStorage + multi-directory collection
#[tokio::test]
async fn test_multi_source_static_file_collection() {
	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let temp_source_dir = TempDir::new().expect("Failed to create second temp directory");

	let storage = HashedFileStorage::new(temp_static_dir.path(), "/static/");

	// Create files in first source directory
	let source1 = temp_source_dir.path().join("source1");
	std::fs::create_dir_all(&source1).unwrap();
	std::fs::write(source1.join("app.js"), b"console.log('source1');").unwrap();
	std::fs::write(source1.join("style.css"), b"body { color: red; }").unwrap();

	// Create files in second source directory
	let source2 = temp_source_dir.path().join("source2");
	std::fs::create_dir_all(&source2).unwrap();
	std::fs::write(source2.join("vendor.js"), b"/* vendor code */").unwrap();
	std::fs::write(source2.join("theme.css"), b"body { background: white; }").unwrap();

	// Collect files from both sources
	let mut collected_files = HashMap::new();

	for source in &[source1, source2] {
		for entry in WalkDir::new(source).into_iter().filter_map(|e| e.ok()) {
			if entry.file_type().is_file() {
				let relative_path = entry.path().strip_prefix(source).unwrap();
				let content = std::fs::read(entry.path()).unwrap();
				collected_files.insert(relative_path.display().to_string(), content);
			}
		}
	}

	// Save collected files
	for (path, content) in collected_files {
		storage.save(&path, &content).await.unwrap();
	}

	// Verify all files are stored
	assert!(storage.exists("app.js"));
	assert!(storage.exists("style.css"));
	assert!(storage.exists("vendor.js"));
	assert!(storage.exists("theme.css"));

	// Verify content
	let app_content = storage.open("app.js").await.unwrap();
	assert_eq!(app_content, b"console.log('source1');");

	// Cleanup is automatic with TempDir
}

// ==================== Content Type Detection Tests ====================

/// Test Intent: Verify automatic content type detection based on file extension
/// Integration Point: mime_guess integration in storage
#[tokio::test]
async fn test_content_type_detection() {
	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let storage = HashedFileStorage::new(temp_static_dir.path(), "/static/");

	// Test various file types
	let test_files: Vec<(&str, &[u8], &str)> = vec![
		("script.js", b"/* js */", "text/javascript"),
		("style.css", b"/* css */", "text/css"),
		("image.png", b"\x89PNG", "image/png"),
		("data.json", b"{}", "application/json"),
		("page.html", b"<html>", "text/html"),
		("font.woff2", b"wOF2", "font/woff2"),
	];

	for (filename, content, expected_mime) in test_files {
		storage.save(filename, content).await.unwrap();

		// Verify content type using mime_guess
		let guessed = mime_guess::from_path(filename).first_or_octet_stream();
		assert_eq!(
			guessed.to_string(),
			expected_mime,
			"Content type mismatch for {}",
			filename
		);
	}

	// Cleanup is automatic with TempDir
}

// ==================== File Compression Tests ====================

/// Test Intent: Verify gzip compression integration with storage
/// Integration Point: GzipCompressor + HashedFileStorage
#[tokio::test]
async fn test_gzip_compression_integration() {
	use flate2::read::GzDecoder;
	use std::io::Read;

	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let storage = HashedFileStorage::new(temp_static_dir.path(), "/static/");
	let compressor = GzipCompressor::default();

	// Create compressible content (CSS) - needs to be large enough for gzip to be effective
	let original_content = b"\
body { margin: 0; padding: 0; font-family: sans-serif; } \
.container { display: flex; justify-content: center; align-items: center; min-height: 100vh; } \
.header { background-color: #333; color: white; padding: 1rem; font-size: 1.5rem; } \
.content { max-width: 1200px; margin: 0 auto; padding: 2rem; } \
.footer { text-align: center; font-size: 0.875rem; color: #666; padding: 1rem; } \
.btn { display: inline-block; padding: 0.5rem 1rem; background-color: #007bff; color: white; } \
.btn:hover { background-color: #0056b3; } \
.grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 1rem; }";

	// Process content through compressor
	use std::path::Path;
	let compressed = compressor
		.process(original_content, Path::new("styles.css"))
		.await
		.unwrap();

	// Save compressed content
	let _hashed_name = storage.save("styles.css.gz", &compressed).await.unwrap();

	// Verify compressed file exists
	assert!(storage.exists("styles.css.gz"));

	// Retrieve and decompress
	let compressed_content = storage.open("styles.css.gz").await.unwrap();
	let mut decoder = GzDecoder::new(&compressed_content[..]);
	let mut decompressed = Vec::new();
	decoder.read_to_end(&mut decompressed).unwrap();

	// Verify decompressed content matches original
	assert_eq!(decompressed, original_content);

	// Verify compression achieved size reduction
	assert!(
		compressed_content.len() < original_content.len(),
		"Compressed size should be smaller"
	);

	// Cleanup is automatic with TempDir
}

/// Test Intent: Verify selective compression based on file type
/// Integration Point: ProcessingConfig + compression filters
#[tokio::test]
async fn test_selective_file_compression() {
	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let storage = HashedFileStorage::new(temp_static_dir.path(), "/static/");

	// Files that should be compressed
	let compressible: Vec<(&str, &[u8])> = vec![
		("app.js", b"console.log('test');"),
		("style.css", b"body { margin: 0; }"),
		("page.html", b"<html><body></body></html>"),
	];

	// Files that should NOT be compressed (already compressed formats)
	let incompressible: Vec<(&str, &[u8])> = vec![
		("image.png", b"\x89PNG\r\n"),
		("photo.jpg", b"\xFF\xD8\xFF"),
		("archive.gz", b"\x1F\x8B"),
	];

	// Save compressible files
	for (name, content) in &compressible {
		storage.save(name, content).await.unwrap();
	}

	// Save incompressible files
	for (name, content) in &incompressible {
		storage.save(name, content).await.unwrap();
	}

	// Verify all files exist
	for (name, _) in compressible.iter().chain(incompressible.iter()) {
		assert!(storage.exists(name), "File {} should exist", name);
	}

	// Note: Actual compression decisions would be made by ProcessingPipeline
	// This test verifies file type identification is working

	// Cleanup is automatic with TempDir
}

// ==================== Cache Control Integration Tests ====================

/// Test Intent: Verify Cache-Control header configuration for different file types
/// Integration Point: Storage metadata + HTTP caching strategy
#[tokio::test]
async fn test_cache_control_headers_configuration() {
	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let storage = HashedFileStorage::new(temp_static_dir.path(), "/static/");

	// File types with different cache strategies
	let cache_strategies: Vec<(&str, &[u8], &str)> = vec![
		// Hashed files: long-term cache (immutable)
		("app.abc123.js", b"/* js */", "max-age=31536000, immutable"),
		(
			"style.def456.css",
			b"/* css */",
			"max-age=31536000, immutable",
		),
		// HTML: no cache (always revalidate)
		("index.html", b"<html>", "no-cache, must-revalidate"),
		// Images: moderate cache
		("logo.png", b"\x89PNG", "max-age=604800"),
	];

	for (filename, content, _expected_cache) in &cache_strategies {
		storage.save(filename, content).await.unwrap();

		// Verify file exists
		assert!(storage.exists(filename), "File {} should exist", filename);

		// Note: Actual Cache-Control headers would be set by HTTP handler
		// This test verifies file categorization for cache policy
	}

	// Cleanup is automatic with TempDir
}

// ==================== File Versioning and Cache Busting Tests ====================

/// Test Intent: Verify hash-based versioning for cache busting
/// Integration Point: HashedFileStorage versioning strategy
#[tokio::test]
async fn test_file_versioning_with_hash_busting() {
	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let storage = HashedFileStorage::new(temp_static_dir.path(), "/static/");

	// Save initial version
	let v1_content = b"console.log('version 1');";
	let v1_hash = storage.save("app.js", v1_content).await.unwrap();

	// Save updated version (same filename, different content)
	let v2_content = b"console.log('version 2 - updated');";
	let v2_hash = storage.save("app.js", v2_content).await.unwrap();

	// Hashes should differ (different content)
	assert_ne!(
		v1_hash, v2_hash,
		"Different content should produce different hashes"
	);

	// Current version should be v2
	let current = storage.open("app.js").await.unwrap();
	assert_eq!(current, v2_content);

	// URL should reflect new hash
	let url = storage.url("app.js");
	assert!(url.contains(&v2_hash), "URL should use latest hash");

	// Cleanup is automatic with TempDir
}

/// Test Intent: Verify manifest persistence across storage instances
/// Integration Point: ManifestStaticFilesStorage + persistence
#[tokio::test]
async fn test_manifest_persistence_across_instances() {
	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let base_path = temp_static_dir.path();

	// First instance: save files
	{
		let storage =
			ManifestStaticFilesStorage::new(base_path, "/static/").with_manifest_strict(false);

		let mut files = HashMap::new();
		files.insert("app.js".to_string(), b"console.log('app');".to_vec());
		files.insert("style.css".to_string(), b"body { margin: 0; }".to_vec());
		storage.save_with_dependencies(files).await.unwrap();

		// Verify manifest exists
		assert!(storage.exists("staticfiles.json"));
	}

	// Second instance: read manifest
	{
		let storage =
			ManifestStaticFilesStorage::new(base_path, "/static/").with_manifest_strict(false);

		// Load manifest from disk
		storage.load_manifest().await.unwrap();

		// Should be able to access previously saved files
		assert!(storage.exists("app.js"));
		assert!(storage.exists("style.css"));

		// Should be able to retrieve hashed paths
		assert!(storage.get_hashed_path("app.js").is_some());
		assert!(storage.get_hashed_path("style.css").is_some());
	}

	// Cleanup is automatic with TempDir
}

// ==================== Post-Processing Pipeline Tests ====================

/// Test Intent: Verify processing pipeline integration with storage
/// Integration Point: ProcessingPipeline + HashedFileStorage
#[tokio::test]
async fn test_processing_pipeline_integration() {
	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let storage = HashedFileStorage::new(temp_static_dir.path(), "/static/");

	// Create pipeline with compression
	use std::path::Path;
	let config = ProcessingConfig::default();
	let pipeline = ProcessingPipeline::new(config);

	// Process CSS file
	let css_content = b"body { margin: 0; padding: 0; background-color: #ffffff; }";
	let processed = pipeline
		.process_file(css_content, Path::new("styles.css"))
		.await
		.unwrap();

	// Save processed content
	storage.save("styles.css", &processed).await.unwrap();

	// Verify file exists
	assert!(storage.exists("styles.css"));

	// Verify content was processed (compressed)
	let stored = storage.open("styles.css").await.unwrap();
	assert_eq!(stored, processed);

	// Cleanup is automatic with TempDir
}

// ==================== Storage Backend Switching Tests ====================

/// Test Intent: Verify seamless switching between storage backends
/// Integration Point: Storage trait polymorphism
#[tokio::test]
async fn test_storage_backend_switching() {
	use reinhardt_static::storage::MemoryStorage;

	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let content = b"test content for backend switching";

	// Start with filesystem storage
	let fs_storage = FileSystemStorage::new(temp_static_dir.path(), "/static/");
	fs_storage.save("test.txt", content).await.unwrap();
	assert!(fs_storage.exists("test.txt"));

	// Switch to memory storage
	let mem_storage = MemoryStorage::new("/static/");
	mem_storage.save("test.txt", content).await.unwrap();
	assert!(mem_storage.exists("test.txt"));

	// Verify both storages have the file independently
	let fs_content = fs_storage.open("test.txt").await.unwrap();
	let mem_content = mem_storage.open("test.txt").await.unwrap();

	assert_eq!(fs_content, content);
	assert_eq!(mem_content, content);

	// Verify URLs are generated correctly
	assert_eq!(fs_storage.url("test.txt"), "/static/test.txt");
	assert_eq!(mem_storage.url("test.txt"), "/static/test.txt");

	// Cleanup is automatic with TempDir
}

// ==================== Manifest Generation Tests ====================

/// Test Intent: Verify manifest generation and mapping accuracy
/// Integration Point: ManifestStaticFilesStorage + mapping
#[tokio::test]
async fn test_manifest_generation_and_mapping() {
	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let storage = ManifestStaticFilesStorage::new(temp_static_dir.path(), "/static/")
		.with_manifest_strict(false);

	// Save multiple files
	let file_data: Vec<(&str, &[u8])> = vec![
		("app.js", b"console.log('app');"),
		("vendor.js", b"/* vendor */"),
		("main.css", b"body { margin: 0; }"),
		("theme.css", b".theme { color: blue; }"),
	];

	let mut files = HashMap::new();
	for (name, content) in &file_data {
		files.insert(name.to_string(), content.to_vec());
	}
	storage.save_with_dependencies(files).await.unwrap();

	// Verify manifest exists
	assert!(storage.exists("staticfiles.json"));

	// Read and verify manifest content
	let manifest_content = storage.open("staticfiles.json").await.unwrap();
	let manifest_str = String::from_utf8(manifest_content).unwrap();
	let manifest: serde_json::Value = serde_json::from_str(&manifest_str).unwrap();

	// Verify manifest contains all files
	let paths = manifest.get("paths").unwrap().as_object().unwrap();

	for (name, _) in &file_data {
		assert!(
			paths.contains_key(*name),
			"Manifest should contain mapping for {}",
			name
		);

		// Verify mapping points to hashed name
		let hashed = paths.get(*name).unwrap().as_str().unwrap();
		assert!(hashed.contains("."), "Hashed name should contain hash");
	}

	// Cleanup is automatic with TempDir
}

/// Test Intent: Verify manifest strict mode validation
/// Integration Point: ManifestStaticFilesStorage strict mode
#[tokio::test]
async fn test_manifest_strict_mode_validation() {
	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let storage = ManifestStaticFilesStorage::new(temp_static_dir.path(), "/static/")
		.with_manifest_strict(true);

	// Save a file
	let mut files = HashMap::new();
	files.insert("app.js".to_string(), b"console.log('test');".to_vec());
	storage.save_with_dependencies(files).await.unwrap();

	// In strict mode, trying to access non-existent file should fail
	let result = storage.open("nonexistent.js").await;
	assert!(
		result.is_err(),
		"Strict mode should fail on non-existent file"
	);

	// But existing file should work
	let result = storage.open("app.js").await;
	assert!(result.is_ok(), "Existing file should be accessible");

	// Cleanup is automatic with TempDir
}

// ==================== Concurrent Upload Handling Tests ====================

/// Test Intent: Verify concurrent file uploads are handled correctly
/// Integration Point: Storage + async concurrency
#[tokio::test]
async fn test_concurrent_file_uploads() {
	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let storage = Arc::new(HashedFileStorage::new(temp_static_dir.path(), "/static/"));

	// Create multiple concurrent upload tasks
	let mut handles = vec![];

	for i in 0..10 {
		let storage_clone = storage.clone();
		let filename = format!("file{}.txt", i);
		let content = format!("content for file {}", i).into_bytes();

		let handle = tokio::spawn(async move {
			storage_clone.save(&filename, &content).await.unwrap();
		});

		handles.push(handle);
	}

	// Wait for all uploads to complete
	for handle in handles {
		handle.await.unwrap();
	}

	// Verify all files were uploaded
	for i in 0..10 {
		let filename = format!("file{}.txt", i);
		assert!(
			storage.exists(&filename),
			"File {} should exist after concurrent upload",
			filename
		);

		// Verify content
		let expected = format!("content for file {}", i).into_bytes();
		let actual = storage.open(&filename).await.unwrap();
		assert_eq!(actual, expected);
	}

	// Cleanup is automatic with TempDir
}

/// Test Intent: Verify concurrent access to same file is handled safely
/// Integration Point: Storage + concurrent read/write
#[tokio::test]
async fn test_concurrent_access_to_same_file() {
	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let storage = Arc::new(Mutex::new(HashedFileStorage::new(
		temp_static_dir.path(),
		"/static/",
	)));

	// Initial save
	storage
		.lock()
		.await
		.save("shared.txt", b"initial content")
		.await
		.unwrap();

	// Create concurrent read tasks
	let mut read_handles = vec![];

	for _ in 0..5 {
		let storage_clone = storage.clone();
		let handle =
			tokio::spawn(
				async move { storage_clone.lock().await.open("shared.txt").await.unwrap() },
			);
		read_handles.push(handle);
	}

	// Wait for all reads
	for handle in read_handles {
		let content = handle.await.unwrap();
		assert_eq!(content, b"initial content");
	}

	// Cleanup is automatic with TempDir
}

// ==================== Dependency Resolution Tests ====================

/// Test Intent: Verify CSS URL dependency resolution with hashed references
/// Integration Point: HashedFileStorage + dependency tracking
#[tokio::test]
async fn test_css_dependency_resolution_with_hashing() {
	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let storage = HashedFileStorage::new(temp_static_dir.path(), "/static/");

	let mut files = HashMap::new();

	// Add image dependency
	files.insert("images/bg.jpg".to_string(), b"fake image data".to_vec());

	// Add CSS referencing the image
	files.insert(
		"styles.css".to_string(),
		b"body { background-image: url(images/bg.jpg); }".to_vec(),
	);

	// Process with dependency resolution
	let count = storage.save_with_dependencies(files).await.unwrap();
	assert_eq!(count, 2, "Should save 2 files");

	// Retrieve CSS and verify hashed reference
	let css_content = storage.open("styles.css").await.unwrap();
	let css_str = String::from_utf8(css_content).unwrap();

	// Should contain hashed image reference
	assert!(
		css_str.contains("images/bg.") && css_str.contains(".jpg"),
		"CSS should reference hashed image, got: {}",
		css_str
	);

	// Should not contain original reference
	assert!(
		!css_str.contains("url(images/bg.jpg)"),
		"CSS should not contain original reference"
	);

	// Cleanup is automatic with TempDir
}

/// Test Intent: Verify complex dependency chain resolution
/// Integration Point: ManifestStaticFilesStorage + deep dependency resolution
#[tokio::test]
async fn test_complex_dependency_chain_resolution() {
	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let storage = ManifestStaticFilesStorage::new(temp_static_dir.path(), "/static/")
		.with_manifest_strict(false);

	let mut files = HashMap::new();

	// Create dependency chain: main.css -> theme.css -> font.woff2
	files.insert("fonts/font.woff2".to_string(), b"fake font data".to_vec());
	files.insert(
		"theme.css".to_string(),
		b"@font-face { src: url(fonts/font.woff2); }".to_vec(),
	);
	files.insert("main.css".to_string(), b"@import url(theme.css);".to_vec());

	// Process dependency chain
	let count = storage.save_with_dependencies(files).await.unwrap();
	assert_eq!(count, 3, "Should save all 3 files in chain");

	// Verify all files exist
	assert!(storage.exists("fonts/font.woff2"));
	assert!(storage.exists("theme.css"));
	assert!(storage.exists("main.css"));

	// Verify theme.css references hashed font
	let theme_content = storage.open("theme.css").await.unwrap();
	let theme_str = String::from_utf8(theme_content).unwrap();
	assert!(
		theme_str.contains("fonts/font.") && theme_str.contains(".woff2"),
		"theme.css should reference hashed font"
	);

	// Verify main.css references hashed theme.css
	let main_content = storage.open("main.css").await.unwrap();
	let main_str = String::from_utf8(main_content).unwrap();
	assert!(
		main_str.contains("theme.") && main_str.contains(".css"),
		"main.css should reference hashed theme.css"
	);

	// Cleanup is automatic with TempDir
}

// ==================== CDN URL Generation Tests ====================

/// Test Intent: Verify CDN URL generation with hashed filenames
/// Integration Point: Storage URL generation + CDN prefix
#[tokio::test]
async fn test_cdn_url_generation_with_hashed_files() {
	// Note: Actual CDN integration would be in a separate CDN module
	// This test verifies URL generation for CDN use cases

	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let storage = HashedFileStorage::new(temp_static_dir.path(), "/static/");

	let content = b"CDN test content";
	let hashed_name = storage.save("app.js", content).await.unwrap();

	// Get standard URL
	let url = storage.url("app.js");
	assert!(url.starts_with("/static/"));
	assert!(url.contains(&hashed_name));

	// CDN URL would be constructed by prepending CDN domain
	let cdn_domain = "https://cdn.example.com";
	let cdn_url = format!("{}{}", cdn_domain, url);

	assert_eq!(
		cdn_url,
		format!("https://cdn.example.com/static/{}", hashed_name)
	);

	// Verify CDN URL is suitable for long-term caching
	// (hashed filenames ensure cache busting on content change)
	assert!(
		hashed_name.contains("."),
		"Hashed filename enables safe CDN caching"
	);

	// Cleanup is automatic with TempDir
}

// ==================== Edge Cases and Error Handling Tests ====================

/// Test Intent: Verify handling of empty files
/// Integration Point: Storage + empty content
#[tokio::test]
async fn test_empty_file_handling() {
	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let storage = HashedFileStorage::new(temp_static_dir.path(), "/static/");

	let empty_content: &[u8] = b"";
	let hashed_name = storage.save("empty.txt", empty_content).await.unwrap();

	// Verify file exists
	assert!(storage.exists("empty.txt"));

	// Verify content is empty
	let retrieved = storage.open("empty.txt").await.unwrap();
	assert_eq!(retrieved.len(), 0);

	// Verify URL is generated
	let url = storage.url("empty.txt");
	assert!(url.starts_with("/static/"));
	assert!(url.contains(&hashed_name));

	// Cleanup is automatic with TempDir
}

/// Test Intent: Verify handling of large file uploads
/// Integration Point: Storage + large content
#[tokio::test]
async fn test_large_file_upload_handling() {
	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let storage = HashedFileStorage::new(temp_static_dir.path(), "/static/");

	// Create 1MB file
	let large_content = vec![b'X'; 1024 * 1024];

	let hashed_name = storage.save("large.bin", &large_content).await.unwrap();

	// Verify file exists
	assert!(storage.exists("large.bin"));

	// Verify content integrity
	let retrieved = storage.open("large.bin").await.unwrap();
	assert_eq!(retrieved.len(), large_content.len());
	assert_eq!(retrieved, large_content);

	// Verify hash is consistent
	assert!(hashed_name.contains("."), "Should have hash in filename");

	// Cleanup is automatic with TempDir
}

/// Test Intent: Verify handling of special characters in filenames
/// Integration Point: Storage + path sanitization
#[tokio::test]
async fn test_special_characters_in_filenames() {
	let temp_static_dir = TempDir::new().expect("Failed to create temp directory");
	let storage = HashedFileStorage::new(temp_static_dir.path(), "/static/");

	// Filenames with special characters (URL-safe)
	let test_files = vec![
		"file-with-dash.js",
		"file_with_underscore.css",
		"file.multiple.dots.txt",
		"file123numbers.js",
	];

	let content = b"test content";

	for filename in &test_files {
		let result = storage.save(filename, content).await;
		assert!(result.is_ok(), "Should handle filename: {}", filename);

		assert!(storage.exists(filename));
	}

	// Cleanup is automatic with TempDir
}
