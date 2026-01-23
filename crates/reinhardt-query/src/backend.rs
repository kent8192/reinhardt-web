//! SQL Backend implementations
//!
//! This module provides database-specific SQL generation backends for PostgreSQL,
//! MySQL, and SQLite.

use crate::{
	query::{DeleteStatement, InsertStatement, SelectStatement, UpdateStatement},
	value::Values,
};

mod mysql;
mod postgres;
mod sql_writer;
mod sqlite;

pub use mysql::MySqlQueryBuilder;
pub use postgres::PostgresQueryBuilder;
pub use sql_writer::SqlWriter;
pub use sqlite::SqliteQueryBuilder;

/// Query builder trait for generating SQL from query statements
///
/// This trait defines the interface for database-specific query builders
/// that generate SQL syntax for different backends.
///
/// # Implementations
///
/// - [`PostgresQueryBuilder`] - PostgreSQL backend
/// - [`MySqlQueryBuilder`] - MySQL backend
/// - [`SqliteQueryBuilder`] - SQLite backend
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_query::backend::{QueryBuilder, PostgresQueryBuilder};
/// use reinhardt_query::prelude::*;
///
/// let builder = PostgresQueryBuilder::new();
/// let stmt = Query::select()
///     .column("id")
///     .column("name")
///     .from("users")
///     .and_where(Expr::col("active").eq(true));
///
/// let (sql, values) = builder.build_select(&stmt);
/// // sql: SELECT "id", "name" FROM "users" WHERE "active" = $1
/// // values: [Value::Bool(true)]
/// ```
pub trait QueryBuilder {
	/// Build SELECT statement
	///
	/// Generates SQL and parameter values for a SELECT statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The SELECT statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_select(&self, stmt: &SelectStatement) -> (String, Values);

	/// Build INSERT statement
	///
	/// Generates SQL and parameter values for an INSERT statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The INSERT statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_insert(&self, stmt: &InsertStatement) -> (String, Values);

	/// Build UPDATE statement
	///
	/// Generates SQL and parameter values for an UPDATE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The UPDATE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_update(&self, stmt: &UpdateStatement) -> (String, Values);

	/// Build DELETE statement
	///
	/// Generates SQL and parameter values for a DELETE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DELETE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_delete(&self, stmt: &DeleteStatement) -> (String, Values);

	/// Escape an identifier (table name, column name, etc.)
	///
	/// # Arguments
	///
	/// * `ident` - The identifier to escape
	///
	/// # Returns
	///
	/// The escaped identifier string
	///
	/// # Examples
	///
	/// - PostgreSQL: `escape_identifier("user")` -> `"user"`
	/// - MySQL: `escape_identifier("user")` -> `` `user` ``
	/// - SQLite: `escape_identifier("user")` -> `"user"`
	fn escape_identifier(&self, ident: &str) -> String;

	/// Format a value for SQL
	///
	/// # Arguments
	///
	/// * `value` - The value to format
	/// * `index` - The parameter index (1-based)
	///
	/// # Returns
	///
	/// The formatted placeholder string
	///
	/// # Examples
	///
	/// - PostgreSQL: `format_placeholder(1)` -> `$1`
	/// - MySQL: `format_placeholder(1)` -> `?`
	/// - SQLite: `format_placeholder(1)` -> `?`
	fn format_placeholder(&self, index: usize) -> String;
}
