//! Checked SQLite connection-local database-list inspection.

use crate::{QueryBuildError, backend::SqliteQueryBuilder, value::Values};

/// Inspects the databases associated with the executing SQLite connection.
///
/// The result columns are `seq` (integer), `name` (text), and `file` (text),
/// in that order. An empty `file` is preserved and does not establish whether
/// other connections share the database. Execute the generated SQL on the
/// connection whose main, TEMP, and attached databases should be inspected.
///
/// Only checked rendering is exposed. PostgreSQL, MySQL, and CockroachDB are
/// rejected before execution. This SQL-generation API has P2 native/WASM parity;
/// connection ownership, execution, and result decoding remain caller-owned.
/// See <https://www.sqlite.org/pragma.html#pragma_database_list>.
///
/// # Examples
///
/// ```rust
/// use reinhardt_query::Query;
///
/// let (sql, values) = Query::sqlite_database_list().build_sqlite_checked()?;
/// assert_eq!(sql, "PRAGMA database_list");
/// assert!(values.is_empty());
/// # Ok::<(), reinhardt_query::QueryBuildError>(())
/// ```
///
/// Unchecked rendering with arbitrary builders is unavailable:
///
/// ```compile_fail
/// use reinhardt_query::{MySqlQueryBuilder, Query, QueryStatementBuilder};
/// Query::sqlite_database_list().build(MySqlQueryBuilder);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SqliteDatabaseListStatement;

impl SqliteDatabaseListStatement {
	/// Constructs a connection-local inspection statement with P2 native/WASM parity.
	pub const fn new() -> Self {
		Self
	}

	/// Builds SQLite inspection SQL and empty bind values with P2 native/WASM parity.
	pub fn build_sqlite_checked(&self) -> Result<(String, Values), QueryBuildError> {
		Ok(SqliteQueryBuilder.build_database_list(self))
	}

	/// Rejects SQLite inspection on PostgreSQL with P2 native/WASM parity.
	pub fn build_postgres_checked(&self) -> Result<(String, Values), QueryBuildError> {
		self.unsupported_backend("PostgreSQL")
	}

	/// Rejects SQLite inspection on MySQL with P2 native/WASM parity.
	pub fn build_mysql_checked(&self) -> Result<(String, Values), QueryBuildError> {
		self.unsupported_backend("MySQL")
	}

	/// Rejects SQLite inspection on CockroachDB with P2 native/WASM parity.
	pub fn build_cockroachdb_checked(&self) -> Result<(String, Values), QueryBuildError> {
		self.unsupported_backend("CockroachDB")
	}

	fn unsupported_backend(
		&self,
		backend: &'static str,
	) -> Result<(String, Values), QueryBuildError> {
		Err(QueryBuildError::UnsupportedBackendFeature {
			feature: "SQLite database-list inspection",
			backend,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::Query;
	use rstest::*;

	#[rstest]
	#[case(SqliteDatabaseListStatement::build_postgres_checked, "PostgreSQL")]
	#[case(SqliteDatabaseListStatement::build_mysql_checked, "MySQL")]
	#[case(SqliteDatabaseListStatement::build_cockroachdb_checked, "CockroachDB")]
	fn database_list_rejects_unsupported_backends(
		#[case] build: fn(
			&SqliteDatabaseListStatement,
		) -> Result<(String, Values), QueryBuildError>,
		#[case] backend: &'static str,
	) {
		// Arrange
		let statement = Query::sqlite_database_list();

		// Act
		let result = build(&statement);

		// Assert
		assert_eq!(
			result,
			Err(QueryBuildError::UnsupportedBackendFeature {
				feature: "SQLite database-list inspection",
				backend,
			})
		);
	}
}
