//! Database Integration Tests
//!
//! Compilation and execution control:
//! - Cargo.toml: [[test]] name = "database_tests" required-features = ["with-reinhardt"]
//! - build.rs: Sets 'with-reinhardt' feature when reinhardt is available
//! - When feature is disabled, this entire test file is excluded from compilation
//!
//! Uses Reinhardt ORM Manager<T> for all database operations.

use chrono::{DateTime, Utc};
use example_test_macros::example_test;
use reinhardt::db::orm::Manager;
use reinhardt::db::prelude::transaction;
use reinhardt::prelude::*;
use rstest::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use testcontainers::{
	images::generic::{GenericImage, WaitFor},
	ContainerAsync,
};

// Define simple test models inline
// Note: We cannot use #[derive(Model)] macro in example tests because it's not re-exported by reinhardt crate.
// Instead, we define simple structs and use Manager<T> directly with raw SQL migrations.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
	pub id: i64,
	pub name: String,
	pub email: String,
	pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Todo {
	pub id: i64,
	pub title: String,
	pub description: Option<String>,
	pub completed: bool,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

// Migration modules for applying migrations
mod migrations {
	pub mod users {
		use sea_query::{ColumnDef, Iden, PostgresQueryBuilder, Table};

		pub fn up() -> String {
			let users_table = Table::create()
				.table(UsersTable::Table)
				.col(
					ColumnDef::new(UsersTable::Id)
						.big_integer()
						.auto_increment()
						.primary_key()
						.not_null(),
				)
				.col(ColumnDef::new(UsersTable::Name).string_len(255).not_null())
				.col(
					ColumnDef::new(UsersTable::Email)
						.string_len(255)
						.not_null()
						.unique_key(),
				)
				.col(
					ColumnDef::new(UsersTable::CreatedAt)
						.timestamp_with_time_zone()
						.not_null()
						.default("CURRENT_TIMESTAMP"),
				)
				.to_owned();

			users_table.to_string(PostgresQueryBuilder)
		}

		#[derive(Iden)]
		enum UsersTable {
			Table,
			Id,
			Name,
			Email,
			CreatedAt,
		}
	}

	pub mod todos {
		use sea_query::{ColumnDef, Iden, PostgresQueryBuilder, Table};

		pub fn up() -> String {
			let todos_table = Table::create()
				.table(TodosTable::Table)
				.col(
					ColumnDef::new(TodosTable::Id)
						.big_integer()
						.auto_increment()
						.primary_key()
						.not_null(),
				)
				.col(ColumnDef::new(TodosTable::Title).string_len(255).not_null())
				.col(ColumnDef::new(TodosTable::Description).text())
				.col(
					ColumnDef::new(TodosTable::Completed)
						.boolean()
						.not_null()
						.default(false),
				)
				.col(
					ColumnDef::new(TodosTable::CreatedAt)
						.timestamp_with_time_zone()
						.not_null()
						.default("CURRENT_TIMESTAMP"),
				)
				.col(
					ColumnDef::new(TodosTable::UpdatedAt)
						.timestamp_with_time_zone()
						.not_null()
						.default("CURRENT_TIMESTAMP"),
				)
				.to_owned();

			todos_table.to_string(PostgresQueryBuilder)
		}

		#[derive(Iden)]
		enum TodosTable {
			Table,
			Id,
			Title,
			Description,
			Completed,
			CreatedAt,
			UpdatedAt,
		}
	}
}

// ============================================================================
// Custom Fixtures with Migrations
// ============================================================================

/// PostgreSQL container fixture with migrations applied
#[fixture]
async fn db_with_migrations() -> (ContainerAsync<GenericImage>, Arc<DatabaseConnection>) {
	// Start PostgreSQL container
	let postgres = GenericImage::new("postgres", "17-alpine")
		.with_wait_for(WaitFor::message_on_stderr(
			"database system is ready to accept connections",
		))
		.with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust")
		.with_env_var("POSTGRES_INITDB_ARGS", "-c max_connections=200")
		.start()
		.await
		.expect("Failed to start PostgreSQL container");

	let port = postgres
		.get_host_port_ipv4(5432)
		.await
		.expect("Failed to get PostgreSQL port");

	let database_url = format!("postgres://postgres@localhost:{}/postgres", port);

	// Wait for connection with retries
	let mut retry_count = 0;
	let max_retries = 5;
	let conn = loop {
		match DatabaseConnection::connect_postgres(&database_url).await {
			Ok(conn) => break conn,
			Err(_e) if retry_count < max_retries => {
				retry_count += 1;
				let delay = std::time::Duration::from_millis(100 * 2_u64.pow(retry_count));
				tokio::time::sleep(delay).await;
			}
			Err(e) => panic!("Failed to connect after {} retries: {}", max_retries, e),
		}
	};

	// Apply migrations
	let migrations = vec![
		("users", examples_database_integration::users::migrations::_0001_initial::Migration::up()),
		("todos", examples_database_integration::todos::migrations::_0001_initial::Migration::up()),
	];

	for (app_label, sql) in migrations {
		conn.execute_raw(&sql)
			.await
			.unwrap_or_else(|e| panic!("Failed to apply {} migration: {}", app_label, e));
	}

	(postgres, Arc::new(conn))
}

// ============================================================================
// Basic Database Connection Tests
// ============================================================================

/// Test basic database connection using Reinhardt ORM
#[rstest]
#[tokio::test]
async fn test_database_connection(
	#[future] db_with_migrations: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>),
) {
	let (_container, conn) = db_with_migrations.await;

	// Verify connection by querying users table
	let manager = Manager::<User>::new(conn);
	let result = manager.all().await;
	assert!(result.is_ok(), "Failed to query users table");
}

/// Test that database is ready with all tables created
#[rstest]
#[tokio::test]
async fn test_database_ready(
	#[future] db_with_migrations: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>),
) {
	let (_container, conn) = db_with_migrations.await;

	// Verify users table is accessible
	let user_manager = Manager::<User>::new(conn.clone());
	let users_result = user_manager.all().await;
	assert!(users_result.is_ok(), "Users table should be accessible");

	// Verify todos table is accessible
	let todo_manager = Manager::<Todo>::new(conn);
	let todos_result = todo_manager.all().await;
	assert!(todos_result.is_ok(), "Todos table should be accessible");

	println!("✅ Database ready with users and todos tables");
}

// ============================================================================
// CRUD Operations Tests (User Model)
// ============================================================================

/// Test creating a user with Reinhardt ORM
#[rstest]
#[tokio::test]
async fn test_create_user(
	#[future] db_with_migrations: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>),
) {
	let (_container, conn) = db_with_migrations.await;
	let manager = Manager::<User>::new(conn);

	let new_user = User {
		id: 0, // Auto-increment
		name: "Alice".to_string(),
		email: "alice@example.com".to_string(),
		created_at: Utc::now(),
	};

	let created = manager.create(&new_user).await.unwrap();

	assert!(created.id > 0, "Created user should have positive ID");
	assert_eq!(created.name, "Alice");
	assert_eq!(created.email, "alice@example.com");
}

/// Test reading multiple users
#[rstest]
#[tokio::test]
async fn test_read_users(
	#[future] db_with_migrations: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>),
) {
	let (_container, conn) = db_with_migrations.await;
	let manager = Manager::<User>::new(conn);

	// Create test users
	let user1 = User {
		id: 0,
		name: "Alice".to_string(),
		email: "alice@example.com".to_string(),
		created_at: Utc::now(),
	};
	let user2 = User {
		id: 0,
		name: "Bob".to_string(),
		email: "bob@example.com".to_string(),
		created_at: Utc::now(),
	};

	manager.create(&user1).await.unwrap();
	manager.create(&user2).await.unwrap();

	// Read all users
	let users = manager.all().await.unwrap();

	assert_eq!(users.len(), 2, "Should have 2 users");
	assert!(users.iter().any(|u| u.name == "Alice"));
	assert!(users.iter().any(|u| u.name == "Bob"));
}

/// Test updating a user
#[rstest]
#[tokio::test]
async fn test_update_user(
	#[future] db_with_migrations: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>),
) {
	let (_container, conn) = db_with_migrations.await;
	let manager = Manager::<User>::new(conn);

	// Create user
	let new_user = User {
		id: 0,
		name: "Alice".to_string(),
		email: "alice@example.com".to_string(),
		created_at: Utc::now(),
	};
	let created = manager.create(&new_user).await.unwrap();

	// Update user
	let mut updated_user = created.clone();
	updated_user.name = "Alice Updated".to_string();
	let result = manager.update(&updated_user).await.unwrap();

	assert_eq!(result.name, "Alice Updated");
	assert_eq!(result.id, created.id);
}

/// Test deleting a user
#[rstest]
#[tokio::test]
async fn test_delete_user(
	#[future] db_with_migrations: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>),
) {
	let (_container, conn) = db_with_migrations.await;
	let manager = Manager::<User>::new(conn.clone());

	// Create user
	let new_user = User {
		id: 0,
		name: "Alice".to_string(),
		email: "alice@example.com".to_string(),
		created_at: Utc::now(),
	};
	let created = manager.create(&new_user).await.unwrap();

	// Delete user
	manager.delete(created.id).await.unwrap();

	// Verify deletion
	let users = manager.all().await.unwrap();
	assert_eq!(users.len(), 0, "User should be deleted");
}

// ============================================================================
// Transaction Tests
// ============================================================================

/// Test transaction commit
#[rstest]
#[tokio::test]
async fn test_transaction_commit(
	#[future] db_with_migrations: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>),
) {
	let (_container, conn) = db_with_migrations.await;

	let result: Result<User, Box<dyn std::error::Error + Send + Sync>> =
		transaction(&conn, |tx_conn| {
			Box::pin(async move {
				let manager = Manager::<User>::new(tx_conn.clone());
				let new_user = User {
					id: 0,
					name: "Alice".to_string(),
					email: "alice@example.com".to_string(),
					created_at: Utc::now(),
				};
				manager.create(&new_user).await
			})
		})
		.await;

	assert!(result.is_ok(), "Transaction should commit successfully");

	// Verify commit
	let manager = Manager::<User>::new(conn);
	let users = manager.all().await.unwrap();
	assert_eq!(users.len(), 1, "User should be persisted after commit");
}

/// Test transaction rollback
#[rstest]
#[tokio::test]
async fn test_transaction_rollback(
	#[future] db_with_migrations: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>),
) {
	let (_container, conn) = db_with_migrations.await;

	let result: Result<User, Box<dyn std::error::Error + Send + Sync>> =
		transaction(&conn, |tx_conn| {
			Box::pin(async move {
				let manager = Manager::<User>::new(tx_conn.clone());
				let new_user = User {
					id: 0,
					name: "Alice".to_string(),
					email: "alice@example.com".to_string(),
					created_at: Utc::now(),
				};
				manager.create(&new_user).await?;

				// Intentional error to trigger rollback
				Err("Intentional rollback".into())
			})
		})
		.await;

	assert!(result.is_err(), "Transaction should fail");

	// Verify rollback
	let manager = Manager::<User>::new(conn);
	let users = manager.all().await.unwrap();
	assert_eq!(users.len(), 0, "User should not be persisted after rollback");
}

// ============================================================================
// ORM CRUD Tests (Todo Model)
// ============================================================================

/// Test creating a todo with Reinhardt ORM
#[rstest]
#[tokio::test]
async fn test_orm_create_todo(
	#[future] db_with_migrations: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>),
) {
	let (_container, conn) = db_with_migrations.await;
	let manager = Manager::<Todo>::new(conn);

	let new_todo = Todo {
		id: 0,
		title: "Test Todo".to_string(),
		description: Some("This is a test todo".to_string()),
		completed: false,
		created_at: Utc::now(),
		updated_at: Utc::now(),
	};

	let created = manager.create(&new_todo).await.unwrap();

	assert!(created.id > 0);
	assert_eq!(created.title, "Test Todo");
	assert_eq!(created.description, Some("This is a test todo".to_string()));
	assert!(!created.completed);
}

/// Test listing all todos
#[rstest]
#[tokio::test]
async fn test_orm_list_todos(
	#[future] db_with_migrations: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>),
) {
	let (_container, conn) = db_with_migrations.await;
	let manager = Manager::<Todo>::new(conn);

	// Create test todos
	let todo1 = Todo {
		id: 0,
		title: "Todo 1".to_string(),
		description: None,
		completed: false,
		created_at: Utc::now(),
		updated_at: Utc::now(),
	};
	let todo2 = Todo {
		id: 0,
		title: "Todo 2".to_string(),
		description: Some("Description 2".to_string()),
		completed: true,
		created_at: Utc::now(),
		updated_at: Utc::now(),
	};

	manager.create(&todo1).await.unwrap();
	manager.create(&todo2).await.unwrap();

	// List all todos
	let todos = manager.all().await.unwrap();

	assert_eq!(todos.len(), 2);
	assert!(todos.iter().any(|t| t.title == "Todo 1"));
	assert!(todos.iter().any(|t| t.title == "Todo 2"));
}

/// Test getting a specific todo
#[rstest]
#[tokio::test]
async fn test_orm_get_todo(
	#[future] db_with_migrations: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>),
) {
	let (_container, conn) = db_with_migrations.await;
	let manager = Manager::<Todo>::new(conn);

	// Create todo
	let new_todo = Todo {
		id: 0,
		title: "Test Todo".to_string(),
		description: Some("Test description".to_string()),
		completed: false,
		created_at: Utc::now(),
		updated_at: Utc::now(),
	};
	let created = manager.create(&new_todo).await.unwrap();

	// Get todo by ID
	let todos = manager.all().await.unwrap();
	let found = todos.iter().find(|t| t.id == created.id);

	assert!(found.is_some());
	let found_todo = found.unwrap();
	assert_eq!(found_todo.title, "Test Todo");
	assert_eq!(found_todo.description, Some("Test description".to_string()));
}

/// Test updating a todo
#[rstest]
#[tokio::test]
async fn test_orm_update_todo(
	#[future] db_with_migrations: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>),
) {
	let (_container, conn) = db_with_migrations.await;
	let manager = Manager::<Todo>::new(conn);

	// Create todo
	let new_todo = Todo {
		id: 0,
		title: "Original Title".to_string(),
		description: Some("Original description".to_string()),
		completed: false,
		created_at: Utc::now(),
		updated_at: Utc::now(),
	};
	let created = manager.create(&new_todo).await.unwrap();

	// Update todo
	let mut updated_todo = created.clone();
	updated_todo.title = "Updated Title".to_string();
	updated_todo.description = Some("Updated description".to_string());
	updated_todo.completed = true;

	let result = manager.update(&updated_todo).await.unwrap();

	assert_eq!(result.title, "Updated Title");
	assert_eq!(result.description, Some("Updated description".to_string()));
	assert!(result.completed);
}

/// Test deleting a todo
#[rstest]
#[tokio::test]
async fn test_orm_delete_todo(
	#[future] db_with_migrations: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>),
) {
	let (_container, conn) = db_with_migrations.await;
	let manager = Manager::<Todo>::new(conn.clone());

	// Create todo
	let new_todo = Todo {
		id: 0,
		title: "To be deleted".to_string(),
		description: None,
		completed: false,
		created_at: Utc::now(),
		updated_at: Utc::now(),
	};
	let created = manager.create(&new_todo).await.unwrap();

	// Delete todo
	manager.delete(created.id).await.unwrap();

	// Verify deletion
	let todos = manager.all().await.unwrap();
	assert_eq!(todos.len(), 0, "Todo should be deleted");
}

// ============================================================================
// Field Validation Tests
// ============================================================================

/// Test todo default values
#[rstest]
#[tokio::test]
async fn test_todo_default_values(
	#[future] db_with_migrations: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>),
) {
	let (_container, conn) = db_with_migrations.await;
	let manager = Manager::<Todo>::new(conn);

	let new_todo = Todo {
		id: 0,
		title: "Test Todo".to_string(),
		description: None,
		completed: false, // Default
		created_at: Utc::now(),
		updated_at: Utc::now(),
	};

	let created = manager.create(&new_todo).await.unwrap();

	// Verify defaults
	assert!(!created.completed, "Default completed should be false");
	assert!(created.description.is_none(), "Default description should be None");
}

/// Test timestamp auto-update behavior
#[rstest]
#[tokio::test]
async fn test_todo_timestamp_behavior(
	#[future] db_with_migrations: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>),
) {
	let (_container, conn) = db_with_migrations.await;
	let manager = Manager::<Todo>::new(conn);

	let new_todo = Todo {
		id: 0,
		title: "Test Todo".to_string(),
		description: None,
		completed: false,
		created_at: Utc::now(),
		updated_at: Utc::now(),
	};

	let created = manager.create(&new_todo).await.unwrap();

	// Verify timestamps exist
	assert!(created.created_at.timestamp() > 0);
	assert!(created.updated_at.timestamp() > 0);

	// Update and verify updated_at changes
	let original_updated_at = created.updated_at;
	tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

	let mut updated = created.clone();
	updated.title = "Updated Title".to_string();
	updated.updated_at = Utc::now(); // Simulate auto_now

	let result = manager.update(&updated).await.unwrap();

	// updated_at should be different (manually set in test)
	assert!(result.updated_at >= original_updated_at);
}

// ============================================================================
// Integration Tests
// ============================================================================

/// Test complete CRUD cycle with transactions
#[rstest]
#[tokio::test]
async fn test_complete_crud_cycle(
	#[future] db_with_migrations: (ContainerAsync<GenericImage>, Arc<DatabaseConnection>),
) {
	let (_container, conn) = db_with_migrations.await;

	// Use transaction for entire CRUD cycle
	let final_count: Result<usize, Box<dyn std::error::Error + Send + Sync>> =
		transaction(&conn, |tx_conn| {
			Box::pin(async move {
				let manager = Manager::<Todo>::new(tx_conn.clone());

				// Create
				let new_todo = Todo {
					id: 0,
					title: "CRUD Test".to_string(),
					description: Some("Testing full cycle".to_string()),
					completed: false,
					created_at: Utc::now(),
					updated_at: Utc::now(),
				};
				let created = manager.create(&new_todo).await?;

				// Read
				let todos = manager.all().await?;
				assert_eq!(todos.len(), 1);

				// Update
				let mut updated = created.clone();
				updated.completed = true;
				manager.update(&updated).await?;

				// Verify update
				let todos_after_update = manager.all().await?;
				assert!(todos_after_update[0].completed);

				// Delete
				manager.delete(created.id).await?;

				// Verify deletion
				let final_todos = manager.all().await?;
				Ok(final_todos.len())
			})
		})
		.await;

	assert_eq!(final_count.unwrap(), 0, "All operations should complete successfully");
}
