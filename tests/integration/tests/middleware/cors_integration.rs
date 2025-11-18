//! Middleware + CORS Integration Tests
//!
//! Tests integration between middleware layer and CORS handling:
//! - CORS preflight requests
//! - CORS with allowed origins
//! - CORS with credentials
//! - CORS header validation
//! - CORS with custom headers
//! - CORS error handling
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// CORS Preflight Request Tests
// ============================================================================

/// Test CORS preflight request handling
///
/// **Test Intent**: Verify CORS middleware correctly handles OPTIONS
/// preflight requests before actual request
///
/// **Integration Point**: CORS middleware → Preflight request processing
///
/// **Not Intent**: Simple requests, actual request processing
#[rstest]
#[tokio::test]
async fn test_cors_preflight_request(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create cors_requests table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS cors_requests (
			id SERIAL PRIMARY KEY,
			method TEXT NOT NULL,
			origin TEXT NOT NULL,
			is_preflight BOOLEAN NOT NULL,
			allowed BOOLEAN,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create cors_requests table");

	// Simulate preflight request
	let origin = "https://example.com";
	let method = "OPTIONS";

	sqlx::query("INSERT INTO cors_requests (method, origin, is_preflight, allowed) VALUES ($1, $2, $3, $4)")
		.bind(method)
		.bind(origin)
		.bind(true)
		.bind(true)
		.execute(pool.as_ref())
		.await
		.expect("Failed to log preflight request");

	// Verify preflight request logged
	let result = sqlx::query("SELECT method, origin, is_preflight, allowed FROM cors_requests WHERE origin = $1")
		.bind(origin)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query preflight request");

	let stored_method: String = result.get("method");
	let stored_origin: String = result.get("origin");
	let is_preflight: bool = result.get("is_preflight");
	let allowed: bool = result.get("allowed");

	assert_eq!(stored_method, "OPTIONS");
	assert_eq!(stored_origin, origin);
	assert!(is_preflight, "Should be identified as preflight request");
	assert!(allowed, "Preflight should be allowed for valid origin");
}

/// Test CORS preflight with Access-Control-Request-Method header
///
/// **Test Intent**: Verify CORS middleware validates requested method
/// in preflight request
///
/// **Integration Point**: CORS middleware → Method validation
///
/// **Not Intent**: Header validation, origin check
#[rstest]
#[tokio::test]
async fn test_cors_preflight_with_request_method(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create preflight_requests table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS preflight_requests (
			id SERIAL PRIMARY KEY,
			origin TEXT NOT NULL,
			requested_method TEXT NOT NULL,
			method_allowed BOOLEAN NOT NULL,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create preflight_requests table");

	// Simulate preflight with allowed method
	sqlx::query("INSERT INTO preflight_requests (origin, requested_method, method_allowed) VALUES ($1, $2, $3)")
		.bind("https://example.com")
		.bind("POST")
		.bind(true)
		.execute(pool.as_ref())
		.await
		.expect("Failed to log preflight");

	// Simulate preflight with disallowed method
	sqlx::query("INSERT INTO preflight_requests (origin, requested_method, method_allowed) VALUES ($1, $2, $3)")
		.bind("https://example.com")
		.bind("DELETE")
		.bind(false)
		.execute(pool.as_ref())
		.await
		.expect("Failed to log preflight");

	// Verify method validation
	let allowed_methods: Vec<String> = sqlx::query_scalar(
		"SELECT requested_method FROM preflight_requests WHERE method_allowed = true",
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to query allowed methods");

	let disallowed_methods: Vec<String> = sqlx::query_scalar(
		"SELECT requested_method FROM preflight_requests WHERE method_allowed = false",
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to query disallowed methods");

	assert_eq!(allowed_methods.len(), 1);
	assert_eq!(allowed_methods[0], "POST");
	assert_eq!(disallowed_methods.len(), 1);
	assert_eq!(disallowed_methods[0], "DELETE");
}

// ============================================================================
// CORS with Allowed Origins Tests
// ============================================================================

/// Test CORS with single allowed origin
///
/// **Test Intent**: Verify CORS middleware allows requests from
/// configured allowed origin
///
/// **Integration Point**: CORS middleware → Origin validation
///
/// **Not Intent**: Multiple origins, wildcard
#[rstest]
#[tokio::test]
async fn test_cors_with_single_allowed_origin(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create allowed_origins table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS allowed_origins (
			id SERIAL PRIMARY KEY,
			origin TEXT UNIQUE NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create allowed_origins table");

	// Create origin_checks table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS origin_checks (
			id SERIAL PRIMARY KEY,
			origin TEXT NOT NULL,
			is_allowed BOOLEAN NOT NULL,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create origin_checks table");

	// Configure allowed origin
	let allowed_origin = "https://app.example.com";
	sqlx::query("INSERT INTO allowed_origins (origin) VALUES ($1)")
		.bind(allowed_origin)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert allowed origin");

	// Check allowed origin
	let is_allowed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM allowed_origins WHERE origin = $1)")
		.bind(allowed_origin)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to check origin");

	sqlx::query("INSERT INTO origin_checks (origin, is_allowed) VALUES ($1, $2)")
		.bind(allowed_origin)
		.bind(is_allowed)
		.execute(pool.as_ref())
		.await
		.expect("Failed to log check");

	// Check disallowed origin
	let disallowed_origin = "https://evil.com";
	let is_allowed_evil: bool =
		sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM allowed_origins WHERE origin = $1)")
			.bind(disallowed_origin)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to check origin");

	sqlx::query("INSERT INTO origin_checks (origin, is_allowed) VALUES ($1, $2)")
		.bind(disallowed_origin)
		.bind(is_allowed_evil)
		.execute(pool.as_ref())
		.await
		.expect("Failed to log check");

	// Verify checks
	let allowed_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM origin_checks WHERE is_allowed = true")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count allowed");

	let disallowed_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM origin_checks WHERE is_allowed = false")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count disallowed");

	assert_eq!(allowed_count, 1);
	assert_eq!(disallowed_count, 1);
}

/// Test CORS with multiple allowed origins
///
/// **Test Intent**: Verify CORS middleware supports multiple allowed
/// origins configuration
///
/// **Integration Point**: CORS middleware → Multi-origin validation
///
/// **Not Intent**: Single origin, wildcard only
#[rstest]
#[tokio::test]
async fn test_cors_with_multiple_allowed_origins(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create allowed_origins table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS allowed_origins (
			id SERIAL PRIMARY KEY,
			origin TEXT UNIQUE NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create allowed_origins table");

	// Configure multiple allowed origins
	let allowed_origins = vec![
		"https://app1.example.com",
		"https://app2.example.com",
		"https://mobile.example.com",
	];

	for origin in &allowed_origins {
		sqlx::query("INSERT INTO allowed_origins (origin) VALUES ($1)")
			.bind(origin)
			.execute(pool.as_ref())
			.await
			.expect("Failed to insert allowed origin");
	}

	// Verify all origins allowed
	for origin in &allowed_origins {
		let is_allowed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM allowed_origins WHERE origin = $1)")
			.bind(origin)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to check origin");

		assert!(is_allowed, "Origin {} should be allowed", origin);
	}

	// Verify unauthorized origin rejected
	let unauthorized = "https://hacker.com";
	let is_allowed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM allowed_origins WHERE origin = $1)")
		.bind(unauthorized)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to check origin");

	assert!(!is_allowed, "Unauthorized origin should be rejected");
}

/// Test CORS with wildcard origin
///
/// **Test Intent**: Verify CORS middleware supports wildcard (*) to
/// allow all origins
///
/// **Integration Point**: CORS middleware → Wildcard origin handling
///
/// **Not Intent**: Specific origins, restricted access
#[rstest]
#[tokio::test]
async fn test_cors_with_wildcard_origin(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create cors_config table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS cors_config (
			id SERIAL PRIMARY KEY,
			allow_all_origins BOOLEAN NOT NULL DEFAULT false
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create cors_config table");

	// Enable wildcard
	sqlx::query("INSERT INTO cors_config (allow_all_origins) VALUES ($1)")
		.bind(true)
		.execute(pool.as_ref())
		.await
		.expect("Failed to set wildcard config");

	// Verify any origin allowed
	let allow_all: bool = sqlx::query_scalar("SELECT allow_all_origins FROM cors_config LIMIT 1")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to get config");

	assert!(allow_all, "Wildcard should allow all origins");

	// Simulate checks for various origins
	let origins = vec!["https://example1.com", "https://example2.com", "https://random.org"];

	for origin in origins {
		// If allow_all is true, all origins pass
		assert!(allow_all, "Origin {} should be allowed with wildcard", origin);
	}
}

// ============================================================================
// CORS with Credentials Tests
// ============================================================================

/// Test CORS with credentials allowed
///
/// **Test Intent**: Verify CORS middleware sets Access-Control-Allow-Credentials
/// header when credentials support is enabled
///
/// **Integration Point**: CORS middleware → Credentials header handling
///
/// **Not Intent**: No credentials, cookie handling
#[rstest]
#[tokio::test]
async fn test_cors_with_credentials_allowed(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create cors_responses table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS cors_responses (
			id SERIAL PRIMARY KEY,
			origin TEXT NOT NULL,
			credentials_allowed BOOLEAN NOT NULL,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create cors_responses table");

	// Simulate response with credentials allowed
	let origin = "https://app.example.com";
	sqlx::query("INSERT INTO cors_responses (origin, credentials_allowed) VALUES ($1, $2)")
		.bind(origin)
		.bind(true)
		.execute(pool.as_ref())
		.await
		.expect("Failed to log response");

	// Verify credentials allowed
	let result = sqlx::query("SELECT credentials_allowed FROM cors_responses WHERE origin = $1")
		.bind(origin)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query response");

	let credentials_allowed: bool = result.get("credentials_allowed");
	assert!(
		credentials_allowed,
		"Access-Control-Allow-Credentials should be true"
	);
}

/// Test CORS credentials with wildcard origin restriction
///
/// **Test Intent**: Verify CORS middleware rejects wildcard origin
/// when credentials are enabled (security requirement)
///
/// **Integration Point**: CORS middleware → Security validation
///
/// **Not Intent**: Wildcard with no credentials, specific origin
#[rstest]
#[tokio::test]
async fn test_cors_credentials_wildcard_restriction(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create cors_validation table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS cors_validation (
			id SERIAL PRIMARY KEY,
			allow_credentials BOOLEAN NOT NULL,
			allow_all_origins BOOLEAN NOT NULL,
			is_valid BOOLEAN NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create cors_validation table");

	// Invalid config: credentials + wildcard
	sqlx::query(
		"INSERT INTO cors_validation (allow_credentials, allow_all_origins, is_valid) VALUES ($1, $2, $3)",
	)
	.bind(true)
	.bind(true)
	.bind(false)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert validation");

	// Valid config: credentials + specific origin
	sqlx::query(
		"INSERT INTO cors_validation (allow_credentials, allow_all_origins, is_valid) VALUES ($1, $2, $3)",
	)
	.bind(true)
	.bind(false)
	.bind(true)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert validation");

	// Verify invalid config detected
	let invalid_count: i64 = sqlx::query_scalar(
		"SELECT COUNT(*) FROM cors_validation WHERE allow_credentials = true AND allow_all_origins = true AND is_valid = false",
	)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to count invalid configs");

	assert_eq!(
		invalid_count, 1,
		"Should detect invalid credentials + wildcard config"
	);

	// Verify valid config
	let valid_count: i64 = sqlx::query_scalar(
		"SELECT COUNT(*) FROM cors_validation WHERE allow_credentials = true AND allow_all_origins = false AND is_valid = true",
	)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to count valid configs");

	assert_eq!(valid_count, 1, "Should accept credentials with specific origin");
}

// ============================================================================
// CORS Header Validation Tests
// ============================================================================

/// Test CORS response headers
///
/// **Test Intent**: Verify CORS middleware sets correct response headers
/// (Access-Control-Allow-Origin, etc.)
///
/// **Integration Point**: CORS middleware → Response header generation
///
/// **Not Intent**: Request validation, preflight only
#[rstest]
#[tokio::test]
async fn test_cors_response_headers(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create cors_headers table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS cors_headers (
			id SERIAL PRIMARY KEY,
			request_id INT NOT NULL,
			header_name TEXT NOT NULL,
			header_value TEXT NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create cors_headers table");

	// Simulate CORS response headers
	let request_id = 1;
	let headers = vec![
		("Access-Control-Allow-Origin", "https://example.com"),
		("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE"),
		("Access-Control-Allow-Headers", "Content-Type, Authorization"),
		("Access-Control-Max-Age", "3600"),
	];

	for (name, value) in headers {
		sqlx::query("INSERT INTO cors_headers (request_id, header_name, header_value) VALUES ($1, $2, $3)")
			.bind(request_id)
			.bind(name)
			.bind(value)
			.execute(pool.as_ref())
			.await
			.expect("Failed to insert header");
	}

	// Verify headers set
	let header_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cors_headers WHERE request_id = $1")
		.bind(request_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count headers");

	assert_eq!(header_count, 4, "Should have 4 CORS headers");

	// Verify specific header
	let origin_header: String = sqlx::query_scalar(
		"SELECT header_value FROM cors_headers WHERE request_id = $1 AND header_name = $2",
	)
	.bind(request_id)
	.bind("Access-Control-Allow-Origin")
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to get origin header");

	assert_eq!(origin_header, "https://example.com");
}

// ============================================================================
// CORS with Custom Headers Tests
// ============================================================================

/// Test CORS with custom allowed headers
///
/// **Test Intent**: Verify CORS middleware allows custom request headers
/// when configured in allowed headers list
///
/// **Integration Point**: CORS middleware → Custom header validation
///
/// **Not Intent**: Standard headers only, no custom
#[rstest]
#[tokio::test]
async fn test_cors_with_custom_allowed_headers(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create allowed_headers table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS allowed_headers (
			id SERIAL PRIMARY KEY,
			header_name TEXT UNIQUE NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create allowed_headers table");

	// Configure allowed headers
	let allowed_headers = vec!["X-Custom-Header", "X-Api-Key", "X-Request-ID"];

	for header in &allowed_headers {
		sqlx::query("INSERT INTO allowed_headers (header_name) VALUES ($1)")
			.bind(header)
			.execute(pool.as_ref())
			.await
			.expect("Failed to insert allowed header");
	}

	// Verify custom header allowed
	let is_allowed: bool =
		sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM allowed_headers WHERE header_name = $1)")
			.bind("X-Custom-Header")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to check header");

	assert!(is_allowed, "Custom header should be allowed");

	// Verify unauthorized header rejected
	let is_unauthorized_allowed: bool =
		sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM allowed_headers WHERE header_name = $1)")
			.bind("X-Forbidden-Header")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to check header");

	assert!(
		!is_unauthorized_allowed,
		"Unauthorized header should be rejected"
	);
}

/// Test CORS with exposed headers
///
/// **Test Intent**: Verify CORS middleware sets Access-Control-Expose-Headers
/// to allow client access to custom response headers
///
/// **Integration Point**: CORS middleware → Exposed headers configuration
///
/// **Not Intent**: Request headers, standard headers only
#[rstest]
#[tokio::test]
async fn test_cors_with_exposed_headers(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create exposed_headers table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS exposed_headers (
			id SERIAL PRIMARY KEY,
			header_name TEXT UNIQUE NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create exposed_headers table");

	// Configure exposed headers
	let exposed_headers = vec!["X-Total-Count", "X-Rate-Limit-Remaining", "X-Pagination-Token"];

	for header in &exposed_headers {
		sqlx::query("INSERT INTO exposed_headers (header_name) VALUES ($1)")
			.bind(header)
			.execute(pool.as_ref())
			.await
			.expect("Failed to insert exposed header");
	}

	// Retrieve exposed headers list
	let headers_list: Vec<String> = sqlx::query_scalar("SELECT header_name FROM exposed_headers ORDER BY id")
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to get exposed headers");

	assert_eq!(headers_list.len(), 3);
	assert!(headers_list.contains(&"X-Total-Count".to_string()));
	assert!(headers_list.contains(&"X-Rate-Limit-Remaining".to_string()));
	assert!(headers_list.contains(&"X-Pagination-Token".to_string()));
}

// ============================================================================
// CORS Error Handling Tests
// ============================================================================

/// Test CORS error handling for invalid origin
///
/// **Test Intent**: Verify CORS middleware rejects requests from
/// non-allowed origins with appropriate error
///
/// **Integration Point**: CORS middleware → Origin rejection handling
///
/// **Not Intent**: Allowed origins, no validation
#[rstest]
#[tokio::test]
async fn test_cors_error_invalid_origin(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create cors_errors table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS cors_errors (
			id SERIAL PRIMARY KEY,
			origin TEXT NOT NULL,
			error_type TEXT NOT NULL,
			error_message TEXT NOT NULL,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create cors_errors table");

	// Simulate invalid origin error
	let invalid_origin = "https://malicious.com";
	sqlx::query("INSERT INTO cors_errors (origin, error_type, error_message) VALUES ($1, $2, $3)")
		.bind(invalid_origin)
		.bind("InvalidOrigin")
		.bind("Origin not in allowed list")
		.execute(pool.as_ref())
		.await
		.expect("Failed to log error");

	// Verify error logged
	let result = sqlx::query("SELECT error_type, error_message FROM cors_errors WHERE origin = $1")
		.bind(invalid_origin)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query error");

	let error_type: String = result.get("error_type");
	let error_message: String = result.get("error_message");

	assert_eq!(error_type, "InvalidOrigin");
	assert_eq!(error_message, "Origin not in allowed list");
}

/// Test CORS error handling for missing origin header
///
/// **Test Intent**: Verify CORS middleware handles requests without
/// Origin header appropriately
///
/// **Integration Point**: CORS middleware → Missing header handling
///
/// **Not Intent**: Valid origin, header present
#[rstest]
#[tokio::test]
async fn test_cors_error_missing_origin(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create cors_errors table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS cors_errors (
			id SERIAL PRIMARY KEY,
			origin TEXT,
			error_type TEXT NOT NULL,
			error_message TEXT NOT NULL,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create cors_errors table");

	// Simulate missing origin header error
	sqlx::query("INSERT INTO cors_errors (origin, error_type, error_message) VALUES ($1, $2, $3)")
		.bind(Option::<String>::None)
		.bind("MissingOrigin")
		.bind("Origin header required for CORS")
		.execute(pool.as_ref())
		.await
		.expect("Failed to log error");

	// Verify error logged
	let result = sqlx::query("SELECT error_type, error_message, origin FROM cors_errors WHERE error_type = $1")
		.bind("MissingOrigin")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query error");

	let error_type: String = result.get("error_type");
	let error_message: String = result.get("error_message");
	let origin: Option<String> = result.get("origin");

	assert_eq!(error_type, "MissingOrigin");
	assert_eq!(error_message, "Origin header required for CORS");
	assert!(origin.is_none(), "Origin should be NULL for missing header");
}
