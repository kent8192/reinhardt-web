//! DETACH DATABASE statement builder
//!
//! This module provides the `DetachDatabaseStatement` type for building SQL DETACH DATABASE queries.
//! DETACH DATABASE is a SQLite-specific feature for disconnecting previously attached database files.

use crate::types::{DynIden, IntoIden};

use super::super::traits::{QueryBuilderTrait, QueryStatementBuilder, QueryStatementWriter};

/// DETACH DATABASE statement builder (SQLite-specific)
///
/// This struct provides a fluent API for constructing DETACH DATABASE queries.
/// DETACH DATABASE allows detaching previously attached database files from the current connection.
///
/// # Examples
///
/// ```rust
/// use reinhardt_query::prelude::*;
///
/// // DETACH DATABASE auxiliary
/// let query = Query::detach_database()
///     .name("auxiliary");
/// ```
#[derive(Debug, Clone)]
pub struct DetachDatabaseStatement {
	pub(crate) database_name: Option<DynIden>,
}

impl DetachDatabaseStatement {
	/// Create a new DETACH DATABASE statement
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::detach_database();
	/// ```
	pub fn new() -> Self {
		Self {
			database_name: None,
		}
	}

	/// Take the ownership of data in the current [`DetachDatabaseStatement`]
	pub fn take(&mut self) -> Self {
		Self {
			database_name: self.database_name.take(),
		}
	}

	/// Set the name of the database to detach
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::detach_database()
	///     .name("auxiliary");
	/// ```
	pub fn name<N>(&mut self, name: N) -> &mut Self
	where
		N: IntoIden,
	{
		self.database_name = Some(name.into_iden());
		self
	}
}

impl Default for DetachDatabaseStatement {
	fn default() -> Self {
		Self::new()
	}
}

impl QueryStatementBuilder for DetachDatabaseStatement {
	fn build_any(&self, query_builder: &dyn QueryBuilderTrait) -> (String, crate::value::Values) {
		// Downcast to concrete QueryBuilder type
		use std::any::Any;
		if let Some(_builder) =
			(query_builder as &dyn Any).downcast_ref::<crate::backend::PostgresQueryBuilder>()
		{
			// TODO: Implement build_detach_database in backend
			todo!("Implement build_detach_database for PostgreSQL");
		}
		if let Some(_builder) =
			(query_builder as &dyn Any).downcast_ref::<crate::backend::MySqlQueryBuilder>()
		{
			// TODO: Implement build_detach_database in backend
			todo!("Implement build_detach_database for MySQL");
		}
		if let Some(_builder) =
			(query_builder as &dyn Any).downcast_ref::<crate::backend::SqliteQueryBuilder>()
		{
			// TODO: Implement build_detach_database in backend
			todo!("Implement build_detach_database for SQLite");
		}
		if let Some(_builder) =
			(query_builder as &dyn Any).downcast_ref::<crate::backend::CockroachDBQueryBuilder>()
		{
			// TODO: Implement build_detach_database in backend
			todo!("Implement build_detach_database for CockroachDB");
		}
		panic!("Unsupported query builder type");
	}
}

impl QueryStatementWriter for DetachDatabaseStatement {}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::*;

	#[rstest]
	fn test_detach_database_new() {
		let stmt = DetachDatabaseStatement::new();
		assert!(stmt.database_name.is_none());
	}

	#[rstest]
	fn test_detach_database_with_name() {
		let mut stmt = DetachDatabaseStatement::new();
		stmt.name("auxiliary");
		assert_eq!(
			stmt.database_name.as_ref().unwrap().to_string(),
			"auxiliary"
		);
	}

	#[rstest]
	fn test_detach_database_take() {
		let mut stmt = DetachDatabaseStatement::new();
		stmt.name("auxiliary");
		let taken = stmt.take();
		assert!(stmt.database_name.is_none());
		assert_eq!(
			taken.database_name.as_ref().unwrap().to_string(),
			"auxiliary"
		);
	}

	#[rstest]
	fn test_detach_database_default() {
		let stmt = DetachDatabaseStatement::default();
		assert!(stmt.database_name.is_none());
	}

	#[rstest]
	fn test_detach_database_fluent_api() {
		let mut stmt = DetachDatabaseStatement::new();
		let result = stmt.name("test_db");
		// Verify fluent API returns mutable reference
		assert_eq!(
			result.database_name.as_ref().unwrap().to_string(),
			"test_db"
		);
	}
}
