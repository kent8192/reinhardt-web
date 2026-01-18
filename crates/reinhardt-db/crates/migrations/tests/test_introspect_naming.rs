//! Unit tests for introspect naming module
//!
//! Tests for naming convention utilities including:
//! - PascalCase and snake_case conversions
//! - Rust keyword escaping
//! - Identifier sanitization
//! - Boundary value analysis
//! - Equivalence partitioning for naming patterns
//!
//! **Test Categories:**
//! - Happy path: Normal naming conversions
//! - Edge cases: Empty strings, special characters, Unicode
//! - Boundary values: Maximum length identifiers
//! - Decision table: Naming convention combinations

use reinhardt_db::migrations::{
	NamingConvention, escape_rust_keyword, sanitize_identifier, to_pascal_case, to_snake_case,
};
use rstest::*;

// ============================================================================
// Happy Path Tests
// ============================================================================

/// Test PascalCase conversion for common snake_case inputs
///
/// **Test Intent**: Verify standard snake_case → PascalCase conversions
#[rstest]
#[case("users", "Users")]
#[case("user_profiles", "UserProfiles")]
#[case("my_table_name", "MyTableName")]
#[case("orders", "Orders")]
#[case("order_items", "OrderItems")]
fn test_to_pascal_case_from_snake_case(#[case] input: &str, #[case] expected: &str) {
	let result = to_pascal_case(input);
	assert_eq!(result, expected);
}

/// Test PascalCase conversion preserves already PascalCase
///
/// **Test Intent**: Ensure PascalCase input is not mangled
#[rstest]
#[case("Users", "Users")]
#[case("UserProfiles", "UserProfiles")]
#[case("MyTableName", "MyTableName")]
fn test_to_pascal_case_preserves_pascal(#[case] input: &str, #[case] expected: &str) {
	let result = to_pascal_case(input);
	assert_eq!(result, expected);
}

/// Test snake_case conversion for common PascalCase inputs
///
/// **Test Intent**: Verify standard PascalCase → snake_case conversions
#[rstest]
#[case("Users", "users")]
#[case("UserProfiles", "user_profiles")]
#[case("MyTableName", "my_table_name")]
#[case("Orders", "orders")]
#[case("OrderItems", "order_items")]
fn test_to_snake_case_from_pascal_case(#[case] input: &str, #[case] expected: &str) {
	let result = to_snake_case(input);
	assert_eq!(result, expected);
}

/// Test snake_case conversion preserves already snake_case
///
/// **Test Intent**: Ensure snake_case input is not mangled
#[rstest]
#[case("users", "users")]
#[case("user_profiles", "user_profiles")]
#[case("my_table_name", "my_table_name")]
fn test_to_snake_case_preserves_snake(#[case] input: &str, #[case] expected: &str) {
	let result = to_snake_case(input);
	assert_eq!(result, expected);
}

/// Test Rust keyword escaping
///
/// **Test Intent**: Verify all strict keywords are properly escaped with r# prefix
#[rstest]
#[case("type", "r#type")]
#[case("struct", "r#struct")]
#[case("impl", "r#impl")]
#[case("fn", "r#fn")]
#[case("mod", "r#mod")]
#[case("use", "r#use")]
#[case("pub", "r#pub")]
#[case("let", "r#let")]
#[case("mut", "r#mut")]
#[case("const", "r#const")]
#[case("static", "r#static")]
#[case("async", "r#async")]
#[case("await", "r#await")]
#[case("loop", "r#loop")]
#[case("if", "r#if")]
#[case("else", "r#else")]
#[case("match", "r#match")]
#[case("return", "r#return")]
#[case("break", "r#break")]
#[case("continue", "r#continue")]
fn test_escape_rust_keyword_strict(#[case] input: &str, #[case] expected: &str) {
	let result = escape_rust_keyword(input);
	assert_eq!(result, expected);
}

/// Test reserved keywords escaping (may be used in future Rust)
///
/// **Test Intent**: Verify reserved keywords are also escaped
#[rstest]
#[case("abstract", "r#abstract")]
#[case("become", "r#become")]
#[case("box", "r#box")]
#[case("do", "r#do")]
#[case("final", "r#final")]
#[case("macro", "r#macro")]
#[case("override", "r#override")]
#[case("priv", "r#priv")]
#[case("try", "r#try")]
#[case("typeof", "r#typeof")]
#[case("unsized", "r#unsized")]
#[case("virtual", "r#virtual")]
#[case("yield", "r#yield")]
fn test_escape_rust_keyword_reserved(#[case] input: &str, #[case] expected: &str) {
	let result = escape_rust_keyword(input);
	assert_eq!(result, expected);
}

/// Test non-keyword identifiers are not escaped
///
/// **Test Intent**: Ensure valid identifiers are passed through unchanged
#[rstest]
#[case("users", "users")]
#[case("id", "id")]
#[case("name", "name")]
#[case("created_at", "created_at")]
#[case("UserProfile", "UserProfile")]
fn test_escape_rust_keyword_non_keyword(#[case] input: &str, #[case] expected: &str) {
	let result = escape_rust_keyword(input);
	assert_eq!(result, expected);
}

/// Test identifier sanitization
///
/// **Test Intent**: Verify invalid characters are replaced/removed
#[rstest]
#[case("valid_name", "valid_name")]
#[case("my-field", "my_field")]
#[case("my field", "my_field")]
#[case("1column", "_1column")]
#[case("123start", "_123start")]
#[case("type", "r#type")]
fn test_sanitize_identifier(#[case] input: &str, #[case] expected: &str) {
	let result = sanitize_identifier(input);
	assert_eq!(result, expected);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

/// Test empty string handling
///
/// **Test Intent**: Verify empty strings produce valid identifiers
#[rstest]
#[test]
fn test_empty_string_pascal_case() {
	let result = to_pascal_case("");
	assert_eq!(result, "");
}

#[rstest]
#[test]
fn test_empty_string_snake_case() {
	let result = to_snake_case("");
	assert_eq!(result, "");
}

#[rstest]
#[test]
fn test_empty_string_sanitize() {
	let result = sanitize_identifier("");
	assert_eq!(result, "_");
}

/// Test single character inputs
///
/// **Test Intent**: Verify minimum length inputs are handled
#[rstest]
#[case("a", "A")]
#[case("z", "Z")]
#[case("A", "A")]
#[case("_", "")]
fn test_single_char_pascal_case(#[case] input: &str, #[case] expected: &str) {
	let result = to_pascal_case(input);
	assert_eq!(result, expected);
}

#[rstest]
#[case("a", "a")]
#[case("Z", "z")]
#[case("A", "a")]
fn test_single_char_snake_case(#[case] input: &str, #[case] expected: &str) {
	let result = to_snake_case(input);
	assert_eq!(result, expected);
}

/// Test consecutive uppercase handling (e.g., HTTPRequest)
///
/// **Test Intent**: Verify acronyms are handled correctly
#[rstest]
#[case("HTTPRequest", "http_request")]
#[case("APIResponse", "api_response")]
#[case("XMLParser", "xml_parser")]
#[case("JSONData", "json_data")]
#[case("URLEncoder", "url_encoder")]
fn test_acronym_to_snake_case(#[case] input: &str, #[case] expected: &str) {
	let result = to_snake_case(input);
	assert_eq!(result, expected);
}

/// Test SCREAMING_SNAKE_CASE to PascalCase
///
/// **Test Intent**: Verify uppercase snake_case is converted correctly
#[rstest]
#[case("USER_PROFILES", "UserProfiles")]
#[case("ORDER_ITEMS", "OrderItems")]
#[case("HTTP_REQUEST", "HttpRequest")]
fn test_screaming_snake_to_pascal(#[case] input: &str, #[case] expected: &str) {
	let result = to_pascal_case(input);
	assert_eq!(result, expected);
}

/// Test mixed case handling
///
/// **Test Intent**: Verify camelCase and other mixed patterns
#[rstest]
#[case("userProfiles", "UserProfiles")]
#[case("orderItems", "OrderItems")]
#[case("firstName", "FirstName")]
fn test_camel_case_to_pascal(#[case] input: &str, #[case] expected: &str) {
	let result = to_pascal_case(input);
	assert_eq!(result, expected);
}

/// Test special character handling in identifiers
///
/// **Test Intent**: Verify special characters are sanitized
#[rstest]
#[case("my-column-name", "my_column_name")]
#[case("my column name", "my_column_name")]
#[case("my.column.name", "mycolumnname")]
#[case("my@column#name", "mycolumnname")]
fn test_special_char_sanitization(#[case] input: &str, #[case] expected: &str) {
	let result = sanitize_identifier(input);
	assert_eq!(result, expected);
}

/// Test numeric prefix handling
///
/// **Test Intent**: Verify numeric prefixes get underscore added
#[rstest]
#[case("1column", "_1column")]
#[case("123table", "_123table")]
#[case("2nd_option", "_2nd_option")]
#[case("0_indexed", "_0_indexed")]
fn test_numeric_prefix_sanitization(#[case] input: &str, #[case] expected: &str) {
	let result = sanitize_identifier(input);
	assert_eq!(result, expected);
}

// ============================================================================
// Boundary Value Analysis Tests
// ============================================================================

/// Test maximum length PostgreSQL identifiers (63 characters)
///
/// **Test Intent**: Verify maximum length identifiers are handled
#[rstest]
#[test]
fn test_max_length_identifier_postgres() {
	// PostgreSQL max identifier length is 63 characters
	let max_ident = "a".repeat(63);
	let result = to_pascal_case(&max_ident);
	assert_eq!(result.len(), 63);
	assert!(result.chars().next().unwrap().is_uppercase());
}

/// Test identifier just over max length
///
/// **Test Intent**: Verify long identifiers don't crash (but may need truncation upstream)
#[rstest]
#[test]
fn test_over_max_length_identifier() {
	let long_ident = "a".repeat(100);
	let result = to_pascal_case(&long_ident);
	// Should not panic, result length equals input length
	assert_eq!(result.len(), 100);
}

/// Test underscore-heavy identifiers
///
/// **Test Intent**: Verify multiple consecutive underscores are handled
#[rstest]
#[case("user__profile", "UserProfile")]
#[case("my___table___name", "MyTableName")]
#[case("_leading_underscore", "LeadingUnderscore")]
#[case("trailing_underscore_", "TrailingUnderscore")]
fn test_multiple_underscores_pascal(#[case] input: &str, #[case] expected: &str) {
	let result = to_pascal_case(input);
	assert_eq!(result, expected);
}

/// Test underscore handling in snake_case
///
/// **Test Intent**: Verify underscores are normalized
#[rstest]
#[case("User__Profile", "user_profile")]
#[case("My___Table___Name", "my_table_name")]
fn test_multiple_underscores_snake(#[case] input: &str, #[case] expected: &str) {
	let result = to_snake_case(input);
	assert_eq!(result, expected);
}

// ============================================================================
// Decision Table Tests: Naming Convention Application
// ============================================================================

/// Decision table for naming convention selection
///
/// **Test Intent**: Verify correct output for all convention combinations
#[rstest]
#[case("users", NamingConvention::PascalCase, "Users")]
#[case("user_profiles", NamingConvention::PascalCase, "UserProfiles")]
#[case("ORDER_ITEMS", NamingConvention::PascalCase, "OrderItems")]
#[case("Users", NamingConvention::SnakeCase, "users")]
#[case("UserProfiles", NamingConvention::SnakeCase, "user_profiles")]
#[case("OrderItems", NamingConvention::SnakeCase, "order_items")]
#[case("users", NamingConvention::Preserve, "users")]
#[case("UserProfiles", NamingConvention::Preserve, "UserProfiles")]
#[case("ORDER_ITEMS", NamingConvention::Preserve, "ORDER_ITEMS")]
fn test_naming_convention_decision_table(
	#[case] input: &str,
	#[case] convention: NamingConvention,
	#[case] expected: &str,
) {
	let result = match convention {
		NamingConvention::PascalCase => to_pascal_case(input),
		NamingConvention::SnakeCase => to_snake_case(input),
		NamingConvention::Preserve => input.to_string(),
	};
	assert_eq!(result, expected);
}

// ============================================================================
// Roundtrip Property Tests
// ============================================================================

/// Test that snake_case(pascal_case(x)) produces valid snake_case
///
/// **Test Intent**: Verify naming conversions are consistent
#[rstest]
#[case("users")]
#[case("user_profiles")]
#[case("order_items")]
#[case("my_table_name")]
fn test_roundtrip_snake_to_pascal_to_snake(#[case] input: &str) {
	let pascal = to_pascal_case(input);
	let back_to_snake = to_snake_case(&pascal);
	assert_eq!(back_to_snake, input);
}

/// Test that pascal_case is idempotent
///
/// **Test Intent**: Verify applying pascal_case twice gives same result
#[rstest]
#[case("Users")]
#[case("UserProfiles")]
#[case("OrderItems")]
fn test_pascal_case_idempotent(#[case] input: &str) {
	let once = to_pascal_case(input);
	let twice = to_pascal_case(&once);
	assert_eq!(once, twice);
}

/// Test that snake_case is idempotent
///
/// **Test Intent**: Verify applying snake_case twice gives same result
#[rstest]
#[case("users")]
#[case("user_profiles")]
#[case("order_items")]
fn test_snake_case_idempotent(#[case] input: &str) {
	let once = to_snake_case(input);
	let twice = to_snake_case(&once);
	assert_eq!(once, twice);
}

// ============================================================================
// SQL Table Name Patterns
// ============================================================================

/// Test common SQL table naming patterns
///
/// **Test Intent**: Verify real-world database naming patterns
#[rstest]
#[case("auth_users", "AuthUsers")]
#[case("django_migrations", "DjangoMigrations")]
#[case("public.users", "PublicUsers")]
#[case("pg_catalog_tables", "PgCatalogTables")]
fn test_sql_table_names_to_pascal(#[case] input: &str, #[case] expected: &str) {
	let result = to_pascal_case(input);
	assert_eq!(result, expected);
}

/// Test PostgreSQL system table prefixes
///
/// **Test Intent**: Verify system table names are handled
#[rstest]
#[case("pg_tables", "PgTables")]
#[case("pg_indexes", "PgIndexes")]
#[case("information_schema", "InformationSchema")]
fn test_postgres_system_tables(#[case] input: &str, #[case] expected: &str) {
	let result = to_pascal_case(input);
	assert_eq!(result, expected);
}
