//! Django-style accessor for ManyToMany relationships.
//!
//! This module provides the ManyToManyAccessor type, which implements
//! Django-style API for managing many-to-many relationships:
//! - `add_with_conn()` - Add a relationship through a supplied executor
//! - `remove_with_conn()` - Remove a relationship through a supplied executor
//! - `all_with_conn()` - Get all related records through a supplied executor
//! - `clear_with_conn()` - Remove all relationships through a supplied executor
//! - `set_with_conn()` - Replace all relationships through a supplied executor

use super::Manager;
use super::connection::{DatabaseBackend, OrmExecutor, QueryRow};
use super::relationship::RelationshipType;
use crate::m2m_naming::{default_m2m_columns, default_through_table};
use crate::orm::Model;
use reinhardt_query::prelude::{
	Alias, BinOper, ColumnRef, DeleteStatement, Expr, Func, InsertStatement, MySqlQueryBuilder,
	PostgresQueryBuilder, Query, QueryBuilder, SelectStatement, SqliteQueryBuilder, Values,
};
use serde::{Serialize, de::DeserializeOwned};
use std::marker::PhantomData;
use std::time::Instant;

/// Build SELECT SQL using the appropriate QueryBuilder for the given backend.
fn build_select_sql(stmt: &SelectStatement, backend: DatabaseBackend) -> (String, Values) {
	match backend {
		DatabaseBackend::Postgres => PostgresQueryBuilder.build_select(stmt),
		DatabaseBackend::MySql => MySqlQueryBuilder.build_select(stmt),
		DatabaseBackend::Sqlite => SqliteQueryBuilder.build_select(stmt),
	}
}

fn value_samples(values: &Values) -> Vec<String> {
	values.iter().map(|value| value.to_sql_literal()).collect()
}

/// Build INSERT SQL using the appropriate QueryBuilder for the given backend.
fn build_insert_sql(stmt: &InsertStatement, backend: DatabaseBackend) -> (String, Values) {
	match backend {
		DatabaseBackend::Postgres => PostgresQueryBuilder.build_insert(stmt),
		DatabaseBackend::MySql => MySqlQueryBuilder.build_insert(stmt),
		DatabaseBackend::Sqlite => SqliteQueryBuilder.build_insert(stmt),
	}
}

/// Build DELETE SQL using the appropriate QueryBuilder for the given backend.
fn build_delete_sql(stmt: &DeleteStatement, backend: DatabaseBackend) -> (String, Values) {
	match backend {
		DatabaseBackend::Postgres => PostgresQueryBuilder.build_delete(stmt),
		DatabaseBackend::MySql => MySqlQueryBuilder.build_delete(stmt),
		DatabaseBackend::Sqlite => SqliteQueryBuilder.build_delete(stmt),
	}
}

/// Django-style accessor for ManyToMany relationships.
///
/// This type provides methods to manage many-to-many relationships
/// using an intermediate/through table.
///
/// # Type Parameters
///
/// - `S`: Source model type (the model that owns the ManyToMany field)
/// - `T`: Target model type (the related model)
///
/// # Examples
///
/// ```rust,ignore
/// # #[tokio::main]
/// # async fn main() {
/// use reinhardt_db::orm::{Model, ManyToManyAccessor};
///
/// let user = User::find_by_id(&mut db, user_id).await?;
/// let accessor = ManyToManyAccessor::new(&user, "groups");
///
/// // Add a relationship
/// accessor.add_with_conn(&mut db, &group).await?;
///
/// // Get all related records
/// let groups = accessor.all_with_conn(&mut db).await?;
///
/// // Remove a relationship
/// accessor.remove_with_conn(&mut db, &group).await?;
///
/// // Clear all relationships
/// accessor.clear_with_conn(&mut db).await?;
///
/// # }
/// ```
pub struct ManyToManyAccessor<S, T>
where
	S: Model,
	S::PrimaryKey: reinhardt_query::IntoValue,
	T: Model + Serialize + DeserializeOwned,
	T::PrimaryKey: reinhardt_query::IntoValue,
{
	source_id: S::PrimaryKey,
	through_table: String,
	source_field: String,
	target_field: String,
	limit: Option<usize>,
	offset: Option<usize>,
	_phantom_source: PhantomData<S>,
	_phantom_target: PhantomData<T>,
}

impl<S, T> ManyToManyAccessor<S, T>
where
	S: Model,
	S::PrimaryKey: reinhardt_query::IntoValue,
	T: Model + Serialize + DeserializeOwned,
	T::PrimaryKey: reinhardt_query::IntoValue,
{
	/// Create a new ManyToManyAccessor.
	///
	/// # Parameters
	///
	/// - `source`: The source model instance
	/// - `field_name`: The name of the ManyToMany field
	///
	/// # Panics
	///
	/// Panics if:
	/// - The field_name does not correspond to a ManyToMany field
	/// - The source model has no primary key
	pub fn new(source: &S, field_name: &str) -> Self {
		// Try to get through table info from model metadata
		let rel_info = S::relationship_metadata()
			.into_iter()
			.find(|r| r.name == field_name && r.relationship_type == RelationshipType::ManyToMany);

		// Get through table name and FK column names from metadata, falling
		// back to the canonical convention defined in `crate::m2m_naming`
		// (single source of truth shared with the migration autodetector;
		// see issues #4659 and #4665). `default_m2m_columns` applies the
		// `from_/to_` prefix only for self-referential M2M, matching what
		// `MigrationAutodetector::create_intermediate_table_for_m2m` emits.
		let through_table = rel_info
			.as_ref()
			.and_then(|r| r.through_table.clone())
			.unwrap_or_else(|| default_through_table(S::table_name(), field_name));

		let source_id = source
			.primary_key()
			.expect("Source model must have primary key")
			.clone();

		let (default_source_field, default_target_field) =
			default_m2m_columns(S::table_name(), T::table_name());
		let source_field = rel_info
			.as_ref()
			.and_then(|r| r.source_field.clone())
			.unwrap_or(default_source_field);

		let target_field = rel_info
			.as_ref()
			.and_then(|r| r.target_field.clone())
			.unwrap_or(default_target_field);

		Self {
			source_id,
			through_table,
			source_field,
			target_field,
			limit: None,
			offset: None,
			_phantom_source: PhantomData,
			_phantom_target: PhantomData,
		}
	}

	/// Add a relationship to the target model.
	///
	/// Creates a record in the intermediate table linking the source and target.
	///
	/// # Parameters
	///
	/// - `conn`: Caller-owned ORM executor
	/// - `target`: The target model to add
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The target model has no primary key
	/// - The database operation fails
	///
	/// # Examples
	///
	/// ```ignore
	/// accessor.add_with_conn(&mut db, &group).await?;
	/// ```
	pub async fn add_with_conn<E>(
		&self,
		conn: &mut E,
		target: &T,
	) -> reinhardt_core::exception::Result<()>
	where
		E: OrmExecutor,
	{
		let target_id = target.primary_key().ok_or_else(|| {
			reinhardt_core::exception::Error::from(reinhardt_core::exception::DatabaseError::new(
				reinhardt_core::exception::DatabaseErrorKind::Query,
				"Target model has no primary key",
			))
		})?;

		let query = Query::insert()
			.into_table(Alias::new(&self.through_table))
			.columns([
				Alias::new(&self.source_field),
				Alias::new(&self.target_field),
			])
			.values_panic([
				Expr::val(self.source_id.clone()),
				Expr::val(target_id.clone()),
			])
			.to_owned();

		let (sql, values) = build_insert_sql(&query, conn.backend());
		conn.execute(&sql, super::execution::convert_values(values))
			.await?;

		Ok(())
	}

	/// Remove a relationship to the target model.
	///
	/// Deletes the record in the intermediate table linking the source and target.
	///
	/// # Parameters
	///
	/// - `conn`: Caller-owned ORM executor
	/// - `target`: The target model to remove
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The target model has no primary key
	/// - The database operation fails
	///
	/// # Examples
	///
	/// ```ignore
	/// accessor.remove_with_conn(&mut db, &group).await?;
	/// ```
	pub async fn remove_with_conn<E>(
		&self,
		conn: &mut E,
		target: &T,
	) -> reinhardt_core::exception::Result<()>
	where
		E: OrmExecutor,
	{
		let target_id = target.primary_key().ok_or_else(|| {
			reinhardt_core::exception::Error::from(reinhardt_core::exception::DatabaseError::new(
				reinhardt_core::exception::DatabaseErrorKind::Query,
				"Target model has no primary key",
			))
		})?;

		let query = Query::delete()
			.from_table(Alias::new(&self.through_table))
			.and_where(
				Expr::col(Alias::new(&self.source_field))
					.binary(BinOper::Equal, Expr::val(self.source_id.clone())),
			)
			.and_where(
				Expr::col(Alias::new(&self.target_field))
					.binary(BinOper::Equal, Expr::val(target_id.clone())),
			)
			.to_owned();

		let (sql, values) = build_delete_sql(&query, conn.backend());
		conn.execute(&sql, super::execution::convert_values(values))
			.await?;

		Ok(())
	}

	/// Returns whether the target is related through the caller-owned executor.
	pub async fn contains_with_conn<E>(
		&self,
		conn: &mut E,
		target: &T,
	) -> reinhardt_core::exception::Result<bool>
	where
		E: OrmExecutor,
	{
		let target_id = target.primary_key().ok_or_else(|| {
			reinhardt_core::exception::Error::from(reinhardt_core::exception::DatabaseError::new(
				reinhardt_core::exception::DatabaseErrorKind::Query,
				"Target model has no primary key",
			))
		})?;
		let query = Query::select()
			.from(Alias::new(&self.through_table))
			.expr(Expr::asterisk())
			.and_where(
				Expr::col(Alias::new(&self.source_field))
					.binary(BinOper::Equal, Expr::val(self.source_id.clone())),
			)
			.and_where(
				Expr::col(Alias::new(&self.target_field))
					.binary(BinOper::Equal, Expr::val(target_id.clone())),
			)
			.to_owned();
		let (sql, values) = build_select_sql(&query, conn.backend());
		Ok(!conn
			.fetch_all(&sql, super::execution::convert_values(values))
			.await?
			.is_empty())
	}

	/// Set LIMIT clause
	///
	/// Limits the number of records returned by the query.
	///
	/// # Examples
	///
	/// ```ignore
	/// let followers = accessor.limit(10).all_with_conn(&mut db).await?;
	/// ```
	pub fn limit(mut self, limit: usize) -> Self {
		self.limit = Some(limit);
		self
	}

	/// Set OFFSET clause
	///
	/// Skips the specified number of records before returning results.
	///
	/// # Examples
	///
	/// ```ignore
	/// let followers = accessor.offset(20).limit(10).all_with_conn(&mut db).await?;
	/// ```
	pub fn offset(mut self, offset: usize) -> Self {
		self.offset = Some(offset);
		self
	}

	/// Paginate results using page number and page size
	///
	/// Convenience method that calculates offset automatically.
	///
	/// # Examples
	///
	/// ```ignore
	/// // Page 3, 10 items per page (offset=20, limit=10)
	/// let followers = accessor.paginate(3, 10).all_with_conn(&mut db).await?;
	/// ```
	pub fn paginate(self, page: usize, page_size: usize) -> Self {
		let offset = page.saturating_sub(1) * page_size;
		self.offset(offset).limit(page_size)
	}

	/// Count total number of related items
	///
	/// Executes a COUNT(*) query to get the total number of related records
	/// without fetching them.
	///
	/// # Errors
	///
	/// Returns an error if the database operation fails.
	///
	/// # Examples
	///
	/// ```ignore
	/// let total_followers = accessor.count_with_conn(&mut db).await?;
	/// ```
	pub async fn count_with_conn<E>(&self, conn: &mut E) -> reinhardt_core::exception::Result<usize>
	where
		E: OrmExecutor,
	{
		let mut query = Query::select();
		query
			.from(Alias::new(&self.through_table))
			.expr_as(
				Func::count(Expr::asterisk().into_simple_expr()),
				Alias::new("count"),
			)
			.and_where(
				Expr::col(Alias::new(&self.source_field))
					.binary(BinOper::Equal, Expr::val(self.source_id.clone())),
			);

		let query = query.to_owned();
		let (sql, values) = build_select_sql(&query, conn.backend());
		let params = value_samples(&values);
		let query_values = super::execution::convert_values(values);
		let started_at = Instant::now();
		let query_result = conn.fetch_all(&sql, query_values).await;
		let duration = started_at.elapsed();
		let rows = match query_result {
			Ok(rows) => {
				super::instrumentation::instrumentation()
					.orm_query_end_with_params(&sql, &params, duration)
					.await;
				rows
			}
			Err(error) => {
				super::instrumentation::instrumentation()
					.orm_query_error(&sql, &error.to_string())
					.await;
				return Err(error);
			}
		};

		if let Some(row) = rows.into_iter().next().map(QueryRow::from_backend_row)
			&& let Some(count) = row.get::<i64>("count")
		{
			return Ok(count as usize);
		}

		Ok(0)
	}

	/// Get all related target models.
	///
	/// Queries the target table joined with the intermediate table to fetch all
	/// related records.
	///
	/// # Errors
	///
	/// Returns an error if the database operation fails.
	///
	/// # Examples
	///
	/// ```ignore
	/// let groups = accessor.all_with_conn(&mut db).await?;
	/// ```
	pub async fn all_with_conn<E>(&self, conn: &mut E) -> reinhardt_core::exception::Result<Vec<T>>
	where
		E: OrmExecutor,
	{
		let mut query = Query::select();
		query.from(Alias::new(T::table_name()));

		// Use explicit column selection instead of SELECT * to avoid conflicts
		// with intermediate table columns in JOIN queries.
		// When JOIN is used with SELECT *, all columns from both tables are returned,
		// which can cause type conflicts (e.g., intermediate table's INTEGER id vs
		// target table's UUID id).
		let field_metadata = T::field_metadata();
		if field_metadata.is_empty() {
			// Fallback: if no field metadata is available, select all from target table only
			query.column(ColumnRef::table_asterisk(Alias::new(T::table_name())));
		} else {
			// Explicitly select only target table columns
			for field in field_metadata {
				query.column((
					Alias::new(T::table_name()),
					Alias::new(field.db_column_name()),
				));
			}
		}

		query
			.inner_join(
				Alias::new(&self.through_table),
				Expr::col((
					Alias::new(T::table_name()),
					Alias::new(T::primary_key_column()),
				))
				.equals((
					Alias::new(&self.through_table),
					Alias::new(&self.target_field),
				)),
			)
			.and_where(
				Expr::col((
					Alias::new(&self.through_table),
					Alias::new(&self.source_field),
				))
				.binary(BinOper::Equal, Expr::val(self.source_id.clone())),
			);

		// Apply LIMIT/OFFSET
		if let Some(limit) = self.limit {
			query.limit(limit as u64);
		}
		if let Some(offset) = self.offset {
			query.offset(offset as u64);
		}

		let query = query.to_owned();
		let (sql, values) = build_select_sql(&query, conn.backend());
		let params = value_samples(&values);
		let query_values = super::execution::convert_values(values);
		let started_at = Instant::now();
		let query_result = conn.fetch_all(&sql, query_values).await;
		let duration = started_at.elapsed();
		let rows = match query_result {
			Ok(rows) => {
				super::instrumentation::instrumentation()
					.orm_query_end_with_params(&sql, &params, duration)
					.await;
				rows
			}
			Err(error) => {
				super::instrumentation::instrumentation()
					.orm_query_error(&sql, &error.to_string())
					.await;
				return Err(error);
			}
		};

		rows.into_iter()
			.map(QueryRow::from_backend_row)
			.map(|row| {
				row.deserialize_model::<T>().map_err(|error| {
					reinhardt_core::exception::Error::from(
						reinhardt_core::exception::DatabaseError::new(
							reinhardt_core::exception::DatabaseErrorKind::Serialization,
							error.to_string(),
						),
					)
				})
			})
			.collect()
	}

	/// Remove all relationships.
	///
	/// Deletes all records in the intermediate table for this source instance.
	///
	/// # Errors
	///
	/// Returns an error if the database operation fails.
	///
	/// # Examples
	///
	/// ```ignore
	/// accessor.clear_with_conn(&mut db).await?;
	/// ```
	pub async fn clear_with_conn<E>(&self, conn: &mut E) -> reinhardt_core::exception::Result<()>
	where
		E: OrmExecutor,
	{
		let query = Query::delete()
			.from_table(Alias::new(&self.through_table))
			.and_where(
				Expr::col(Alias::new(&self.source_field))
					.binary(BinOper::Equal, Expr::val(self.source_id.clone())),
			)
			.to_owned();

		let (sql, values) = build_delete_sql(&query, conn.backend());
		conn.execute(&sql, super::execution::convert_values(values))
			.await?;

		Ok(())
	}

	/// Replace all relationships with a new set.
	///
	/// The caller controls atomicity. Pass an [`AtomicTransaction`](super::AtomicTransaction)
	/// when clearing and adding must be committed or rolled back together.
	///
	/// # Parameters
	///
	/// - `targets`: The new set of target models
	///
	/// # Errors
	///
	/// Returns an error if the database operation fails.
	///
	/// # Examples
	///
	/// ```ignore
	/// accessor.set_with_conn(&mut transaction, &[group1, group2, group3]).await?;
	/// ```
	pub async fn set_with_conn<E>(
		&self,
		conn: &mut E,
		targets: &[T],
	) -> reinhardt_core::exception::Result<()>
	where
		E: OrmExecutor,
	{
		self.clear_with_conn(conn).await?;
		for target in targets {
			self.add_with_conn(conn, target).await?;
		}
		Ok(())
	}

	/// Filter source models by target model via many-to-many relationship
	///
	/// Returns all source model instances that have a relationship with the given target.
	/// This is more efficient than loading all source instances and checking relationships
	/// individually, as it uses a single JOIN query.
	///
	/// # Type Parameters
	///
	/// - `S`: Source model type (the model that owns the ManyToMany field)
	/// - `T`: Target model type (the related model)
	///
	/// # Arguments
	///
	/// - `source_manager`: Manager for the source model
	/// - `field_name`: Name of the ManyToMany field on the source model
	/// - `target`: The target model instance to filter by
	/// - `conn`: Caller-owned ORM executor
	///
	/// # Returns
	///
	/// All source model instances related to the target
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The target model has no primary key
	/// - The database operation fails
	/// - The query results cannot be deserialized
	///
	/// # Examples
	///
	/// ```ignore
	/// // Find all rooms where a specific user is a member
	/// let user = User::find_by_id(&mut db, user_id).await?;
	/// let rooms = ManyToManyAccessor::<DMRoom, User>::filter_by_target_with_conn(
	///     &DMRoom::objects(),
	///     "members",
	///     &user,
	///     &mut db
	/// ).await?;
	/// ```
	///
	/// SQL equivalent:
	/// ```sql
	/// SELECT source_table.*
	/// FROM source_table
	/// INNER JOIN through_table ON source_table.id = through_table.source_id
	/// WHERE through_table.target_id = $1
	/// ```
	pub async fn filter_by_target_with_conn<E>(
		_source_manager: &Manager<S>,
		field_name: &str,
		target: &T,
		conn: &mut E,
	) -> reinhardt_core::exception::Result<Vec<S>>
	where
		E: OrmExecutor,
	{
		let target_id = target.primary_key().ok_or_else(|| {
			reinhardt_core::exception::Error::from(reinhardt_core::exception::DatabaseError::new(
				reinhardt_core::exception::DatabaseErrorKind::Query,
				"Target model has no primary key",
			))
		})?;

		// Resolve through-table and FK column names through the same
		// metadata-aware path as `new()`, routing the fallbacks through
		// `crate::m2m_naming` (single source of truth shared with the
		// migration autodetector; see issues #4659, #4665). The helpers
		// apply `from_/to_` prefixes for self-referential M2M, matching
		// `MigrationAutodetector::create_intermediate_table_for_m2m`.
		let rel_info = S::relationship_metadata()
			.into_iter()
			.find(|r| r.name == field_name && r.relationship_type == RelationshipType::ManyToMany);

		let through_table = rel_info
			.as_ref()
			.and_then(|r| r.through_table.clone())
			.unwrap_or_else(|| default_through_table(S::table_name(), field_name));

		let (default_source_field, default_target_field) =
			default_m2m_columns(S::table_name(), T::table_name());
		let source_field = rel_info
			.as_ref()
			.and_then(|r| r.source_field.clone())
			.unwrap_or(default_source_field);
		let target_field = rel_info
			.as_ref()
			.and_then(|r| r.target_field.clone())
			.unwrap_or(default_target_field);

		// Build JOIN query using reinhardt-query
		let mut query = Query::select();
		query.from(Alias::new(S::table_name()));

		// Use explicit column selection instead of SELECT * to avoid conflicts
		// with intermediate table columns in JOIN queries.
		let field_metadata = S::field_metadata();
		if field_metadata.is_empty() {
			query.column(ColumnRef::table_asterisk(Alias::new(S::table_name())));
		} else {
			for field in field_metadata {
				query.column((
					Alias::new(S::table_name()),
					Alias::new(field.db_column_name()),
				));
			}
		}

		let query = query
			.inner_join(
				Alias::new(&through_table),
				Expr::col((
					Alias::new(S::table_name()),
					Alias::new(S::primary_key_column()),
				))
				.equals((Alias::new(&through_table), Alias::new(&source_field))),
			)
			.and_where(
				Expr::col((Alias::new(&through_table), Alias::new(&target_field)))
					.binary(BinOper::Equal, Expr::val(target_id.clone())),
			)
			.to_owned();

		let (sql, values) = build_select_sql(&query, conn.backend());
		let params = value_samples(&values);
		let query_values = super::execution::convert_values(values);
		let started_at = Instant::now();
		let query_result = conn.fetch_all(&sql, query_values).await;
		let duration = started_at.elapsed();
		let rows = match query_result {
			Ok(rows) => {
				super::instrumentation::instrumentation()
					.orm_query_end_with_params(&sql, &params, duration)
					.await;
				rows
			}
			Err(error) => {
				super::instrumentation::instrumentation()
					.orm_query_error(&sql, &error.to_string())
					.await;
				return Err(error);
			}
		};

		rows.into_iter()
			.map(QueryRow::from_backend_row)
			.map(|row| {
				row.deserialize_model::<S>().map_err(|error| {
					reinhardt_core::exception::Error::from(
						reinhardt_core::exception::DatabaseError::new(
							reinhardt_core::exception::DatabaseErrorKind::Serialization,
							error.to_string(),
						),
					)
				})
			})
			.collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_query::prelude::QueryStatementBuilder;

	/// Regression test for #4659: the runtime accessor's default
	/// through-table name MUST agree with the name `makemigrations`
	/// synthesizes. The autodetector uses
	/// `format!("{}_{}", source_model.table_name, field_name)`; the
	/// accessor must do the same. Previously it prepended `S::app_label()`
	/// as a separate segment, producing names like `auth_users_groups`
	/// instead of `users_groups` (or `dm_dm_room_members` instead of
	/// `dm_room_members`), so runtime M2M queries targeted a table that
	/// `makemigrations` never created.
	#[test]
	fn default_through_table_matches_autodetector_convention() {
		// Arrange / Act: TestUser has table_name = "users". The accessor now
		// routes through `crate::m2m_naming::default_through_table`, so this
		// regression test exercises the same helper the autodetector uses.
		let through = default_through_table(TestUser::table_name(), "members");

		// Assert
		assert_eq!(through, "users_members");
		assert!(
			!through.starts_with("auth_"),
			"app_label must NOT be prepended; that would double-count it \
			 when table_name already carries the prefix (e.g. \"dm_room\"). \
			 See #4659 for the breakage this causes."
		);
	}

	#[test]
	fn test_sql_generation_add() {
		// Test that INSERT SQL is generated correctly
		let query = Query::insert()
			.into_table(Alias::new("auth_users_groups"))
			.columns([Alias::new("users_id"), Alias::new("groups_id")])
			.values_panic([Expr::val("1"), Expr::val("10")])
			.to_owned();

		let (sql, _) = query.build(PostgresQueryBuilder);
		assert!(sql.contains("INSERT INTO"));
		assert!(sql.contains("auth_users_groups"));
		assert!(sql.contains("users_id"));
		assert!(sql.contains("groups_id"));
	}

	#[test]
	fn test_sql_generation_remove() {
		// Test that DELETE SQL is generated correctly
		let query = Query::delete()
			.from_table(Alias::new("auth_users_groups"))
			.and_where(Expr::col(Alias::new("users_id")).binary(BinOper::Equal, Expr::val("1")))
			.and_where(Expr::col(Alias::new("groups_id")).binary(BinOper::Equal, Expr::val("10")))
			.to_owned();

		let (sql, _) = query.build(PostgresQueryBuilder);
		assert!(sql.contains("DELETE FROM"));
		assert!(sql.contains("auth_users_groups"));
		assert!(sql.contains("users_id"));
		assert!(sql.contains("groups_id"));
	}

	#[test]
	fn test_sql_generation_clear() {
		// Test that DELETE SQL for clear is generated correctly
		let query = Query::delete()
			.from_table(Alias::new("auth_users_groups"))
			.and_where(Expr::col(Alias::new("users_id")).binary(BinOper::Equal, Expr::val("1")))
			.to_owned();

		let (sql, _) = query.build(PostgresQueryBuilder);
		assert!(sql.contains("DELETE FROM"));
		assert!(sql.contains("auth_users_groups"));
		assert!(sql.contains("users_id"));
	}

	#[test]
	fn test_sql_generation_all() {
		// Test that SELECT SQL with JOIN is generated correctly
		let query = Query::select()
			.from(Alias::new("groups"))
			.column((Alias::new("groups"), Alias::new("*")))
			.inner_join(
				Alias::new("auth_users_groups"),
				Expr::col((Alias::new("groups"), Alias::new("id")))
					.equals((Alias::new("auth_users_groups"), Alias::new("groups_id"))),
			)
			.and_where(
				Expr::col((Alias::new("auth_users_groups"), Alias::new("users_id")))
					.binary(BinOper::Equal, Expr::val("1")),
			)
			.to_owned();

		let (sql, _) = query.build(PostgresQueryBuilder);
		assert!(sql.contains("SELECT"));
		assert!(sql.contains("INNER JOIN"));
		assert!(sql.contains("auth_users_groups"));
	}

	#[test]
	fn test_sql_generation_filter_by_target() {
		// Test that SELECT SQL with JOIN for filter_by_target is generated correctly
		let query = Query::select()
			.from(Alias::new("dm_room"))
			.column((Alias::new("dm_room"), Alias::new("*")))
			.inner_join(
				Alias::new("dm_room_members"),
				Expr::col((Alias::new("dm_room"), Alias::new("id")))
					.equals((Alias::new("dm_room_members"), Alias::new("dmroom_id"))),
			)
			.and_where(
				Expr::col((Alias::new("dm_room_members"), Alias::new("user_id")))
					.binary(BinOper::Equal, Expr::val("test-user-id")),
			)
			.to_owned();

		let (sql, _) = query.build(PostgresQueryBuilder);
		assert!(sql.contains("SELECT"));
		assert!(sql.contains("dm_room"));
		assert!(sql.contains("INNER JOIN"));
		assert!(sql.contains("dm_room_members"));
		assert!(sql.contains("user_id"));
		// Note: reinhardt-query uses parameterized queries, so the value may be in a parameter
		// instead of inline in the SQL string
	}

	// Test models for SQL generation tests
	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	struct TestUser {
		id: i64,
		username: String,
	}

	#[derive(Debug, Clone)]
	struct TestUserFields;

	impl crate::orm::model::FieldSelector for TestUserFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for TestUser {
		type PrimaryKey = i64;
		type Fields = TestUserFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"users"
		}

		fn app_label() -> &'static str {
			"auth"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			Some(self.id)
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = value;
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn new_fields() -> Self::Fields {
			TestUserFields
		}
	}

	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	struct TestGroup {
		id: i64,
		name: String,
	}

	#[derive(Clone)]
	struct TestGroupFields;
	impl crate::orm::model::FieldSelector for TestGroupFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for TestGroup {
		type PrimaryKey = i64;
		type Fields = TestGroupFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"groups"
		}

		fn app_label() -> &'static str {
			"auth"
		}

		fn new_fields() -> Self::Fields {
			TestGroupFields
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			Some(self.id)
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = value;
		}
	}
}
