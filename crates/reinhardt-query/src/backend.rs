//! SQL Backend implementations
//!
//! This module provides database-specific SQL generation backends for PostgreSQL,
//! MySQL, SQLite, and CockroachDB.

use crate::{
	query::{
		AlterDatabaseStatement, AlterIndexStatement, AlterSchemaStatement, AlterSequenceStatement,
		AlterTableStatement, CheckTableStatement, CommentStatement, CreateDatabaseStatement,
		CreateIndexStatement, CreateSchemaStatement, CreateSequenceStatement, CreateTableStatement,
		CreateTriggerStatement, CreateViewStatement, DeleteStatement, DropDatabaseStatement,
		DropIndexStatement, DropSchemaStatement, DropSequenceStatement, DropTableStatement,
		DropTriggerStatement, DropViewStatement, InsertStatement, OptimizeTableStatement,
		ReindexStatement, RepairTableStatement, SelectStatement, TruncateTableStatement,
		UpdateStatement,
	},
	value::Values,
};

mod cockroachdb;
mod mysql;
mod postgres;
mod sql_writer;
mod sqlite;

pub use cockroachdb::CockroachDBQueryBuilder;
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
/// - [`CockroachDBQueryBuilder`] - CockroachDB backend
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

	/// Build CREATE TABLE statement
	///
	/// Generates SQL and parameter values for a CREATE TABLE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE TABLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_table(&self, stmt: &CreateTableStatement) -> (String, Values);

	/// Build ALTER TABLE statement
	///
	/// Generates SQL and parameter values for an ALTER TABLE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The ALTER TABLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_alter_table(&self, stmt: &AlterTableStatement) -> (String, Values);

	/// Build DROP TABLE statement
	///
	/// Generates SQL and parameter values for a DROP TABLE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP TABLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_table(&self, stmt: &DropTableStatement) -> (String, Values);

	/// Build CREATE INDEX statement
	///
	/// Generates SQL and parameter values for a CREATE INDEX statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE INDEX statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_index(&self, stmt: &CreateIndexStatement) -> (String, Values);

	/// Build DROP INDEX statement
	///
	/// Generates SQL and parameter values for a DROP INDEX statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP INDEX statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_index(&self, stmt: &DropIndexStatement) -> (String, Values);

	/// Build CREATE VIEW statement
	///
	/// Generates SQL and parameter values for a CREATE VIEW statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE VIEW statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_view(&self, stmt: &CreateViewStatement) -> (String, Values);

	/// Build DROP VIEW statement
	///
	/// Generates SQL and parameter values for a DROP VIEW statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP VIEW statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_view(&self, stmt: &DropViewStatement) -> (String, Values);

	/// Build TRUNCATE TABLE statement
	///
	/// Generates SQL and parameter values for a TRUNCATE TABLE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The TRUNCATE TABLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_truncate_table(&self, stmt: &TruncateTableStatement) -> (String, Values);

	/// Build CREATE TRIGGER statement
	///
	/// Generates SQL and parameter values for a CREATE TRIGGER statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE TRIGGER statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_trigger(&self, stmt: &CreateTriggerStatement) -> (String, Values);

	/// Build DROP TRIGGER statement
	///
	/// Generates SQL and parameter values for a DROP TRIGGER statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP TRIGGER statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_trigger(&self, stmt: &DropTriggerStatement) -> (String, Values);

	/// Build ALTER INDEX statement
	///
	/// Generates SQL and parameter values for an ALTER INDEX statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The ALTER INDEX statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_alter_index(&self, stmt: &AlterIndexStatement) -> (String, Values);

	/// Build REINDEX statement
	///
	/// Generates SQL and parameter values for a REINDEX statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The REINDEX statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_reindex(&self, stmt: &ReindexStatement) -> (String, Values);

	/// Build CREATE SCHEMA statement
	///
	/// Generates SQL and parameter values for a CREATE SCHEMA statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE SCHEMA statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_schema(&self, stmt: &CreateSchemaStatement) -> (String, Values);

	/// Build ALTER SCHEMA statement
	///
	/// Generates SQL and parameter values for an ALTER SCHEMA statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The ALTER SCHEMA statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_alter_schema(&self, stmt: &AlterSchemaStatement) -> (String, Values);

	/// Build DROP SCHEMA statement
	///
	/// Generates SQL and parameter values for a DROP SCHEMA statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP SCHEMA statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_schema(&self, stmt: &DropSchemaStatement) -> (String, Values);

	/// Build CREATE SEQUENCE statement
	///
	/// Generates SQL and parameter values for a CREATE SEQUENCE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE SEQUENCE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_sequence(&self, stmt: &CreateSequenceStatement) -> (String, Values);

	/// Build ALTER SEQUENCE statement
	///
	/// Generates SQL and parameter values for an ALTER SEQUENCE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The ALTER SEQUENCE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_alter_sequence(&self, stmt: &AlterSequenceStatement) -> (String, Values);

	/// Build DROP SEQUENCE statement
	///
	/// Generates SQL and parameter values for a DROP SEQUENCE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP SEQUENCE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_sequence(&self, stmt: &DropSequenceStatement) -> (String, Values);

	/// Build COMMENT ON statement
	///
	/// Generates SQL and parameter values for a COMMENT ON statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The COMMENT ON statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_comment(&self, stmt: &CommentStatement) -> (String, Values);

	/// Build CREATE DATABASE statement
	///
	/// Generates SQL and parameter values for a CREATE DATABASE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The CREATE DATABASE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_create_database(&self, stmt: &CreateDatabaseStatement) -> (String, Values);

	/// Build ALTER DATABASE statement
	///
	/// Generates SQL and parameter values for an ALTER DATABASE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The ALTER DATABASE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_alter_database(&self, stmt: &AlterDatabaseStatement) -> (String, Values);

	/// Build DROP DATABASE statement
	///
	/// Generates SQL and parameter values for a DROP DATABASE statement.
	///
	/// # Arguments
	///
	/// * `stmt` - The DROP DATABASE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_drop_database(&self, stmt: &DropDatabaseStatement) -> (String, Values);

	/// Build OPTIMIZE TABLE statement
	///
	/// Generates SQL and parameter values for an OPTIMIZE TABLE statement.
	/// **MySQL-only**: Other backends will panic with a helpful message.
	///
	/// # Arguments
	///
	/// * `stmt` - The OPTIMIZE TABLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_optimize_table(&self, stmt: &OptimizeTableStatement) -> (String, Values);

	/// Build REPAIR TABLE statement
	///
	/// Generates SQL and parameter values for a REPAIR TABLE statement.
	/// **MySQL-only**: Other backends will panic with a helpful message.
	///
	/// # Arguments
	///
	/// * `stmt` - The REPAIR TABLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_repair_table(&self, stmt: &RepairTableStatement) -> (String, Values);

	/// Build CHECK TABLE statement
	///
	/// Generates SQL and parameter values for a CHECK TABLE statement.
	/// **MySQL-only**: Other backends will panic with a helpful message.
	///
	/// # Arguments
	///
	/// * `stmt` - The CHECK TABLE statement to build
	///
	/// # Returns
	///
	/// A tuple of (SQL string, parameter values)
	fn build_check_table(&self, stmt: &CheckTableStatement) -> (String, Values);

	// TODO: Advanced DDL operations (function, procedure, materialized view, type)
	// will be implemented in future commits
	// /// Build CREATE FUNCTION statement
	// ///
	// /// Generates SQL and parameter values for a CREATE FUNCTION statement.
	// ///
	// /// # Arguments
	// ///
	// /// * `stmt` - The CREATE FUNCTION statement to build
	// ///
	// /// # Returns
	// ///
	// /// A tuple of (SQL string, parameter values)
	// fn build_create_function(&self, stmt: &CreateFunctionStatement) -> (String, Values);

// 	/// Build ALTER FUNCTION statement
// 	///
// 	/// Generates SQL and parameter values for an ALTER FUNCTION statement.
// 	///
// 	/// # Arguments
// 	///
// 	/// * `stmt` - The ALTER FUNCTION statement to build
// 	///
// 	/// # Returns
// 	///
// 	/// A tuple of (SQL string, parameter values)
// 	fn build_alter_function(&self, stmt: &AlterFunctionStatement) -> (String, Values);
// 
// 	/// Build DROP FUNCTION statement
// 	///
// 	/// Generates SQL and parameter values for a DROP FUNCTION statement.
// 	///
// 	/// # Arguments
// 	///
// 	/// * `stmt` - The DROP FUNCTION statement to build
// 	///
// 	/// # Returns
// 	///
// 	/// A tuple of (SQL string, parameter values)
// 	fn build_drop_function(&self, stmt: &DropFunctionStatement) -> (String, Values);
// 
// 	/// Build CREATE PROCEDURE statement
// 	///
// 	/// Generates SQL and parameter values for a CREATE PROCEDURE statement.
// 	///
// 	/// # Arguments
// 	///
// 	/// * `stmt` - The CREATE PROCEDURE statement to build
// 	///
// 	/// # Returns
// 	///
// 	/// A tuple of (SQL string, parameter values)
// 	fn build_create_procedure(&self, stmt: &CreateProcedureStatement) -> (String, Values);
// 
// 	/// Build ALTER PROCEDURE statement
// 	///
// 	/// Generates SQL and parameter values for an ALTER PROCEDURE statement.
// 	///
// 	/// # Arguments
// 	///
// 	/// * `stmt` - The ALTER PROCEDURE statement to build
// 	///
// 	/// # Returns
// 	///
// 	/// A tuple of (SQL string, parameter values)
// 	fn build_alter_procedure(&self, stmt: &AlterProcedureStatement) -> (String, Values);
// 
// 	/// Build DROP PROCEDURE statement
// 	///
// 	/// Generates SQL and parameter values for a DROP PROCEDURE statement.
// 	///
// 	/// # Arguments
// 	///
// 	/// * `stmt` - The DROP PROCEDURE statement to build
// 	///
// 	/// # Returns
// 	///
// 	/// A tuple of (SQL string, parameter values)
// 	fn build_drop_procedure(&self, stmt: &DropProcedureStatement) -> (String, Values);
// 
// 	/// Build CREATE TYPE statement
// 	///
// 	/// Generates SQL and parameter values for a CREATE TYPE statement.
// 	///
// 	/// # Arguments
// 	///
// 	/// * `stmt` - The CREATE TYPE statement to build
// 	///
// 	/// # Returns
// 	///
// 	/// A tuple of (SQL string, parameter values)
// 	fn build_create_type(&self, stmt: &CreateTypeStatement) -> (String, Values);
// 
// 	/// Build ALTER TYPE statement
// 	///
// 	/// Generates SQL and parameter values for an ALTER TYPE statement.
// 	///
// 	/// # Arguments
// 	///
// 	/// * `stmt` - The ALTER TYPE statement to build
// 	///
// 	/// # Returns
// 	///
// 	/// A tuple of (SQL string, parameter values)
// 	fn build_alter_type(&self, stmt: &AlterTypeStatement) -> (String, Values);
// 
// 	/// Build DROP TYPE statement
// 	///
// 	/// Generates SQL and parameter values for a DROP TYPE statement.
// 	///
// 	/// # Arguments
// 	///
// 	/// * `stmt` - The DROP TYPE statement to build
// 	///
// 	/// # Returns
// 	///
// 	/// A tuple of (SQL string, parameter values)
// 	fn build_drop_type(&self, stmt: &DropTypeStatement) -> (String, Values);
// 
// 	/// Build ANALYZE statement
// 	///
// 	/// Generates SQL and parameter values for an ANALYZE statement.
// 	///
// 	/// # Arguments
// 	///
// 	/// * `stmt` - The ANALYZE statement to build
// 	///
// 	/// # Returns
// 	///
// 	/// A tuple of (SQL string, parameter values)
// 	fn build_analyze(&self, stmt: &AnalyzeStatement) -> (String, Values);
// 
// 	/// Build VACUUM statement
// 	///
// 	/// Generates SQL and parameter values for a VACUUM statement.
// 	///
// 	/// # Arguments
// 	///
// 	/// * `stmt` - The VACUUM statement to build
// 	///
// 	/// # Returns
// 	///
// 	/// A tuple of (SQL string, parameter values)
// 	fn build_vacuum(&self, stmt: &VacuumStatement) -> (String, Values);
// 
// 	/// Build OPTIMIZE TABLE statement
// 	///
// 	/// Generates SQL and parameter values for an OPTIMIZE TABLE statement.
// 	/// **MySQL-only**: Other backends will panic with a helpful message.
// 	///
// 	/// # Arguments
// 	///
// 	/// * `stmt` - The OPTIMIZE TABLE statement to build
// 	///
// 	/// # Returns
// 	///
// 	/// A tuple of (SQL string, parameter values)
// 	fn build_optimize_table(&self, stmt: &OptimizeTableStatement) -> (String, Values);
// 
// 	/// Build REPAIR TABLE statement
// 	///
// 	/// Generates SQL and parameter values for a REPAIR TABLE statement.
// 	/// **MySQL-only**: Other backends will panic with a helpful message.
// 	///
// 	/// # Arguments
// 	///
// 	/// * `stmt` - The REPAIR TABLE statement to build
// 	///
// 	/// # Returns
// 	///
// 	/// A tuple of (SQL string, parameter values)
// 	fn build_repair_table(&self, stmt: &RepairTableStatement) -> (String, Values);
// 
// 	/// Build CHECK TABLE statement
// 	///
// 	/// Generates SQL and parameter values for a CHECK TABLE statement.
// 	/// **MySQL-only**: Other backends will panic with a helpful message.
// 	///
// 	/// # Arguments
// 	///
// 	/// * `stmt` - The CHECK TABLE statement to build
// 	///
// 	/// # Returns
// 	///
// 	/// A tuple of (SQL string, parameter values)
// 	fn build_check_table(&self, stmt: &CheckTableStatement) -> (String, Values);
// 
// 	/// Build CREATE MATERIALIZED VIEW statement
// 	///
// 	/// Generates SQL and parameter values for a CREATE MATERIALIZED VIEW statement.
// 	///
// 	/// # Arguments
// 	///
// 	/// * `stmt` - The CREATE MATERIALIZED VIEW statement to build
// 	///
// 	/// # Returns
// 	///
// 	/// A tuple of (SQL string, parameter values)
// 	fn build_create_materialized_view(
// 		&self,
// 		stmt: &CreateMaterializedViewStatement,
// 	) -> (String, Values);
// 
// 	/// Build ALTER MATERIALIZED VIEW statement
// 	///
// 	/// Generates SQL and parameter values for an ALTER MATERIALIZED VIEW statement.
// 	///
// 	/// # Arguments
// 	///
// 	/// * `stmt` - The ALTER MATERIALIZED VIEW statement to build
// 	///
// 	/// # Returns
// 	///
// 	/// A tuple of (SQL string, parameter values)
// 	fn build_alter_materialized_view(
// 		&self,
// 		stmt: &AlterMaterializedViewStatement,
// 	) -> (String, Values);
// 
// 	/// Build DROP MATERIALIZED VIEW statement
// 	///
// 	/// Generates SQL and parameter values for a DROP MATERIALIZED VIEW statement.
// 	///
// 	/// # Arguments
// 	///
// 	/// * `stmt` - The DROP MATERIALIZED VIEW statement to build
// 	///
// 	/// # Returns
// 	///
// 	/// A tuple of (SQL string, parameter values)
// 	fn build_drop_materialized_view(
// 		&self,
// 		stmt: &DropMaterializedViewStatement,
// 	) -> (String, Values);
// 
// 	/// Build REFRESH MATERIALIZED VIEW statement
// 	///
// 	/// Generates SQL and parameter values for a REFRESH MATERIALIZED VIEW statement.
// 	///
// 	/// # Arguments
// 	///
// 	/// * `stmt` - The REFRESH MATERIALIZED VIEW statement to build
// 	///
// 	/// # Returns
// 	///
// 	/// A tuple of (SQL string, parameter values)
// 	fn build_refresh_materialized_view(
// 		&self,
// 		stmt: &RefreshMaterializedViewStatement,
// 	) -> (String, Values);
}
