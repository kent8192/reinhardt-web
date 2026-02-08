//! # DCL (Data Control Language) Test Fixtures
//!
//! This module provides rstest fixtures for DCL integration testing.
//!
//! ## Available Fixtures
//!
//! - `dcl_test_table` - Creates a test table for privilege testing
//! - `test_role` - Creates a test role
//! - `test_role_with_attrs` - Creates a role with specific attributes
//! - `test_user` - Creates a test user
//! - `test_user_with_password` - Creates a user with password
//! - `cleanup_dcl_objects` - Cleans up test DCL objects
//! - `test_database` - Creates a test database
//! - `test_schema` - Creates a test schema (PostgreSQL only)
//!
//! ## Usage
//!
//! ```rust,no_run
//! use reinhardt_test::fixtures::dcl::*;
//! use rstest::rstest;
//!
//! #[rstest]
//! #[tokio::test]
//! async fn test_grant_select(
//!     dcl_test_table: String,
//!     test_role: String,
//!     cleanup_dcl_objects: Vec<String>
//! ) {
//!     // Test table name: dcl_test_table
//!     // Test role name: test_role
//!     // Cleanup list: cleanup_dcl_objects
//! }
//! ```

use reinhardt_query::prelude::{
	Alias, ColumnDef, CreateTableStatement, ForeignKey, ForeignKeyAction, Query,
};
use std::sync::Mutex;

// Store for tracking created DCL objects for cleanup
static DCL_OBJECTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Track a DCL object for cleanup
fn track_object(object_name: String) {
	let mut objects = DCL_OBJECTS.lock().unwrap();
	objects.push(object_name);
}

/// Create a test table for DCL privilege testing
///
/// Returns the table name for use in tests.
///
/// # Table Schema
///
/// ```text
/// dcl_test_<timestamp> (
///     id BIGINT PRIMARY KEY,
///     name VARCHAR(100) NOT NULL,
///     value TEXT,
///     created_at TIMESTAMP
/// )
/// ```
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::dcl::dcl_test_table;
/// use rstest::rstest;
///
/// #[rstest]
/// fn test_table_operations(dcl_test_table: String) {
///     assert!(dcl_test_table.starts_with("dcl_test_"));
/// }
/// ```
pub fn dcl_test_table() -> String {
	let timestamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_secs();
	let table_name = format!("dcl_test_{}", timestamp);
	track_object(format!("TABLE:{}", table_name));
	table_name
}

/// Create a test role name
///
/// Returns a unique role name for testing.
///
/// # Naming Convention
///
/// Format: `test_role_<timestamp>`
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::dcl::test_role;
/// use rstest::rstest;
///
/// #[rstest]
/// fn test_role_creation(test_role: String) {
///     assert!(test_role.starts_with("test_role_"));
/// }
/// ```
pub fn test_role() -> String {
	let timestamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_secs();
	let role_name = format!("test_role_{}", timestamp);
	track_object(format!("ROLE:{}", role_name));
	role_name
}

/// Create a test role with specific attributes
///
/// Returns a tuple of (role_name, attributes) where attributes is a comma-separated
/// string of role attributes (e.g., "LOGIN,CREATEDB").
///
/// # Supported Attributes
///
/// - PostgreSQL: LOGIN, NOLOGIN, CREATEDB, NOCREATEDB, CREATEROLE, NOCREATEROLE,
///   SUPERUSER, NOSUPERUSER, INHERIT, NOINHERIT, REPLICATION, NOREPLICATION
/// - MySQL: (none - MySQL doesn't support role attributes)
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::dcl::test_role_with_attrs;
/// use rstest::rstest;
///
/// #[rstest]
/// fn test_role_with_attributes(test_role_with_attrs: (String, String)) {
///     let (role_name, attrs) = test_role_with_attrs;
///     assert!(role_name.starts_with("test_role_"));
///     assert!(attrs.contains("LOGIN"));
/// }
/// ```
pub fn test_role_with_attrs() -> (String, String) {
	let timestamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_secs();
	let role_name = format!("test_role_attrs_{}", timestamp);
	let attributes = "LOGIN,CREATEDB".to_string();
	track_object(format!("ROLE:{}", role_name));
	(role_name, attributes)
}

/// Create a test user name
///
/// Returns a unique user name for testing.
///
/// # Naming Convention
///
/// Format: `test_user_<timestamp>`
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::dcl::test_user;
/// use rstest::rstest;
///
/// #[rstest]
/// fn test_user_creation(test_user: String) {
///     assert!(test_user.starts_with("test_user_"));
/// }
/// ```
pub fn test_user() -> String {
	let timestamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_secs();
	let user_name = format!("test_user_{}", timestamp);
	track_object(format!("USER:{}", user_name));
	user_name
}

/// Create a test user with password
///
/// Returns a tuple of (username, password).
///
/// # Password
///
/// The password is auto-generated and unique for each test.
///
/// # Security Note
///
/// **WARNING**: These passwords are for testing only. Never use in production.
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::dcl::test_user_with_password;
/// use rstest::rstest;
///
/// #[rstest]
/// fn test_user_auth(test_user_with_password: (String, String)) {
///     let (username, password) = test_user_with_password;
///     assert!(username.starts_with("test_user_"));
///     assert!(!password.is_empty());
/// }
/// ```
pub fn test_user_with_password() -> (String, String) {
	let timestamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_secs();
	let user_name = format!("test_user_pass_{}", timestamp);
	let password = format!("test_password_{}", timestamp);
	track_object(format!("USER:{}", user_name));
	(user_name, password)
}

/// Cleanup DCL objects created during test
///
/// Returns a list of all DCL objects created in the current test for cleanup.
///
/// # Object Format
///
/// Each entry is formatted as `<TYPE>:<name>` where TYPE is one of:
/// - TABLE
/// - ROLE
/// - USER
/// - DATABASE
/// - SCHEMA
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::dcl::{test_role, cleanup_dcl_objects};
/// use rstest::rstest;
///
/// #[rstest]
/// fn test_with_cleanup(test_role: String, cleanup_dcl_objects: Vec<String>) {
///     // After test, cleanup_dcl_objects contains all objects to drop
///     for object in cleanup_dcl_objects {
///         // Drop object...
///     }
/// }
/// ```
pub fn cleanup_dcl_objects() -> Vec<String> {
	let mut objects = DCL_OBJECTS.lock().unwrap();
	let cleanup_list = objects.clone();
	objects.clear();
	cleanup_list
}

/// Create a test database name
///
/// Returns a unique database name for testing.
///
/// # Naming Convention
///
/// Format: `test_db_<timestamp>`
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::dcl::test_database;
/// use rstest::rstest;
///
/// #[rstest]
/// fn test_database_creation(test_database: String) {
///     assert!(test_database.starts_with("test_db_"));
/// }
/// ```
pub fn test_database() -> String {
	let timestamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_secs();
	let db_name = format!("test_db_{}", timestamp);
	track_object(format!("DATABASE:{}", db_name));
	db_name
}

/// Create a test schema name (PostgreSQL only)
///
/// Returns a unique schema name for testing.
///
/// # Naming Convention
///
/// Format: `test_schema_<timestamp>`
///
/// # Database Support
///
/// - PostgreSQL: Supported
/// - MySQL: Not supported (use CREATE DATABASE instead)
/// - SQLite: Not supported
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::dcl::test_schema;
/// use rstest::rstest;
///
/// #[rstest]
/// fn test_schema_creation(test_schema: String) {
///     assert!(test_schema.starts_with("test_schema_"));
/// }
/// ```
pub fn test_schema() -> String {
	let timestamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_secs();
	let schema_name = format!("test_schema_{}", timestamp);
	track_object(format!("SCHEMA:{}", schema_name));
	schema_name
}

/// Generate reinhardt-query `CreateTableStatement` for DCL test table
///
/// Returns a table creation statement that can be built into SQL for any backend.
///
/// # Schema
///
/// ```text
/// dcl_test_<timestamp> (
///     id BIGINT PRIMARY KEY,
///     name VARCHAR(100) NOT NULL,
///     value TEXT,
///     created_at TIMESTAMP
/// )
/// ```
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::dcl::dcl_test_table_stmt;
/// use reinhardt_query::prelude::{PostgresQueryBuilder, MySqlQueryBuilder, QueryStatementBuilder};
///
/// #[test]
/// fn test_table_sql_generation() {
///     let stmt = dcl_test_table_stmt();
///
///     let sql = stmt.to_string(PostgresQueryBuilder::new());
///     assert!(sql.contains("CREATE TABLE"));
///
///     let sql = stmt.to_string(MySqlQueryBuilder::new());
///     assert!(sql.contains("CREATE TABLE"));
/// }
/// ```
pub fn dcl_test_table_stmt() -> CreateTableStatement {
	let table_name = dcl_test_table();

	let mut stmt = Query::create_table();
	stmt.table(Alias::new(&table_name))
		.col(
			ColumnDef::new(Alias::new("id"))
				.big_integer()
				.not_null(true)
				.primary_key(true),
		)
		.col(
			ColumnDef::new(Alias::new("name"))
				.string_len(100)
				.not_null(true),
		)
		.col(ColumnDef::new(Alias::new("value")).text())
		.col(ColumnDef::new(Alias::new("created_at")).timestamp());
	stmt.take()
}

/// Generate reinhardt-query `CreateTableStatement` for DCL test table with foreign key
///
/// Returns a table creation statement with a foreign key constraint for testing
/// privilege management on related tables.
///
/// # Schema
///
/// ```text
/// dcl_test_parent_<timestamp> (
///     id BIGINT PRIMARY KEY,
///     name VARCHAR(100) NOT NULL
/// )
///
/// dcl_test_child_<timestamp> (
///     id BIGINT PRIMARY KEY,
///     parent_id BIGINT NOT NULL,
///     value TEXT,
///     FOREIGN KEY (parent_id) REFERENCES dcl_test_parent(id) ON DELETE CASCADE
/// )
/// ```
///
/// # Returns
///
/// A tuple of (parent_table_stmt, child_table_stmt, parent_name, child_name)
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_test::fixtures::dcl::dcl_test_table_with_fk;
/// use reinhardt_query::prelude::{PostgresQueryBuilder, QueryStatementBuilder};
///
/// #[test]
/// fn test_foreign_key_table() {
///     let (parent_stmt, child_stmt, parent_name, child_name) = dcl_test_table_with_fk();
///
///     let sql = child_stmt.to_string(PostgresQueryBuilder::new());
///     assert!(sql.contains("FOREIGN KEY"));
/// }
/// ```
pub fn dcl_test_table_with_fk() -> (CreateTableStatement, CreateTableStatement, String, String) {
	let parent_name = format!(
		"dcl_test_parent_{}",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_secs()
	);
	let child_name = format!(
		"dcl_test_child_{}",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_secs()
	);

	track_object(format!("TABLE:{}", parent_name));
	track_object(format!("TABLE:{}", child_name));

	let mut parent_stmt = Query::create_table();
	parent_stmt
		.table(Alias::new(&parent_name))
		.col(
			ColumnDef::new(Alias::new("id"))
				.big_integer()
				.not_null(true)
				.primary_key(true),
		)
		.col(
			ColumnDef::new(Alias::new("name"))
				.string_len(100)
				.not_null(true),
		);

	let mut fk = ForeignKey::create();
	fk.name(Alias::new(format!("fk_{}_parent", child_name)))
		.from_tbl(Alias::new(&child_name))
		.from_col(Alias::new("parent_id"))
		.to_tbl(Alias::new(&parent_name))
		.to_col(Alias::new("id"))
		.on_delete(ForeignKeyAction::Cascade)
		.on_update(ForeignKeyAction::Cascade);

	let mut child_stmt = Query::create_table();
	child_stmt
		.table(Alias::new(&child_name))
		.col(
			ColumnDef::new(Alias::new("id"))
				.big_integer()
				.not_null(true)
				.primary_key(true),
		)
		.col(
			ColumnDef::new(Alias::new("parent_id"))
				.big_integer()
				.not_null(true),
		)
		.col(ColumnDef::new(Alias::new("value")).text())
		.foreign_key_from_builder(&mut fk);

	(
		parent_stmt.take(),
		child_stmt.take(),
		parent_name,
		child_name,
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_query::prelude::{
		MySqlQueryBuilder, PostgresQueryBuilder, QueryStatementBuilder,
	};
	use serial_test::serial;

	#[test]
	#[serial]
	fn test_dcl_test_table_format() {
		let table = dcl_test_table();
		assert!(table.starts_with("dcl_test_"));
	}

	#[test]
	#[serial]
	fn test_test_role_format() {
		let role = test_role();
		assert!(role.starts_with("test_role_"));
	}

	#[test]
	#[serial]
	fn test_test_role_with_attrs_format() {
		let (role, attrs) = test_role_with_attrs();
		assert!(role.starts_with("test_role_attrs_"));
		assert_eq!(attrs, "LOGIN,CREATEDB");
	}

	#[test]
	#[serial]
	fn test_test_user_format() {
		let user = test_user();
		assert!(user.starts_with("test_user_"));
	}

	#[test]
	#[serial]
	fn test_test_user_with_password_format() {
		let (user, pass) = test_user_with_password();
		assert!(user.starts_with("test_user_pass_"));
		assert!(pass.starts_with("test_password_"));
	}

	#[test]
	#[serial]
	fn test_test_database_format() {
		let db = test_database();
		assert!(db.starts_with("test_db_"));
	}

	#[test]
	#[serial]
	fn test_test_schema_format() {
		let schema = test_schema();
		assert!(schema.starts_with("test_schema_"));
	}

	#[test]
	#[serial]
	fn test_cleanup_dcl_objects_returns_objects() {
		// Clear any existing objects
		{
			let mut objects = DCL_OBJECTS.lock().unwrap();
			objects.clear();
		}

		// Create some test objects
		let _role = test_role();
		let _user = test_user();
		let _table = dcl_test_table();

		// Get cleanup list
		let cleanup = cleanup_dcl_objects();

		assert_eq!(cleanup.len(), 3);
		assert!(cleanup.iter().any(|o| o.starts_with("ROLE:")));
		assert!(cleanup.iter().any(|o| o.starts_with("USER:")));
		assert!(cleanup.iter().any(|o| o.starts_with("TABLE:")));
	}

	#[test]
	#[serial]
	fn test_cleanup_clears_objects() {
		// Clear any existing objects
		{
			let mut objects = DCL_OBJECTS.lock().unwrap();
			objects.clear();
		}

		// Create test objects
		let _role = test_role();
		let _user = test_user();

		// Get cleanup list
		let cleanup1 = cleanup_dcl_objects();
		assert_eq!(cleanup1.len(), 2);

		// Get cleanup list again - should be empty
		let cleanup2 = cleanup_dcl_objects();
		assert_eq!(cleanup2.len(), 0);
	}

	#[test]
	#[serial]
	fn test_dcl_test_table_stmt_generates_valid_sql() {
		let stmt = dcl_test_table_stmt();
		assert!(
			stmt.to_string(PostgresQueryBuilder::new())
				.contains("CREATE TABLE")
		);
		assert!(
			stmt.to_string(MySqlQueryBuilder::new())
				.contains("CREATE TABLE")
		);
	}

	#[test]
	#[serial]
	fn test_dcl_test_table_with_fk_format() {
		let (parent_stmt, child_stmt, parent_name, child_name) = dcl_test_table_with_fk();

		assert!(parent_name.starts_with("dcl_test_parent_"));
		assert!(child_name.starts_with("dcl_test_child_"));

		let parent_sql = parent_stmt.to_string(PostgresQueryBuilder::new());
		assert!(parent_sql.contains("CREATE TABLE"));
		assert!(parent_sql.contains(&parent_name));

		let child_sql = child_stmt.to_string(PostgresQueryBuilder::new());
		assert!(child_sql.contains("CREATE TABLE"));
		assert!(child_sql.contains(&child_name));
		assert!(child_sql.contains("FOREIGN KEY"));
	}
}
