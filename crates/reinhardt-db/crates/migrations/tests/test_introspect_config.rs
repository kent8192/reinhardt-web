//! Unit tests for introspect configuration module
//!
//! Tests for configuration parsing and validation including:
//! - TOML parsing with valid/invalid content
//! - Table filtering with regex patterns
//! - CLI argument merging
//! - Environment variable resolution
//! - Default value verification
//!
//! **Test Categories:**
//! - Happy path: Valid configuration parsing
//! - Error path: Invalid TOML, missing required fields
//! - Edge cases: Empty patterns, special characters
//! - Decision table: Filter include/exclude combinations

use reinhardt_db::migrations::{
	GenerationConfig, IntrospectConfig, NamingConvention, OutputConfig, TableFilterConfig,
};
use rstest::*;
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// Happy Path Tests: Valid TOML Parsing
// ============================================================================

/// Test parsing minimal valid TOML
///
/// **Test Intent**: Verify minimal config can be parsed with defaults
#[rstest]
#[test]
fn test_parse_minimal_toml() {
	let toml = r#"
[database]
url = "postgres://localhost/test"
"#;

	let config = IntrospectConfig::from_toml(toml).unwrap();

	assert_eq!(config.database.url, "postgres://localhost/test");
	// Defaults should be applied
	assert_eq!(
		config.output.directory,
		PathBuf::from("src/models/generated")
	);
	assert_eq!(config.generation.app_label, "app");
}

/// Test parsing full TOML configuration
///
/// **Test Intent**: Verify all fields can be parsed
#[rstest]
#[test]
fn test_parse_full_toml() {
	let toml = r#"
[database]
url = "postgres://user:pass@localhost:5432/myapp"

[output]
directory = "src/models/generated"
single_file = true
single_file_name = "all_models.rs"

[generation]
app_label = "myapp"
detect_relationships = true
derives = ["Debug", "Clone", "Serialize"]
include_column_comments = false
struct_naming = "pascal_case"
field_naming = "snake_case"

[tables]
include = ["users", "posts", "comments"]
exclude = ["^pg_", "^reinhardt_", "temp_.*"]

[type_overrides]
"users.status" = "UserStatus"
"posts.category" = "PostCategory"

[imports]
additional = ["use crate::enums::*;", "use serde::{Serialize, Deserialize};"]
"#;

	let config = IntrospectConfig::from_toml(toml).unwrap();

	// Verify all sections parsed correctly
	assert_eq!(
		config.database.url,
		"postgres://user:pass@localhost:5432/myapp"
	);
	assert_eq!(
		config.output.directory,
		PathBuf::from("src/models/generated")
	);
	assert!(config.output.single_file);
	assert_eq!(config.output.single_file_name, "all_models.rs");
	assert_eq!(config.generation.app_label, "myapp");
	assert!(!config.generation.include_column_comments);
	assert_eq!(config.tables.include.len(), 3);
	assert_eq!(config.tables.exclude.len(), 3);
	assert_eq!(config.type_overrides.len(), 2);
	assert_eq!(config.imports.additional.len(), 2);
}

/// Test parsing with environment variable placeholder
///
/// **Test Intent**: Verify ${VAR} syntax is preserved for later resolution
#[rstest]
#[test]
fn test_parse_env_var_placeholder() {
	let toml = r#"
[database]
url = "${DATABASE_URL}"
"#;

	let config = IntrospectConfig::from_toml(toml).unwrap();
	assert_eq!(config.database.url, "${DATABASE_URL}");
}

// ============================================================================
// Error Path Tests: Invalid TOML
// ============================================================================

/// Test parsing invalid TOML syntax
///
/// **Test Intent**: Verify syntax errors return ParseError
#[rstest]
#[test]
fn test_parse_invalid_toml_syntax() {
	let toml = r#"
[database
url = "postgres://localhost/test"
"#;

	let result = IntrospectConfig::from_toml(toml);
	assert!(result.is_err());
}

/// Test parsing TOML with wrong types
///
/// **Test Intent**: Verify type mismatches return ParseError
#[rstest]
#[test]
fn test_parse_wrong_type_toml() {
	let toml = r#"
[database]
url = 123
"#;

	let result = IntrospectConfig::from_toml(toml);
	assert!(result.is_err());
}

/// Test parsing TOML with invalid array
///
/// **Test Intent**: Verify array type mismatches return ParseError
#[rstest]
#[test]
fn test_parse_invalid_array_toml() {
	let toml = r#"
[tables]
include = "not_an_array"
"#;

	let result = IntrospectConfig::from_toml(toml);
	assert!(result.is_err());
}

// ============================================================================
// Default Value Tests
// ============================================================================

/// Test default IntrospectConfig values
///
/// **Test Intent**: Verify all defaults are sensible
#[rstest]
#[test]
fn test_default_config_values() {
	let config = IntrospectConfig::default();

	// Database defaults
	assert!(config.database.url.is_empty());

	// Output defaults
	assert_eq!(
		config.output.directory,
		PathBuf::from("src/models/generated")
	);
	assert!(!config.output.single_file);
	assert_eq!(config.output.single_file_name, "models.rs");

	// Generation defaults
	assert_eq!(config.generation.app_label, "app");
	assert!(config.generation.detect_relationships);
	assert!(config.generation.include_column_comments);
	assert!(config.generation.derives.contains(&"Debug".to_string()));
	assert!(config.generation.derives.contains(&"Clone".to_string()));

	// Table filter defaults
	assert!(!config.tables.include.is_empty());
	assert!(!config.tables.exclude.is_empty());
	assert!(config.tables.exclude.iter().any(|p| p == "^pg_"));
}

/// Test OutputConfig defaults
///
/// **Test Intent**: Verify output defaults are sensible
#[rstest]
#[test]
fn test_output_config_defaults() {
	let output = OutputConfig::default();

	assert_eq!(output.directory, PathBuf::from("src/models/generated"));
	assert!(!output.single_file);
	assert_eq!(output.single_file_name, "models.rs");
}

/// Test GenerationConfig defaults
///
/// **Test Intent**: Verify generation defaults are sensible
#[rstest]
#[test]
fn test_generation_config_defaults() {
	let generation = GenerationConfig::default();

	assert_eq!(generation.app_label, "app");
	assert!(generation.detect_relationships);
	assert!(generation.include_column_comments);
	assert_eq!(
		generation.struct_naming_convention(),
		NamingConvention::PascalCase
	);
	// Rust convention: struct fields should be snake_case
	assert_eq!(
		generation.field_naming_convention(),
		NamingConvention::SnakeCase
	);
}

/// Test TableFilterConfig defaults
///
/// **Test Intent**: Verify filter defaults exclude system tables
#[rstest]
#[test]
fn test_table_filter_config_defaults() {
	let filter = TableFilterConfig::default();

	// Include all by default
	assert!(filter.include.contains(&".*".to_string()));

	// Exclude system tables
	assert!(filter.exclude.contains(&"^pg_".to_string()));
	assert!(
		filter
			.exclude
			.contains(&"^reinhardt_migrations".to_string())
	);
	assert!(filter.exclude.contains(&"^django_".to_string()));
}

// ============================================================================
// Table Filtering Tests
// ============================================================================

/// Test table filtering with include patterns
///
/// **Test Intent**: Verify include patterns work correctly
#[rstest]
#[case(&["users", "posts"], &[], "users", true)]
#[case(&["users", "posts"], &[], "posts", true)]
#[case(&["users", "posts"], &[], "comments", false)]
#[case(&["user.*"], &[], "users", true)]
#[case(&["user.*"], &[], "user_profiles", true)]
#[case(&["user.*"], &[], "posts", false)]
fn test_include_patterns(
	#[case] include: &[&str],
	#[case] exclude: &[&str],
	#[case] table_name: &str,
	#[case] expected: bool,
) {
	let config = IntrospectConfig {
		tables: TableFilterConfig {
			include: include.iter().map(|s| s.to_string()).collect(),
			exclude: exclude.iter().map(|s| s.to_string()).collect(),
		},
		..Default::default()
	};

	assert_eq!(config.should_include_table(table_name), expected);
}

/// Test table filtering with exclude patterns
///
/// **Test Intent**: Verify exclude patterns take precedence
#[rstest]
#[case(&[".*"], &["^pg_"], "users", true)]
#[case(&[".*"], &["^pg_"], "pg_tables", false)]
#[case(&[".*"], &["^pg_", "temp_.*"], "temp_data", false)]
#[case(&[".*"], &["^pg_", "temp_.*"], "permanent_data", true)]
fn test_exclude_patterns(
	#[case] include: &[&str],
	#[case] exclude: &[&str],
	#[case] table_name: &str,
	#[case] expected: bool,
) {
	let config = IntrospectConfig {
		tables: TableFilterConfig {
			include: include.iter().map(|s| s.to_string()).collect(),
			exclude: exclude.iter().map(|s| s.to_string()).collect(),
		},
		..Default::default()
	};

	assert_eq!(config.should_include_table(table_name), expected);
}

/// Test table filtering with empty patterns
///
/// **Test Intent**: Verify empty patterns behave correctly
#[rstest]
#[test]
fn test_empty_include_patterns() {
	let config = IntrospectConfig {
		tables: TableFilterConfig {
			include: vec![],
			exclude: vec![],
		},
		..Default::default()
	};

	// Empty include means include all
	assert!(config.should_include_table("any_table"));
}

/// Test table filtering excludes take precedence over includes
///
/// **Test Intent**: Verify exclude wins when both match
#[rstest]
#[test]
fn test_exclude_takes_precedence() {
	let config = IntrospectConfig {
		tables: TableFilterConfig {
			include: vec![".*".to_string()],
			exclude: vec!["excluded_table".to_string()],
		},
		..Default::default()
	};

	assert!(config.should_include_table("any_table"));
	assert!(!config.should_include_table("excluded_table"));
}

// ============================================================================
// Type Override Tests
// ============================================================================

/// Test type override retrieval
///
/// **Test Intent**: Verify type overrides can be retrieved
#[rstest]
#[test]
fn test_type_override_retrieval() {
	let mut type_overrides = HashMap::new();
	type_overrides.insert("users.status".to_string(), "UserStatus".to_string());
	type_overrides.insert("posts.category".to_string(), "PostCategory".to_string());

	let config = IntrospectConfig {
		type_overrides,
		..Default::default()
	};

	assert_eq!(
		config.get_type_override("users", "status"),
		Some("UserStatus")
	);
	assert_eq!(
		config.get_type_override("posts", "category"),
		Some("PostCategory")
	);
	assert_eq!(config.get_type_override("users", "name"), None);
	assert_eq!(config.get_type_override("other", "status"), None);
}

// ============================================================================
// Builder Pattern Tests
// ============================================================================

/// Test builder pattern chaining
///
/// **Test Intent**: Verify builder methods can be chained
#[rstest]
#[test]
fn test_builder_pattern_chaining() {
	let config = IntrospectConfig::default()
		.with_database_url("postgres://localhost/test")
		.with_output_dir("./output")
		.with_app_label("test_app");

	assert_eq!(config.database.url, "postgres://localhost/test");
	assert_eq!(config.output.directory, PathBuf::from("./output"));
	assert_eq!(config.generation.app_label, "test_app");
}

/// Test builder preserves other defaults
///
/// **Test Intent**: Verify setting one field doesn't affect others
#[rstest]
#[test]
fn test_builder_preserves_defaults() {
	let config = IntrospectConfig::default().with_database_url("postgres://localhost/test");

	// Changed field
	assert_eq!(config.database.url, "postgres://localhost/test");

	// Preserved defaults
	assert_eq!(
		config.output.directory,
		PathBuf::from("src/models/generated")
	);
	assert_eq!(config.generation.app_label, "app");
}

// ============================================================================
// CLI Args Merge Tests
// ============================================================================

/// Test CLI args override config values
///
/// **Test Intent**: Verify CLI args take precedence
#[rstest]
#[test]
fn test_cli_args_override() {
	use reinhardt_db::migrations::introspect::CliArgs;

	let mut config = IntrospectConfig::default()
		.with_database_url("postgres://config/db")
		.with_app_label("config_app");

	let args = CliArgs {
		database_url: Some("postgres://cli/db".to_string()),
		app_label: Some("cli_app".to_string()),
		..Default::default()
	};

	config.merge_cli_args(&args);

	// CLI values should override
	assert_eq!(config.database.url, "postgres://cli/db");
	assert_eq!(config.generation.app_label, "cli_app");
}

/// Test CLI args with None values don't override
///
/// **Test Intent**: Verify None CLI args preserve config values
#[rstest]
#[test]
fn test_cli_args_none_preserves() {
	use reinhardt_db::migrations::introspect::CliArgs;

	let mut config = IntrospectConfig::default()
		.with_database_url("postgres://config/db")
		.with_app_label("config_app");

	let args = CliArgs::default();

	config.merge_cli_args(&args);

	// Original values should be preserved
	assert_eq!(config.database.url, "postgres://config/db");
	assert_eq!(config.generation.app_label, "config_app");
}

/// Test CLI args with output directory
///
/// **Test Intent**: Verify output directory can be overridden
#[rstest]
#[test]
fn test_cli_args_output_dir() {
	use reinhardt_db::migrations::introspect::CliArgs;

	let mut config = IntrospectConfig::default();

	let args = CliArgs {
		output_dir: Some(PathBuf::from("/custom/output")),
		..Default::default()
	};

	config.merge_cli_args(&args);

	assert_eq!(config.output.directory, PathBuf::from("/custom/output"));
}

/// Test CLI args add to exclude patterns
///
/// **Test Intent**: Verify exclude patterns are appended
#[rstest]
#[test]
fn test_cli_args_exclude_append() {
	use reinhardt_db::migrations::introspect::CliArgs;

	let mut config = IntrospectConfig {
		tables: TableFilterConfig {
			include: vec![".*".to_string()],
			exclude: vec!["^pg_".to_string()],
		},
		..Default::default()
	};

	let args = CliArgs {
		exclude_tables: Some("temp_.*".to_string()),
		..Default::default()
	};

	config.merge_cli_args(&args);

	// Both patterns should be present
	assert!(config.tables.exclude.contains(&"^pg_".to_string()));
	assert!(config.tables.exclude.contains(&"temp_.*".to_string()));
}

// ============================================================================
// Naming Convention Tests
// ============================================================================

/// Test naming convention config conversion
///
/// **Test Intent**: Verify NamingConventionConfig converts correctly
#[rstest]
#[test]
fn test_naming_convention_struct() {
	let toml = r#"
[generation]
struct_naming = "pascal_case"
field_naming = "snake_case"
"#;

	let config = IntrospectConfig::from_toml(toml).unwrap();

	assert_eq!(
		config.generation.struct_naming_convention(),
		NamingConvention::PascalCase
	);
	assert_eq!(
		config.generation.field_naming_convention(),
		NamingConvention::SnakeCase
	);
}

/// Test naming convention preserve option
///
/// **Test Intent**: Verify preserve naming convention works
#[rstest]
#[test]
fn test_naming_convention_preserve() {
	let toml = r#"
[generation]
struct_naming = "preserve"
field_naming = "preserve"
"#;

	let config = IntrospectConfig::from_toml(toml).unwrap();

	assert_eq!(
		config.generation.struct_naming_convention(),
		NamingConvention::Preserve
	);
	assert_eq!(
		config.generation.field_naming_convention(),
		NamingConvention::Preserve
	);
}

// ============================================================================
// Decision Table Tests: Filter Combinations
// ============================================================================

/// Decision table for table filtering
///
/// **Test Intent**: Verify all filter combinations work correctly
///
/// | include_empty | exclude_empty | matches_include | matches_exclude | result |
/// |---------------|---------------|-----------------|-----------------|--------|
/// | true          | true          | N/A             | N/A             | true   |
/// | false         | true          | true            | N/A             | true   |
/// | false         | true          | false           | N/A             | false  |
/// | true          | false         | N/A             | true            | false  |
/// | true          | false         | N/A             | false           | true   |
/// | false         | false         | true            | true            | false  |
/// | false         | false         | true            | false           | true   |
/// | false         | false         | false           | true            | false  |
/// | false         | false         | false           | false           | false  |
#[rstest]
#[case(vec![], vec![], "any_table", true)] // Both empty
#[case(vec!["users"], vec![], "users", true)] // Include match
#[case(vec!["users"], vec![], "posts", false)] // Include no match
#[case(vec![], vec!["^temp_"], "temp_data", false)] // Exclude match
#[case(vec![], vec!["^temp_"], "permanent", true)] // Exclude no match
#[case(vec![".*"], vec!["^temp_"], "temp_data", false)] // Both, exclude wins
#[case(vec![".*"], vec!["^temp_"], "permanent", true)] // Both, no exclude match
#[case(vec!["users"], vec!["^temp_"], "users", true)] // Specific include
#[case(vec!["users"], vec!["users"], "users", false)] // Exclude overrides include
fn test_filter_decision_table(
	#[case] include: Vec<&str>,
	#[case] exclude: Vec<&str>,
	#[case] table_name: &str,
	#[case] expected: bool,
) {
	let config = IntrospectConfig {
		tables: TableFilterConfig {
			include: include.iter().map(|s| s.to_string()).collect(),
			exclude: exclude.iter().map(|s| s.to_string()).collect(),
		},
		..Default::default()
	};

	assert_eq!(
		config.should_include_table(table_name),
		expected,
		"Failed for include={:?}, exclude={:?}, table={}",
		include,
		exclude,
		table_name
	);
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Test config with special characters in patterns
///
/// **Test Intent**: Verify regex special chars work in patterns
#[rstest]
#[test]
fn test_special_chars_in_patterns() {
	let config = IntrospectConfig {
		tables: TableFilterConfig {
			include: vec!["user\\..*".to_string()], // escaped dot
			exclude: vec![],
		},
		..Default::default()
	};

	assert!(config.should_include_table("user.profile"));
	assert!(!config.should_include_table("user_profile"));
}

/// Test config with invalid regex pattern (should not crash)
///
/// **Test Intent**: Verify invalid regex doesn't cause panic
#[rstest]
#[test]
fn test_invalid_regex_pattern_no_crash() {
	let config = IntrospectConfig {
		tables: TableFilterConfig {
			include: vec!["[invalid".to_string()], // Invalid regex
			exclude: vec![],
		},
		..Default::default()
	};

	// Should not panic, just return false
	let result = config.should_include_table("any_table");
	assert!(!result);
}

/// Test empty database URL
///
/// **Test Intent**: Verify empty URL is handled
#[rstest]
#[test]
fn test_empty_database_url() {
	let config = IntrospectConfig::default();

	assert!(config.database.url.is_empty());
}
