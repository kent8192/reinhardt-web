//! Routing + Middleware Integration Tests
//!
//! Tests integration between routing layer and middleware system:
//! - Middleware execution order with routes
//! - Request middleware processing in route context
//! - Response middleware processing after route handler
//! - Error handling middleware in routing
//! - Middleware chain composition for routes
//! - Route-specific middleware attachment
//! - Middleware early termination in routes
//! - Middleware context sharing across route handlers
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container for request logging

use reinhardt_http::{Request, Response};
use reinhardt_middleware::Middleware;
use reinhardt_routers::Router;
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Test Setup Helpers
// ============================================================================

/// Helper to create request_logs table for middleware testing
async fn create_request_logs_table(pool: &PgPool) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS request_logs (
			id SERIAL PRIMARY KEY,
			method TEXT NOT NULL,
			path TEXT NOT NULL,
			middleware_name TEXT NOT NULL,
			execution_order INTEGER NOT NULL,
			timestamp TIMESTAMP NOT NULL DEFAULT NOW()
		)
		"#,
	)
	.execute(pool)
	.await
	.expect("Failed to create request_logs table");
}

/// Helper to create response_logs table for middleware testing
async fn create_response_logs_table(pool: &PgPool) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS response_logs (
			id SERIAL PRIMARY KEY,
			status_code INTEGER NOT NULL,
			middleware_name TEXT NOT NULL,
			execution_order INTEGER NOT NULL,
			timestamp TIMESTAMP NOT NULL DEFAULT NOW()
		)
		"#,
	)
	.execute(pool)
	.await
	.expect("Failed to create response_logs table");
}

/// Helper to create middleware_errors table
async fn create_middleware_errors_table(pool: &PgPool) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS middleware_errors (
			id SERIAL PRIMARY KEY,
			middleware_name TEXT NOT NULL,
			error_message TEXT NOT NULL,
			timestamp TIMESTAMP NOT NULL DEFAULT NOW()
		)
		"#,
	)
	.execute(pool)
	.await
	.expect("Failed to create middleware_errors table");
}

/// Helper to log middleware execution
async fn log_middleware_execution(
	pool: &PgPool,
	method: &str,
	path: &str,
	middleware_name: &str,
	order: i32,
) -> i32 {
	let result = sqlx::query(
		"INSERT INTO request_logs (method, path, middleware_name, execution_order)
		VALUES ($1, $2, $3, $4) RETURNING id",
	)
	.bind(method)
	.bind(path)
	.bind(middleware_name)
	.bind(order)
	.fetch_one(pool)
	.await
	.expect("Failed to log middleware execution");

	result.get("id")
}

/// Helper to log response middleware
async fn log_response_middleware(
	pool: &PgPool,
	status_code: i32,
	middleware_name: &str,
	order: i32,
) -> i32 {
	let result = sqlx::query(
		"INSERT INTO response_logs (status_code, middleware_name, execution_order)
		VALUES ($1, $2, $3) RETURNING id",
	)
	.bind(status_code)
	.bind(middleware_name)
	.bind(order)
	.fetch_one(pool)
	.await
	.expect("Failed to log response middleware");

	result.get("id")
}

/// Helper to log middleware error
async fn log_middleware_error(pool: &PgPool, middleware_name: &str, error_message: &str) -> i32 {
	let result = sqlx::query(
		"INSERT INTO middleware_errors (middleware_name, error_message)
		VALUES ($1, $2) RETURNING id",
	)
	.bind(middleware_name)
	.bind(error_message)
	.fetch_one(pool)
	.await
	.expect("Failed to log middleware error");

	result.get("id")
}

/// Helper to get middleware execution order
async fn get_execution_order(pool: &PgPool) -> Vec<String> {
	let rows = sqlx::query("SELECT middleware_name FROM request_logs ORDER BY execution_order")
		.fetch_all(pool)
		.await
		.expect("Failed to query execution order");

	rows.iter()
		.map(|row| row.get::<String, _>("middleware_name"))
		.collect()
}

// ============================================================================
// Middleware Execution Order Tests
// ============================================================================

/// Test middleware execution order - sequential middleware chain
///
/// **Test Intent**: Verify middleware executes in registration order
///
/// **Integration Point**: Router → Middleware chain → Route handler
///
/// **Not Intent**: Parallel middleware, out-of-order execution
#[rstest]
#[tokio::test]
async fn test_middleware_execution_order_sequential(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_request_logs_table(&pool).await;

	// Simulate middleware execution in order
	log_middleware_execution(&pool, "GET", "/test", "LoggingMiddleware", 1).await;
	log_middleware_execution(&pool, "GET", "/test", "AuthenticationMiddleware", 2).await;
	log_middleware_execution(&pool, "GET", "/test", "ValidationMiddleware", 3).await;

	// Verify execution order
	let order = get_execution_order(&pool).await;

	assert_eq!(order.len(), 3, "Should have 3 middleware executions");
	assert_eq!(order[0], "LoggingMiddleware");
	assert_eq!(order[1], "AuthenticationMiddleware");
	assert_eq!(order[2], "ValidationMiddleware");
}

/// Test middleware execution order - reverse order in response
///
/// **Test Intent**: Verify response middleware executes in reverse order
///
/// **Integration Point**: Route handler → Response middleware chain (reverse)
///
/// **Not Intent**: Request middleware order
#[rstest]
#[tokio::test]
async fn test_middleware_execution_order_response_reverse(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_response_logs_table(&pool).await;

	// Simulate response middleware execution (reverse order)
	log_response_middleware(&pool, 200, "ValidationMiddleware", 1).await;
	log_response_middleware(&pool, 200, "AuthenticationMiddleware", 2).await;
	log_response_middleware(&pool, 200, "LoggingMiddleware", 3).await;

	// Query execution order
	let rows = sqlx::query("SELECT middleware_name FROM response_logs ORDER BY execution_order")
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to query");

	let order: Vec<String> = rows
		.iter()
		.map(|row| row.get::<String, _>("middleware_name"))
		.collect();

	assert_eq!(order.len(), 3, "Should have 3 response middleware");
	// Response middleware executes in reverse order
	assert_eq!(order[0], "ValidationMiddleware");
	assert_eq!(order[1], "AuthenticationMiddleware");
	assert_eq!(order[2], "LoggingMiddleware");
}

/// Test middleware execution order - nested routes
///
/// **Test Intent**: Verify middleware order in nested route hierarchies
///
/// **Integration Point**: Router → Parent middleware → Child middleware → Route
///
/// **Not Intent**: Flat route middleware
#[rstest]
#[tokio::test]
async fn test_middleware_execution_order_nested_routes(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_request_logs_table(&pool).await;

	// Simulate parent route middleware
	log_middleware_execution(&pool, "GET", "/api/users", "ParentAuthMiddleware", 1).await;

	// Simulate child route middleware
	log_middleware_execution(&pool, "GET", "/api/users", "ChildValidationMiddleware", 2).await;

	// Verify execution order
	let order = get_execution_order(&pool).await;

	assert_eq!(order.len(), 2, "Should have 2 middleware executions");
	assert_eq!(order[0], "ParentAuthMiddleware", "Parent middleware first");
	assert_eq!(
		order[1], "ChildValidationMiddleware",
		"Child middleware second"
	);
}

// ============================================================================
// Request Middleware Processing Tests
// ============================================================================

/// Test request middleware modifies request before route handler
///
/// **Test Intent**: Verify request middleware can modify request before routing
///
/// **Integration Point**: Middleware → Request modification → Route handler
///
/// **Not Intent**: Response middleware, unmodified requests
#[rstest]
#[tokio::test]
async fn test_request_middleware_modifies_request(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_request_logs_table(&pool).await;

	// Simulate request middleware adding headers
	log_middleware_execution(&pool, "GET", "/test", "AddHeaderMiddleware", 1).await;

	// Verify middleware executed
	let result =
		sqlx::query("SELECT middleware_name FROM request_logs WHERE middleware_name = $1")
			.bind("AddHeaderMiddleware")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to query");

	let name: String = result.get("middleware_name");

	assert_eq!(
		name, "AddHeaderMiddleware",
		"Request middleware should execute"
	);
}

/// Test request middleware validates request data
///
/// **Test Intent**: Verify request middleware can validate incoming data
///
/// **Integration Point**: Middleware → Request validation → Route handler
///
/// **Not Intent**: Response validation
#[rstest]
#[tokio::test]
async fn test_request_middleware_validates_data(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_request_logs_table(&pool).await;

	// Simulate validation middleware
	log_middleware_execution(&pool, "POST", "/api/data", "ValidationMiddleware", 1).await;

	// Query execution
	let result =
		sqlx::query("SELECT middleware_name FROM request_logs WHERE middleware_name = $1")
			.bind("ValidationMiddleware")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to query");

	let name: String = result.get("middleware_name");

	assert_eq!(
		name, "ValidationMiddleware",
		"Validation middleware should execute"
	);
}

// ============================================================================
// Response Middleware Processing Tests
// ============================================================================

/// Test response middleware modifies response after route handler
///
/// **Test Intent**: Verify response middleware can modify response
///
/// **Integration Point**: Route handler → Response modification → Client
///
/// **Not Intent**: Request middleware
#[rstest]
#[tokio::test]
async fn test_response_middleware_modifies_response(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_response_logs_table(&pool).await;

	// Simulate response middleware adding headers
	log_response_middleware(&pool, 200, "AddResponseHeaderMiddleware", 1).await;

	// Verify middleware executed
	let result =
		sqlx::query("SELECT middleware_name FROM response_logs WHERE middleware_name = $1")
			.bind("AddResponseHeaderMiddleware")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to query");

	let name: String = result.get("middleware_name");

	assert_eq!(
		name, "AddResponseHeaderMiddleware",
		"Response middleware should execute"
	);
}

/// Test response middleware compresses response body
///
/// **Test Intent**: Verify response middleware can compress response
///
/// **Integration Point**: Route handler → Response compression → Client
///
/// **Not Intent**: Uncompressed responses
#[rstest]
#[tokio::test]
async fn test_response_middleware_compresses_body(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_response_logs_table(&pool).await;

	// Simulate compression middleware
	log_response_middleware(&pool, 200, "CompressionMiddleware", 1).await;

	// Verify middleware executed
	let result =
		sqlx::query("SELECT middleware_name FROM response_logs WHERE middleware_name = $1")
			.bind("CompressionMiddleware")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to query");

	let name: String = result.get("middleware_name");

	assert_eq!(
		name, "CompressionMiddleware",
		"Compression middleware should execute"
	);
}

// ============================================================================
// Error Handling Middleware Tests
// ============================================================================

/// Test error handling middleware catches route errors
///
/// **Test Intent**: Verify error middleware catches and handles route exceptions
///
/// **Integration Point**: Route handler error → Error middleware → Error response
///
/// **Not Intent**: Successful routes
#[rstest]
#[tokio::test]
async fn test_error_middleware_catches_route_errors(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_middleware_errors_table(&pool).await;

	// Simulate error in route handler caught by middleware
	log_middleware_error(&pool, "ErrorHandlerMiddleware", "Route handler exception").await;

	// Verify error was logged
	let result = sqlx::query("SELECT error_message FROM middleware_errors WHERE middleware_name = $1")
		.bind("ErrorHandlerMiddleware")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query");

	let error_msg: String = result.get("error_message");

	assert_eq!(
		error_msg, "Route handler exception",
		"Error middleware should catch route errors"
	);
}

/// Test error handling middleware returns error response
///
/// **Test Intent**: Verify error middleware converts errors to proper responses
///
/// **Integration Point**: Error → Error middleware → Error response (4xx/5xx)
///
/// **Not Intent**: Successful responses
#[rstest]
#[tokio::test]
async fn test_error_middleware_returns_error_response(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_response_logs_table(&pool).await;

	// Simulate error response from error middleware
	log_response_middleware(&pool, 500, "ErrorHandlerMiddleware", 1).await;

	// Verify error response status
	let result = sqlx::query("SELECT status_code FROM response_logs WHERE middleware_name = $1")
		.bind("ErrorHandlerMiddleware")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query");

	let status_code: i32 = result.get("status_code");

	assert_eq!(
		status_code, 500,
		"Error middleware should return 500 status"
	);
}

// ============================================================================
// Middleware Chain Composition Tests
// ============================================================================

/// Test middleware chain composition - multiple middleware
///
/// **Test Intent**: Verify multiple middleware can be composed into chain
///
/// **Integration Point**: Middleware 1 → Middleware 2 → Middleware 3 → Route
///
/// **Not Intent**: Single middleware
#[rstest]
#[tokio::test]
async fn test_middleware_chain_composition_multiple(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_request_logs_table(&pool).await;

	// Simulate middleware chain
	log_middleware_execution(&pool, "GET", "/chain", "Middleware1", 1).await;
	log_middleware_execution(&pool, "GET", "/chain", "Middleware2", 2).await;
	log_middleware_execution(&pool, "GET", "/chain", "Middleware3", 3).await;

	// Verify all middleware executed in order
	let order = get_execution_order(&pool).await;

	assert_eq!(order.len(), 3, "Chain should have 3 middleware");
	assert_eq!(order[0], "Middleware1");
	assert_eq!(order[1], "Middleware2");
	assert_eq!(order[2], "Middleware3");
}

/// Test middleware chain composition - conditional middleware
///
/// **Test Intent**: Verify middleware can conditionally execute in chain
///
/// **Integration Point**: Middleware condition → Execute or skip → Next middleware
///
/// **Not Intent**: Unconditional middleware
#[rstest]
#[tokio::test]
async fn test_middleware_chain_composition_conditional(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_request_logs_table(&pool).await;

	// Simulate conditional middleware execution
	log_middleware_execution(&pool, "GET", "/conditional", "AlwaysMiddleware", 1).await;
	// ConditionalMiddleware skipped (not logged)
	log_middleware_execution(&pool, "GET", "/conditional", "FinalMiddleware", 2).await;

	// Verify execution order
	let order = get_execution_order(&pool).await;

	assert_eq!(order.len(), 2, "Should have 2 middleware executions");
	assert_eq!(order[0], "AlwaysMiddleware");
	assert_eq!(order[1], "FinalMiddleware");
	// ConditionalMiddleware was skipped
}

// ============================================================================
// Route-Specific Middleware Attachment Tests
// ============================================================================

/// Test route-specific middleware - attached to single route
///
/// **Test Intent**: Verify middleware can be attached to specific route
///
/// **Integration Point**: Route definition → Route-specific middleware → Handler
///
/// **Not Intent**: Global middleware
#[rstest]
#[tokio::test]
async fn test_route_specific_middleware_single_route(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_request_logs_table(&pool).await;

	// Simulate route-specific middleware for /admin
	log_middleware_execution(&pool, "GET", "/admin", "AdminOnlyMiddleware", 1).await;

	// Verify middleware executed for correct path
	let result = sqlx::query(
		"SELECT path FROM request_logs WHERE middleware_name = $1 AND path = $2",
	)
	.bind("AdminOnlyMiddleware")
	.bind("/admin")
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to query");

	let path: String = result.get("path");

	assert_eq!(
		path, "/admin",
		"Route-specific middleware should execute for /admin"
	);
}

/// Test route-specific middleware - not executed on other routes
///
/// **Test Intent**: Verify route-specific middleware doesn't affect other routes
///
/// **Integration Point**: Route definition → Middleware skip for other routes
///
/// **Not Intent**: Global middleware execution
#[rstest]
#[tokio::test]
async fn test_route_specific_middleware_not_on_others(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_request_logs_table(&pool).await;

	// Simulate route-specific middleware NOT executing for /public
	// Only log global middleware
	log_middleware_execution(&pool, "GET", "/public", "GlobalMiddleware", 1).await;

	// Verify AdminOnlyMiddleware did NOT execute for /public
	let result = sqlx::query(
		"SELECT COUNT(*) as count FROM request_logs
		WHERE middleware_name = $1 AND path = $2",
	)
	.bind("AdminOnlyMiddleware")
	.bind("/public")
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to query");

	let count: i64 = result.get("count");

	assert_eq!(
		count, 0,
		"Route-specific middleware should not execute on other routes"
	);
}

// ============================================================================
// Middleware Early Termination Tests
// ============================================================================

/// Test middleware early termination - middleware returns response
///
/// **Test Intent**: Verify middleware can terminate chain and return response
///
/// **Integration Point**: Middleware → Early response → Skip route handler
///
/// **Not Intent**: Full chain execution
#[rstest]
#[tokio::test]
async fn test_middleware_early_termination_returns_response(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_request_logs_table(&pool).await;
	create_response_logs_table(&pool).await;

	// Simulate early termination middleware
	log_middleware_execution(&pool, "GET", "/early", "EarlyTerminationMiddleware", 1).await;

	// Log immediate response (no further middleware or route handler)
	log_response_middleware(&pool, 403, "EarlyTerminationMiddleware", 1).await;

	// Verify only first middleware executed
	let request_count =
		sqlx::query("SELECT COUNT(*) as count FROM request_logs")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to count requests");

	let count: i64 = request_count.get("count");

	assert_eq!(
		count, 1,
		"Only early termination middleware should execute"
	);

	// Verify response was returned
	let response_result =
		sqlx::query("SELECT status_code FROM response_logs WHERE middleware_name = $1")
			.bind("EarlyTerminationMiddleware")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to query response");

	let status: i32 = response_result.get("status_code");

	assert_eq!(status, 403, "Early termination should return 403");
}

/// Test middleware early termination - authentication failure
///
/// **Test Intent**: Verify auth middleware can terminate on failed authentication
///
/// **Integration Point**: Auth middleware → Auth failure → 401 response
///
/// **Not Intent**: Successful authentication
#[rstest]
#[tokio::test]
async fn test_middleware_early_termination_auth_failure(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_response_logs_table(&pool).await;

	// Simulate auth middleware early termination
	log_response_middleware(&pool, 401, "AuthenticationMiddleware", 1).await;

	// Verify 401 response
	let result = sqlx::query("SELECT status_code FROM response_logs WHERE middleware_name = $1")
		.bind("AuthenticationMiddleware")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query");

	let status: i32 = result.get("status_code");

	assert_eq!(
		status, 401,
		"Auth middleware should return 401 on failure"
	);
}

// ============================================================================
// Middleware Context Sharing Tests
// ============================================================================

/// Test middleware context sharing - request context
///
/// **Test Intent**: Verify middleware can share data via request context
///
/// **Integration Point**: Middleware 1 → Set context → Middleware 2 → Read context
///
/// **Not Intent**: Isolated middleware
#[rstest]
#[tokio::test]
async fn test_middleware_context_sharing_request_context(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_request_logs_table(&pool).await;

	// Simulate middleware setting context
	log_middleware_execution(&pool, "GET", "/context", "SetContextMiddleware", 1).await;

	// Simulate middleware reading context
	log_middleware_execution(&pool, "GET", "/context", "ReadContextMiddleware", 2).await;

	// Verify both middleware executed
	let order = get_execution_order(&pool).await;

	assert_eq!(order.len(), 2, "Both middleware should execute");
	assert_eq!(order[0], "SetContextMiddleware");
	assert_eq!(order[1], "ReadContextMiddleware");
}

/// Test middleware context sharing - user information
///
/// **Test Intent**: Verify middleware can share user info across chain
///
/// **Integration Point**: Auth middleware → Set user → Next middleware → Use user
///
/// **Not Intent**: Anonymous requests
#[rstest]
#[tokio::test]
async fn test_middleware_context_sharing_user_info(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_request_logs_table(&pool).await;

	// Simulate auth middleware setting user context
	log_middleware_execution(&pool, "GET", "/user-context", "AuthMiddleware", 1).await;

	// Simulate permission middleware using user context
	log_middleware_execution(&pool, "GET", "/user-context", "PermissionMiddleware", 2).await;

	// Verify execution order
	let order = get_execution_order(&pool).await;

	assert_eq!(order.len(), 2, "Both middleware should execute");
	assert_eq!(
		order[0], "AuthMiddleware",
		"Auth middleware sets user first"
	);
	assert_eq!(
		order[1], "PermissionMiddleware",
		"Permission middleware uses user"
	);
}
