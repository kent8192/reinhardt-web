//! SQL Backend implementations
//!
//! This module provides database-specific SQL generation backends for PostgreSQL,
//! MySQL, and SQLite.

use crate::{
	dcl::{
		AlterRoleStatement, AlterUserStatement, CreateRoleStatement, CreateUserStatement,
		DropRoleStatement, DropUserStatement, GrantRoleStatement, GrantStatement,
		RenameUserStatement, ResetRoleStatement, RevokeRoleStatement, RevokeStatement,
		SetDefaultRoleStatement, SetRoleStatement,
	},
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

	/// Build GRANT statement
	///
	/// Generates SQL and parameter values for a GRANT statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The GRANT statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_grant(&self, stmt: &GrantStatement) -> (String, Values);

	/// Build REVOKE statement
	///
	/// Generates SQL and parameter values for a REVOKE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The REVOKE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_revoke(&self, stmt: &RevokeStatement) -> (String, Values);

	/// Build GRANT role membership statement
	///
	/// Generates SQL and parameter values for a GRANT role membership statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The GRANT role statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_grant_role(&self, stmt: &GrantRoleStatement) -> (String, Values);

	/// Build REVOKE role membership statement
	///
	/// Generates SQL and parameter values for a REVOKE role membership statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The REVOKE role statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_revoke_role(&self, stmt: &RevokeRoleStatement) -> (String, Values);

	/// Build CREATE ROLE statement
	///
	/// Generates SQL and parameter values for a CREATE ROLE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE ROLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_role(&self, stmt: &CreateRoleStatement) -> (String, Values);

	/// Build DROP ROLE statement
	///
	/// Generates SQL and parameter values for a DROP ROLE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP ROLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_role(&self, stmt: &DropRoleStatement) -> (String, Values);

	/// Build ALTER ROLE statement
	///
	/// Generates SQL and parameter values for an ALTER ROLE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The ALTER ROLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_alter_role(&self, stmt: &AlterRoleStatement) -> (String, Values);

	/// Build CREATE USER statement
	///
	/// Generates SQL and parameter values for a CREATE USER statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE USER statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_user(&self, stmt: &CreateUserStatement) -> (String, Values);

	/// Build DROP USER statement
	///
	/// Generates SQL and parameter values for a DROP USER statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP USER statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_user(&self, stmt: &DropUserStatement) -> (String, Values);

	/// Build ALTER USER statement
	///
	/// Generates SQL and parameter values for an ALTER USER statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The ALTER USER statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_alter_user(&self, stmt: &AlterUserStatement) -> (String, Values);

	/// Build RENAME USER statement
	///
	/// Generates SQL and parameter values for a RENAME USER statement.
	/// This is MySQL-only; PostgreSQL and SQLite will panic.
	///
	/// # Arguments
	///
	/// * `stmt` - The RENAME USER statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_rename_user(&self, stmt: &RenameUserStatement) -> (String, Values);

	/// Build SET ROLE statement
	///
	/// Generates SQL and parameter values for a SET ROLE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The SET ROLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_set_role(&self, stmt: &SetRoleStatement) -> (String, Values);

	/// Build RESET ROLE statement
	///
	/// Generates SQL and parameter values for a RESET ROLE statement.
	/// This is PostgreSQL-only; MySQL and SQLite will panic.
	///
	/// # Arguments
	///
	/// * `stmt` - The RESET ROLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_reset_role(&self, stmt: &ResetRoleStatement) -> (String, Values);

	/// Build SET DEFAULT ROLE statement
	///
	/// Generates SQL and parameter values for a SET DEFAULT ROLE statement.
	/// This is MySQL-only; PostgreSQL and SQLite will panic.
	///
	/// # Arguments
	///
	/// * `stmt` - The SET DEFAULT ROLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_set_default_role(&self, stmt: &SetDefaultRoleStatement) -> (String, Values);

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
