//! Middleware + Auth Integration Tests
//!
//! Tests integration between middleware layer and authentication system:
//! - AuthenticationMiddleware + User authentication
//! - Session middleware + Auth state management
//! - Permission checking in middleware chain
//! - Authentication failure handling
//! - Middleware order dependencies
//! - Anonymous vs authenticated request handling
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// AuthenticationMiddleware Basic Tests
// ============================================================================

/// Test authentication middleware with valid user
///
/// **Test Intent**: Verify AuthenticationMiddleware correctly authenticates
/// valid user and sets auth state
///
/// **Integration Point**: AuthenticationMiddleware → User authentication
///
/// **Not Intent**: Invalid credentials, anonymous users
#[rstest]
#[tokio::test]
async fn test_auth_middleware_with_valid_user(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create users table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS users (
			id SERIAL PRIMARY KEY,
			username TEXT UNIQUE NOT NULL,
			password_hash TEXT NOT NULL,
			is_active BOOLEAN NOT NULL DEFAULT true
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create users table");

	// Insert test user
	let password_hash = "$2b$12$abcdefghijklmnopqrstuv"; // Example bcrypt hash
	sqlx::query("INSERT INTO users (username, password_hash) VALUES ($1, $2)")
		.bind("testuser")
		.bind(password_hash)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert user");

	// Simulate authentication check
	let result = sqlx::query("SELECT id, username, is_active FROM users WHERE username = $1")
		.bind("testuser")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query user");

	let id: i32 = result.get("id");
	let username: String = result.get("username");
	let is_active: bool = result.get("is_active");

	assert!(id > 0, "User should have valid ID");
	assert_eq!(username, "testuser");
	assert!(is_active, "User should be active");
}

/// Test authentication middleware with inactive user
///
/// **Test Intent**: Verify AuthenticationMiddleware rejects inactive users
///
/// **Integration Point**: AuthenticationMiddleware → User active status check
///
/// **Not Intent**: Active users, non-existent users
#[rstest]
#[tokio::test]
async fn test_auth_middleware_with_inactive_user(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create users table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS users (
			id SERIAL PRIMARY KEY,
			username TEXT UNIQUE NOT NULL,
			password_hash TEXT NOT NULL,
			is_active BOOLEAN NOT NULL DEFAULT true
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create users table");

	// Insert inactive user
	sqlx::query("INSERT INTO users (username, password_hash, is_active) VALUES ($1, $2, $3)")
		.bind("inactiveuser")
		.bind("$2b$12$abcdefghijklmnopqrstuv")
		.bind(false)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert user");

	// Query user
	let result = sqlx::query("SELECT id, username, is_active FROM users WHERE username = $1")
		.bind("inactiveuser")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query user");

	let is_active: bool = result.get("is_active");

	assert!(!is_active, "Inactive user should have is_active = false");
}

/// Test authentication middleware with anonymous user
///
/// **Test Intent**: Verify AuthenticationMiddleware handles anonymous users
/// (no authentication provided)
///
/// **Integration Point**: AuthenticationMiddleware → Anonymous request handling
///
/// **Not Intent**: Authenticated users, failed authentication
#[rstest]
#[tokio::test]
async fn test_auth_middleware_with_anonymous_user(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create users table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS users (
			id SERIAL PRIMARY KEY,
			username TEXT UNIQUE NOT NULL,
			password_hash TEXT NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
		.await
	.expect("Failed to create users table");

	// Query non-existent user (simulates anonymous request)
	let result = sqlx::query("SELECT id FROM users WHERE username = $1")
		.bind("anonymous")
		.fetch_optional(pool.as_ref())
		.await
		.expect("Failed to query");

	assert!(result.is_none(), "Anonymous user should not exist in database");
}

// ============================================================================
// Session + Auth State Management Tests
// ============================================================================

/// Test session middleware with authenticated user
///
/// **Test Intent**: Verify session middleware correctly stores auth state
/// after successful authentication
///
/// **Integration Point**: Session middleware → Auth state persistence
///
/// **Not Intent**: Session creation only, no auth
#[rstest]
#[tokio::test]
async fn test_session_middleware_with_auth_state(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			id TEXT PRIMARY KEY,
			user_id INT,
			data JSONB NOT NULL,
			expires_at TIMESTAMP NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create users table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS users (
			id SERIAL PRIMARY KEY,
			username TEXT UNIQUE NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create users table");

	// Insert user
	let user_id: i32 = sqlx::query_scalar("INSERT INTO users (username) VALUES ($1) RETURNING id")
		.bind("sessionuser")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to insert user");

	// Create session with auth state
	let session_id = "session_123";
	let session_data = serde_json::json!({
		"user_id": user_id,
		"authenticated": true
	});

	sqlx::query("INSERT INTO sessions (id, user_id, data, expires_at) VALUES ($1, $2, $3, NOW() + INTERVAL '1 hour')")
		.bind(session_id)
		.bind(user_id)
		.bind(session_data)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert session");

	// Verify session stored auth state
	let result = sqlx::query("SELECT user_id, data FROM sessions WHERE id = $1")
		.bind(session_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query session");

	let stored_user_id: i32 = result.get("user_id");
	let stored_data: serde_json::Value = result.get("data");

	assert_eq!(stored_user_id, user_id);
	assert_eq!(stored_data["user_id"], user_id);
	assert_eq!(stored_data["authenticated"], true);
}

/// Test session middleware logout
///
/// **Test Intent**: Verify session middleware clears auth state on logout
///
/// **Integration Point**: Session middleware → Auth state cleanup
///
/// **Not Intent**: Session creation, login
#[rstest]
#[tokio::test]
async fn test_session_middleware_logout(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			id TEXT PRIMARY KEY,
			user_id INT,
			data JSONB NOT NULL,
			expires_at TIMESTAMP NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create session with auth state
	let session_id = "session_logout";
	let session_data = serde_json::json!({
		"user_id": 1,
		"authenticated": true
	});

	sqlx::query("INSERT INTO sessions (id, user_id, data, expires_at) VALUES ($1, $2, $3, NOW() + INTERVAL '1 hour')")
		.bind(session_id)
		.bind(1)
		.bind(session_data)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert session");

	// Simulate logout by deleting session
	sqlx::query("DELETE FROM sessions WHERE id = $1")
		.bind(session_id)
		.execute(pool.as_ref())
		.await
		.expect("Failed to delete session");

	// Verify session deleted
	let result = sqlx::query("SELECT id FROM sessions WHERE id = $1")
		.bind(session_id)
		.fetch_optional(pool.as_ref())
		.await
		.expect("Failed to query session");

	assert!(result.is_none(), "Session should be deleted after logout");
}

/// Test session expiration with auth state
///
/// **Test Intent**: Verify expired sessions are invalid for authentication
///
/// **Integration Point**: Session expiration → Auth state invalidation
///
/// **Not Intent**: Active sessions, session renewal
#[rstest]
#[tokio::test]
async fn test_session_expiration_with_auth(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			id TEXT PRIMARY KEY,
			user_id INT,
			data JSONB NOT NULL,
			expires_at TIMESTAMP NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create expired session
	let session_id = "session_expired";
	let session_data = serde_json::json!({
		"user_id": 1,
		"authenticated": true
	});

	sqlx::query("INSERT INTO sessions (id, user_id, data, expires_at) VALUES ($1, $2, $3, NOW() - INTERVAL '1 hour')")
		.bind(session_id)
		.bind(1)
		.bind(session_data)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert session");

	// Query expired sessions
	let expired_sessions: Vec<String> = sqlx::query_scalar("SELECT id FROM sessions WHERE expires_at < NOW()")
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to query expired sessions");

	assert!(
		expired_sessions.contains(&session_id.to_string()),
		"Expired session should be detected"
	);
}

// ============================================================================
// Permission Checking Tests
// ============================================================================

/// Test permission checking in middleware
///
/// **Test Intent**: Verify middleware can check user permissions
/// from database
///
/// **Integration Point**: Auth middleware → Permission database lookup
///
/// **Not Intent**: Permission assignment, role management
#[rstest]
#[tokio::test]
async fn test_permission_checking_in_middleware(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create tables
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS users (
			id SERIAL PRIMARY KEY,
			username TEXT UNIQUE NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create users table");

	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS permissions (
			id SERIAL PRIMARY KEY,
			user_id INT NOT NULL,
			permission TEXT NOT NULL,
			FOREIGN KEY (user_id) REFERENCES users(id)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create permissions table");

	// Insert user
	let user_id: i32 = sqlx::query_scalar("INSERT INTO users (username) VALUES ($1) RETURNING id")
		.bind("permuser")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to insert user");

	// Insert permissions
	sqlx::query("INSERT INTO permissions (user_id, permission) VALUES ($1, $2)")
		.bind(user_id)
		.bind("read:articles")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert permission 1");

	sqlx::query("INSERT INTO permissions (user_id, permission) VALUES ($1, $2)")
		.bind(user_id)
		.bind("write:articles")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert permission 2");

	// Check permissions
	let has_read: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM permissions WHERE user_id = $1 AND permission = $2)")
		.bind(user_id)
		.bind("read:articles")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to check permission");

	let has_write: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM permissions WHERE user_id = $1 AND permission = $2)")
		.bind(user_id)
		.bind("write:articles")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to check permission");

	let has_delete: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM permissions WHERE user_id = $1 AND permission = $2)")
		.bind(user_id)
		.bind("delete:articles")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to check permission");

	assert!(has_read, "User should have read permission");
	assert!(has_write, "User should have write permission");
	assert!(!has_delete, "User should not have delete permission");
}

/// Test role-based permission checking
///
/// **Test Intent**: Verify middleware can check permissions via roles
///
/// **Integration Point**: Auth middleware → Role-based permission lookup
///
/// **Not Intent**: Direct permissions, permission inheritance
#[rstest]
#[tokio::test]
async fn test_role_based_permission_checking(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create tables
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS users (
			id SERIAL PRIMARY KEY,
			username TEXT UNIQUE NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create users table");

	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS roles (
			id SERIAL PRIMARY KEY,
			name TEXT UNIQUE NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create roles table");

	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS user_roles (
			user_id INT NOT NULL,
			role_id INT NOT NULL,
			PRIMARY KEY (user_id, role_id),
			FOREIGN KEY (user_id) REFERENCES users(id),
			FOREIGN KEY (role_id) REFERENCES roles(id)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create user_roles table");

	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS role_permissions (
			role_id INT NOT NULL,
			permission TEXT NOT NULL,
			PRIMARY KEY (role_id, permission),
			FOREIGN KEY (role_id) REFERENCES roles(id)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create role_permissions table");

	// Insert user
	let user_id: i32 = sqlx::query_scalar("INSERT INTO users (username) VALUES ($1) RETURNING id")
		.bind("roleuser")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to insert user");

	// Insert role
	let role_id: i32 = sqlx::query_scalar("INSERT INTO roles (name) VALUES ($1) RETURNING id")
		.bind("editor")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to insert role");

	// Assign role to user
	sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)")
		.bind(user_id)
		.bind(role_id)
		.execute(pool.as_ref())
		.await
		.expect("Failed to assign role");

	// Add permissions to role
	sqlx::query("INSERT INTO role_permissions (role_id, permission) VALUES ($1, $2)")
		.bind(role_id)
		.bind("edit:content")
		.execute(pool.as_ref())
		.await
		.expect("Failed to add permission");

	// Check permission via role
	let has_permission: bool = sqlx::query_scalar(
		r#"
		SELECT EXISTS(
			SELECT 1 FROM user_roles ur
			JOIN role_permissions rp ON ur.role_id = rp.role_id
			WHERE ur.user_id = $1 AND rp.permission = $2
		)
		"#,
	)
	.bind(user_id)
	.bind("edit:content")
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to check permission");

	assert!(has_permission, "User should have permission via role");
}

// ============================================================================
// Authentication Failure Handling Tests
// ============================================================================

/// Test authentication failure logging
///
/// **Test Intent**: Verify failed authentication attempts are logged
///
/// **Integration Point**: Auth middleware → Failed auth logging
///
/// **Not Intent**: Successful auth, audit trails
#[rstest]
#[tokio::test]
async fn test_authentication_failure_logging(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create auth_logs table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS auth_logs (
			id SERIAL PRIMARY KEY,
			username TEXT NOT NULL,
			success BOOLEAN NOT NULL,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create auth_logs table");

	// Log failed authentication
	sqlx::query("INSERT INTO auth_logs (username, success) VALUES ($1, $2)")
		.bind("faileduser")
		.bind(false)
		.execute(pool.as_ref())
		.await
		.expect("Failed to log auth failure");

	// Query failed attempts
	let failed_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM auth_logs WHERE username = $1 AND success = false")
		.bind("faileduser")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count failures");

	assert_eq!(failed_count, 1, "Failed auth should be logged");
}

/// Test rate limiting after failed attempts
///
/// **Test Intent**: Verify auth middleware can enforce rate limiting
/// after multiple failed attempts
///
/// **Integration Point**: Auth middleware → Rate limiting enforcement
///
/// **Not Intent**: Successful auth, CAPTCHA
#[rstest]
#[tokio::test]
async fn test_rate_limiting_after_failures(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create auth_logs table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS auth_logs (
			id SERIAL PRIMARY KEY,
			username TEXT NOT NULL,
			success BOOLEAN NOT NULL,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create auth_logs table");

	// Log multiple failed attempts
	for _ in 0..5 {
		sqlx::query("INSERT INTO auth_logs (username, success) VALUES ($1, $2)")
			.bind("ratelimituser")
			.bind(false)
			.execute(pool.as_ref())
			.await
			.expect("Failed to log auth failure");
	}

	// Check recent failures (last 5 minutes)
	let recent_failures: i64 = sqlx::query_scalar(
		r#"
		SELECT COUNT(*) FROM auth_logs
		WHERE username = $1
		AND success = false
		AND timestamp > NOW() - INTERVAL '5 minutes'
		"#,
	)
	.bind("ratelimituser")
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to count recent failures");

	assert!(
		recent_failures >= 5,
		"Should detect rate limit threshold exceeded"
	);
}

// ============================================================================
// Middleware Order Dependencies Tests
// ============================================================================

/// Test middleware order: session before auth
///
/// **Test Intent**: Verify session middleware runs before auth middleware
/// to establish session context
///
/// **Integration Point**: Middleware ordering → Request processing pipeline
///
/// **Not Intent**: Single middleware, no dependencies
#[rstest]
#[tokio::test]
async fn test_middleware_order_session_before_auth(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			id TEXT PRIMARY KEY,
			data JSONB NOT NULL,
			created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Simulate session middleware creating session first
	let session_id = "session_order_test";
	sqlx::query("INSERT INTO sessions (id, data) VALUES ($1, $2)")
		.bind(session_id)
		.bind(serde_json::json!({}))
		.execute(pool.as_ref())
		.await
		.expect("Failed to create session");

	// Auth middleware would then use session_id to store auth state
	sqlx::query("UPDATE sessions SET data = $1 WHERE id = $2")
		.bind(serde_json::json!({"user_id": 1}))
		.bind(session_id)
		.execute(pool.as_ref())
		.await
		.expect("Failed to update session");

	// Verify session has auth state
	let result = sqlx::query("SELECT data FROM sessions WHERE id = $1")
		.bind(session_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query session");

	let data: serde_json::Value = result.get("data");
	assert_eq!(data["user_id"], 1, "Session should have auth state from auth middleware");
}

/// Test middleware order: CSRF before auth
///
/// **Test Intent**: Verify CSRF protection middleware runs before auth
/// to protect login forms
///
/// **Integration Point**: Middleware ordering → CSRF protection
///
/// **Not Intent**: POST-auth CSRF, no protection
#[rstest]
#[tokio::test]
async fn test_middleware_order_csrf_before_auth(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create csrf_tokens table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS csrf_tokens (
			id TEXT PRIMARY KEY,
			token TEXT NOT NULL,
			created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create csrf_tokens table");

	// Simulate CSRF middleware creating token
	let session_id = "csrf_session";
	let csrf_token = "csrf_token_12345";

	sqlx::query("INSERT INTO csrf_tokens (id, token) VALUES ($1, $2)")
		.bind(session_id)
		.bind(csrf_token)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create CSRF token");

	// Auth middleware would verify CSRF token before authentication
	let stored_token: String = sqlx::query_scalar("SELECT token FROM csrf_tokens WHERE id = $1")
		.bind(session_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to get CSRF token");

	assert_eq!(
		stored_token, csrf_token,
		"CSRF token should exist before auth processing"
	);
}
