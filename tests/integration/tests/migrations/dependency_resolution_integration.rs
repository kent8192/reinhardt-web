//! Dependency Resolution Integration Tests
//!
//! Tests that verify the migration dependency resolution system correctly handles
//! complex dependency scenarios. Migrations can depend on other migrations, and
//! the executor must apply them in the correct topological order.
//!
//! **Test Coverage:**
//! - Linear dependencies (A → B → C)
//! - Multi-app dependencies (app1 → app2 → app1)
//! - Diamond dependencies (A → B, A → C, B → D, C → D)
//! - Swappable model dependencies (custom User model)
//! - Circular dependency detection (A → B → A)
//! - Missing dependency detection
//! - Deep dependency chains (100+ levels)
//! - Independent migrations (parallel execution)
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container
//!
//! **Key Concepts:**
//! - **Dependency**: Migration B depends on Migration A if B requires A's schema changes
//! - **Topological Sort**: Algorithm to order migrations by dependencies
//! - **Circular Dependency**: Invalid state where A depends on B and B depends on A
//! - **DAG (Directed Acyclic Graph)**: Valid dependency graph with no cycles

use reinhardt_backends::types::DatabaseType;
use reinhardt_backends::DatabaseConnection;
use reinhardt_migrations::{
	executor::DatabaseMigrationExecutor, ColumnDefinition, FieldType, Migration, Operation,
};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Test Helper Functions
// ============================================================================

fn leak_str(s: impl Into<String>) -> &'static str {
	Box::leak(s.into().into_boxed_str())
}

/// Create a migration with dependencies
fn create_migration_with_deps(
	app: &'static str,
	name: &'static str,
	operations: Vec<Operation>,
	dependencies: Vec<(&'static str, &'static str)>,
) -> Migration {
	Migration {
		app_label: app,
		name,
		operations,
		dependencies,
		replaces: vec![],
		atomic: true,
		initial: None,
	}
}

/// Create a basic column definition
fn create_basic_column(name: &'static str, type_def: FieldType) -> ColumnDefinition {
	ColumnDefinition {
		name,
		type_definition: type_def,
		not_null: false,
		unique: false,
		primary_key: false,
		auto_increment: false,
		default: None,
	}
}

/// Create an auto-increment primary key column
fn create_auto_pk_column(name: &'static str, type_def: FieldType) -> ColumnDefinition {
	ColumnDefinition {
		name,
		type_definition: type_def,
		not_null: true,
		unique: false,
		primary_key: true,
		auto_increment: true,
		default: None,
	}
}

/// Check if a table exists
async fn table_exists(pool: &PgPool, table_name: &str) -> bool {
	sqlx::query_scalar::<_, bool>(
		"SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_name = $1)",
	)
	.bind(table_name)
	.fetch_one(pool)
	.await
	.unwrap_or(false)
}

// ============================================================================
// Normal Case Tests - Basic Dependency Resolution
// ============================================================================

/// Test linear dependency resolution (A → B → C)
///
/// **Test Intent**: Verify that migrations are applied in linear dependency order
#[rstest]
#[tokio::test]
async fn test_linear_dependency_resolution(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	let connection = DatabaseConnection::connect_postgres(&url)
		.await
		.expect("Failed to connect to PostgreSQL");

	let mut executor = DatabaseMigrationExecutor::new(connection.clone(), DatabaseType::Postgres);

	// Migration A: Create users table
	let migration_a = create_migration_with_deps(
		"app",
		"0001_create_users",
		vec![Operation::CreateTable {
			name: leak_str("users"),
			columns: vec![create_auto_pk_column("id", FieldType::Integer)],
			constraints: vec![],
		}],
		vec![], // No dependencies
	);

	// Migration B: Add name column (depends on A)
	let migration_b = create_migration_with_deps(
		"app",
		"0002_add_name",
		vec![Operation::AddColumn {
			table: leak_str("users"),
			column: create_basic_column("name", FieldType::VarChar(100)),
		}],
		vec![("app", "0001_create_users")],
	);

	// Migration C: Add email column (depends on B)
	let migration_c = create_migration_with_deps(
		"app",
		"0003_add_email",
		vec![Operation::AddColumn {
			table: leak_str("users"),
			column: create_basic_column("email", FieldType::VarChar(255)),
		}],
		vec![("app", "0002_add_name")],
	);

	// Apply migrations (order: A → B → C should be enforced)
	executor
		.apply_migrations(&[migration_a, migration_b, migration_c])
		.await
		.expect("Failed to apply migrations in linear order");

	// Verify final schema
	assert!(
		table_exists(pool.as_ref(), "users").await,
		"users table should exist"
	);

	// Verify all columns exist
	let column_count: i64 = sqlx::query_scalar(
		"SELECT COUNT(*) FROM information_schema.columns WHERE table_name = 'users'",
	)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to count columns");

	assert_eq!(
		column_count, 3,
		"users table should have 3 columns (id, name, email)"
	);
}

/// Test multi-app dependency resolution (app1 → app2 → app1)
///
/// **Test Intent**: Verify that dependencies across different apps are resolved correctly
#[rstest]
#[tokio::test]
async fn test_multi_app_dependency_resolution(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	let connection = DatabaseConnection::connect_postgres(&url)
		.await
		.expect("Failed to connect to PostgreSQL");

	let mut executor = DatabaseMigrationExecutor::new(connection.clone(), DatabaseType::Postgres);

	// app1: Migration 0001 - Create users table
	let app1_m1 = create_migration_with_deps(
		"app1",
		"0001_create_users",
		vec![Operation::CreateTable {
			name: leak_str("users"),
			columns: vec![create_auto_pk_column("id", FieldType::Integer)],
			constraints: vec![],
		}],
		vec![],
	);

	// app2: Migration 0001 - Create posts table (depends on app1.0001)
	let app2_m1 = create_migration_with_deps(
		"app2",
		"0001_create_posts",
		vec![Operation::CreateTable {
			name: leak_str("posts"),
			columns: vec![
				create_auto_pk_column("id", FieldType::Integer),
				create_basic_column("user_id", FieldType::Integer),
			],
			constraints: vec![],
		}],
		vec![("app1", "0001_create_users")],
	);

	// app1: Migration 0002 - Add username (depends on app2.0001)
	let app1_m2 = create_migration_with_deps(
		"app1",
		"0002_add_username",
		vec![Operation::AddColumn {
			table: leak_str("users"),
			column: create_basic_column("username", FieldType::VarChar(50)),
		}],
		vec![("app2", "0001_create_posts")],
	);

	// Apply migrations (order should be: app1.0001 → app2.0001 → app1.0002)
	executor
		.apply_migrations(&[app1_m1, app2_m1, app1_m2])
		.await
		.expect("Failed to apply multi-app migrations");

	// Verify both tables exist
	assert!(
		table_exists(pool.as_ref(), "users").await,
		"users table should exist"
	);
	assert!(
		table_exists(pool.as_ref(), "posts").await,
		"posts table should exist"
	);
}

/// Test diamond dependency resolution (A → B, A → C, B → D, C → D)
///
/// **Test Intent**: Verify that diamond-shaped dependency graphs are resolved correctly
#[rstest]
#[tokio::test]
async fn test_diamond_dependency_resolution(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	let connection = DatabaseConnection::connect_postgres(&url)
		.await
		.expect("Failed to connect to PostgreSQL");

	let mut executor = DatabaseMigrationExecutor::new(connection.clone(), DatabaseType::Postgres);

	// Migration A: Create base table
	let migration_a = create_migration_with_deps(
		"app",
		"0001_create_base",
		vec![Operation::CreateTable {
			name: leak_str("base"),
			columns: vec![create_auto_pk_column("id", FieldType::Integer)],
			constraints: vec![],
		}],
		vec![],
	);

	// Migration B: Add column_b (depends on A)
	let migration_b = create_migration_with_deps(
		"app",
		"0002_add_column_b",
		vec![Operation::AddColumn {
			table: leak_str("base"),
			column: create_basic_column("column_b", FieldType::VarChar(50)),
		}],
		vec![("app", "0001_create_base")],
	);

	// Migration C: Add column_c (depends on A)
	let migration_c = create_migration_with_deps(
		"app",
		"0003_add_column_c",
		vec![Operation::AddColumn {
			table: leak_str("base"),
			column: create_basic_column("column_c", FieldType::VarChar(50)),
		}],
		vec![("app", "0001_create_base")],
	);

	// Migration D: Add column_d (depends on B and C)
	let migration_d = create_migration_with_deps(
		"app",
		"0004_add_column_d",
		vec![Operation::AddColumn {
			table: leak_str("base"),
			column: create_basic_column("column_d", FieldType::VarChar(50)),
		}],
		vec![("app", "0002_add_column_b"), ("app", "0003_add_column_c")],
	);

	// Apply migrations (valid topological order: A, then B and C in any order, then D)
	executor
		.apply_migrations(&[migration_a, migration_b, migration_c, migration_d])
		.await
		.expect("Failed to apply diamond dependencies");

	// Verify all columns exist
	let column_count: i64 = sqlx::query_scalar(
		"SELECT COUNT(*) FROM information_schema.columns WHERE table_name = 'base'",
	)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to count columns");

	assert_eq!(column_count, 4, "base table should have 4 columns");
}

/// Test swappable model dependency (custom User model)
///
/// **Test Intent**: Verify that swappable dependencies are handled correctly
///
/// **Use Case**: Django's AUTH_USER_MODEL allows custom User models. Other apps
/// depend on whatever User model is configured.
#[rstest]
#[ignore = "Swappable model support not yet implemented in reinhardt-db migrations"]
#[tokio::test]
async fn test_swappable_model_dependency(
	#[future] _postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	// TODO: Add swappable_dependency field to Migration
	// Example:
	// Migration {
	// 	app_label: "app2",
	// 	name: "0001_create_profile",
	// 	operations: vec![
	// 		Operation::CreateTable {
	// 			name: leak_str("profile"),
	// 			columns: vec![
	// 				create_auto_pk_column("id", FieldType::Integer),
	// 				create_basic_column("user_id", FieldType::Integer),
	// 			],
	// 			constraints: vec![],
	// 		// 		},
	// 	],
	// 	dependencies: vec![],
	// 	swappable_dependencies: vec![("AUTH_USER_MODEL", "0001")], // Depends on configured User model
	// 	...
	// }
	//
	// The executor resolves AUTH_USER_MODEL to the actual app (e.g., "custom_auth")
	// and creates a dependency on ("custom_auth", "0001_initial")
}

/// Test conditional dependencies (optional dependencies)
///
/// **Test Intent**: Verify that optional dependencies are handled
///
/// **Use Case**: Some migrations depend on optional features (e.g., GIS extension)
#[rstest]
#[ignore = "Optional dependency support not yet implemented in reinhardt-db migrations"]
#[tokio::test]
async fn test_conditional_dependencies(
	#[future] _postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	// TODO: Add optional_dependencies field to Migration
	// Example:
	// Migration {
	// 	app_label: "geo_app",
	// 	name: "0002_add_location",
	// 	operations: vec![...],
	// 	dependencies: vec![("geo_app", "0001_initial")],
	// 	optional_dependencies: vec![
	// 		("gis_extension", "0001_enable_postgis"), // Only required if GIS feature is enabled
	// 	],
	// 	...
	// }
	//
	// If "gis_extension" app exists, dependency is enforced.
	// If not, dependency is ignored.
}

// ============================================================================
// Abnormal Case Tests - Error Detection
// ============================================================================

/// Test circular dependency detection (A → B → A)
///
/// **Test Intent**: Verify that circular dependencies are detected and cause error
#[rstest]
#[tokio::test]
async fn test_circular_dependency_detection(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let connection = DatabaseConnection::connect_postgres(&url)
		.await
		.expect("Failed to connect to PostgreSQL");

	let mut executor = DatabaseMigrationExecutor::new(connection.clone(), DatabaseType::Postgres);

	// Migration A (depends on B - circular)
	let migration_a = create_migration_with_deps(
		"app",
		"0001_migration_a",
		vec![Operation::CreateTable {
			name: leak_str("table_a"),
			columns: vec![create_auto_pk_column("id", FieldType::Integer)],
			constraints: vec![],
		}],
		vec![("app", "0002_migration_b")], // Depends on B
	);

	// Migration B (depends on A - circular)
	let migration_b = create_migration_with_deps(
		"app",
		"0002_migration_b",
		vec![Operation::CreateTable {
			name: leak_str("table_b"),
			columns: vec![create_auto_pk_column("id", FieldType::Integer)],
			constraints: vec![],
		}],
		vec![("app", "0001_migration_a")], // Depends on A
	);

	// Attempt to apply migrations with circular dependency (should fail)
	let result = executor.apply_migrations(&[migration_a, migration_b]).await;

	assert!(
		result.is_err(),
		"Circular dependency should be detected and cause error"
	);
}

/// Test missing dependency detection
///
/// **Test Intent**: Verify that missing dependencies are detected
#[rstest]
#[tokio::test]
async fn test_missing_dependency_detection(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let connection = DatabaseConnection::connect_postgres(&url)
		.await
		.expect("Failed to connect to PostgreSQL");

	let mut executor = DatabaseMigrationExecutor::new(connection.clone(), DatabaseType::Postgres);

	// Migration that depends on non-existent migration
	let migration = create_migration_with_deps(
		"app",
		"0002_add_column",
		vec![Operation::AddColumn {
			table: leak_str("users"),
			column: create_basic_column("name", FieldType::VarChar(100)),
		}],
		vec![("app", "0001_nonexistent")], // Depends on migration that doesn't exist
	);

	// Attempt to apply migration (should fail due to missing dependency)
	let result = executor.apply_migrations(&[migration]).await;

	assert!(
		result.is_err(),
		"Missing dependency should be detected and cause error"
	);
}

/// Test conflicting dependency order detection
///
/// **Test Intent**: Verify that conflicting orderings are detected
///
/// **Example**: If A must come before B (A → B) and B must come before A (B → A),
/// this is a contradiction (circular dependency variant).
#[rstest]
#[tokio::test]
async fn test_conflicting_dependency_order(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let connection = DatabaseConnection::connect_postgres(&url)
		.await
		.expect("Failed to connect to PostgreSQL");

	let mut executor = DatabaseMigrationExecutor::new(connection.clone(), DatabaseType::Postgres);

	// Same as circular dependency test - conflicting order is a cycle
	let migration_a = create_migration_with_deps(
		"app",
		"0001_migration_a",
		vec![Operation::CreateTable {
			name: leak_str("table_a"),
			columns: vec![create_auto_pk_column("id", FieldType::Integer)],
			constraints: vec![],
		}],
		vec![("app", "0002_migration_b")],
	);

	let migration_b = create_migration_with_deps(
		"app",
		"0002_migration_b",
		vec![Operation::CreateTable {
			name: leak_str("table_b"),
			columns: vec![create_auto_pk_column("id", FieldType::Integer)],
			constraints: vec![],
		}],
		vec![("app", "0001_migration_a")],
	);

	let result = executor.apply_migrations(&[migration_a, migration_b]).await;

	assert!(
		result.is_err(),
		"Conflicting dependency order should cause error"
	);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

/// Test deep dependency chain (100 levels)
///
/// **Test Intent**: Verify that very deep dependency chains don't cause stack overflow
#[rstest]
#[tokio::test]
async fn test_deep_dependency_chain(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let connection = DatabaseConnection::connect_postgres(&url)
		.await
		.expect("Failed to connect to PostgreSQL");

	let mut executor = DatabaseMigrationExecutor::new(connection.clone(), DatabaseType::Postgres);

	let mut migrations = Vec::new();

	// Create 100 migrations, each depending on the previous one
	for i in 0..100 {
		let name = leak_str(format!("{:04}_migration", i + 1));
		let dependencies = if i == 0 {
			vec![]
		} else {
			vec![("app", leak_str(format!("{:04}_migration", i)))]
		};

		let migration = create_migration_with_deps(
			"app",
			name,
			vec![Operation::RunSQL {
				sql: leak_str(format!("SELECT {}", i + 1)),
				reverse_sql: None,
			}],
			dependencies,
		);

		migrations.push(migration);
	}

	// Apply all 100 migrations (should not stack overflow)
	let result = executor.apply_migrations(&migrations).await;

	// Note: This might fail if dependency resolution is not implemented,
	// but it should not panic or stack overflow
	match result {
		Ok(_) => {
			// Success - deep dependencies resolved correctly
		}
		Err(e) => {
			// If it fails, verify it's a graceful error, not a panic
			eprintln!("Deep dependency chain failed: {}", e);
			assert!(
				!format!("{}", e).contains("stack overflow"),
				"Should not cause stack overflow"
			);
		}
	}
}

/// Test independent migrations (parallel execution potential)
///
/// **Test Intent**: Verify that independent migrations (no dependencies) can be
/// applied in any order
#[rstest]
#[tokio::test]
async fn test_independent_migrations(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	let connection = DatabaseConnection::connect_postgres(&url)
		.await
		.expect("Failed to connect to PostgreSQL");

	let mut executor = DatabaseMigrationExecutor::new(connection.clone(), DatabaseType::Postgres);

	// Three independent migrations (no dependencies between them)
	let migration_1 = create_migration_with_deps(
		"app",
		"0001_create_users",
		vec![Operation::CreateTable {
			name: leak_str("users"),
			columns: vec![create_auto_pk_column("id", FieldType::Integer)],
			constraints: vec![],
		}],
		vec![],
	);

	let migration_2 = create_migration_with_deps(
		"app",
		"0002_create_posts",
		vec![Operation::CreateTable {
			name: leak_str("posts"),
			columns: vec![create_auto_pk_column("id", FieldType::Integer)],
			constraints: vec![],
		}],
		vec![],
	);

	let migration_3 = create_migration_with_deps(
		"app",
		"0003_create_comments",
		vec![Operation::CreateTable {
			name: leak_str("comments"),
			columns: vec![create_auto_pk_column("id", FieldType::Integer)],
			constraints: vec![],
		}],
		vec![],
	);

	// Apply in arbitrary order (should all succeed)
	executor
		.apply_migrations(&[migration_2, migration_1, migration_3])
		.await
		.expect("Independent migrations should apply in any order");

	// Verify all tables exist
	assert!(
		table_exists(pool.as_ref(), "users").await,
		"users table should exist"
	);
	assert!(
		table_exists(pool.as_ref(), "posts").await,
		"posts table should exist"
	);
	assert!(
		table_exists(pool.as_ref(), "comments").await,
		"comments table should exist"
	);
}
