/// Schema editor factory module
///
/// This module provides a factory for creating database-specific schema editors,
/// enabling uniform database operations across different database backends.
///
/// # Architecture
///
/// ```text
/// reinhardt-orm/reinhardt-migrations
///           ↓
///    SchemaEditorFactory
///           ↓
///    BaseDatabaseSchemaEditor (trait)
///           ↓
///    ┌──────┴──────┬──────────┐
///    ↓             ↓          ↓
/// PostgreSQL    MySQL     SQLite
/// SchemaEditor  SchemaEditor SchemaEditor
/// ```
///
/// # Example
///
/// ```rust
/// use reinhardt_db::backends::schema::factory::{SchemaEditorFactory, DatabaseType};
///
/// let factory = SchemaEditorFactory::new();
/// let editor = factory.create_for_database(DatabaseType::PostgreSQL);
/// // Use the editor for DDL operations via BaseDatabaseSchemaEditor trait methods
/// ```
use crate::schema::{BaseDatabaseSchemaEditor, SchemaEditorError, SchemaEditorResult};

#[cfg(feature = "postgres")]
use crate::drivers::postgresql::schema::PostgreSQLSchemaEditor;

#[cfg(feature = "mysql")]
use crate::drivers::mysql::schema::MySQLSchemaEditor;

#[cfg(feature = "sqlite")]
use crate::drivers::sqlite::schema::SQLiteSchemaEditor;

use std::sync::Arc;

/// Database type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DatabaseType {
	/// PostgreSQL database
	PostgreSQL,
	/// MySQL database
	MySQL,
	/// SQLite database
	SQLite,
}

impl DatabaseType {
	/// Parse database type from connection string
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::backends::schema::factory::DatabaseType;
	///
	/// let db_type = DatabaseType::from_connection_string("postgres://localhost/mydb");
	/// assert_eq!(db_type, Some(DatabaseType::PostgreSQL));
	///
	/// let db_type = DatabaseType::from_connection_string("mysql://localhost/mydb");
	/// assert_eq!(db_type, Some(DatabaseType::MySQL));
	///
	/// let db_type = DatabaseType::from_connection_string("sqlite:///data.db");
	/// assert_eq!(db_type, Some(DatabaseType::SQLite));
	/// ```
	pub fn from_connection_string(conn_str: &str) -> Option<Self> {
		if conn_str.starts_with("postgres://") || conn_str.starts_with("postgresql://") {
			Some(DatabaseType::PostgreSQL)
		} else if conn_str.starts_with("mysql://") {
			Some(DatabaseType::MySQL)
		} else if conn_str.starts_with("sqlite://") {
			Some(DatabaseType::SQLite)
		} else {
			None
		}
	}

	/// Get the database type name as a string
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::backends::schema::factory::DatabaseType;
	///
	/// assert_eq!(DatabaseType::PostgreSQL.as_str(), "postgresql");
	/// assert_eq!(DatabaseType::MySQL.as_str(), "mysql");
	/// assert_eq!(DatabaseType::SQLite.as_str(), "sqlite");
	/// ```
	pub fn as_str(&self) -> &'static str {
		match self {
			DatabaseType::PostgreSQL => "postgresql",
			DatabaseType::MySQL => "mysql",
			DatabaseType::SQLite => "sqlite",
		}
	}
}

/// Schema editor factory for creating database-specific schema editors
///
/// This factory provides a unified interface for creating schema editors
/// that work with different database backends.
///
/// # Example
///
/// ```rust
/// use reinhardt_db::backends::schema::factory::{SchemaEditorFactory, DatabaseType};
///
/// let factory = SchemaEditorFactory::new();
/// // Create a PostgreSQL schema editor
/// let pg_editor = factory.create_for_database(DatabaseType::PostgreSQL);
/// // Use BaseDatabaseSchemaEditor trait methods for DDL operations
/// ```
pub struct SchemaEditorFactory {
	_config: (),
}

impl SchemaEditorFactory {
	/// Create a new schema editor factory
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::backends::schema::factory::SchemaEditorFactory;
	///
	/// let factory = SchemaEditorFactory::new();
	/// ```
	pub fn new() -> Self {
		Self { _config: () }
	}

	/// Create a schema editor for the specified database type
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::backends::schema::factory::{SchemaEditorFactory, DatabaseType};
	///
	/// let factory = SchemaEditorFactory::new();
	/// let editor = factory.create_for_database(DatabaseType::PostgreSQL);
	/// ```
	pub fn create_for_database(&self, db_type: DatabaseType) -> Box<dyn BaseDatabaseSchemaEditor> {
		match db_type {
			#[cfg(feature = "postgres")]
			DatabaseType::PostgreSQL => Box::new(PostgreSQLSchemaEditor::new()),

			#[cfg(not(feature = "postgres"))]
			DatabaseType::PostgreSQL => {
				panic!("PostgreSQL support not enabled. Enable 'postgres' feature.")
			}

			#[cfg(feature = "mysql")]
			DatabaseType::MySQL => Box::new(MySQLSchemaEditor::new()),

			#[cfg(not(feature = "mysql"))]
			DatabaseType::MySQL => {
				panic!("MySQL support not enabled. Enable 'mysql' feature.")
			}

			#[cfg(feature = "sqlite")]
			DatabaseType::SQLite => Box::new(SQLiteSchemaEditor::new()),

			#[cfg(not(feature = "sqlite"))]
			DatabaseType::SQLite => {
				panic!("SQLite support not enabled. Enable 'sqlite' feature.")
			}
		}
	}

	/// Create a schema editor from a connection string
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::backends::schema::factory::SchemaEditorFactory;
	///
	/// let factory = SchemaEditorFactory::new();
	/// let result = factory.create_from_connection_string("postgres://localhost/mydb");
	/// assert!(result.is_ok());
	/// ```
	pub fn create_from_connection_string(
		&self,
		conn_str: &str,
	) -> SchemaEditorResult<Box<dyn BaseDatabaseSchemaEditor>> {
		let db_type = DatabaseType::from_connection_string(conn_str).ok_or_else(|| {
			SchemaEditorError::InvalidOperation(format!(
				"Could not determine database type from connection string: {}",
				conn_str
			))
		})?;

		Ok(self.create_for_database(db_type))
	}

	/// Create an Arc-wrapped schema editor for shared access
	///
	/// This is useful when you need to share the schema editor across multiple threads.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_db::backends::schema::factory::{SchemaEditorFactory, DatabaseType};
	/// use std::sync::Arc;
	///
	/// let factory = SchemaEditorFactory::new();
	/// let editor = factory.create_shared(DatabaseType::PostgreSQL);
	///
	// Can be cloned and shared across threads
	/// let editor_clone = Arc::clone(&editor);
	/// ```
	pub fn create_shared(
		&self,
		db_type: DatabaseType,
	) -> Arc<dyn BaseDatabaseSchemaEditor + Send + Sync> {
		match db_type {
			#[cfg(feature = "postgres")]
			DatabaseType::PostgreSQL => Arc::new(PostgreSQLSchemaEditor::new()),

			#[cfg(not(feature = "postgres"))]
			DatabaseType::PostgreSQL => {
				panic!("PostgreSQL support not enabled. Enable 'postgres' feature.")
			}

			#[cfg(feature = "mysql")]
			DatabaseType::MySQL => Arc::new(MySQLSchemaEditor::new()),

			#[cfg(not(feature = "mysql"))]
			DatabaseType::MySQL => {
				panic!("MySQL support not enabled. Enable 'mysql' feature.")
			}

			#[cfg(feature = "sqlite")]
			DatabaseType::SQLite => Arc::new(SQLiteSchemaEditor::new()),

			#[cfg(not(feature = "sqlite"))]
			DatabaseType::SQLite => {
				panic!("SQLite support not enabled. Enable 'sqlite' feature.")
			}
		}
	}
}

impl Default for SchemaEditorFactory {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_database_type_from_connection_string() {
		assert_eq!(
			DatabaseType::from_connection_string("postgres://localhost/mydb"),
			Some(DatabaseType::PostgreSQL)
		);
		assert_eq!(
			DatabaseType::from_connection_string("postgresql://localhost/mydb"),
			Some(DatabaseType::PostgreSQL)
		);
		assert_eq!(
			DatabaseType::from_connection_string("mysql://localhost/mydb"),
			Some(DatabaseType::MySQL)
		);
		assert_eq!(
			DatabaseType::from_connection_string("sqlite:///data.db"),
			Some(DatabaseType::SQLite)
		);
		assert_eq!(
			DatabaseType::from_connection_string("unknown://localhost/mydb"),
			None
		);
	}

	#[test]
	fn test_database_type_as_str() {
		assert_eq!(DatabaseType::PostgreSQL.as_str(), "postgresql");
		assert_eq!(DatabaseType::MySQL.as_str(), "mysql");
		assert_eq!(DatabaseType::SQLite.as_str(), "sqlite");
	}

	#[test]
	fn test_factory_creation() {
		let factory = SchemaEditorFactory::new();
		let _factory2 = SchemaEditorFactory::default();
		// Factory should be creatable
		drop(factory);
	}

	#[cfg(feature = "postgres")]
	#[test]
	fn test_create_postgresql_editor() {
		let factory = SchemaEditorFactory::new();
		let _editor = factory.create_for_database(DatabaseType::PostgreSQL);
		// Editor created successfully
	}

	#[cfg(feature = "postgres")]
	#[test]
	fn test_create_from_connection_string() {
		let factory = SchemaEditorFactory::new();
		let result = factory.create_from_connection_string("postgres://localhost/mydb");
		assert!(result.is_ok());
	}

	#[test]
	fn test_create_from_invalid_connection_string() {
		let factory = SchemaEditorFactory::new();
		let result = factory.create_from_connection_string("invalid://localhost/mydb");
		assert!(result.is_err());
	}

	#[cfg(feature = "postgres")]
	#[test]
	fn test_create_shared_editor() {
		let factory = SchemaEditorFactory::new();
		let editor = factory.create_shared(DatabaseType::PostgreSQL);

		let editor_clone = Arc::clone(&editor);
		assert_eq!(Arc::strong_count(&editor), 2);
	}
}
