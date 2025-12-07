//! Django-style accessor for ManyToMany relationships.
//!
//! This module provides the ManyToManyAccessor type, which implements
//! Django-style API for managing many-to-many relationships:
//! - `add()` - Add a relationship
//! - `remove()` - Remove a relationship
//! - `all()` - Get all related records
//! - `clear()` - Remove all relationships
//! - `set()` - Replace all relationships

use crate::connection::DatabaseConnection;
use crate::Model;
use sea_query::{Alias, BinOper, Expr, ExprTrait, PostgresQueryBuilder, Query};
use serde::{Serialize, de::DeserializeOwned};
use std::marker::PhantomData;

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
/// ```ignore
/// use reinhardt_orm::{Model, ManyToManyAccessor};
///
/// let user = User::find_by_id(&db, user_id).await?;
/// let accessor = ManyToManyAccessor::new(&user, "groups", db.clone());
///
/// // Add a relationship
/// accessor.add(&group).await?;
///
/// // Get all related records
/// let groups = accessor.all().await?;
///
/// // Remove a relationship
/// accessor.remove(&group).await?;
///
/// // Clear all relationships
/// accessor.clear().await?;
/// ```
pub struct ManyToManyAccessor<S, T>
where
	S: Model,
	T: Model + Serialize + DeserializeOwned,
{
	source_id: S::PrimaryKey,
	through_table: String,
	source_field: String,
	target_field: String,
	db: DatabaseConnection,
	_phantom_source: PhantomData<S>,
	_phantom_target: PhantomData<T>,
}

impl<S, T> ManyToManyAccessor<S, T>
where
	S: Model,
	T: Model + Serialize + DeserializeOwned,
{
	/// Create a new ManyToManyAccessor.
	///
	/// # Parameters
	///
	/// - `source`: The source model instance
	/// - `field_name`: The name of the ManyToMany field
	/// - `db`: Database connection
	///
	/// # Panics
	///
	/// Panics if:
	/// - The field_name does not correspond to a ManyToMany field
	/// - The source model has no primary key
	pub fn new(source: &S, field_name: &str, db: DatabaseConnection) -> Self {
		// Get through table name from metadata
		// For now, use Django naming convention: {app}_{model}_{field}
		let through_table = format!(
			"{}_{}_{}",
			S::app_label(),
			Self::table_name_lower(S::table_name()),
			field_name
		);

		let source_id = source
			.primary_key()
			.expect("Source model must have primary key")
			.clone();

		let source_field = format!("{}_id", Self::table_name_lower(S::table_name()));
		let target_field = format!("{}_id", Self::table_name_lower(T::table_name()));

		Self {
			source_id,
			through_table,
			source_field,
			target_field,
			db,
			_phantom_source: PhantomData,
			_phantom_target: PhantomData,
		}
	}

	/// Convert table name to lowercase for field naming.
	fn table_name_lower(table_name: &str) -> String {
		table_name.to_lowercase()
	}

	/// Add a relationship to the target model.
	///
	/// Creates a record in the intermediate table linking the source and target.
	///
	/// # Parameters
	///
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
	/// accessor.add(&group).await?;
	/// ```
	pub async fn add(&self, target: &T) -> Result<(), String> {
		let target_id = target
			.primary_key()
			.ok_or_else(|| "Target model has no primary key".to_string())?;

		let query = Query::insert()
			.into_table(Alias::new(&self.through_table))
			.columns([
				Alias::new(&self.source_field),
				Alias::new(&self.target_field),
			])
			.values_panic([
				Expr::val(self.source_id.to_string()),
				Expr::val(target_id.to_string()),
			])
			.to_owned();

		let (sql, _values) = query.build(PostgresQueryBuilder);

		self.db
			.execute(&sql, vec![])
			.await
			.map_err(|e| e.to_string())?;

		Ok(())
	}

	/// Remove a relationship to the target model.
	///
	/// Deletes the record in the intermediate table linking the source and target.
	///
	/// # Parameters
	///
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
	/// accessor.remove(&group).await?;
	/// ```
	pub async fn remove(&self, target: &T) -> Result<(), String> {
		let target_id = target
			.primary_key()
			.ok_or_else(|| "Target model has no primary key".to_string())?;

		let query = Query::delete()
			.from_table(Alias::new(&self.through_table))
			.and_where(
				Expr::col(Alias::new(&self.source_field))
					.binary(BinOper::Equal, Expr::val(self.source_id.to_string())),
			)
			.and_where(
				Expr::col(Alias::new(&self.target_field))
					.binary(BinOper::Equal, Expr::val(target_id.to_string())),
			)
			.to_owned();

		let (sql, _values) = query.build(PostgresQueryBuilder);

		self.db
			.execute(&sql, vec![])
			.await
			.map_err(|e| e.to_string())?;

		Ok(())
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
	/// let groups = accessor.all().await?;
	/// ```
	pub async fn all(&self) -> Result<Vec<T>, String> {
		let query = Query::select()
			.from(Alias::new(T::table_name()))
			.column((Alias::new(T::table_name()), Alias::new("*")))
			.inner_join(
				Alias::new(&self.through_table),
				Expr::col((Alias::new(T::table_name()), Alias::new("id"))).equals((
					Alias::new(&self.through_table),
					Alias::new(&self.target_field),
				)),
			)
			.and_where(
				Expr::col((
					Alias::new(&self.through_table),
					Alias::new(&self.source_field),
				))
				.binary(BinOper::Equal, Expr::val(self.source_id.to_string())),
			)
			.to_owned();

		let (sql, _values) = query.build(PostgresQueryBuilder);

		let rows = self
			.db
			.query(&sql, vec![])
			.await
			.map_err(|e| e.to_string())?;

		rows.into_iter()
			.map(|row| serde_json::from_value(row.data).map_err(|e| e.to_string()))
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
	/// accessor.clear().await?;
	/// ```
	pub async fn clear(&self) -> Result<(), String> {
		let query = Query::delete()
			.from_table(Alias::new(&self.through_table))
			.and_where(
				Expr::col(Alias::new(&self.source_field))
					.binary(BinOper::Equal, Expr::val(self.source_id.to_string())),
			)
			.to_owned();

		let (sql, _values) = query.build(PostgresQueryBuilder);

		self.db
			.execute(&sql, vec![])
			.await
			.map_err(|e| e.to_string())?;

		Ok(())
	}

	/// Replace all relationships with a new set.
	///
	/// This is a transactional operation that:
	/// 1. Removes all existing relationships
	/// 2. Adds new relationships
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
	/// accessor.set(&[group1, group2, group3]).await?;
	/// ```
	pub async fn set(&self, targets: &[T]) -> Result<(), String> {
		// Use transaction for atomicity
		let mut tx = self.db.begin().await.map_err(|e| e.to_string())?;

		// Build and execute clear query within transaction
		let clear_query = Query::delete()
			.from_table(Alias::new(&self.through_table))
			.and_where(
				Expr::col(Alias::new(&self.source_field))
					.binary(BinOper::Equal, Expr::val(self.source_id.to_string())),
			)
			.to_owned();
		let (clear_sql, _) = clear_query.build(PostgresQueryBuilder);
		tx.execute(&clear_sql, vec![])
			.await
			.map_err(|e| e.to_string())?;

		// Add new relationships within transaction
		for target in targets {
			let target_id = target
				.primary_key()
				.ok_or_else(|| "Target model has no primary key".to_string())?;

			let insert_query = Query::insert()
				.into_table(Alias::new(&self.through_table))
				.columns([
					Alias::new(&self.source_field),
					Alias::new(&self.target_field),
				])
				.values_panic([
					Expr::val(self.source_id.to_string()),
					Expr::val(target_id.to_string()),
				])
				.to_owned();

			let (insert_sql, _) = insert_query.build(PostgresQueryBuilder);
			tx.execute(&insert_sql, vec![])
				.await
				.map_err(|e| e.to_string())?;
		}

		// Commit transaction
		tx.commit().await.map_err(|e| e.to_string())?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_table_name_lower() {
		assert_eq!(
			ManyToManyAccessor::<TestUser, TestGroup>::table_name_lower("Users"),
			"users"
		);
		assert_eq!(
			ManyToManyAccessor::<TestUser, TestGroup>::table_name_lower("UserGroups"),
			"usergroups"
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

	// Test models for SQL generation tests
	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	struct TestUser {
		id: i64,
		username: String,
	}

	impl Model for TestUser {
		type PrimaryKey = i64;

		fn table_name() -> &'static str {
			"users"
		}

		fn app_label() -> &'static str {
			"auth"
		}

		fn primary_key(&self) -> Option<&Self::PrimaryKey> {
			Some(&self.id)
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = value;
		}
	}

	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	struct TestGroup {
		id: i64,
		name: String,
	}

	impl Model for TestGroup {
		type PrimaryKey = i64;

		fn table_name() -> &'static str {
			"groups"
		}

		fn app_label() -> &'static str {
			"auth"
		}

		fn primary_key(&self) -> Option<&Self::PrimaryKey> {
			Some(&self.id)
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = value;
		}
	}
}
