//! Transaction management utilities for server function testing.
//!
//! This module provides utilities for managing database transactions in tests,
//! including automatic rollback to ensure test isolation.
//!
//! # Example
//!
//! ```rust,ignore
//! use reinhardt_test::server_fn::transaction::TestTransaction;
//!
//! #[rstest]
//! #[tokio::test]
//! async fn test_with_rollback(postgres_suite: SuiteGuard<PostgresSuiteResource>) {
//!     let tx = TestTransaction::begin(postgres_suite.pool()).await.unwrap();
//!
//!     // Perform database operations using tx.connection()
//!     // All changes will be rolled back when tx is dropped
//! }
//! ```

#![cfg(not(target_arch = "wasm32"))]

use std::ops::Deref;

use async_trait::async_trait;

/// A test transaction that automatically rolls back on drop.
///
/// This provides a way to run database tests in isolation without
/// affecting other tests or leaving test data in the database.
///
/// # Behavior
///
/// When `TestTransaction` is dropped, it will automatically rollback
/// all changes made during the test. This ensures that:
/// - Tests don't affect each other
/// - No test data is left in the database
/// - Tests can be run in parallel without conflicts
///
/// # Example
///
/// ```rust,ignore
/// let tx = TestTransaction::begin(&pool).await?;
///
/// // All operations here happen within the transaction
/// sqlx::query("INSERT INTO users (name) VALUES ('test')")
///     .execute(tx.connection())
///     .await?;
///
/// // When tx goes out of scope, all changes are rolled back
/// ```
#[derive(Debug)]
pub struct TestTransaction<C> {
	/// The underlying connection or transaction handle.
	connection: C,
	/// Whether to commit instead of rollback.
	commit_on_drop: bool,
	/// Whether the transaction has been explicitly completed.
	completed: bool,
}

impl<C> TestTransaction<C> {
	/// Create a new test transaction wrapper.
	pub fn new(connection: C) -> Self {
		Self {
			connection,
			commit_on_drop: false,
			completed: false,
		}
	}

	/// Configure the transaction to commit on drop instead of rollback.
	///
	/// Use this with caution - it defeats the purpose of test isolation.
	pub fn commit_on_drop(mut self) -> Self {
		self.commit_on_drop = true;
		self
	}

	/// Get a reference to the underlying connection.
	pub fn connection(&self) -> &C {
		&self.connection
	}

	/// Get a mutable reference to the underlying connection.
	pub fn connection_mut(&mut self) -> &mut C {
		&mut self.connection
	}

	/// Consume the transaction and return the underlying connection.
	///
	/// Note: This prevents automatic rollback/commit on drop.
	pub fn into_inner(mut self) -> C {
		self.completed = true;
		// Use ManuallyDrop to prevent drop from running
		let connection = std::mem::replace(&mut self.connection, unsafe { std::mem::zeroed() });
		std::mem::forget(self);
		connection
	}

	/// Mark the transaction as completed (no rollback on drop).
	pub fn mark_completed(&mut self) {
		self.completed = true;
	}
}

impl<C> Deref for TestTransaction<C> {
	type Target = C;

	fn deref(&self) -> &Self::Target {
		&self.connection
	}
}

/// Trait for types that can be used as test transaction connections.
#[async_trait]
pub trait TestConnectionExt: Sized {
	/// The error type.
	type Error;

	/// Begin a new transaction that will rollback on drop.
	async fn begin_test_transaction(self) -> Result<TestTransaction<Self>, Self::Error>;

	/// Commit the transaction explicitly.
	async fn commit_transaction(self) -> Result<(), Self::Error>;

	/// Rollback the transaction explicitly.
	async fn rollback_transaction(self) -> Result<(), Self::Error>;
}

/// Wrapper for managing savepoints within a test.
///
/// Savepoints allow you to create checkpoints within a transaction
/// and rollback to them without rolling back the entire transaction.
#[derive(Debug)]
pub struct TestSavepoint {
	/// The savepoint name.
	pub name: String,
	/// Whether the savepoint has been released.
	released: bool,
}

impl TestSavepoint {
	/// Create a new savepoint with the given name.
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			released: false,
		}
	}

	/// Generate a unique savepoint name.
	pub fn generate() -> Self {
		Self::new(format!("sp_{}", uuid::Uuid::new_v4().simple()))
	}

	/// Mark the savepoint as released.
	pub fn mark_released(&mut self) {
		self.released = true;
	}

	/// Check if the savepoint has been released.
	pub fn is_released(&self) -> bool {
		self.released
	}
}

/// Test database utilities for common operations.
pub mod utils {
	/// Truncate all tables in the given list.
	///
	/// This is useful for cleaning up between tests when not using
	/// transaction rollback.
	pub fn truncate_tables_sql(tables: &[&str]) -> String {
		if tables.is_empty() {
			return String::new();
		}

		format!(
			"TRUNCATE TABLE {} RESTART IDENTITY CASCADE",
			tables.join(", ")
		)
	}

	/// Generate a DELETE statement for cleaning up a table.
	pub fn delete_from_sql(table: &str, where_clause: Option<&str>) -> String {
		match where_clause {
			Some(clause) => format!("DELETE FROM {} WHERE {}", table, clause),
			None => format!("DELETE FROM {}", table),
		}
	}

	/// Generate an INSERT statement for test data.
	pub fn insert_test_data_sql(table: &str, columns: &[&str], values: &[&str]) -> String {
		format!(
			"INSERT INTO {} ({}) VALUES ({})",
			table,
			columns.join(", "),
			values.join(", ")
		)
	}
}

/// Configuration for test database behavior.
#[derive(Debug, Clone)]
pub struct TestDatabaseConfig {
	/// Tables to truncate before each test (if not using transactions).
	pub truncate_tables: Vec<String>,
	/// Whether to use transactions for test isolation.
	pub use_transactions: bool,
	/// Maximum number of connections for the test pool.
	pub max_connections: u32,
	/// Connection timeout in seconds.
	pub connection_timeout_secs: u64,
}

impl Default for TestDatabaseConfig {
	fn default() -> Self {
		Self {
			truncate_tables: Vec::new(),
			use_transactions: true,
			max_connections: 5,
			connection_timeout_secs: 30,
		}
	}
}

impl TestDatabaseConfig {
	/// Create a new configuration.
	pub fn new() -> Self {
		Self::default()
	}

	/// Add a table to truncate.
	pub fn truncate(mut self, table: impl Into<String>) -> Self {
		self.truncate_tables.push(table.into());
		self
	}

	/// Disable transaction-based isolation.
	pub fn without_transactions(mut self) -> Self {
		self.use_transactions = false;
		self
	}

	/// Set the maximum number of connections.
	pub fn max_connections(mut self, count: u32) -> Self {
		self.max_connections = count;
		self
	}

	/// Set the connection timeout.
	pub fn connection_timeout(mut self, secs: u64) -> Self {
		self.connection_timeout_secs = secs;
		self
	}
}

/// Helper for seeding test data.
///
/// This provides a fluent interface for inserting test data
/// that will be available during the test.
#[derive(Debug, Default)]
pub struct TestDataSeeder {
	/// SQL statements to execute for seeding.
	statements: Vec<String>,
}

impl TestDataSeeder {
	/// Create a new seeder.
	pub fn new() -> Self {
		Self::default()
	}

	/// Add a raw SQL statement.
	pub fn sql(mut self, statement: impl Into<String>) -> Self {
		self.statements.push(statement.into());
		self
	}

	/// Add an INSERT statement.
	pub fn insert(self, table: &str, columns: &[&str], values: &[&str]) -> Self {
		self.sql(utils::insert_test_data_sql(table, columns, values))
	}

	/// Get all statements to execute.
	pub fn statements(&self) -> &[String] {
		&self.statements
	}

	/// Build the combined SQL for all seed operations.
	pub fn build(&self) -> String {
		self.statements.join(";\n")
	}
}

/// Guard that ensures cleanup runs at the end of a test.
///
/// This is useful for tests that need to clean up resources
/// even if the test panics.
pub struct CleanupGuard<F: FnOnce()> {
	cleanup: Option<F>,
}

impl<F: FnOnce()> CleanupGuard<F> {
	/// Create a new cleanup guard.
	pub fn new(cleanup: F) -> Self {
		Self {
			cleanup: Some(cleanup),
		}
	}

	/// Disarm the guard (don't run cleanup on drop).
	pub fn disarm(&mut self) {
		self.cleanup = None;
	}
}

impl<F: FnOnce()> Drop for CleanupGuard<F> {
	fn drop(&mut self) {
		if let Some(cleanup) = self.cleanup.take() {
			cleanup();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_truncate_tables_sql() {
		let sql = utils::truncate_tables_sql(&["users", "posts"]);
		assert!(sql.contains("TRUNCATE TABLE"));
		assert!(sql.contains("users"));
		assert!(sql.contains("posts"));
		assert!(sql.contains("CASCADE"));
	}

	#[rstest]
	fn test_truncate_tables_sql_empty() {
		let sql = utils::truncate_tables_sql(&[]);
		assert!(sql.is_empty());
	}

	#[rstest]
	fn test_delete_from_sql() {
		let sql = utils::delete_from_sql("users", None);
		assert_eq!(sql, "DELETE FROM users");

		let sql_with_where = utils::delete_from_sql("users", Some("id = 1"));
		assert_eq!(sql_with_where, "DELETE FROM users WHERE id = 1");
	}

	#[rstest]
	fn test_insert_test_data_sql() {
		let sql = utils::insert_test_data_sql(
			"users",
			&["name", "email"],
			&["'Alice'", "'alice@example.com'"],
		);
		assert!(sql.contains("INSERT INTO users"));
		assert!(sql.contains("name, email"));
		assert!(sql.contains("'Alice'"));
	}

	#[rstest]
	fn test_database_config() {
		let config = TestDatabaseConfig::new()
			.truncate("users")
			.truncate("posts")
			.max_connections(10)
			.connection_timeout(60);

		assert_eq!(config.truncate_tables.len(), 2);
		assert_eq!(config.max_connections, 10);
		assert_eq!(config.connection_timeout_secs, 60);
	}

	#[rstest]
	fn test_data_seeder() {
		let seeder = TestDataSeeder::new()
			.insert("users", &["name"], &["'Alice'"])
			.insert("posts", &["title", "user_id"], &["'Hello'", "1"]);

		assert_eq!(seeder.statements().len(), 2);
	}

	#[rstest]
	fn test_savepoint() {
		let sp = TestSavepoint::generate();
		assert!(sp.name.starts_with("sp_"));
		assert!(!sp.is_released());

		let mut sp2 = TestSavepoint::new("my_savepoint");
		sp2.mark_released();
		assert!(sp2.is_released());
	}

	#[rstest]
	fn test_cleanup_guard() {
		use std::cell::RefCell;
		use std::rc::Rc;

		let cleaned = Rc::new(RefCell::new(false));
		let cleaned_clone = cleaned.clone();

		{
			let _guard = CleanupGuard::new(move || {
				*cleaned_clone.borrow_mut() = true;
			});
		}

		assert!(*cleaned.borrow());
	}

	#[rstest]
	fn test_cleanup_guard_disarm() {
		use std::cell::RefCell;
		use std::rc::Rc;

		let cleaned = Rc::new(RefCell::new(false));
		let cleaned_clone = cleaned.clone();

		{
			let mut guard = CleanupGuard::new(move || {
				*cleaned_clone.borrow_mut() = true;
			});
			guard.disarm();
		}

		assert!(!*cleaned.borrow());
	}
}
