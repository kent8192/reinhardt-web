//! Integration tests for Settings System
//!
//! These tests verify that reinhardt-conf settings works correctly with loading,
//! validation, and access patterns for composable settings fragments such as
//! `TemplateConfig` and `DatabaseConfig`.

use reinhardt_conf::settings::TemplateConfig;
use std::path::PathBuf;

// ============================================================================
// Template Configuration Tests
// ============================================================================

#[test]
fn test_template_config_default() {
	let config = TemplateConfig::default();

	assert_eq!(config.backend, "reinhardt.template.backends.jinja2.Jinja2");
	assert!(config.app_dirs);
	assert!(config.dirs.is_empty());
	assert!(config.options.contains_key("context_processors"));
}

#[test]
fn test_template_config_custom_dirs() {
	let config = TemplateConfig::new("MyTemplateBackend")
		.add_dir("/app/templates")
		.add_dir("/app/custom_templates");

	assert_eq!(config.backend, "MyTemplateBackend");
	assert_eq!(config.dirs.len(), 2);
	assert_eq!(config.dirs[0], PathBuf::from("/app/templates"));
	assert_eq!(config.dirs[1], PathBuf::from("/app/custom_templates"));
}
