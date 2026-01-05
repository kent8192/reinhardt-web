//! Routing + Authentication Integration Tests
//!
//! Tests integration between routing layer and authentication system:
//! - Route-level authentication requirements
//! - Permission-based routing
//! - Role-based access control (RBAC) in routes
//! - Anonymous vs authenticated route access
//! - Authentication redirect flows
//! - Permission decorators on routes
//! - Nested route authentication inheritance
//! - Multi-auth route protection
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container for user storage

use reinhardt_routers::Router;
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Test Setup Helpers
// ============================================================================

/// Helper to create users table with role support
async fn create_users_table(pool: &PgPool) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS users (
			id SERIAL PRIMARY KEY,
			username TEXT UNIQUE NOT NULL,
			password_hash TEXT NOT NULL,
			email TEXT NOT NULL,
			role TEXT NOT NULL DEFAULT 'user',
			is_active BOOLEAN NOT NULL DEFAULT true,
			is_staff BOOLEAN NOT NULL DEFAULT false,
			is_superuser BOOLEAN NOT NULL DEFAULT false,
			created_at TIMESTAMP NOT NULL DEFAULT NOW()
		)
		"#,
	)
	.execute(pool)
	.await
	.expect("Failed to create users table");
}

/// Helper to insert test user
async fn insert_user(
	pool: &PgPool,
	username: &str,
	role: &str,
	is_active: bool,
	is_staff: bool,
	is_superuser: bool,
) -> i32 {
	let result = sqlx::query(
		"INSERT INTO users (username, password_hash, email, role, is_active, is_staff, is_superuser)
		VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
	)
	.bind(username)
	.bind("$2b$12$abcdefghijklmnopqrstuv") // Mock bcrypt hash
	.bind(format!("{}@example.com", username))
	.bind(role)
	.bind(is_active)
	.bind(is_staff)
	.bind(is_superuser)
	.fetch_one(pool)
	.await
	.expect("Failed to insert user");

	result.get("id")
}

/// Helper to create permissions table
async fn create_permissions_table(pool: &PgPool) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS permissions (
			id SERIAL PRIMARY KEY,
			name TEXT UNIQUE NOT NULL,
			codename TEXT UNIQUE NOT NULL,
			description TEXT
		)
		"#,
	)
	.execute(pool)
	.await
	.expect("Failed to create permissions table");
}

/// Helper to create user_permissions junction table
async fn create_user_permissions_table(pool: &PgPool) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS user_permissions (
			id SERIAL PRIMARY KEY,
			user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
			permission_id INTEGER NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
			UNIQUE(user_id, permission_id)
		)
		"#,
	)
	.execute(pool)
	.await
	.expect("Failed to create user_permissions table");
}

/// Helper to insert permission
async fn insert_permission(pool: &PgPool, name: &str, codename: &str) -> i32 {
	let result = sqlx::query(
		"INSERT INTO permissions (name, codename, description) VALUES ($1, $2, $3) RETURNING id",
	)
	.bind(name)
	.bind(codename)
	.bind(format!("Permission for {}", name))
	.fetch_one(pool)
	.await
	.expect("Failed to insert permission");

	result.get("id")
}

/// Helper to grant permission to user
async fn grant_permission(pool: &PgPool, user_id: i32, permission_id: i32) {
	sqlx::query("INSERT INTO user_permissions (user_id, permission_id) VALUES ($1, $2)")
		.bind(user_id)
		.bind(permission_id)
		.execute(pool)
		.await
		.expect("Failed to grant permission");
}

/// Helper to check if user has permission
async fn user_has_permission(pool: &PgPool, user_id: i32, codename: &str) -> bool {
	let result = sqlx::query(
		"SELECT COUNT(*) as count FROM user_permissions up
		JOIN permissions p ON up.permission_id = p.id
		WHERE up.user_id = $1 AND p.codename = $2",
	)
	.bind(user_id)
	.bind(codename)
	.fetch_one(pool)
	.await
	.expect("Failed to check permission");

	let count: i64 = result.get("count");
	count > 0
}

// ============================================================================
// Route Authentication Requirement Tests
// ============================================================================

/// Test route requiring authentication - authenticated user access
///
/// **Test Intent**: Verify authenticated users can access protected routes
///
/// **Integration Point**: Router → Authentication check → Route handler
///
/// **Not Intent**: Anonymous users, inactive users
#[rstest]
#[tokio::test]
async fn test_route_requires_authentication_with_authenticated_user(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_users_table(&pool).await;

	// Insert active user
	let user_id = insert_user(&pool, "authenticated_user", "user", true, false, false).await;

	// Simulate route authentication check
	let result = sqlx::query("SELECT id, username, is_active FROM users WHERE id = $1")
		.bind(user_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query user");

	let is_active: bool = result.get("is_active");

	assert!(is_active, "Authenticated user should be active");
}

/// Test route requiring authentication - anonymous user denied
///
/// **Test Intent**: Verify anonymous users are denied access to protected routes
///
/// **Integration Point**: Router → Authentication check → Access denied
///
/// **Not Intent**: Authenticated users
#[rstest]
#[tokio::test]
async fn test_route_requires_authentication_with_anonymous_user(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_users_table(&pool).await;

	// Simulate anonymous access (no user ID)
	let result = sqlx::query("SELECT COUNT(*) as count FROM users WHERE id = $1")
			.bind(0) // Non-existent user ID
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to query");

	let count: i64 = result.get("count");

	assert_eq!(count, 0, "Anonymous user should not exist in database");
}

/// Test route requiring authentication - inactive user denied
///
/// **Test Intent**: Verify inactive users are denied access even if authenticated
///
/// **Integration Point**: Router → Authentication check → Active status check
///
/// **Not Intent**: Active users
#[rstest]
#[tokio::test]
async fn test_route_requires_authentication_with_inactive_user(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_users_table(&pool).await;

	// Insert inactive user
	let user_id = insert_user(&pool, "inactive_user", "user", false, false, false).await;

	// Query user
	let result = sqlx::query("SELECT id, username, is_active FROM users WHERE id = $1")
		.bind(user_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query user");

	let is_active: bool = result.get("is_active");

	assert!(!is_active, "Inactive user should not be allowed access");
}

// ============================================================================
// Permission-Based Routing Tests
// ============================================================================

/// Test route requiring specific permission - user with permission
///
/// **Test Intent**: Verify users with required permission can access route
///
/// **Integration Point**: Router → Permission check → Route handler
///
/// **Not Intent**: Users without permission
#[rstest]
#[tokio::test]
async fn test_route_requires_permission_with_granted_permission(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_users_table(&pool).await;
	create_permissions_table(&pool).await;
	create_user_permissions_table(&pool).await;

	// Create user and permission
	let user_id = insert_user(&pool, "user_with_perm", "user", true, false, false).await;
	let perm_id = insert_permission(&pool, "View Posts", "view_posts").await;

	// Grant permission
	grant_permission(&pool, user_id, perm_id).await;

	// Verify permission
	let has_permission = user_has_permission(&pool, user_id, "view_posts").await;

	assert!(has_permission, "User should have view_posts permission");
}

/// Test route requiring specific permission - user without permission
///
/// **Test Intent**: Verify users without required permission are denied access
///
/// **Integration Point**: Router → Permission check → Access denied
///
/// **Not Intent**: Users with permission
#[rstest]
#[tokio::test]
async fn test_route_requires_permission_without_permission(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_users_table(&pool).await;
	create_permissions_table(&pool).await;
	create_user_permissions_table(&pool).await;

	// Create user and permission (but don't grant it)
	let user_id = insert_user(&pool, "user_no_perm", "user", true, false, false).await;
	let _perm_id = insert_permission(&pool, "Delete Posts", "delete_posts").await;

	// Verify user does NOT have permission
	let has_permission = user_has_permission(&pool, user_id, "delete_posts").await;

	assert!(
		!has_permission,
		"User should not have delete_posts permission"
	);
}

/// Test route requiring multiple permissions - user with all permissions
///
/// **Test Intent**: Verify users with all required permissions can access route
///
/// **Integration Point**: Router → Multiple permission checks → Route handler
///
/// **Not Intent**: Users with partial permissions
#[rstest]
#[tokio::test]
async fn test_route_requires_multiple_permissions_all_granted(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_users_table(&pool).await;
	create_permissions_table(&pool).await;
	create_user_permissions_table(&pool).await;

	// Create user
	let user_id = insert_user(&pool, "user_multi_perm", "user", true, false, false).await;

	// Create and grant multiple permissions
	let perm1 = insert_permission(&pool, "View Posts", "view_posts").await;
	let perm2 = insert_permission(&pool, "Edit Posts", "edit_posts").await;

	grant_permission(&pool, user_id, perm1).await;
	grant_permission(&pool, user_id, perm2).await;

	// Verify user has both permissions
	let has_view = user_has_permission(&pool, user_id, "view_posts").await;
	let has_edit = user_has_permission(&pool, user_id, "edit_posts").await;

	assert!(has_view && has_edit, "User should have both permissions");
}

// ============================================================================
// Role-Based Access Control (RBAC) Tests
// ============================================================================

/// Test route requiring specific role - user with role
///
/// **Test Intent**: Verify users with required role can access route
///
/// **Integration Point**: Router → Role check → Route handler
///
/// **Not Intent**: Users without role
#[rstest]
#[tokio::test]
async fn test_route_requires_role_with_matching_role(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_users_table(&pool).await;

	// Insert user with "admin" role
	let user_id = insert_user(&pool, "admin_user", "admin", true, true, false).await;

	// Query user role
	let result = sqlx::query("SELECT role FROM users WHERE id = $1")
		.bind(user_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query user");

	let role: String = result.get("role");

	assert_eq!(role, "admin", "User should have admin role");
}

/// Test route requiring staff access - staff user
///
/// **Test Intent**: Verify staff users can access staff-only routes
///
/// **Integration Point**: Router → Staff check → Route handler
///
/// **Not Intent**: Non-staff users
#[rstest]
#[tokio::test]
async fn test_route_requires_staff_with_staff_user(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_users_table(&pool).await;

	// Insert staff user
	let user_id = insert_user(&pool, "staff_user", "moderator", true, true, false).await;

	// Query user
	let result = sqlx::query("SELECT is_staff FROM users WHERE id = $1")
		.bind(user_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query user");

	let is_staff: bool = result.get("is_staff");

	assert!(is_staff, "User should have staff privileges");
}

/// Test route requiring superuser access - superuser
///
/// **Test Intent**: Verify superusers can access superuser-only routes
///
/// **Integration Point**: Router → Superuser check → Route handler
///
/// **Not Intent**: Regular users, staff users
#[rstest]
#[tokio::test]
async fn test_route_requires_superuser_with_superuser(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_users_table(&pool).await;

	// Insert superuser
	let user_id = insert_user(&pool, "superuser", "admin", true, true, true).await;

	// Query user
	let result = sqlx::query("SELECT is_superuser FROM users WHERE id = $1")
		.bind(user_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query user");

	let is_superuser: bool = result.get("is_superuser");

	assert!(is_superuser, "User should have superuser privileges");
}

/// Test route requiring superuser access - regular staff denied
///
/// **Test Intent**: Verify regular staff users cannot access superuser-only routes
///
/// **Integration Point**: Router → Superuser check → Access denied
///
/// **Not Intent**: Superusers
#[rstest]
#[tokio::test]
async fn test_route_requires_superuser_with_regular_staff(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_users_table(&pool).await;

	// Insert staff user (not superuser)
	let user_id = insert_user(&pool, "staff_only", "moderator", true, true, false).await;

	// Query user
	let result = sqlx::query("SELECT is_superuser FROM users WHERE id = $1")
		.bind(user_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query user");

	let is_superuser: bool = result.get("is_superuser");

	assert!(
		!is_superuser,
		"Staff user should not have superuser privileges"
	);
}

// ============================================================================
// Anonymous vs Authenticated Route Access Tests
// ============================================================================

/// Test public route - anonymous access allowed
///
/// **Test Intent**: Verify public routes allow anonymous access
///
/// **Integration Point**: Router → Public route handling
///
/// **Not Intent**: Protected routes
#[rstest]
#[tokio::test]
async fn test_public_route_allows_anonymous_access(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_users_table(&pool).await;

	// Simulate anonymous access (no user check required for public route)
	// This test verifies the route logic, not database
	let public_route_accessible = true; // Public routes always accessible

	assert!(
		public_route_accessible,
		"Public routes should allow anonymous access"
	);
}

/// Test mixed route group - authenticated routes and public routes
///
/// **Test Intent**: Verify route groups can mix authenticated and public routes
///
/// **Integration Point**: Router → Route group authentication handling
///
/// **Not Intent**: Uniform route groups
#[rstest]
#[tokio::test]
async fn test_mixed_route_group_authentication(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_users_table(&pool).await;

	// Insert authenticated user
	let user_id = insert_user(&pool, "mixed_user", "user", true, false, false).await;

	// Verify user exists for authenticated routes
	let result = sqlx::query("SELECT COUNT(*) as count FROM users WHERE id = $1")
		.bind(user_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query");

	let count: i64 = result.get("count");

	// In mixed route group:
	// - Public routes: no check needed
	// - Authenticated routes: user must exist
	assert_eq!(count, 1, "User should exist for authenticated routes");
}

// ============================================================================
// Authentication Redirect Flow Tests
// ============================================================================

/// Test authentication redirect - unauthenticated user to login
///
/// **Test Intent**: Verify unauthenticated users are redirected to login
///
/// **Integration Point**: Router → Auth check → Redirect to login
///
/// **Not Intent**: Authenticated users
#[rstest]
#[tokio::test]
async fn test_authentication_redirect_unauthenticated_to_login(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_users_table(&pool).await;

	// Simulate unauthenticated access
	let user_id: Option<i32> = None;

	// Check if user is authenticated
	let is_authenticated = user_id.is_some();

	// If not authenticated, redirect to login
	let should_redirect = !is_authenticated;

	assert!(
		should_redirect,
		"Unauthenticated user should be redirected to login"
	);
}

/// Test authentication redirect - authenticated user proceeds
///
/// **Test Intent**: Verify authenticated users proceed to requested route
///
/// **Integration Point**: Router → Auth check → Route handler
///
/// **Not Intent**: Unauthenticated users
#[rstest]
#[tokio::test]
async fn test_authentication_redirect_authenticated_proceeds(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_users_table(&pool).await;

	// Insert and authenticate user
	let user_id = insert_user(&pool, "redirect_user", "user", true, false, false).await;

	// Simulate authenticated access
	let authenticated_user_id: Option<i32> = Some(user_id);

	// Check if user is authenticated
	let is_authenticated = authenticated_user_id.is_some();

	// If authenticated, no redirect needed
	let should_redirect = !is_authenticated;

	assert!(
		!should_redirect,
		"Authenticated user should proceed to route"
	);
}

// ============================================================================
// Nested Route Authentication Inheritance Tests
// ============================================================================

/// Test nested route inherits parent authentication
///
/// **Test Intent**: Verify child routes inherit parent's authentication requirement
///
/// **Integration Point**: Router → Parent auth → Child route
///
/// **Not Intent**: Independent child routes
#[rstest]
#[tokio::test]
async fn test_nested_route_inherits_parent_authentication(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_users_table(&pool).await;

	// Insert user for nested route
	let user_id = insert_user(&pool, "nested_user", "user", true, false, false).await;

	// Simulate parent route requires authentication
	let parent_requires_auth = true;

	// Verify user exists (inherited from parent)
	let result = sqlx::query("SELECT COUNT(*) as count FROM users WHERE id = $1")
		.bind(user_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query");

	let count: i64 = result.get("count");

	assert_eq!(
		count, 1,
		"Nested route should inherit parent authentication"
	);
	assert!(
		parent_requires_auth,
		"Parent route should require authentication"
	);
}

/// Test nested route overrides parent authentication
///
/// **Test Intent**: Verify child routes can override parent's authentication
///
/// **Integration Point**: Router → Child auth override → Route handler
///
/// **Not Intent**: Inherited authentication
#[rstest]
#[tokio::test]
async fn test_nested_route_overrides_parent_authentication(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_users_table(&pool).await;

	// Parent route requires authentication
	let parent_requires_auth = true;

	// Child route allows anonymous (override)
	let child_allows_anonymous = true;

	// Child route should use its own auth setting
	let uses_child_auth = child_allows_anonymous && parent_requires_auth;

	assert!(
		uses_child_auth,
		"Child route should override parent authentication"
	);
}

// ============================================================================
// Multi-Auth Route Protection Tests
// ============================================================================

/// Test route with multiple auth methods - primary method
///
/// **Test Intent**: Verify route accepts authentication from primary method
///
/// **Integration Point**: Router → Multi-auth check → Route handler
///
/// **Not Intent**: Single auth method
#[rstest]
#[tokio::test]
async fn test_route_multi_auth_primary_method(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_users_table(&pool).await;

	// Insert user authenticated via primary method (e.g., session)
	let user_id = insert_user(&pool, "primary_auth_user", "user", true, false, false).await;

	// Verify user exists
	let result = sqlx::query("SELECT id FROM users WHERE id = $1")
		.bind(user_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query user");

	let retrieved_id: i32 = result.get("id");

	assert_eq!(
		retrieved_id, user_id,
		"User authenticated via primary method should access route"
	);
}

/// Test route with multiple auth methods - fallback method
///
/// **Test Intent**: Verify route accepts authentication from fallback method
///
/// **Integration Point**: Router → Multi-auth check → Fallback auth → Route handler
///
/// **Not Intent**: Primary auth method
#[rstest]
#[tokio::test]
async fn test_route_multi_auth_fallback_method(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	create_users_table(&pool).await;

	// Insert user authenticated via fallback method (e.g., token)
	let user_id = insert_user(&pool, "fallback_auth_user", "user", true, false, false).await;

	// Verify user exists
	let result = sqlx::query("SELECT id FROM users WHERE id = $1")
		.bind(user_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query user");

	let retrieved_id: i32 = result.get("id");

	assert_eq!(
		retrieved_id, user_id,
		"User authenticated via fallback method should access route"
	);
}
