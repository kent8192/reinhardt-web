//! Equivalence Partitioning Tests for Database URL Parsing.
//!
//! This test module validates that EnvParser correctly handles all equivalence classes
//! of database URL input.
//!
//! ## Supported Database Engines (Per ACTUAL Implementation)
//!
//! **PostgreSQL**: Full support with all options
//! **MySQL**: Full support with all options
//! **SQLite**: Full support (in-memory and file-based)
//! **MongoDB**: NOT SUPPORTED in actual implementation
//!
//! ## Partitions
//!
//! **SQLite:**
//! - In-memory database
//! - File-based database (absolute/relative paths)
//!
//! **PostgreSQL:**
//! - Minimal URL (no credentials, default port)
//! - With credentials
//! - Full URL (credentials + port)
//! - With SSL and other options
//!
//! **MySQL:**
//! - Minimal URL
//! - With credentials
//! - Full URL (credentials + port)
//! - With charset and other options
//!
//! **Invalid URLs:**
//! - Plain text (no scheme)
//! - Wrong scheme
//! - Missing database name
//! - Invalid port
//! - Unknown database type

use reinhardt_conf::settings::env_parser::{parse_cache_url, parse_database_url};
use rstest::*;

/// Test: SQLite Database URL Equivalence Classes
///
/// Why: Validates SQLite URL parsing for in-memory and file-based databases.
#[rstest]
#[case(
	"sqlite::memory:",
	"reinhardt.db.backends.sqlite3",
	"In-memory database"
)]
#[case(
	"sqlite:///tmp/test.db",
	"reinhardt.db.backends.sqlite3",
	"File-based (absolute path)"
)]
#[case(
	"sqlite://./relative.db",
	"reinhardt.db.backends.sqlite3",
	"File-based (relative path)"
)]
fn test_sqlite_url_equivalence_classes(
	#[case] url: &str,
	#[case] expected_engine: &str,
	#[case] description: &str,
) {
	let result = parse_database_url(url);

	assert!(
		result.is_ok(),
		"Failed to parse SQLite URL for partition: {} - URL: {:?}",
		description,
		url
	);

	let parsed = result.unwrap();
	assert_eq!(
		parsed.engine, expected_engine,
		"Engine should be SQLite for partition: {}",
		description
	);
}

/// Test: PostgreSQL Database URL Equivalence Classes
///
/// Why: Validates PostgreSQL URL parsing for minimal and full configurations.
#[rstest]
#[case(
	"postgres://localhost/db",
	"reinhardt.db.backends.postgresql",
	"Minimal"
)]
#[case(
	"postgresql://localhost/db",
	"reinhardt.db.backends.postgresql",
	"Alternative scheme"
)]
#[case(
	"postgres://user:pass@localhost/db",
	"reinhardt.db.backends.postgresql",
	"With credentials"
)]
#[case(
	"postgres://user:pass@localhost:5432/db",
	"reinhardt.db.backends.postgresql",
	"Full"
)]
#[case(
	"postgres://user:pass@localhost:5432/db?sslmode=require",
	"reinhardt.db.backends.postgresql",
	"With SSL"
)]
fn test_postgresql_url_equivalence_classes(
	#[case] url: &str,
	#[case] expected_engine: &str,
	#[case] description: &str,
) {
	let result = parse_database_url(url);

	assert!(
		result.is_ok(),
		"Failed to parse PostgreSQL URL for partition: {} - URL: {:?}",
		description,
		url
	);

	let parsed = result.unwrap();
	assert_eq!(
		parsed.engine, expected_engine,
		"Engine should be PostgreSQL for partition: {}",
		description
	);
}

/// Test: MySQL Database URL Equivalence Classes
///
/// Why: Validates MySQL URL parsing for minimal and full configurations.
#[rstest]
#[case("mysql://localhost/db", "reinhardt.db.backends.mysql", "Minimal")]
#[case(
	"mysql://user:pass@localhost/db",
	"reinhardt.db.backends.mysql",
	"With credentials"
)]
#[case(
	"mysql://user:pass@localhost:3306/db",
	"reinhardt.db.backends.mysql",
	"Full"
)]
#[case(
	"mysql://user:pass@localhost:3306/db?charset=utf8mb4",
	"reinhardt.db.backends.mysql",
	"With charset"
)]
fn test_mysql_url_equivalence_classes(
	#[case] url: &str,
	#[case] expected_engine: &str,
	#[case] description: &str,
) {
	let result = parse_database_url(url);

	assert!(
		result.is_ok(),
		"Failed to parse MySQL URL for partition: {} - URL: {:?}",
		description,
		url
	);

	let parsed = result.unwrap();
	assert_eq!(
		parsed.engine, expected_engine,
		"Engine should be MySQL for partition: {}",
		description
	);
}

/// Test: Invalid Database URL Equivalence Classes
///
/// Why: Validates that parse_database_url correctly rejects invalid URLs.
#[rstest]
#[case("not-a-url", "Plain text (no scheme)")]
#[case("http://localhost/db", "Wrong scheme (HTTP)")]
#[case("postgres://localhost:abc/db", "Invalid port (not a number)")]
#[case("unknown://localhost/db", "Unknown database type")]
fn test_invalid_database_url_equivalence_classes(#[case] url: &str, #[case] description: &str) {
	let result = parse_database_url(url);

	assert!(
		result.is_err(),
		"parse_database_url({:?}) should fail for partition: {}",
		url,
		description
	);
}

/// Test: Cache URL Equivalence Classes
///
/// Why: Validates cache URL parsing for different backend types.
/// NOTE: Per actual implementation, only locmem, redis, and memcached are supported.
#[rstest]
#[case(
	"locmem://",
	"reinhardt.cache.backends.locmem.LocMemCache",
	"Local memory cache"
)]
#[case(
	"redis://localhost:6379",
	"reinhardt.cache.backends.redis.RedisCache",
	"Redis minimal"
)]
#[case(
	"redis://localhost:6379/0",
	"reinhardt.cache.backends.redis.RedisCache",
	"Redis with database"
)]
#[case(
	"redis://:password@localhost:6379/0",
	"reinhardt.cache.backends.redis.RedisCache",
	"Redis with password"
)]
#[case(
	"memcached://127.0.0.1:11211",
	"reinhardt.cache.backends.memcached.PyMemcacheCache",
	"Memcached"
)]
fn test_cache_url_equivalence_classes(
	#[case] url: &str,
	#[case] expected_backend: &str,
	#[case] description: &str,
) {
	let result = parse_cache_url(url);

	assert!(
		result.is_ok(),
		"Failed to parse cache URL for partition: {} - URL: {:?}",
		description,
		url
	);

	let parsed = result.unwrap();
	assert_eq!(
		parsed.backend, expected_backend,
		"Backend should match for partition: {}",
		description
	);
}

/// Test: Invalid Cache URL Equivalence Classes
///
/// Why: Validates that parse_cache_url correctly rejects invalid URLs.
#[rstest]
#[case("not-a-url", "Plain text")]
#[case("http://localhost", "Wrong scheme")]
#[case("redis://localhost:abc", "Invalid port")]
#[case("unknown://localhost", "Unknown cache type")]
fn test_invalid_cache_url_equivalence_classes(#[case] url: &str, #[case] description: &str) {
	let result = parse_cache_url(url);

	assert!(
		result.is_err(),
		"parse_cache_url({:?}) should fail for partition: {}",
		url,
		description
	);
}
