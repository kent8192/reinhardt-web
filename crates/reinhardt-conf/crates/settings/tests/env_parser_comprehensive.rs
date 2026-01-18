//! Comprehensive integration tests for EnvParser utility functions.
//!
//! This test module validates all parsing utilities for environment variables,
//! including boolean parsing, list parsing, dictionary parsing, and URL parsing
//! for databases and caches.
//!
//! IMPORTANT: These tests validate the ACTUAL implementation behavior, not idealized
//! behavior. Supported features:
//! - Database engines: PostgreSQL, MySQL, SQLite (MongoDB NOT supported)
//! - Cache backends: locmem, redis, memcached (dummy NOT supported)
//! - Dict format: key=value (colon syntax NOT supported)
//! - List delimiters: comma only (semicolon, pipe NOT supported)

use reinhardt_conf::settings::env_parser::{
	parse_bool, parse_cache_url, parse_database_url, parse_dict, parse_list,
};
use rstest::*;

/// Test: parse_bool with all valid variants
///
/// Why: Validates that parse_bool correctly handles all common boolean string representations
/// including true/false, yes/no, on/off, and 1/0 in various cases.
#[rstest]
#[case("true", true)]
#[case("false", false)]
#[case("1", true)]
#[case("0", false)]
#[case("yes", true)]
#[case("no", false)]
#[case("on", true)]
#[case("off", false)]
#[case("TRUE", true)]
#[case("FALSE", false)]
#[case("YES", true)]
#[case("NO", false)]
#[case("ON", true)]
#[case("OFF", false)]
#[case("True", true)]
#[case("False", false)]
#[case("Yes", true)]
#[case("No", false)]
#[case("On", true)]
#[case("Off", false)]
#[test]
fn test_parse_bool_all_variants(#[case] input: &str, #[case] expected: bool) {
	let result = parse_bool(input);
	assert_eq!(
		result.unwrap(),
		expected,
		"parse_bool('{}') should return {}",
		input,
		expected
	);
}

/// Test: parse_database_url with query string options
///
/// Why: Validates that parse_database_url correctly extracts query parameters
/// from PostgreSQL URLs.
#[rstest]
#[test]
fn test_parse_database_url_with_options() {
	let url = "postgres://user:pass@host:5432/db?sslmode=require&pool_size=10";
	let result = parse_database_url(url);

	assert!(
		result.is_ok(),
		"PostgreSQL URL with options should parse successfully"
	);

	let db_url = result.unwrap();
	assert_eq!(
		db_url.engine, "reinhardt.db.backends.postgresql",
		"Engine should be reinhardt.db.backends.postgresql"
	);

	// Options should be parsed
	assert!(
		db_url.options.contains_key("sslmode"),
		"Options should contain sslmode"
	);
	assert_eq!(
		db_url.options.get("sslmode"),
		Some(&"require".to_string()),
		"sslmode should be require"
	);
	assert_eq!(
		db_url.options.get("pool_size"),
		Some(&"10".to_string()),
		"pool_size should be 10"
	);
}

/// Test: parse_cache_url for Memcached
///
/// Why: Validates that parse_cache_url correctly parses Memcached connection strings.
#[rstest]
#[test]
fn test_parse_cache_url_memcached() {
	let url = "memcached://127.0.0.1:11211";
	let result = parse_cache_url(url);

	assert!(result.is_ok(), "Memcached URL should parse successfully");

	let cache_url = result.unwrap();
	assert_eq!(
		cache_url.backend, "reinhardt.cache.backends.memcached.PyMemcacheCache",
		"Backend should be reinhardt.cache.backends.memcached.PyMemcacheCache"
	);
	assert_eq!(
		cache_url.location,
		Some("127.0.0.1:11211".to_string()),
		"Location should be 127.0.0.1:11211"
	);
}

/// Test: parse_dict with equals-separated key-value pairs
///
/// Why: Validates that parse_dict handles key=value format (NOT colon format).
#[rstest]
#[test]
fn test_parse_dict_basic() {
	let input = "key1=value1,key2=value2,key3=value3";
	let dict = parse_dict(input);

	assert_eq!(dict.get("key1"), Some(&"value1".to_string()));
	assert_eq!(dict.get("key2"), Some(&"value2".to_string()));
	assert_eq!(dict.get("key3"), Some(&"value3".to_string()));
	assert_eq!(dict.len(), 3, "Dictionary should have 3 entries");
}

/// Test: parse_list with comma delimiter
///
/// Why: Validates that parse_list handles comma-separated values (ONLY comma supported).
#[rstest]
#[case("a,b,c", vec!["a", "b", "c"])]
#[case("a, b, c", vec!["a", "b", "c"])] // With spaces
#[case("single", vec!["single"])] // Single item
#[test]
fn test_parse_list_comma_delimiter(#[case] input: &str, #[case] expected: Vec<&str>) {
	let list = parse_list(input);
	let expected_strings: Vec<String> = expected.iter().map(|s| s.to_string()).collect();

	assert_eq!(
		list, expected_strings,
		"Parsed list should match expected values"
	);
}

/// Test: parse_database_url for all supported engines
///
/// Why: Validates comprehensive database URL parsing across all ACTUALLY supported database types
/// (PostgreSQL, MySQL, SQLite only - MongoDB NOT supported).
#[rstest]
#[case(
	"sqlite:///path/to/db.sqlite",
	"reinhardt.db.backends.sqlite3",
	"path/to/db.sqlite",
	None,
	None
)]
#[case(
	"postgres://localhost/db",
	"reinhardt.db.backends.postgresql",
	"db",
	Some("localhost"),
	None
)]
#[case(
	"mysql://localhost:3306/db",
	"reinhardt.db.backends.mysql",
	"db",
	Some("localhost"),
	Some(3306)
)]
#[test]
fn test_parse_database_url_all_engines(
	#[case] url: &str,
	#[case] expected_engine: &str,
	#[case] expected_name: &str,
	#[case] expected_host: Option<&str>,
	#[case] expected_port: Option<u16>,
) {
	let result = parse_database_url(url);

	assert!(
		result.is_ok(),
		"Database URL '{}' should parse successfully",
		url
	);

	let db_url = result.unwrap();
	assert_eq!(db_url.engine, expected_engine);
	assert_eq!(
		db_url.name, expected_name,
		"Database name should be '{}'",
		expected_name
	);

	if let Some(host) = expected_host {
		assert_eq!(db_url.host, Some(host.to_string()));
	}

	if let Some(port) = expected_port {
		assert_eq!(db_url.port, Some(port));
	}
}

/// Test: parse_cache_url for Redis
///
/// Why: Validates Redis cache URL parsing. Note that the entire URL is stored in location.
#[rstest]
#[test]
fn test_parse_cache_url_redis() {
	let url = "redis://localhost:6379/0";
	let result = parse_cache_url(url);

	assert!(result.is_ok(), "Redis URL should parse successfully");

	let cache_url = result.unwrap();
	assert_eq!(
		cache_url.backend, "reinhardt.cache.backends.redis.RedisCache",
		"Backend should be reinhardt.cache.backends.redis.RedisCache"
	);
	// Redis location stores the entire URL string
	assert_eq!(cache_url.location, Some(url.to_string()));
}

/// Test: parse_cache_url for locmem
///
/// Why: Validates locmem (local memory) cache URL parsing.
#[rstest]
#[test]
fn test_parse_cache_url_locmem() {
	let url = "locmem://";
	let result = parse_cache_url(url);

	assert!(result.is_ok(), "Locmem URL should parse successfully");

	let cache_url = result.unwrap();
	assert_eq!(
		cache_url.backend, "reinhardt.cache.backends.locmem.LocMemCache",
		"Backend should be reinhardt.cache.backends.locmem.LocMemCache"
	);
	assert_eq!(cache_url.location, None, "Locmem has no location");
}

/// Test: parse_list empty string
///
/// Why: Validates that parse_list handles empty input gracefully.
#[rstest]
#[test]
fn test_parse_list_empty() {
	let result = parse_list("");

	// Should return empty list
	assert!(result.is_empty(), "Empty string should return empty list");
}

/// Test: parse_dict empty string
///
/// Why: Validates that parse_dict handles empty input gracefully.
#[rstest]
#[test]
fn test_parse_dict_empty() {
	let result = parse_dict("");

	// Should return empty dict
	assert!(result.is_empty(), "Empty string should return empty dict");
}

/// Test: parse_dict with spaces
///
/// Why: Validates that parse_dict correctly trims whitespace.
#[rstest]
#[test]
fn test_parse_dict_with_spaces() {
	let input = "host = localhost , port = 5432 , user = admin";
	let dict = parse_dict(input);

	assert_eq!(dict.get("host"), Some(&"localhost".to_string()));
	assert_eq!(dict.get("port"), Some(&"5432".to_string()));
	assert_eq!(dict.get("user"), Some(&"admin".to_string()));
}

/// Test: parse_database_url for SQLite memory database
///
/// Why: Validates special SQLite :memory: database handling.
#[rstest]
#[test]
fn test_parse_database_url_sqlite_memory() {
	let url = "sqlite::memory:";
	let result = parse_database_url(url);

	assert!(
		result.is_ok(),
		"SQLite :memory: URL should parse successfully"
	);

	let db_url = result.unwrap();
	assert_eq!(
		db_url.engine, "reinhardt.db.backends.sqlite3",
		"Engine should be reinhardt.db.backends.sqlite3"
	);
	assert_eq!(db_url.name, ":memory:", "Name should be :memory:");
	assert_eq!(db_url.host, None, "SQLite has no host");
	assert_eq!(db_url.port, None, "SQLite has no port");
}

/// Test: parse_database_url for PostgreSQL with full credentials
///
/// Why: Validates complete PostgreSQL URL parsing with all components.
#[rstest]
#[test]
fn test_parse_database_url_postgresql_full() {
	let url = "postgresql://user:pass@localhost:5432/mydb";
	let result = parse_database_url(url);

	assert!(result.is_ok(), "PostgreSQL URL should parse successfully");

	let db_url = result.unwrap();
	assert_eq!(
		db_url.engine, "reinhardt.db.backends.postgresql",
		"Engine should be reinhardt.db.backends.postgresql"
	);
	assert_eq!(db_url.name, "mydb", "Database name should be mydb");
	assert_eq!(db_url.user, Some("user".to_string()), "User should be user");
	assert_eq!(
		db_url.password,
		Some("pass".to_string()),
		"Password should be pass"
	);
	assert_eq!(
		db_url.host,
		Some("localhost".to_string()),
		"Host should be localhost"
	);
	assert_eq!(db_url.port, Some(5432), "Port should be 5432");
}

/// Test: parse_database_url for MySQL with full credentials
///
/// Why: Validates complete MySQL URL parsing with all components.
#[rstest]
#[test]
fn test_parse_database_url_mysql_full() {
	let url = "mysql://root:secret@127.0.0.1:3306/testdb";
	let result = parse_database_url(url);

	assert!(result.is_ok(), "MySQL URL should parse successfully");

	let db_url = result.unwrap();
	assert_eq!(
		db_url.engine, "reinhardt.db.backends.mysql",
		"Engine should be reinhardt.db.backends.mysql"
	);
	assert_eq!(db_url.name, "testdb", "Database name should be testdb");
	assert_eq!(db_url.user, Some("root".to_string()), "User should be root");
	assert_eq!(
		db_url.password,
		Some("secret".to_string()),
		"Password should be secret"
	);
	assert_eq!(
		db_url.host,
		Some("127.0.0.1".to_string()),
		"Host should be 127.0.0.1"
	);
	assert_eq!(db_url.port, Some(3306), "Port should be 3306");
}
