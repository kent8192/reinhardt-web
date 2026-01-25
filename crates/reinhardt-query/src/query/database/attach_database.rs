//! ATTACH DATABASE statement builder
//!
//! This module provides the `AttachDatabaseStatement` type for building SQL ATTACH DATABASE queries.
//! ATTACH DATABASE is a SQLite-specific feature for connecting additional database files.

use crate::types::{DynIden, IntoIden};

use super::super::traits::{QueryBuilderTrait, QueryStatementBuilder, QueryStatementWriter};

/// ATTACH DATABASE statement builder (SQLite-specific)
///
/// This struct provides a fluent API for constructing ATTACH DATABASE queries.
/// ATTACH DATABASE allows attaching additional database files to the current connection.
///
/// # Examples
///
/// ```rust
/// use reinhardt_query::prelude::*;
///
/// // ATTACH DATABASE 'path/to/db.sqlite' AS auxiliary
/// let query = Query::attach_database()
///     .file_path("path/to/db.sqlite")
///     .as_name("auxiliary");
/// ```
#[derive(Debug, Clone)]
pub struct AttachDatabaseStatement {
	pub(crate) file_path: Option<String>,
	pub(crate) database_name: Option<DynIden>,
}

impl AttachDatabaseStatement {
	/// Create a new ATTACH DATABASE statement
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::attach_database();
	/// ```
	pub fn new() -> Self {
		Self {
			file_path: None,
			database_name: None,
		}
	}

	/// Take the ownership of data in the current [`AttachDatabaseStatement`]
	pub fn take(&mut self) -> Self {
		Self {
			file_path: self.file_path.take(),
			database_name: self.database_name.take(),
		}
	}

	/// Set the file path to the database file
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::attach_database()
	///     .file_path("path/to/db.sqlite");
	/// ```
	pub fn file_path<S>(&mut self, path: S) -> &mut Self
	where
		S: Into<String>,
	{
		self.file_path = Some(path.into());
		self
	}

	/// Set the alias name for the attached database
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::attach_database()
	///     .file_path("path/to/db.sqlite")
	///     .as_name("auxiliary");
	/// ```
	pub fn as_name<N>(&mut self, name: N) -> &mut Self
	where
		N: IntoIden,
	{
		self.database_name = Some(name.into_iden());
		self
	}
}

impl Default for AttachDatabaseStatement {
	fn default() -> Self {
		Self::new()
	}
}

impl QueryStatementBuilder for AttachDatabaseStatement {
	fn build_any(&self, query_builder: &dyn QueryBuilderTrait) -> (String, crate::value::Values) {
		// Downcast to concrete QueryBuilder type
		use std::any::Any;
		if let Some(_builder) =
			(query_builder as &dyn Any).downcast_ref::<crate::backend::PostgresQueryBuilder>()
		{
			// TODO: Implement build_attach_database in backend
			todo!("Implement build_attach_database for PostgreSQL");
		}
		if let Some(_builder) =
			(query_builder as &dyn Any).downcast_ref::<crate::backend::MySqlQueryBuilder>()
		{
			// TODO: Implement build_attach_database in backend
			todo!("Implement build_attach_database for MySQL");
		}
		if let Some(_builder) =
			(query_builder as &dyn Any).downcast_ref::<crate::backend::SqliteQueryBuilder>()
		{
			// TODO: Implement build_attach_database in backend
			todo!("Implement build_attach_database for SQLite");
		}
		if let Some(_builder) =
			(query_builder as &dyn Any).downcast_ref::<crate::backend::CockroachDBQueryBuilder>()
		{
			// TODO: Implement build_attach_database in backend
			todo!("Implement build_attach_database for CockroachDB");
		}
		panic!("Unsupported query builder type");
	}

	fn to_string<T: QueryBuilderTrait>(&self, query_builder: T) -> String {
		let (sql, _) = self.build_any(&query_builder);
		sql
	}
}

impl QueryStatementWriter for AttachDatabaseStatement {}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::*;

	#[rstest]
	fn test_attach_database_new() {
		let stmt = AttachDatabaseStatement::new();
		assert!(stmt.file_path.is_none());
		assert!(stmt.database_name.is_none());
	}

	#[rstest]
	fn test_attach_database_with_file_path() {
		let mut stmt = AttachDatabaseStatement::new();
		stmt.file_path("path/to/db.sqlite");
		assert_eq!(stmt.file_path.as_ref().unwrap(), "path/to/db.sqlite");
	}

	#[rstest]
	fn test_attach_database_with_as_name() {
		let mut stmt = AttachDatabaseStatement::new();
		stmt.as_name("auxiliary");
		assert_eq!(
			stmt.database_name.as_ref().unwrap().to_string(),
			"auxiliary"
		);
	}

	#[rstest]
	fn test_attach_database_full() {
		let mut stmt = AttachDatabaseStatement::new();
		stmt.file_path("path/to/db.sqlite").as_name("auxiliary");
		assert_eq!(stmt.file_path.as_ref().unwrap(), "path/to/db.sqlite");
		assert_eq!(
			stmt.database_name.as_ref().unwrap().to_string(),
			"auxiliary"
		);
	}

	#[rstest]
	fn test_attach_database_take() {
		let mut stmt = AttachDatabaseStatement::new();
		stmt.file_path("path/to/db.sqlite").as_name("auxiliary");
		let taken = stmt.take();
		assert!(stmt.file_path.is_none());
		assert!(stmt.database_name.is_none());
		assert_eq!(taken.file_path.as_ref().unwrap(), "path/to/db.sqlite");
		assert_eq!(
			taken.database_name.as_ref().unwrap().to_string(),
			"auxiliary"
		);
	}

	#[rstest]
	fn test_attach_database_default() {
		let stmt = AttachDatabaseStatement::default();
		assert!(stmt.file_path.is_none());
		assert!(stmt.database_name.is_none());
	}
}
