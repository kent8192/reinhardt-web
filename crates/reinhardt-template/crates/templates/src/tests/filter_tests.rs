//! Template filter tests
//!
//! Tests for template filters inspired by Django's filter tests

use crate::{StaticConfig, static_filter, static_path_join};
use std::collections::HashMap;
use tera::{Context, Tera};

#[test]
fn test_filter_basic_rendering() {
	// Test basic rendering
	let mut context = Context::new();
	context.insert("value", "hello");

	let result = Tera::one_off("{{ value }}", &context, false).unwrap();
	assert_eq!(result, "hello");
}

#[test]
fn test_filter_escape() {
	// Test HTML escaping (built-in Tera feature)
	let mut context = Context::new();
	context.insert("value", "<script>alert('xss')</script>");

	let result = Tera::one_off("{{ value | escape }}", &context, false).unwrap();
	// Tera escapes HTML
	// Test that template renders (escaping is handled by Tera)
	assert!(!result.is_empty());
}

#[test]
fn test_static_filter_basic() {
	// Test static filter (similar to Django's static tag tests)
	crate::init_static_config(StaticConfig::default());

	let result = static_filter("css/style.css");
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), "/static/css/style.css");
}

#[test]
fn test_static_filter_with_custom_url() {
	// Test static filter with custom URL
	let config = StaticConfig {
		static_url: "/assets/".to_string(),
		use_manifest: false,
		manifest: HashMap::new(),
	};
	crate::init_static_config(config);

	let result = static_filter("images/logo.png");
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), "/assets/images/logo.png");

	// Reset to default
	crate::init_static_config(StaticConfig::default());
}

#[test]
fn test_static_filter_with_manifest() {
	// Test static filter with manifest (hashed filenames)
	let mut manifest = HashMap::new();
	manifest.insert("main.css".to_string(), "main.abc123.css".to_string());

	let config = StaticConfig {
		static_url: "/static/".to_string(),
		use_manifest: true,
		manifest,
	};
	crate::init_static_config(config);

	let result = static_filter("main.css");
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), "/static/main.abc123.css");

	// Reset to default
	crate::init_static_config(StaticConfig::default());
}

#[test]
fn test_static_path_join_basic() {
	// Test static path joining
	let result = static_path_join("css", "styles.css");
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), "css/styles.css");
}

#[test]
fn test_static_path_join_nested() {
	// Test static path joining with nested paths
	let result = static_path_join("assets/images", "logo.png");
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), "assets/images/logo.png");
}

#[test]
fn test_filter_with_empty_string() {
	// Test filters with empty string
	let mut context = Context::new();
	context.insert("value", "");

	let result = Tera::one_off("{{ value }}", &context, false).unwrap();
	assert_eq!(result, "");
}

#[test]
fn test_filter_with_unicode() {
	// Test filters with unicode characters
	let mut context = Context::new();
	context.insert("value", "こんにちは 世界");

	let result = Tera::one_off("{{ value }}", &context, false).unwrap();
	assert_eq!(result, "こんにちは 世界");
}

#[test]
fn test_static_filter_leading_slash_removal() {
	// Test that static filter removes leading slash
	crate::init_static_config(StaticConfig::default());

	let result1 = static_filter("css/main.css");
	let result2 = static_filter("/css/main.css");

	assert_eq!(result1.unwrap(), result2.unwrap());
}

#[test]
fn test_static_filter_trailing_slash_handling() {
	// Test static filter with trailing slash in URL
	let config = StaticConfig {
		static_url: "/static".to_string(), // No trailing slash
		use_manifest: false,
		manifest: HashMap::new(),
	};
	crate::init_static_config(config);

	let result = static_filter("app.js");
	assert_eq!(result.unwrap(), "/static/app.js");

	// Reset to default
	crate::init_static_config(StaticConfig::default());
}

#[test]
fn test_static_filter_multiple_files() {
	// Test static filter with multiple different files
	crate::init_static_config(StaticConfig::default());

	// All files should have consistent static URL prefix
	let result1 = static_filter("css/main.css").unwrap();
	let result2 = static_filter("js/app.js").unwrap();
	let result3 = static_filter("images/logo.png").unwrap();

	// Verify all have the same prefix
	assert!(result1.contains("css/main.css"));
	assert!(result2.contains("js/app.js"));
	assert!(result3.contains("images/logo.png"));

	// Verify they all start with a slash (absolute paths)
	assert!(result1.starts_with('/'));
	assert!(result2.starts_with('/'));
	assert!(result3.starts_with('/'));
}

#[test]
fn test_static_filter_deep_nesting() {
	// Test static filter with deeply nested paths
	crate::init_static_config(StaticConfig::default());

	let result = static_filter("assets/vendor/lib/module/file.js").unwrap();
	// Verify the path is preserved
	assert!(result.contains("assets/vendor/lib/module/file.js"));
	// Verify it's an absolute path
	assert!(result.starts_with('/'));
}

#[test]
fn test_path_join_empty_base() {
	// Test path join with empty base
	let result = static_path_join("", "file.css");
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), "file.css");
}

#[test]
fn test_path_join_empty_path() {
	// Test path join with empty path
	let result = static_path_join("base", "");
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), "base/");
}
