//! File system template loader
//!
//! Provides functionality to load templates from the file system with security checks

use crate::{TemplateError, TemplateResult};
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::RwLock;

/// File system template loader
///
/// Loads templates from the file system with security checks to prevent
/// directory traversal attacks and handle file permissions properly.
#[derive(Debug)]
pub struct FileSystemTemplateLoader {
	/// Base directory for templates
	base_dir: PathBuf,
	/// Cache of loaded templates
	cache: RwLock<HashMap<String, String>>,
	/// Whether to use caching
	use_cache: bool,
}

impl FileSystemTemplateLoader {
	/// Create a new file system template loader
	///
	/// # Arguments
	///
	/// * `base_dir` - Base directory where templates are stored
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_templates::FileSystemTemplateLoader;
	/// use std::path::Path;
	///
	/// let loader = FileSystemTemplateLoader::new(Path::new("/app/templates"));
	/// ```
	pub fn new(base_dir: &Path) -> Self {
		Self {
			base_dir: base_dir.to_path_buf(),
			cache: RwLock::new(HashMap::new()),
			use_cache: true,
		}
	}

	/// Create a loader without caching
	pub fn new_without_cache(base_dir: &Path) -> Self {
		Self {
			base_dir: base_dir.to_path_buf(),
			cache: RwLock::new(HashMap::new()),
			use_cache: false,
		}
	}

	/// Load a template by name
	///
	/// # Security
	///
	/// This method checks for directory traversal attempts and ensures
	/// that only files within the base directory can be accessed.
	///
	/// # Arguments
	///
	/// * `name` - Template name (path relative to base directory)
	///
	/// # Errors
	///
	/// Returns error if:
	/// - Template path contains directory traversal attempts
	/// - Template file doesn't exist
	/// - Template file cannot be read (permissions)
	/// - Path points to a directory instead of a file
	pub fn load(&self, name: &str) -> TemplateResult<String> {
		// Check cache first if caching is enabled
		if self.use_cache {
			let cache = self.cache.read().unwrap();
			if let Some(content) = cache.get(name) {
				return Ok(content.clone());
			}
		}

		// Validate and construct the full path
		let full_path = self.validate_and_resolve_path(name)?;

		// Check if path exists
		if !full_path.exists() {
			return Err(TemplateError::TemplateNotFound(name.to_string()));
		}

		// Check if it's a file (not a directory)
		if full_path.is_dir() {
			return Err(TemplateError::TemplateNotFound(format!(
				"{} is a directory",
				name
			)));
		}

		// Read the file
		let content = fs::read_to_string(&full_path)
			.map_err(|e| TemplateError::TemplateNotFound(format!("Cannot read {}: {}", name, e)))?;

		// Cache the content if caching is enabled
		if self.use_cache {
			let mut cache = self.cache.write().unwrap();
			cache.insert(name.to_string(), content.clone());
		}

		Ok(content)
	}

	/// Validate path and resolve to absolute path
	///
	/// This prevents directory traversal attacks by ensuring the resolved
	/// path is within the base directory.
	fn validate_and_resolve_path(&self, name: &str) -> TemplateResult<PathBuf> {
		// Normalize the template name (remove leading slashes)
		let normalized_name = name.trim_start_matches('/');

		// Check for directory traversal attempts
		let template_path = Path::new(normalized_name);
		for component in template_path.components() {
			match component {
				Component::ParentDir => {
					return Err(TemplateError::TemplateNotFound(format!(
						"Directory traversal attempt detected in: {}",
						name
					)));
				}
				Component::RootDir => {
					return Err(TemplateError::TemplateNotFound(format!(
						"Absolute path not allowed: {}",
						name
					)));
				}
				_ => {}
			}
		}

		// Construct full path
		let full_path = self.base_dir.join(normalized_name);

		// Canonicalize both paths to ensure the template path is within base_dir
		// Note: This requires the base_dir to exist
		if let Ok(canonical_full) = full_path.canonicalize() {
			if let Ok(canonical_base) = self.base_dir.canonicalize() {
				if !canonical_full.starts_with(&canonical_base) {
					return Err(TemplateError::TemplateNotFound(format!(
						"Path escapes base directory: {}",
						name
					)));
				}
			}
		}

		Ok(full_path)
	}

	/// Clear the template cache
	pub fn clear_cache(&self) {
		let mut cache = self.cache.write().unwrap();
		cache.clear();
	}

	/// Get the base directory
	pub fn base_dir(&self) -> &Path {
		&self.base_dir
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::fs;
	use std::io::Write;
	use tempfile::TempDir;

	fn create_test_template(dir: &Path, name: &str, content: &str) -> std::io::Result<()> {
		let file_path = dir.join(name);
		if let Some(parent) = file_path.parent() {
			fs::create_dir_all(parent)?;
		}
		let mut file = fs::File::create(file_path)?;
		file.write_all(content.as_bytes())?;
		Ok(())
	}

	#[test]
	fn test_load_template() {
		let temp_dir = TempDir::new().unwrap();
		create_test_template(temp_dir.path(), "test.html", "Hello {{ name }}!").unwrap();

		let loader = FileSystemTemplateLoader::new(temp_dir.path());
		let content = loader.load("test.html").unwrap();

		assert_eq!(content, "Hello {{ name }}!");
	}

	#[test]
	fn test_load_template_not_found() {
		let temp_dir = TempDir::new().unwrap();
		let loader = FileSystemTemplateLoader::new(temp_dir.path());

		let result = loader.load("nonexistent.html");
		assert!(result.is_err());
	}

	#[test]
	fn test_directory_traversal_prevention() {
		let temp_dir = TempDir::new().unwrap();
		let loader = FileSystemTemplateLoader::new(temp_dir.path());

		// Try various directory traversal attempts
		let result = loader.load("../etc/passwd");
		assert!(result.is_err());

		let result = loader.load("../../secret.txt");
		assert!(result.is_err());

		let result = loader.load("./../../secrets/key.txt");
		assert!(result.is_err());
	}

	#[test]
	fn test_absolute_path_prevention() {
		let temp_dir = TempDir::new().unwrap();
		let loader = FileSystemTemplateLoader::new(temp_dir.path());

		let result = loader.load("/etc/passwd");
		assert!(result.is_err());
	}

	#[test]
	fn test_load_template_from_subdirectory() {
		let temp_dir = TempDir::new().unwrap();
		create_test_template(
			temp_dir.path(),
			"sub/template.html",
			"Subdirectory template",
		)
		.unwrap();

		let loader = FileSystemTemplateLoader::new(temp_dir.path());
		let content = loader.load("sub/template.html").unwrap();

		assert_eq!(content, "Subdirectory template");
	}

	#[test]
	fn test_template_caching() {
		let temp_dir = TempDir::new().unwrap();
		create_test_template(temp_dir.path(), "cached.html", "Original content").unwrap();

		let loader = FileSystemTemplateLoader::new(temp_dir.path());

		// First load
		let content1 = loader.load("cached.html").unwrap();
		assert_eq!(content1, "Original content");

		// Modify the file
		create_test_template(temp_dir.path(), "cached.html", "Modified content").unwrap();

		// Second load (should return cached content)
		let content2 = loader.load("cached.html").unwrap();
		assert_eq!(content2, "Original content"); // Still original due to cache

		// Clear cache and load again
		loader.clear_cache();
		let content3 = loader.load("cached.html").unwrap();
		assert_eq!(content3, "Modified content"); // Now sees the modification
	}

	#[test]
	fn test_no_cache_loader() {
		let temp_dir = TempDir::new().unwrap();
		create_test_template(temp_dir.path(), "nocache.html", "First").unwrap();

		let loader = FileSystemTemplateLoader::new_without_cache(temp_dir.path());

		// First load
		let content1 = loader.load("nocache.html").unwrap();
		assert_eq!(content1, "First");

		// Modify the file
		create_test_template(temp_dir.path(), "nocache.html", "Second").unwrap();

		// Second load (should see new content immediately)
		let content2 = loader.load("nocache.html").unwrap();
		assert_eq!(content2, "Second");
	}

	#[test]
	fn test_directory_instead_of_file() {
		let temp_dir = TempDir::new().unwrap();
		fs::create_dir_all(temp_dir.path().join("subdir")).unwrap();

		let loader = FileSystemTemplateLoader::new(temp_dir.path());
		let result = loader.load("subdir");

		assert!(result.is_err());
	}

	#[test]
	fn test_unicode_template_name() {
		let temp_dir = TempDir::new().unwrap();
		create_test_template(temp_dir.path(), "日本語.html", "Japanese template").unwrap();

		let loader = FileSystemTemplateLoader::new(temp_dir.path());
		let content = loader.load("日本語.html").unwrap();

		assert_eq!(content, "Japanese template");
	}

	// ============================================================================
	// Additional comprehensive file system loader tests
	// ============================================================================

	#[test]
	fn test_unicode_directory_name() {
		let temp_dir = TempDir::new().unwrap();
		create_test_template(
			temp_dir.path(),
			"テンプレート/日本語.html",
			"Japanese template in Japanese directory",
		)
		.unwrap();

		let loader = FileSystemTemplateLoader::new(temp_dir.path());
		let content = loader.load("テンプレート/日本語.html").unwrap();

		assert_eq!(content, "Japanese template in Japanese directory");
	}

	#[test]
	fn test_emoji_template_name() {
		let temp_dir = TempDir::new().unwrap();
		create_test_template(temp_dir.path(), "🚀🌍.html", "Emoji template").unwrap();

		let loader = FileSystemTemplateLoader::new(temp_dir.path());
		let content = loader.load("🚀🌍.html").unwrap();

		assert_eq!(content, "Emoji template");
	}

	#[test]
	fn test_special_characters_in_path() {
		let temp_dir = TempDir::new().unwrap();
		create_test_template(
			temp_dir.path(),
			"special-chars_underscore.html",
			"Special characters template",
		)
		.unwrap();

		let loader = FileSystemTemplateLoader::new(temp_dir.path());
		let content = loader.load("special-chars_underscore.html").unwrap();

		assert_eq!(content, "Special characters template");
	}

	#[test]
	fn test_deep_nested_directory() {
		let temp_dir = TempDir::new().unwrap();
		create_test_template(
			temp_dir.path(),
			"level1/level2/level3/deep.html",
			"Deep nested template",
		)
		.unwrap();

		let loader = FileSystemTemplateLoader::new(temp_dir.path());
		let content = loader.load("level1/level2/level3/deep.html").unwrap();

		assert_eq!(content, "Deep nested template");
	}

	#[test]
	fn test_leading_slash_handling() {
		let temp_dir = TempDir::new().unwrap();
		create_test_template(temp_dir.path(), "test.html", "Test content").unwrap();

		let loader = FileSystemTemplateLoader::new(temp_dir.path());

		// Both should work the same way
		let content1 = loader.load("test.html").unwrap();
		let content2 = loader.load("/test.html").unwrap();

		assert_eq!(content1, content2);
		assert_eq!(content1, "Test content");
	}

	#[test]
	fn test_multiple_leading_slashes() {
		let temp_dir = TempDir::new().unwrap();
		create_test_template(temp_dir.path(), "test.html", "Test content").unwrap();

		let loader = FileSystemTemplateLoader::new(temp_dir.path());
		let content = loader.load("///test.html").unwrap();

		assert_eq!(content, "Test content");
	}

	#[test]
	fn test_dot_dot_variations() {
		let temp_dir = TempDir::new().unwrap();
		let loader = FileSystemTemplateLoader::new(temp_dir.path());

		// Various directory traversal attempts
		let malicious_paths = vec![
			"../test.html",
			"../../test.html",
			"./../test.html",
			"test/../test.html",
			"test/../../test.html",
			"test/./../test.html",
			"test/../other/test.html",
			"test/../..//test.html",
		];

		for path in malicious_paths {
			let result = loader.load(path);
			assert!(
				result.is_err(),
				"Directory traversal should be blocked for: {}",
				path
			);
		}
	}

	#[test]
	fn test_absolute_path_variations() {
		let temp_dir = TempDir::new().unwrap();
		let loader = FileSystemTemplateLoader::new(temp_dir.path());

		// Various absolute path attempts
		let absolute_paths = vec![
			"/etc/passwd",
			"/home/user/secret.txt",
			"C:\\Windows\\System32\\config\\SAM", // Windows path
			"\\etc\\passwd",                      // Windows absolute path
		];

		for path in absolute_paths {
			let result = loader.load(path);
			assert!(
				result.is_err(),
				"Absolute path should be blocked for: {}",
				path
			);
		}
	}

	#[test]
	fn test_empty_template_name() {
		let temp_dir = TempDir::new().unwrap();
		let loader = FileSystemTemplateLoader::new(temp_dir.path());

		let result = loader.load("");
		assert!(result.is_err());
	}

	#[test]
	fn test_whitespace_only_template_name() {
		let temp_dir = TempDir::new().unwrap();
		let loader = FileSystemTemplateLoader::new(temp_dir.path());

		let result = loader.load("   ");
		assert!(result.is_err());
	}

	#[test]
	fn test_template_name_with_only_slashes() {
		let temp_dir = TempDir::new().unwrap();
		let loader = FileSystemTemplateLoader::new(temp_dir.path());

		let result = loader.load("///");
		assert!(result.is_err());
	}

	#[test]
	fn test_cache_behavior_with_multiple_templates() {
		let temp_dir = TempDir::new().unwrap();
		create_test_template(temp_dir.path(), "template1.html", "Template 1").unwrap();
		create_test_template(temp_dir.path(), "template2.html", "Template 2").unwrap();

		let loader = FileSystemTemplateLoader::new(temp_dir.path());

		// Load both templates
		let content1 = loader.load("template1.html").unwrap();
		let content2 = loader.load("template2.html").unwrap();

		assert_eq!(content1, "Template 1");
		assert_eq!(content2, "Template 2");

		// Load again (should use cache)
		let content1_cached = loader.load("template1.html").unwrap();
		let content2_cached = loader.load("template2.html").unwrap();

		assert_eq!(content1, content1_cached);
		assert_eq!(content2, content2_cached);
	}

	#[test]
	fn test_cache_clear_affects_all_templates() {
		let temp_dir = TempDir::new().unwrap();
		create_test_template(temp_dir.path(), "template1.html", "Original 1").unwrap();
		create_test_template(temp_dir.path(), "template2.html", "Original 2").unwrap();

		let loader = FileSystemTemplateLoader::new(temp_dir.path());

		// Load templates
		loader.load("template1.html").unwrap();
		loader.load("template2.html").unwrap();

		// Modify both files
		create_test_template(temp_dir.path(), "template1.html", "Modified 1").unwrap();
		create_test_template(temp_dir.path(), "template2.html", "Modified 2").unwrap();

		// Clear cache
		loader.clear_cache();

		// Load again (should see modifications)
		let content1 = loader.load("template1.html").unwrap();
		let content2 = loader.load("template2.html").unwrap();

		assert_eq!(content1, "Modified 1");
		assert_eq!(content2, "Modified 2");
	}

	#[test]
	fn test_large_template_file() {
		let temp_dir = TempDir::new().unwrap();
		let large_content = "A".repeat(10000); // 10KB content
		create_test_template(temp_dir.path(), "large.html", &large_content).unwrap();

		let loader = FileSystemTemplateLoader::new(temp_dir.path());
		let content = loader.load("large.html").unwrap();

		assert_eq!(content, large_content);
		assert_eq!(content.len(), 10000);
	}

	#[test]
	fn test_binary_template_file() {
		let temp_dir = TempDir::new().unwrap();
		let binary_content = vec![0x00, 0x01, 0x02, 0x03, 0xFF, 0xFE, 0xFD];
		let file_path = temp_dir.path().join("binary.html");
		fs::write(file_path, &binary_content).unwrap();

		let loader = FileSystemTemplateLoader::new(temp_dir.path());
		let result = loader.load("binary.html");

		// Should fail because it's not valid UTF-8
		assert!(result.is_err());
	}

	#[test]
	fn test_template_with_null_bytes() {
		let temp_dir = TempDir::new().unwrap();
		let content_with_null = "Hello\0World";
		create_test_template(temp_dir.path(), "null.html", content_with_null).unwrap();

		let loader = FileSystemTemplateLoader::new(temp_dir.path());
		let content = loader.load("null.html").unwrap();

		assert_eq!(content, content_with_null);
	}

	#[test]
	fn test_concurrent_access() {
		let temp_dir = TempDir::new().unwrap();
		create_test_template(temp_dir.path(), "concurrent.html", "Concurrent test").unwrap();

		let loader = std::sync::Arc::new(FileSystemTemplateLoader::new(temp_dir.path()));

		// Spawn multiple threads to access the same template
		let handles: Vec<_> = (0..10)
			.map(|_| {
				let loader = loader.clone();
				std::thread::spawn(move || loader.load("concurrent.html"))
			})
			.collect();

		// All threads should succeed
		for handle in handles {
			let result = handle.join().unwrap();
			assert!(result.is_ok());
			assert_eq!(result.unwrap(), "Concurrent test");
		}
	}

	#[test]
	fn test_base_dir_access() {
		let temp_dir = TempDir::new().unwrap();
		let loader = FileSystemTemplateLoader::new(temp_dir.path());

		assert_eq!(loader.base_dir(), temp_dir.path());
	}

	#[test]
	fn test_template_name_normalization() {
		let temp_dir = TempDir::new().unwrap();
		create_test_template(temp_dir.path(), "normalize.html", "Normalized").unwrap();

		let loader = FileSystemTemplateLoader::new(temp_dir.path());

		// All these should resolve to the same file
		let variations = vec![
			"normalize.html",
			"/normalize.html",
			"//normalize.html",
			"///normalize.html",
		];

		for variation in variations {
			let content = loader.load(variation).unwrap();
			assert_eq!(content, "Normalized");
		}
	}
}
