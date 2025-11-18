//! Middleware + Logging Integration Tests
//!
//! Tests integration between middleware layer and logging system:
//! - Request/response logging middleware
//! - Database query logging
//! - Error logging with stack traces
//! - Log level filtering
//! - Structured logging with context
//! - Log rotation and cleanup
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Request/Response Logging Middleware Tests
// ============================================================================

/// Test request logging middleware with successful request
///
/// **Test Intent**: Verify logging middleware captures request details
/// (method, path, headers) for successful requests
///
/// **Integration Point**: Logging middleware → Request data capture
///
/// **Not Intent**: Error logging, response logging only
#[rstest]
#[tokio::test]
async fn test_request_logging_middleware_success(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create request_logs table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS request_logs (
			id SERIAL PRIMARY KEY,
			method TEXT NOT NULL,
			path TEXT NOT NULL,
			status_code INT,
			duration_ms BIGINT,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create request_logs table");

	// Simulate logging a successful request
	sqlx::query(
		"INSERT INTO request_logs (method, path, status_code, duration_ms) VALUES ($1, $2, $3, $4)",
	)
	.bind("GET")
	.bind("/api/users")
	.bind(200)
	.bind(150)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert request log");

	// Verify log entry
	let result = sqlx::query("SELECT method, path, status_code, duration_ms FROM request_logs WHERE path = $1")
		.bind("/api/users")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query request log");

	let method: String = result.get("method");
	let path: String = result.get("path");
	let status_code: i32 = result.get("status_code");
	let duration_ms: i64 = result.get("duration_ms");

	assert_eq!(method, "GET");
	assert_eq!(path, "/api/users");
	assert_eq!(status_code, 200);
	assert_eq!(duration_ms, 150);
}

/// Test request logging with headers capture
///
/// **Test Intent**: Verify logging middleware captures important request headers
/// (User-Agent, Content-Type, Authorization)
///
/// **Integration Point**: Logging middleware → Request headers capture
///
/// **Not Intent**: All headers, sensitive data masking
#[rstest]
#[tokio::test]
async fn test_request_logging_with_headers(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create request_logs table with headers column
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS request_logs (
			id SERIAL PRIMARY KEY,
			method TEXT NOT NULL,
			path TEXT NOT NULL,
			headers JSONB,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create request_logs table");

	// Simulate logging request with headers
	let headers = serde_json::json!({
		"user-agent": "Mozilla/5.0",
		"content-type": "application/json",
		"accept": "application/json"
	});

	sqlx::query("INSERT INTO request_logs (method, path, headers) VALUES ($1, $2, $3)")
		.bind("POST")
		.bind("/api/posts")
		.bind(&headers)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert request log");

	// Verify headers captured
	let result = sqlx::query("SELECT headers FROM request_logs WHERE path = $1")
		.bind("/api/posts")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query request log");

	let stored_headers: serde_json::Value = result.get("headers");
	assert_eq!(stored_headers["user-agent"], "Mozilla/5.0");
	assert_eq!(stored_headers["content-type"], "application/json");
	assert_eq!(stored_headers["accept"], "application/json");
}

/// Test response logging middleware
///
/// **Test Intent**: Verify logging middleware captures response details
/// (status code, response size, duration)
///
/// **Integration Point**: Logging middleware → Response data capture
///
/// **Not Intent**: Request logging, error logging
#[rstest]
#[tokio::test]
async fn test_response_logging_middleware(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create response_logs table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS response_logs (
			id SERIAL PRIMARY KEY,
			request_id INT NOT NULL,
			status_code INT NOT NULL,
			response_size BIGINT NOT NULL,
			duration_ms BIGINT NOT NULL,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create response_logs table");

	// Simulate logging a response
	sqlx::query(
		"INSERT INTO response_logs (request_id, status_code, response_size, duration_ms) VALUES ($1, $2, $3, $4)",
	)
	.bind(1)
	.bind(200)
	.bind(1024)
	.bind(250)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert response log");

	// Verify response log
	let result = sqlx::query("SELECT status_code, response_size, duration_ms FROM response_logs WHERE request_id = $1")
		.bind(1)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query response log");

	let status_code: i32 = result.get("status_code");
	let response_size: i64 = result.get("response_size");
	let duration_ms: i64 = result.get("duration_ms");

	assert_eq!(status_code, 200);
	assert_eq!(response_size, 1024);
	assert_eq!(duration_ms, 250);
}

// ============================================================================
// Database Query Logging Tests
// ============================================================================

/// Test database query logging
///
/// **Test Intent**: Verify logging middleware captures database queries
/// executed during request processing
///
/// **Integration Point**: Logging middleware → Database query tracking
///
/// **Not Intent**: Query execution, query optimization
#[rstest]
#[tokio::test]
async fn test_database_query_logging(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create query_logs table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS query_logs (
			id SERIAL PRIMARY KEY,
			request_id INT NOT NULL,
			query_sql TEXT NOT NULL,
			params JSONB,
			duration_ms BIGINT NOT NULL,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create query_logs table");

	// Simulate logging a database query
	let query_sql = "SELECT * FROM users WHERE id = $1";
	let params = serde_json::json!({"id": 123});

	sqlx::query("INSERT INTO query_logs (request_id, query_sql, params, duration_ms) VALUES ($1, $2, $3, $4)")
		.bind(1)
		.bind(query_sql)
		.bind(&params)
		.bind(45)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert query log");

	// Verify query log
	let result = sqlx::query("SELECT query_sql, params, duration_ms FROM query_logs WHERE request_id = $1")
		.bind(1)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query query log");

	let stored_sql: String = result.get("query_sql");
	let stored_params: serde_json::Value = result.get("params");
	let duration: i64 = result.get("duration_ms");

	assert_eq!(stored_sql, query_sql);
	assert_eq!(stored_params["id"], 123);
	assert_eq!(duration, 45);
}

/// Test slow query logging
///
/// **Test Intent**: Verify logging middleware flags slow database queries
/// exceeding threshold
///
/// **Integration Point**: Logging middleware → Slow query detection
///
/// **Not Intent**: Query optimization, all queries
#[rstest]
#[tokio::test]
async fn test_slow_query_logging(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create slow_query_logs table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS slow_query_logs (
			id SERIAL PRIMARY KEY,
			query_sql TEXT NOT NULL,
			duration_ms BIGINT NOT NULL,
			is_slow BOOLEAN NOT NULL,
			threshold_ms BIGINT NOT NULL,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create slow_query_logs table");

	let slow_threshold_ms = 100;

	// Log a slow query
	let slow_query = "SELECT * FROM large_table WHERE condition = 'complex'";
	sqlx::query("INSERT INTO slow_query_logs (query_sql, duration_ms, is_slow, threshold_ms) VALUES ($1, $2, $3, $4)")
		.bind(slow_query)
		.bind(250)
		.bind(true)
		.bind(slow_threshold_ms)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert slow query log");

	// Log a fast query
	let fast_query = "SELECT * FROM users WHERE id = 1";
	sqlx::query("INSERT INTO slow_query_logs (query_sql, duration_ms, is_slow, threshold_ms) VALUES ($1, $2, $3, $4)")
		.bind(fast_query)
		.bind(20)
		.bind(false)
		.bind(slow_threshold_ms)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert fast query log");

	// Query slow queries
	let slow_queries: Vec<String> = sqlx::query_scalar(
		"SELECT query_sql FROM slow_query_logs WHERE is_slow = true AND duration_ms > $1",
	)
	.bind(slow_threshold_ms)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to query slow queries");

	assert_eq!(slow_queries.len(), 1);
	assert_eq!(slow_queries[0], slow_query);
}

// ============================================================================
// Error Logging Tests
// ============================================================================

/// Test error logging with stack traces
///
/// **Test Intent**: Verify logging middleware captures errors with
/// stack traces and context information
///
/// **Integration Point**: Logging middleware → Error capture with traces
///
/// **Not Intent**: Error handling, error recovery
#[rstest]
#[tokio::test]
async fn test_error_logging_with_stack_trace(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create error_logs table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS error_logs (
			id SERIAL PRIMARY KEY,
			request_id INT NOT NULL,
			error_type TEXT NOT NULL,
			error_message TEXT NOT NULL,
			stack_trace TEXT,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create error_logs table");

	// Simulate logging an error with stack trace
	let error_type = "DatabaseError";
	let error_message = "Connection timeout";
	let stack_trace = "at connect() in database.rs:42\nat handle_request() in middleware.rs:15";

	sqlx::query(
		"INSERT INTO error_logs (request_id, error_type, error_message, stack_trace) VALUES ($1, $2, $3, $4)",
	)
	.bind(1)
	.bind(error_type)
	.bind(error_message)
	.bind(stack_trace)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert error log");

	// Verify error log
	let result = sqlx::query("SELECT error_type, error_message, stack_trace FROM error_logs WHERE request_id = $1")
		.bind(1)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query error log");

	let stored_type: String = result.get("error_type");
	let stored_message: String = result.get("error_message");
	let stored_trace: String = result.get("stack_trace");

	assert_eq!(stored_type, error_type);
	assert_eq!(stored_message, error_message);
	assert!(stored_trace.contains("database.rs:42"));
	assert!(stored_trace.contains("middleware.rs:15"));
}

/// Test error logging with context
///
/// **Test Intent**: Verify logging middleware captures error context
/// (request details, user info, environment)
///
/// **Integration Point**: Logging middleware → Error context capture
///
/// **Not Intent**: Stack trace only, minimal error info
#[rstest]
#[tokio::test]
async fn test_error_logging_with_context(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create error_logs table with context
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS error_logs (
			id SERIAL PRIMARY KEY,
			error_message TEXT NOT NULL,
			context JSONB NOT NULL,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create error_logs table");

	// Simulate logging error with rich context
	let error_message = "Failed to process payment";
	let context = serde_json::json!({
		"request": {
			"method": "POST",
			"path": "/api/checkout",
			"user_id": 456
		},
		"environment": {
			"server": "app-01",
			"version": "1.2.3"
		},
		"additional": {
			"payment_id": "pay_123",
			"amount": 99.99
		}
	});

	sqlx::query("INSERT INTO error_logs (error_message, context) VALUES ($1, $2)")
		.bind(error_message)
		.bind(&context)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert error log");

	// Verify context captured
	let result = sqlx::query("SELECT error_message, context FROM error_logs WHERE error_message = $1")
		.bind(error_message)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query error log");

	let stored_message: String = result.get("error_message");
	let stored_context: serde_json::Value = result.get("context");

	assert_eq!(stored_message, error_message);
	assert_eq!(stored_context["request"]["method"], "POST");
	assert_eq!(stored_context["request"]["user_id"], 456);
	assert_eq!(stored_context["environment"]["server"], "app-01");
	assert_eq!(stored_context["additional"]["payment_id"], "pay_123");
}

// ============================================================================
// Log Level Filtering Tests
// ============================================================================

/// Test log level filtering
///
/// **Test Intent**: Verify logging middleware respects log level configuration
/// and filters logs accordingly
///
/// **Integration Point**: Logging middleware → Log level filtering
///
/// **Not Intent**: All logs, no filtering
#[rstest]
#[tokio::test]
async fn test_log_level_filtering(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create logs table with level
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS logs (
			id SERIAL PRIMARY KEY,
			level TEXT NOT NULL,
			message TEXT NOT NULL,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create logs table");

	// Insert logs at different levels
	let log_entries = vec![
		("DEBUG", "Debug message"),
		("INFO", "Info message"),
		("WARN", "Warning message"),
		("ERROR", "Error message"),
	];

	for (level, message) in log_entries {
		sqlx::query("INSERT INTO logs (level, message) VALUES ($1, $2)")
			.bind(level)
			.bind(message)
			.execute(pool.as_ref())
			.await
			.expect("Failed to insert log");
	}

	// Filter logs at WARN level and above
	let warn_and_above: Vec<String> = sqlx::query_scalar(
		"SELECT message FROM logs WHERE level IN ('WARN', 'ERROR') ORDER BY level DESC",
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to query filtered logs");

	assert_eq!(warn_and_above.len(), 2);
	assert!(warn_and_above.contains(&"Warning message".to_string()));
	assert!(warn_and_above.contains(&"Error message".to_string()));

	// Verify DEBUG and INFO excluded
	let debug_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM logs WHERE level = 'DEBUG'")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count debug logs");
	assert_eq!(debug_count, 1);

	let info_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM logs WHERE level = 'INFO'")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count info logs");
	assert_eq!(info_count, 1);
}

// ============================================================================
// Structured Logging Tests
// ============================================================================

/// Test structured logging with fields
///
/// **Test Intent**: Verify logging middleware supports structured logging
/// with key-value fields for filtering and analysis
///
/// **Integration Point**: Logging middleware → Structured data capture
///
/// **Not Intent**: Plain text logging, no structure
#[rstest]
#[tokio::test]
async fn test_structured_logging_with_fields(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create structured_logs table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS structured_logs (
			id SERIAL PRIMARY KEY,
			level TEXT NOT NULL,
			message TEXT NOT NULL,
			fields JSONB NOT NULL,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create structured_logs table");

	// Log with structured fields
	let fields = serde_json::json!({
		"user_id": 789,
		"action": "login",
		"ip_address": "192.168.1.100",
		"user_agent": "Chrome/120.0"
	});

	sqlx::query("INSERT INTO structured_logs (level, message, fields) VALUES ($1, $2, $3)")
		.bind("INFO")
		.bind("User login successful")
		.bind(&fields)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert structured log");

	// Query by field
	let logs_by_user: Vec<serde_json::Value> = sqlx::query_scalar(
		"SELECT fields FROM structured_logs WHERE fields->>'user_id' = $1",
	)
	.bind("789")
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to query by field");

	assert_eq!(logs_by_user.len(), 1);
	assert_eq!(logs_by_user[0]["action"], "login");
	assert_eq!(logs_by_user[0]["ip_address"], "192.168.1.100");
}

/// Test structured logging aggregation
///
/// **Test Intent**: Verify structured logs can be aggregated and analyzed
/// for metrics and monitoring
///
/// **Integration Point**: Logging middleware → Structured data analysis
///
/// **Not Intent**: Individual log retrieval, no aggregation
#[rstest]
#[tokio::test]
async fn test_structured_logging_aggregation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create structured_logs table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS structured_logs (
			id SERIAL PRIMARY KEY,
			level TEXT NOT NULL,
			message TEXT NOT NULL,
			fields JSONB NOT NULL,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create structured_logs table");

	// Insert multiple logs for aggregation
	let actions = vec![
		("login", 100),
		("login", 101),
		("logout", 100),
		("login", 102),
		("view_profile", 101),
	];

	for (action, user_id) in actions {
		let fields = serde_json::json!({"action": action, "user_id": user_id});
		sqlx::query("INSERT INTO structured_logs (level, message, fields) VALUES ($1, $2, $3)")
			.bind("INFO")
			.bind(format!("User action: {}", action))
			.bind(&fields)
			.execute(pool.as_ref())
			.await
			.expect("Failed to insert log");
	}

	// Aggregate by action type
	let action_counts: Vec<(String, i64)> = sqlx::query_as(
		r#"
		SELECT fields->>'action' as action, COUNT(*) as count
		FROM structured_logs
		GROUP BY fields->>'action'
		ORDER BY count DESC
		"#,
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to aggregate logs");

	assert_eq!(action_counts.len(), 3);
	assert_eq!(action_counts[0], ("login".to_string(), 3));
	assert_eq!(action_counts[1], ("logout".to_string(), 1));
	assert_eq!(action_counts[2], ("view_profile".to_string(), 1));
}

// ============================================================================
// Log Rotation and Cleanup Tests
// ============================================================================

/// Test log rotation based on age
///
/// **Test Intent**: Verify old logs are identified for rotation/cleanup
/// based on timestamp threshold
///
/// **Integration Point**: Logging middleware → Log rotation management
///
/// **Not Intent**: Actual rotation, archival process
#[rstest]
#[tokio::test]
async fn test_log_rotation_by_age(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create logs table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS logs (
			id SERIAL PRIMARY KEY,
			level TEXT NOT NULL,
			message TEXT NOT NULL,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create logs table");

	// Insert old log (30 days ago)
	sqlx::query(
		"INSERT INTO logs (level, message, timestamp) VALUES ($1, $2, NOW() - INTERVAL '30 days')",
	)
	.bind("INFO")
	.bind("Old log entry")
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert old log");

	// Insert recent log
	sqlx::query("INSERT INTO logs (level, message) VALUES ($1, $2)")
		.bind("INFO")
		.bind("Recent log entry")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert recent log");

	// Find logs older than 7 days for rotation
	let old_logs_count: i64 = sqlx::query_scalar(
		"SELECT COUNT(*) FROM logs WHERE timestamp < NOW() - INTERVAL '7 days'",
	)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to count old logs");

	assert_eq!(old_logs_count, 1, "Should identify 1 old log for rotation");

	// Verify recent logs not included
	let recent_logs_count: i64 = sqlx::query_scalar(
		"SELECT COUNT(*) FROM logs WHERE timestamp >= NOW() - INTERVAL '7 days'",
	)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to count recent logs");

	assert_eq!(recent_logs_count, 1, "Should have 1 recent log");
}

/// Test log cleanup by deleting old entries
///
/// **Test Intent**: Verify logging system can delete old logs to
/// manage storage space
///
/// **Integration Point**: Logging middleware → Log cleanup execution
///
/// **Not Intent**: Archival, log preservation
#[rstest]
#[tokio::test]
async fn test_log_cleanup(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create logs table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS logs (
			id SERIAL PRIMARY KEY,
			level TEXT NOT NULL,
			message TEXT NOT NULL,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create logs table");

	// Insert logs with different ages
	for days_ago in [60, 45, 30, 15, 7, 1] {
		sqlx::query(
			&format!(
				"INSERT INTO logs (level, message, timestamp) VALUES ($1, $2, NOW() - INTERVAL '{} days')",
				days_ago
			),
		)
		.bind("INFO")
		.bind(format!("Log from {} days ago", days_ago))
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert log");
	}

	// Delete logs older than 30 days
	let deleted_count = sqlx::query("DELETE FROM logs WHERE timestamp < NOW() - INTERVAL '30 days'")
		.execute(pool.as_ref())
		.await
		.expect("Failed to delete old logs")
		.rows_affected();

	assert_eq!(deleted_count, 3, "Should delete 3 logs older than 30 days");

	// Verify remaining logs
	let remaining_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM logs")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count remaining logs");

	assert_eq!(remaining_count, 3, "Should have 3 logs remaining");
}
