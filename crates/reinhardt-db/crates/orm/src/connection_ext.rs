//! Connection trait extension for SeaQuery support
//!
//! This module provides extensions to the database connection trait
//! to support executing SeaQuery statement objects directly.

use async_trait::async_trait;
use sea_query::{DeleteStatement, InsertStatement, SelectStatement, UpdateStatement};

use crate::query_types::{DbBackend, QueryStatement};

/// Row type (placeholder - will be replaced with actual row type)
pub type Row = sqlx::postgres::PgRow;

/// Result type for database operations
pub type DbResult<T> = Result<T, reinhardt_core::exception::Error>;

/// Connection trait extension for SeaQuery support
#[async_trait]
pub trait ConnectionExt {
	/// Get database backend type
	fn backend(&self) -> DbBackend;

	/// Execute a SeaQuery statement (INSERT, UPDATE, DELETE, DDL)
	async fn execute_statement(&self, stmt: &QueryStatement) -> DbResult<u64>;

	/// Query multiple rows with SeaQuery SELECT statement
	async fn query_statement(&self, stmt: &SelectStatement) -> DbResult<Vec<Row>>;

	/// Query one row with SeaQuery SELECT statement
	async fn query_one_statement(&self, stmt: &SelectStatement) -> DbResult<Row>;

	/// Execute a SELECT statement
	async fn execute_select(&self, stmt: &SelectStatement) -> DbResult<Vec<Row>> {
		self.query_statement(stmt).await
	}

	/// Execute an INSERT statement
	async fn execute_insert(&self, stmt: &InsertStatement) -> DbResult<u64>;

	/// Execute an UPDATE statement
	async fn execute_update(&self, stmt: &UpdateStatement) -> DbResult<u64>;

	/// Execute a DELETE statement
	async fn execute_delete(&self, stmt: &DeleteStatement) -> DbResult<u64>;
}

/// Default implementation helper for building SQL from statements
pub mod helpers {
	use super::*;
	use sea_query::{MysqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder, Values};

	/// Build SQL string and values from SelectStatement
	pub fn build_select(stmt: &SelectStatement, backend: DbBackend) -> (String, Values) {
		match backend {
			DbBackend::Postgres => stmt.clone().build(PostgresQueryBuilder),
			DbBackend::Mysql => stmt.clone().build(MysqlQueryBuilder),
			DbBackend::Sqlite => stmt.clone().build(SqliteQueryBuilder),
		}
	}

	/// Build SQL string and values from InsertStatement
	pub fn build_insert(stmt: &InsertStatement, backend: DbBackend) -> (String, Values) {
		match backend {
			DbBackend::Postgres => stmt.clone().build(PostgresQueryBuilder),
			DbBackend::Mysql => stmt.clone().build(MysqlQueryBuilder),
			DbBackend::Sqlite => stmt.clone().build(SqliteQueryBuilder),
		}
	}

	/// Build SQL string and values from UpdateStatement
	pub fn build_update(stmt: &UpdateStatement, backend: DbBackend) -> (String, Values) {
		match backend {
			DbBackend::Postgres => stmt.clone().build(PostgresQueryBuilder),
			DbBackend::Mysql => stmt.clone().build(MysqlQueryBuilder),
			DbBackend::Sqlite => stmt.clone().build(SqliteQueryBuilder),
		}
	}

	/// Build SQL string and values from DeleteStatement
	pub fn build_delete(stmt: &DeleteStatement, backend: DbBackend) -> (String, Values) {
		match backend {
			DbBackend::Postgres => stmt.clone().build(PostgresQueryBuilder),
			DbBackend::Mysql => stmt.clone().build(MysqlQueryBuilder),
			DbBackend::Sqlite => stmt.clone().build(SqliteQueryBuilder),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use sea_query::{Alias, Expr, Query};

	#[test]
	fn test_build_select_postgres() {
		use sea_query::ExprTrait;

		let stmt = Query::select()
			.from(Alias::new("users"))
			.column(Alias::new("id"))
			.and_where(Expr::col(Alias::new("id")).eq(1))
			.to_owned();

		let (sql, values) = helpers::build_select(&stmt, DbBackend::Postgres);
		assert!(sql.contains("SELECT"));
		assert!(sql.contains("users"));
		assert_eq!(values.0.len(), 1);
	}

	#[test]
	fn test_build_select_mysql() {
		let stmt = Query::select()
			.from(Alias::new("users"))
			.column(Alias::new("id"))
			.to_owned();

		let (sql, _) = helpers::build_select(&stmt, DbBackend::Mysql);
		assert!(sql.contains("SELECT"));
		assert!(sql.contains("`users`")); // MySQL uses backticks
	}

	#[test]
	fn test_build_select_sqlite() {
		let stmt = Query::select()
			.from(Alias::new("users"))
			.column(Alias::new("id"))
			.to_owned();

		let (sql, _) = helpers::build_select(&stmt, DbBackend::Sqlite);
		assert!(sql.contains("SELECT"));
		assert!(sql.contains("\"users\"")); // SQLite uses double quotes
	}
}
