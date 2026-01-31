//! Query statement traits
//!
//! This module defines the core traits for building and executing SQL queries.

use std::{any::Any, fmt::Debug};

use crate::value::Values;

/// Trait for building query statements
///
/// This trait provides methods to build SQL statements for different database backends
/// and collect query parameters.
pub trait QueryStatementBuilder: Debug {
	/// Build SQL statement for a database backend and collect query parameters
	///
	/// This is the primary method for generating parameterized SQL queries.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::select()
	///     .column(Expr::col("name"))
	///     .from("users");
	///
	/// // Build for PostgreSQL
	/// let (sql, values) = query.build(PostgresQueryBuilder);
	/// // sql = "SELECT \"name\" FROM \"users\""
	/// // values = Values(vec![])
	/// ```
	fn build_any(&self, query_builder: &dyn QueryBuilderTrait) -> (String, Values);

	/// Build SQL statement for a database backend and return SQL string
	///
	/// This method generates SQL without collecting parameters, suitable for
	/// inspection and debugging.
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_query::prelude::*;
	///
	/// let query = Query::select()
	///     .column(Expr::col("name"))
	///     .from("users")
	///     .and_where(Expr::col("active").eq(true));
	///
	/// let sql = query.to_string(MysqlQueryBuilder);
	/// // sql = "SELECT `name` FROM `users` WHERE `active` = true"
	/// ```
	fn to_string<T: QueryBuilderTrait>(&self, query_builder: T) -> String;

	/// Build SQL statement with parameter collection
	///
	/// This is a convenience method that wraps `build_any()` with a concrete
	/// query builder type.
	fn build<T: QueryBuilderTrait>(&self, query_builder: T) -> (String, Values) {
		self.build_any(&query_builder)
	}
}

/// Trait for query statement writers
///
/// This trait extends [`QueryStatementBuilder`] with additional methods for
/// writing SQL statements.
pub trait QueryStatementWriter: QueryStatementBuilder {}

/// Placeholder trait for query builders (will be implemented in Phase 5)
///
/// This trait defines the interface for database-specific query builders
/// that generate SQL syntax for different backends (PostgreSQL, MySQL, SQLite).
pub trait QueryBuilderTrait: Debug + Any {
	/// Get placeholder format for this backend
	///
	/// Returns a tuple of (placeholder_format, is_numbered):
	/// - PostgreSQL: ("$", true) -> $1, $2, $3...
	/// - MySQL: ("?", false) -> ?, ?, ?...
	/// - SQLite: ("?", false) -> ?, ?, ?...
	fn placeholder(&self) -> (&str, bool);

	/// Get quote character for this backend
	///
	/// - PostgreSQL: " (double quote)
	/// - MySQL: ` (backtick)
	/// - SQLite: " (double quote)
	fn quote_char(&self) -> char;
}
