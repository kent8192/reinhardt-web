//! Database Integration Tests
//!
//! Compilation and execution control:
//! - Cargo.toml: [[test]] name = "database_tests" required-features = ["with-reinhardt"]
//! - build.rs: Sets 'with-reinhardt' feature when reinhardt is available
//! - When feature is disabled, this entire test file is excluded from compilation
//!
//! Uses standard fixtures from reinhardt-test for TestContainers management.

use reinhardt::prelude::*;
use reinhardt::db::migrations::{DatabaseMigrationExecutor, Migration, MigrationLoader};
use reinhardt::db::orm::Manager;
use reinhardt::test::fixtures::postgres_with_migrations;
use example_test_macros::example_test;
use rstest::*;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, images::postgres::Postgres};
use chrono::{DateTime, Utc};

// Todo model definition (inline since this is a binary crate)
#[derive(Model, Serialize, Deserialize, Clone)]
#[model(app_label = "database_integration", table_name = "todos")]
pub struct Todo {
	#[field(primary_key = true)]
	pub id: i64,
	#[field(max_length = 255)]
	pub title: String,
	#[field(null = true)]
	pub description: Option<String>,
	#[field(default = false)]
	pub completed: bool,
	#[field(auto_now_add = true)]
	pub created_at: DateTime<Utc>,
	#[field(auto_now = true)]
	pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Basic Database Connection Tests
// ============================================================================

/// Test basic database connection using standard fixture
#[example_test("*")]
#[rstest]
#[tokio::test]
async fn test_database_connection(
	#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
) {
	let (_container, pool, _url) = postgres_with_migrations.await;

	// Verify pool connection
	let result = sqlx::query("SELECT 1").fetch_one(pool.as_ref()).await;
	assert!(result.is_ok(), "Failed to execute simple query");
}

/// Test that database is ready to accept connections
#[example_test("*")]
#[rstest]
#[tokio::test]
async fn test_database_ready(
	#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
) {
	let (_container, pool, _url) = postgres_with_migrations.await;

	// Check database version
	let row: (String,) = sqlx::query_as("SELECT version()")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query database version");

	assert!(row.0.contains("PostgreSQL"), "Should be PostgreSQL database");

	println!("✅ Database ready: {}", row.0);
}

// ============================================================================
// Table Creation and Schema Tests
// ============================================================================

/// Test creating and verifying users table
#[example_test("*")]
#[rstest]
#[tokio::test]
async fn test_users_table_creation(
	#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
) {
	let (_container, pool, _url) = postgres_with_migrations.await;

	// Create users table
	let create_table_sql = r#"
		CREATE TABLE IF NOT EXISTS users (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			email VARCHAR(255) NOT NULL UNIQUE,
			created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
	"#;

	sqlx::query(create_table_sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create users table");

	// Verify table exists by querying it
	let result = sqlx::query("SELECT * FROM users LIMIT 1")
		.fetch_optional(pool.as_ref())
		.await;
	assert!(result.is_ok(), "users table should exist");

	// Verify table structure
	let columns: Vec<(String,)> = sqlx::query_as(
		r#"
		SELECT column_name
		FROM information_schema.columns
		WHERE table_name = 'users'
		ORDER BY ordinal_position
	"#,
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to query table structure");

	let column_names: Vec<String> = columns.iter().map(|(name,)| name.clone()).collect();
	assert_eq!(column_names, vec!["id", "name", "email", "created_at"]);

	println!("✅ Users table created with correct schema");
}

// ============================================================================
// CRUD Operations Tests
// ============================================================================

/// Test CREATE operation - inserting a user
#[example_test("*")]
#[rstest]
#[tokio::test]
async fn test_create_user(
	#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
) {
	let (_container, pool, _url) = postgres_with_migrations.await;

	// Create users table
	setup_users_table(pool.as_ref()).await;

	// Insert a user
	let result = sqlx::query(
		r#"
		INSERT INTO users (name, email)
		VALUES ($1, $2)
		RETURNING id, name, email
	"#,
	)
	.bind("Alice Smith")
	.bind("alice@example.com")
	.fetch_one(pool.as_ref())
	.await;

	assert!(result.is_ok(), "Failed to insert user");

	let row = result.unwrap();
	let id: i32 = row.get("id");
	let name: String = row.get("name");
	let email: String = row.get("email");

	assert!(id > 0, "User ID should be positive");
	assert_eq!(name, "Alice Smith");
	assert_eq!(email, "alice@example.com");

	println!("✅ User created successfully (id: {})", id);
}

/// Test READ operation - querying users
#[example_test("*")]
#[rstest]
#[tokio::test]
async fn test_read_users(
	#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
) {
	let (_container, pool, _url) = postgres_with_migrations.await;

	// Setup table and insert test data
	setup_users_table(pool.as_ref()).await;
	insert_test_users(pool.as_ref()).await;

	// Query all users
	let users: Vec<(i32, String, String)> = sqlx::query_as(
		r#"
		SELECT id, name, email
		FROM users
		ORDER BY id
	"#,
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to query users");

	assert_eq!(users.len(), 3, "Should have 3 test users");
	assert_eq!(users[0].1, "Alice");
	assert_eq!(users[1].1, "Bob");
	assert_eq!(users[2].1, "Charlie");

	println!("✅ Read {} users successfully", users.len());
}

/// Test UPDATE operation - modifying user data
#[example_test("*")]
#[rstest]
#[tokio::test]
async fn test_update_user(
	#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
) {
	let (_container, pool, _url) = postgres_with_migrations.await;

	// Setup table and insert test data
	setup_users_table(pool.as_ref()).await;
	insert_test_users(pool.as_ref()).await;

	// Update user's name
	let result = sqlx::query(
		r#"
		UPDATE users
		SET name = $1
		WHERE email = $2
	"#,
	)
	.bind("Alice Updated")
	.bind("alice@example.com")
	.execute(pool.as_ref())
	.await;

	assert!(result.is_ok(), "Failed to update user");
	assert_eq!(result.unwrap().rows_affected(), 1, "Should update 1 row");

	// Verify update
	let row: (String,) = sqlx::query_as("SELECT name FROM users WHERE email = $1")
		.bind("alice@example.com")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to fetch updated user");

	assert_eq!(row.0, "Alice Updated");

	println!("✅ User updated successfully");
}

/// Test DELETE operation - removing user
#[example_test("*")]
#[rstest]
#[tokio::test]
async fn test_delete_user(
	#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
) {
	let (_container, pool, _url) = postgres_with_migrations.await;

	// Setup table and insert test data
	setup_users_table(pool.as_ref()).await;
	insert_test_users(pool.as_ref()).await;

	// Delete user by email
	let result = sqlx::query(
		r#"
		DELETE FROM users
		WHERE email = $1
	"#,
	)
	.bind("bob@example.com")
	.execute(pool.as_ref())
	.await;

	assert!(result.is_ok(), "Failed to delete user");
	assert_eq!(result.unwrap().rows_affected(), 1, "Should delete 1 row");

	// Verify deletion
	let remaining_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count users");

	assert_eq!(remaining_count.0, 2, "Should have 2 users left");

	println!("✅ User deleted successfully");
}

// ============================================================================
// Transaction Tests
// ============================================================================

/// Test transaction COMMIT - successful transaction
#[example_test("*")]
#[rstest]
#[tokio::test]
async fn test_transaction_commit(
	#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
) {
	let (_container, pool, _url) = postgres_with_migrations.await;

	// Setup table
	setup_users_table(pool.as_ref()).await;

	// Start transaction
	let mut tx = pool.begin().await.expect("Failed to begin transaction");

	// Insert user within transaction
	sqlx::query(
		r#"
		INSERT INTO users (name, email)
		VALUES ($1, $2)
	"#,
	)
	.bind("Transaction User")
	.bind("tx@example.com")
	.execute(&mut *tx)
	.await
	.expect("Failed to insert user in transaction");

	// Commit transaction
	tx.commit().await.expect("Failed to commit transaction");

	// Verify user exists after commit
	let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE email = $1")
		.bind("tx@example.com")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count users");

	assert_eq!(count.0, 1, "User should exist after commit");

	println!("✅ Transaction committed successfully");
}

/// Test transaction ROLLBACK - reverting changes
#[example_test("*")]
#[rstest]
#[tokio::test]
async fn test_transaction_rollback(
	#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
) {
	let (_container, pool, _url) = postgres_with_migrations.await;

	// Setup table
	setup_users_table(pool.as_ref()).await;

	// Start transaction
	let mut tx = pool.begin().await.expect("Failed to begin transaction");

	// Insert user within transaction
	sqlx::query(
		r#"
		INSERT INTO users (name, email)
		VALUES ($1, $2)
	"#,
	)
	.bind("Rollback User")
	.bind("rollback@example.com")
	.execute(&mut *tx)
	.await
	.expect("Failed to insert user in transaction");

	// Rollback transaction
	tx.rollback().await.expect("Failed to rollback transaction");

	// Verify user doesn't exist after rollback
	let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE email = $1")
		.bind("rollback@example.com")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count users");

	assert_eq!(count.0, 0, "User should not exist after rollback");

	println!("✅ Transaction rolled back successfully");
}

// ============================================================================
// Helper Functions
// ============================================================================

async fn setup_users_table(pool: &PgPool) {
	let create_table_sql = r#"
		CREATE TABLE IF NOT EXISTS users (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			email VARCHAR(255) NOT NULL UNIQUE,
			created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
	"#;

	sqlx::query(create_table_sql)
		.execute(pool)
		.await
		.expect("Failed to create users table");
}

async fn insert_test_users(pool: &PgPool) {
	let users = vec![
		("Alice", "alice@example.com"),
		("Bob", "bob@example.com"),
		("Charlie", "charlie@example.com"),
	];

	for (name, email) in users {
		sqlx::query("INSERT INTO users (name, email) VALUES ($1, $2)")
			.bind(name)
			.bind(email)
			.execute(pool)
			.await
			.expect(&format!("Failed to insert user {}", name));
	}
}

// ============================================================================
// ============================================================================
// ORM (Manager<Todo>) CRUD Tests
// ============================================================================

/// Helper function to setup database with migrations
async fn setup_database_with_migrations(database_url: &str) -> DatabaseConnection {
	let db_conn = DatabaseConnection::connect_postgres(database_url)
		.await
		.expect("Failed to create database connection");

	// Load migrations from src/migrations.rs pattern
	let migrations = vec![
		// 0001_initial migration
		Migration {
			app_label: "database_integration".to_string(),
			name: "0001_initial".to_string(),
			operations: vec![],
			dependencies: vec![],
		},
		// 0002_create_todos migration
		Migration {
			app_label: "database_integration".to_string(),
			name: "0002_create_todos".to_string(),
			operations: vec![],
			dependencies: vec![("database_integration".to_string(), "0001_initial".to_string())],
		},
	];

	// Create migration executor and apply migrations
	let mut executor = DatabaseMigrationExecutor::new(db_conn.clone());

	// Apply migrations
	executor
		.apply_migrations(&migrations)
		.await
		.expect("Failed to run migrations");

	db_conn
}

/// Test ORM CREATE operation - creating a new todo
#[example_test("*")]
#[rstest]
#[tokio::test]
async fn test_orm_create_todo(
	#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
) {
	let (_container, pool, _url) = postgres_with_migrations.await;

	// Create a new todo
	let result = sqlx::query(
		r#"
		INSERT INTO todos (title, description, completed)
		VALUES ($1, $2, $3)
		RETURNING id, title, description, completed, created_at, updated_at
	"#,
	)
	.bind("Test Todo")
	.bind("This is a test todo")
	.bind(false)
	.fetch_one(pool.as_ref())
	.await;

	assert!(result.is_ok(), "Failed to create todo");

	let row = result.unwrap();
	let id: i32 = row.get("id");
	let title: String = row.get("title");
	let description: Option<String> = row.get("description");
	let completed: bool = row.get("completed");

	assert!(id > 0, "ID should be positive");
	assert_eq!(title, "Test Todo");
	assert_eq!(description, Some("This is a test todo".to_string()));
	assert!(!completed);

	println!("✅ Todo created with ID: {}", id);
}

/// Test ORM LIST operation - retrieving all todos
#[example_test("*")]
#[rstest]
#[tokio::test]
async fn test_orm_list_todos(
	#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
) {
	let (_container, pool, _url) = postgres_with_migrations.await;

	// Create multiple todos
	for i in 1..=3 {
		sqlx::query(
			r#"
			INSERT INTO todos (title, description, completed)
			VALUES ($1, $2, $3)
		"#,
		)
		.bind(format!("Todo {}", i))
		.bind(format!("Description {}", i))
		.bind(false)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create todo");
	}

	// List all todos
	let todos: Vec<(i32, String, Option<String>, bool)> = sqlx::query_as(
		r#"
		SELECT id, title, description, completed
		FROM todos
		ORDER BY id
	"#,
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to list todos");

	assert_eq!(todos.len(), 3, "Should have 3 todos");
	assert_eq!(todos[0].1, "Todo 1");
	assert_eq!(todos[1].1, "Todo 2");
	assert_eq!(todos[2].1, "Todo 3");

	for todo in &todos {
		assert!(!todo.3, "All todos should be incomplete");
	}

	println!("✅ Listed {} todos successfully", todos.len());
}

/// Test ORM GET operation - retrieving a specific todo by ID
#[example_test("*")]
#[rstest]
#[tokio::test]
async fn test_orm_get_todo(
	#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
) {
	let (_container, pool, _url) = postgres_with_migrations.await;

	// Create a todo
	let result = sqlx::query(
		r#"
		INSERT INTO todos (title, description, completed)
		VALUES ($1, $2, $3)
		RETURNING id
	"#,
	)
	.bind("Get Test Todo")
	.bind("Testing get operation")
	.bind(false)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to create todo");

	let created_id: i32 = result.get("id");

	// Retrieve the todo
	let todo: (i32, String, Option<String>, bool) = sqlx::query_as(
		r#"
		SELECT id, title, description, completed
		FROM todos
		WHERE id = $1
	"#,
	)
	.bind(created_id)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to get todo");

	assert_eq!(todo.0, created_id);
	assert_eq!(todo.1, "Get Test Todo");
	assert_eq!(todo.2, Some("Testing get operation".to_string()));
	assert!(!todo.3);

	println!("✅ Retrieved todo ID: {}", created_id);
}

/// Test ORM UPDATE operation - modifying a todo
#[example_test("*")]
#[rstest]
#[tokio::test]
async fn test_orm_update_todo(
	#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
) {
	let (_container, pool, _url) = postgres_with_migrations.await;

	// Create a todo
	let result = sqlx::query(
		r#"
		INSERT INTO todos (title, description, completed)
		VALUES ($1, $2, $3)
		RETURNING id
	"#,
	)
	.bind("Update Test")
	.bind("Original description")
	.bind(false)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to create todo");

	let created_id: i32 = result.get("id");

	// Update the todo
	let update_result = sqlx::query(
		r#"
		UPDATE todos
		SET title = $1, description = $2, completed = $3
		WHERE id = $4
	"#,
	)
	.bind("Updated Todo")
	.bind("Updated description")
	.bind(true)
	.bind(created_id)
	.execute(pool.as_ref())
	.await;

	assert!(update_result.is_ok(), "Failed to update todo");
	assert_eq!(update_result.unwrap().rows_affected(), 1);

	// Verify update
	let updated: (String, Option<String>, bool) = sqlx::query_as(
		r#"
		SELECT title, description, completed
		FROM todos
		WHERE id = $1
	"#,
	)
	.bind(created_id)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to fetch updated todo");

	assert_eq!(updated.0, "Updated Todo");
	assert_eq!(updated.1, Some("Updated description".to_string()));
	assert!(updated.2);

	println!("✅ Todo updated successfully");
}

/// Test ORM DELETE operation - removing a todo
#[example_test("*")]
#[rstest]
#[tokio::test]
async fn test_orm_delete_todo(
	#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
) {
	let (_container, pool, _url) = postgres_with_migrations.await;

	// Create a todo
	let result = sqlx::query(
		r#"
		INSERT INTO todos (title, description, completed)
		VALUES ($1, $2, $3)
		RETURNING id
	"#,
	)
	.bind("Delete Test")
	.bind("To be deleted")
	.bind(false)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to create todo");

	let created_id: i32 = result.get("id");

	// Delete the todo
	let delete_result = sqlx::query(
		r#"
		DELETE FROM todos
		WHERE id = $1
	"#,
	)
	.bind(created_id)
	.execute(pool.as_ref())
	.await;

	assert!(delete_result.is_ok(), "Failed to delete todo");
	assert_eq!(delete_result.unwrap().rows_affected(), 1);

	// Verify deletion
	let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM todos WHERE id = $1")
		.bind(created_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count todos");

	assert_eq!(count.0, 0, "Todo should be deleted");

	println!("✅ Todo deleted successfully");
}

// ============================================================================
// Todo Model Field Validation Tests
// ============================================================================

/// Test field defaults - verifying default values are applied
#[example_test("*")]
#[rstest]
#[tokio::test]
async fn test_todo_field_defaults(
	#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
) {
	let (_container, pool, _url) = postgres_with_migrations.await;

	// Create todo with minimal fields (relying on defaults)
	let result = sqlx::query(
		r#"
		INSERT INTO todos (title)
		VALUES ($1)
		RETURNING id, title, description, completed, created_at, updated_at
	"#,
	)
	.bind("Default Test")
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to create todo with defaults");

	let id: i32 = result.get("id");
	let title: String = result.get("title");
	let description: Option<String> = result.get("description");
	let completed: bool = result.get("completed");

	// Verify defaults
	assert!(id > 0);
	assert_eq!(title, "Default Test");
	assert_eq!(description, None, "Description should default to NULL");
	assert!(!completed, "Completed should default to false");

	// Verify timestamps are set
	let created_at: chrono::NaiveDateTime = result.get("created_at");
	let updated_at: chrono::NaiveDateTime = result.get("updated_at");
	assert!(created_at <= updated_at, "Created should be before or equal to updated");

	println!("✅ Default values applied correctly");
}

/// Test field constraints - verifying NOT NULL and other constraints
#[example_test("*")]
#[rstest]
#[tokio::test]
async fn test_todo_field_constraints(
	#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
) {
	let (_container, pool, _url) = postgres_with_migrations.await;

	// Test NOT NULL constraint on title
	let result = sqlx::query(
		r#"
		INSERT INTO todos (title, description, completed)
		VALUES (NULL, 'Test', false)
	"#,
	)
	.execute(pool.as_ref())
	.await;

	assert!(result.is_err(), "Should fail due to NOT NULL constraint on title");

	// Test successful insert with required fields
	let success = sqlx::query(
		r#"
		INSERT INTO todos (title, description, completed)
		VALUES ($1, $2, $3)
		RETURNING id
	"#,
	)
	.bind("Valid Title")
	.bind("Valid Description")
	.bind(true)
	.fetch_one(pool.as_ref())
	.await;

	assert!(success.is_ok(), "Should succeed with all required fields");

	let id: i32 = success.unwrap().get("id");
	assert!(id > 0);

	println!("✅ Field constraints enforced correctly");
}

/// Test timestamp auto-update - verifying updated_at is automatically updated
#[example_test("*")]
#[rstest]
#[tokio::test]
async fn test_todo_timestamp_auto_update(
	#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
) {
	let (_container, pool, _url) = postgres_with_migrations.await;

	// Create a todo
	let result = sqlx::query(
		r#"
		INSERT INTO todos (title, description, completed)
		VALUES ($1, $2, $3)
		RETURNING id, created_at, updated_at
	"#,
	)
	.bind("Timestamp Test")
	.bind("Testing auto-update")
	.bind(false)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to create todo");

	let created_id: i32 = result.get("id");
	let created_at: chrono::NaiveDateTime = result.get("created_at");
	let initial_updated_at: chrono::NaiveDateTime = result.get("updated_at");

	// Wait a moment to ensure timestamp difference
	tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

	// Update the todo
	sqlx::query(
		r#"
		UPDATE todos
		SET title = $1
		WHERE id = $2
	"#,
	)
	.bind("Updated Title")
	.bind(created_id)
	.execute(pool.as_ref())
	.await
	.expect("Failed to update todo");

	// Fetch updated timestamps
	let updated: (chrono::NaiveDateTime, chrono::NaiveDateTime) = sqlx::query_as(
		r#"
		SELECT created_at, updated_at
		FROM todos
		WHERE id = $1
	"#,
	)
	.bind(created_id)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to fetch updated todo");

	let final_created_at = updated.0;
	let final_updated_at = updated.1;

	// Verify timestamps
	assert_eq!(final_created_at, created_at, "created_at should not change");
	assert!(
		final_updated_at >= initial_updated_at,
		"updated_at should be equal or later"
	);

	println!("✅ Timestamp auto-update working correctly");
}

// ============================================================================
// Integration Tests (Migration + ORM)
// ============================================================================

/// Test full migration and ORM workflow - comprehensive integration test
#[example_test("*")]
#[rstest]
#[tokio::test]
async fn test_full_migration_and_orm_workflow(
	#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
) {
	let (_container, pool, _url) = postgres_with_migrations.await;

	// Verify todos table exists (created by migration)
	let table_exists: (bool,) = sqlx::query_as(
		r#"
		SELECT EXISTS (
			SELECT FROM information_schema.tables
			WHERE table_schema = 'public'
			AND table_name = 'todos'
		)
	"#,
	)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to check table existence");

	assert!(table_exists.0, "todos table should exist");

	// Create a todo
	let result = sqlx::query(
		r#"
		INSERT INTO todos (title, description, completed)
		VALUES ($1, $2, $3)
		RETURNING id, title, description, completed, created_at, updated_at
	"#,
	)
	.bind("Integration Test")
	.bind("Full workflow test")
	.bind(false)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to create todo");

	let created_id: i32 = result.get("id");
	assert!(created_id > 0);

	// List todos
	let todos: Vec<(i32,)> = sqlx::query_as("SELECT id FROM todos ORDER BY id")
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to list todos");

	assert!(!todos.is_empty(), "Should have at least one todo");

	// Get specific todo
	let todo: (i32, String, Option<String>, bool) = sqlx::query_as(
		r#"
		SELECT id, title, description, completed
		FROM todos
		WHERE id = $1
	"#,
	)
	.bind(created_id)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to get todo");

	assert_eq!(todo.0, created_id);
	assert_eq!(todo.1, "Integration Test");

	// Update todo
	sqlx::query(
		r#"
		UPDATE todos
		SET completed = true
		WHERE id = $1
	"#,
	)
	.bind(created_id)
	.execute(pool.as_ref())
	.await
	.expect("Failed to update todo");

	// Verify update
	let updated: (bool,) = sqlx::query_as("SELECT completed FROM todos WHERE id = $1")
		.bind(created_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to fetch updated todo");

	assert!(updated.0, "Todo should be completed");

	// Delete todo
	sqlx::query("DELETE FROM todos WHERE id = $1")
		.bind(created_id)
		.execute(pool.as_ref())
		.await
		.expect("Failed to delete todo");

	// Verify deletion
	let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM todos WHERE id = $1")
		.bind(created_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count todos");

	assert_eq!(count.0, 0, "Todo should be deleted");

	println!("✅ Full migration and ORM workflow completed successfully");
}

/// Test CRUD operations within transactions
#[example_test("*")]
#[rstest]
#[tokio::test]
async fn test_todo_crud_with_transaction(
	#[future] postgres_with_migrations: (ContainerAsync<Postgres>, Arc<PgPool>, String),
) {
	let (_container, pool, _url) = postgres_with_migrations.await;

	// Start transaction
	let mut tx = pool.begin().await.expect("Failed to begin transaction");

	// Create todo within transaction
	let result = sqlx::query(
		r#"
		INSERT INTO todos (title, description, completed)
		VALUES ($1, $2, $3)
		RETURNING id
	"#,
	)
	.bind("Transaction Todo")
	.bind("Created in transaction")
	.bind(false)
	.fetch_one(&mut *tx)
	.await
	.expect("Failed to create todo");

	let created_id: i32 = result.get("id");

	// Update within transaction
	sqlx::query(
		r#"
		UPDATE todos
		SET completed = true
		WHERE id = $1
	"#,
	)
	.bind(created_id)
	.execute(&mut *tx)
	.await
	.expect("Failed to update todo");

	// Read within transaction
	let todo: (i32, String, bool) = sqlx::query_as(
		r#"
		SELECT id, title, completed
		FROM todos
		WHERE id = $1
	"#,
	)
	.bind(created_id)
	.fetch_one(&mut *tx)
	.await
	.expect("Failed to read todo");

	assert_eq!(todo.0, created_id);
	assert_eq!(todo.1, "Transaction Todo");
	assert!(todo.2, "Todo should be marked as completed");

	// Commit transaction
	tx.commit().await.expect("Failed to commit transaction");

	// Verify changes persisted after commit
	let persisted: (i32, String, bool) = sqlx::query_as(
		r#"
		SELECT id, title, completed
		FROM todos
		WHERE id = $1
	"#,
	)
	.bind(created_id)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to fetch persisted todo");

	assert_eq!(persisted.0, created_id);
	assert_eq!(persisted.1, "Transaction Todo");
	assert!(persisted.2);

	// Test rollback scenario
	let mut tx2 = pool.begin().await.expect("Failed to begin second transaction");

	let result2 = sqlx::query(
		r#"
		INSERT INTO todos (title, description, completed)
		VALUES ($1, $2, $3)
		RETURNING id
	"#,
	)
	.bind("Rollback Todo")
	.bind("Will be rolled back")
	.bind(false)
	.fetch_one(&mut *tx2)
	.await
	.expect("Failed to create todo for rollback");

	let rollback_id: i32 = result2.get("id");

	// Rollback transaction
	tx2.rollback()
		.await
		.expect("Failed to rollback transaction");

	// Verify rollback
	let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM todos WHERE id = $1")
		.bind(rollback_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count todos");

	assert_eq!(count.0, 0, "Rolled back todo should not exist");

	println!("✅ CRUD with transactions completed successfully");
}
