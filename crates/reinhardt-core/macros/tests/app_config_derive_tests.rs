//! Tests for AppConfig attribute macro
//!
//! Tests that verify the correct generation of AppConfig factory methods.

use reinhardt_macros::app_config;

// Allow this test crate to be referenced as `::reinhardt` for proc macro generated code.
// The macro generates code with absolute paths like ::reinhardt::reinhardt_apps::AppConfig
extern crate self as reinhardt;

// Re-export modules for proc macro generated code paths.
pub mod reinhardt_apps {
	pub use ::reinhardt_apps::*;
}

pub mod macros {
	pub use reinhardt_macros::AppConfig;
}

#[app_config(name = "api", label = "api")]
pub struct ApiConfig;

#[app_config(name = "todos", label = "todos", verbose_name = "TODO Application")]
pub struct TodosConfig;

#[app_config(name = "users", label = "users")]
pub struct UsersConfig;

#[test]
fn test_basic_app_config() {
	let config = ApiConfig::config();
	assert_eq!(config.name, "api");
	assert_eq!(config.label, "api");
	assert_eq!(config.verbose_name, None);
}

#[test]
fn test_app_config_with_verbose_name() {
	let config = TodosConfig::config();
	assert_eq!(config.name, "todos");
	assert_eq!(config.label, "todos");
	assert_eq!(config.verbose_name, Some("TODO Application".to_string()));
}

#[test]
fn test_multiple_configs() {
	let api_config = ApiConfig::config();
	let todos_config = TodosConfig::config();
	let users_config = UsersConfig::config();

	assert_eq!(api_config.name, "api");
	assert_eq!(todos_config.name, "todos");
	assert_eq!(users_config.name, "users");
}

#[test]
fn test_config_builder_methods() {
	let config = ApiConfig::config()
		.with_path("/path/to/api")
		.with_default_auto_field("AutoField");

	assert_eq!(config.name, "api");
	assert_eq!(config.path, Some("/path/to/api".to_string()));
	assert_eq!(config.default_auto_field, Some("AutoField".to_string()));
}
