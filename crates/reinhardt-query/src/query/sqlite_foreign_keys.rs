//! Checked SQLite foreign-key enforcement settings.

use crate::{QueryBuildError, backend::SqliteQueryBuilder, value::Values};

/// A connection-local SQLite foreign-key enforcement setting.
///
/// Only checked rendering is exposed. Requests for PostgreSQL, MySQL, and
/// CockroachDB are rejected before execution. This operation does not acquire
/// a connection or manage a transaction. Execute the generated
/// statement on the caller's connection outside any transaction or savepoint;
/// SQLite ignores changes to this setting inside a transaction.
///
/// This SQL-generation API has P2 native/WASM parity. Database execution and
/// restoration of the previous setting remain the caller's responsibility.
/// See <https://sqlite.org/pragma.html#pragma_foreign_keys>.
///
/// # Examples
///
/// ```rust
/// use reinhardt_query::Query;
///
/// let (sql, values) = Query::sqlite_foreign_keys(false)
///     .build_sqlite_checked()?;
/// assert_eq!(sql, "PRAGMA foreign_keys = OFF");
/// assert!(values.is_empty());
/// # Ok::<(), reinhardt_query::QueryBuildError>(())
/// ```
///
/// Unchecked rendering with arbitrary builders is intentionally unavailable:
///
/// ```compile_fail
/// use reinhardt_query::{MySqlQueryBuilder, Query, QueryStatementBuilder};
/// Query::sqlite_foreign_keys(true).build(MySqlQueryBuilder);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SqliteForeignKeysStatement {
	pub(crate) enabled: bool,
}

impl SqliteForeignKeysStatement {
	/// Selects whether SQLite foreign-key enforcement should be enabled.
	pub const fn new(enabled: bool) -> Self {
		Self { enabled }
	}

	/// Builds a SQLite setting statement with no bound parameters.
	pub fn build_sqlite_checked(&self) -> Result<(String, Values), QueryBuildError> {
		Ok(SqliteQueryBuilder.build_foreign_keys_setting(self))
	}

	/// Rejects SQLite foreign-key settings on PostgreSQL before execution.
	pub fn build_postgres_checked(&self) -> Result<(String, Values), QueryBuildError> {
		self.unsupported_backend("PostgreSQL")
	}

	/// Rejects SQLite foreign-key settings on MySQL before execution.
	pub fn build_mysql_checked(&self) -> Result<(String, Values), QueryBuildError> {
		self.unsupported_backend("MySQL")
	}

	/// Rejects SQLite foreign-key settings on CockroachDB before execution.
	pub fn build_cockroachdb_checked(&self) -> Result<(String, Values), QueryBuildError> {
		self.unsupported_backend("CockroachDB")
	}

	fn unsupported_backend(
		&self,
		backend: &'static str,
	) -> Result<(String, Values), QueryBuildError> {
		Err(QueryBuildError::UnsupportedBackendFeature {
			feature: "SQLite foreign-key enforcement setting",
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
	#[case(true, "PRAGMA foreign_keys = ON")]
	#[case(false, "PRAGMA foreign_keys = OFF")]
	fn sqlite_foreign_keys_renders_without_parameters(#[case] enabled: bool, #[case] sql: &str) {
		// Arrange
		let statement = Query::sqlite_foreign_keys(enabled);

		// Act
		let built = statement.build_sqlite_checked();

		// Assert
		assert_eq!(built, Ok((sql.to_owned(), Values::default())));
	}

	#[rstest]
	#[case(SqliteForeignKeysStatement::build_postgres_checked, "PostgreSQL")]
	#[case(SqliteForeignKeysStatement::build_mysql_checked, "MySQL")]
	#[case(SqliteForeignKeysStatement::build_cockroachdb_checked, "CockroachDB")]
	fn sqlite_foreign_keys_rejects_unsupported_backends(
		#[case] build: fn(&SqliteForeignKeysStatement) -> Result<(String, Values), QueryBuildError>,
		#[case] backend: &'static str,
		#[values(true, false)] enabled: bool,
	) {
		// Arrange
		let statement = SqliteForeignKeysStatement::new(enabled);

		// Act
		let built = build(&statement);

		// Assert
		assert_eq!(
			built,
			Err(QueryBuildError::UnsupportedBackendFeature {
				feature: "SQLite foreign-key enforcement setting",
				backend,
			})
		);
	}
}
