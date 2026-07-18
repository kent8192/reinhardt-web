//! Many-to-many relationship manager.
//!
//! This module provides [`ManyToManyManager`] for managing rows in an
//! intermediate table through a caller-owned ORM executor.

use crate::orm::{DatabaseBackend, OrmExecutor, QueryRow};
use reinhardt_core::exception::{DatabaseError, DatabaseErrorKind, Error, Result};
use reinhardt_query::prelude::{
	Alias, BinOper, Expr, Func, InsertStatement, IntoValue, MySqlQueryBuilder, OnConflict,
	PostgresQueryBuilder, Query, QueryBuilder, SelectStatement, SqliteQueryBuilder, Values,
};
use std::marker::PhantomData;

/// Builds SELECT SQL for the executor's backend.
fn build_select_sql(statement: &SelectStatement, backend: DatabaseBackend) -> (String, Values) {
	match backend {
		DatabaseBackend::Postgres => PostgresQueryBuilder.build_select(statement),
		DatabaseBackend::MySql => MySqlQueryBuilder.build_select(statement),
		DatabaseBackend::Sqlite => SqliteQueryBuilder.build_select(statement),
	}
}

/// Builds INSERT SQL for the executor's backend.
fn build_insert_sql(statement: &InsertStatement, backend: DatabaseBackend) -> (String, Values) {
	match backend {
		DatabaseBackend::Postgres => PostgresQueryBuilder.build_insert(statement),
		DatabaseBackend::MySql => MySqlQueryBuilder.build_insert(statement),
		DatabaseBackend::Sqlite => SqliteQueryBuilder.build_insert(statement),
	}
}

/// Manager for many-to-many relationship operations.
///
/// This type holds only relationship metadata. Every terminal method receives
/// a mutable [`OrmExecutor`] so its work can participate in an outer atomic
/// transaction.
pub struct ManyToManyManager<S, T, PK> {
	source_pk: PK,
	through_table: String,
	source_field: String,
	target_field: String,
	_phantom_s: PhantomData<S>,
	_phantom_t: PhantomData<T>,
}

impl<S, T, PK> ManyToManyManager<S, T, PK>
where
	PK: IntoValue + Clone,
{
	/// Creates a manager for one source instance and intermediate table.
	pub fn new(
		source_pk: PK,
		through_table: String,
		source_field: String,
		target_field: String,
	) -> Self {
		Self {
			source_pk,
			through_table,
			source_field,
			target_field,
			_phantom_s: PhantomData,
			_phantom_t: PhantomData,
		}
	}

	/// Adds a target primary key through the caller-owned executor.
	pub async fn add_with_db<E, TPK>(&self, conn: &mut E, target_pk: TPK) -> Result<()>
	where
		E: OrmExecutor,
		TPK: IntoValue,
	{
		let mut statement = Query::insert();
		statement
			.into_table(Alias::new(&self.through_table))
			.columns([
				Alias::new(&self.source_field),
				Alias::new(&self.target_field),
			])
			.values_panic([Expr::val(self.source_pk.clone()), Expr::val(target_pk)])
			.on_conflict(
				OnConflict::columns([
					Alias::new(&self.source_field),
					Alias::new(&self.target_field),
				])
				.do_nothing()
				.to_owned(),
			);

		let (sql, values) = build_insert_sql(&statement, conn.backend());
		conn.execute(&sql, crate::orm::execution::convert_values(values))
			.await?;
		Ok(())
	}

	/// Removes a target primary key through the caller-owned executor.
	pub async fn remove_with_db<E, TPK>(&self, conn: &mut E, target_pk: TPK) -> Result<()>
	where
		E: OrmExecutor,
		TPK: IntoValue,
	{
		let mut statement = Query::delete();
		statement
			.from_table(Alias::new(&self.through_table))
			.and_where(
				Expr::col(Alias::new(&self.source_field))
					.binary(BinOper::Equal, Expr::val(self.source_pk.clone())),
			)
			.and_where(
				Expr::col(Alias::new(&self.target_field))
					.binary(BinOper::Equal, Expr::val(target_pk)),
			);

		let (sql, values) = match conn.backend() {
			DatabaseBackend::Postgres => PostgresQueryBuilder.build_delete(&statement),
			DatabaseBackend::MySql => MySqlQueryBuilder.build_delete(&statement),
			DatabaseBackend::Sqlite => SqliteQueryBuilder.build_delete(&statement),
		};
		conn.execute(&sql, crate::orm::execution::convert_values(values))
			.await?;
		Ok(())
	}

	/// Returns whether a target primary key is related through the supplied executor.
	pub async fn contains_with_db<E, TPK>(&self, conn: &mut E, target_pk: TPK) -> Result<bool>
	where
		E: OrmExecutor,
		TPK: IntoValue,
	{
		let mut statement = Query::select();
		statement
			.from(Alias::new(&self.through_table))
			.expr(Expr::asterisk())
			.and_where(
				Expr::col(Alias::new(&self.source_field))
					.binary(BinOper::Equal, Expr::val(self.source_pk.clone())),
			)
			.and_where(
				Expr::col(Alias::new(&self.target_field))
					.binary(BinOper::Equal, Expr::val(target_pk)),
			);

		let (sql, values) = build_select_sql(&statement, conn.backend());
		Ok(!conn
			.fetch_all(&sql, crate::orm::execution::convert_values(values))
			.await?
			.is_empty())
	}

	/// Loads all matching target rows through the caller-owned executor.
	pub async fn all_with_db<E>(
		&self,
		conn: &mut E,
		target_table: &str,
		target_pk_field: &str,
	) -> Result<Vec<QueryRow>>
	where
		E: OrmExecutor,
	{
		let mut statement = Query::select();
		statement
			.from(Alias::new(&self.through_table))
			.inner_join(
				Alias::new(target_table),
				Expr::col((
					Alias::new(&self.through_table),
					Alias::new(&self.target_field),
				))
				.equals((Alias::new(target_table), Alias::new(target_pk_field))),
			)
			.expr(Expr::asterisk())
			.and_where(
				Expr::col((
					Alias::new(&self.through_table),
					Alias::new(&self.source_field),
				))
				.binary(BinOper::Equal, Expr::val(self.source_pk.clone())),
			);

		let (sql, values) = build_select_sql(&statement, conn.backend());
		Ok(conn
			.fetch_all(&sql, crate::orm::execution::convert_values(values))
			.await?
			.into_iter()
			.map(QueryRow::from_backend_row)
			.collect())
	}

	/// Removes every relationship for the source instance through the supplied executor.
	pub async fn clear_with_db<E>(&self, conn: &mut E) -> Result<()>
	where
		E: OrmExecutor,
	{
		let mut statement = Query::delete();
		statement
			.from_table(Alias::new(&self.through_table))
			.and_where(
				Expr::col(Alias::new(&self.source_field))
					.binary(BinOper::Equal, Expr::val(self.source_pk.clone())),
			);

		let (sql, values) = match conn.backend() {
			DatabaseBackend::Postgres => PostgresQueryBuilder.build_delete(&statement),
			DatabaseBackend::MySql => MySqlQueryBuilder.build_delete(&statement),
			DatabaseBackend::Sqlite => SqliteQueryBuilder.build_delete(&statement),
		};
		conn.execute(&sql, crate::orm::execution::convert_values(values))
			.await?;
		Ok(())
	}

	/// Counts relationships for the source instance through the supplied executor.
	pub async fn count_with_db<E>(&self, conn: &mut E) -> Result<usize>
	where
		E: OrmExecutor,
	{
		let mut statement = Query::select();
		statement
			.from(Alias::new(&self.through_table))
			.expr_as(
				Func::count(Expr::asterisk().into_simple_expr()),
				Alias::new("count"),
			)
			.and_where(
				Expr::col(Alias::new(&self.source_field))
					.binary(BinOper::Equal, Expr::val(self.source_pk.clone())),
			);

		let (sql, values) = build_select_sql(&statement, conn.backend());
		let row = QueryRow::from_backend_row(
			conn.fetch_one(&sql, crate::orm::execution::convert_values(values))
				.await?,
		);
		let count = row.get::<i64>("count").ok_or_else(|| {
			Error::from(DatabaseError::new(
				DatabaseErrorKind::Query,
				"Failed to extract count value from query result",
			))
		})?;

		Ok(count as usize)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn manager_creation_preserves_relationship_metadata() {
		let manager: ManyToManyManager<(), (), i64> = ManyToManyManager::new(
			42,
			"user_groups".to_string(),
			"user_id".to_string(),
			"group_id".to_string(),
		);

		assert_eq!(manager.through_table, "user_groups");
		assert_eq!(manager.source_field, "user_id");
		assert_eq!(manager.target_field, "group_id");
	}
}
