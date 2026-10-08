// Copyright 2024-2025 the reinhardt-db authors
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at
//
// Unless required by applicable law or agreed to in writing, software distributed under
// the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. See the License for the specific language governing
// permissions and limitations under the License.

//! ORM Session - SQLAlchemy-style database session with identity map and unit of work pattern
//!
//! This module provides a Session object that manages database operations with automatic
//! object tracking, identity mapping, and unit-of-work persistence.
//!
//! Session DML uses checked typed statements and the shared SQLx Any text
//! compatibility codecs. Identifier quoting, primary-key values and projection
//! conversions remain structural until rendering. The final SQL/Values pair is
//! adapted without SQL text rewriting or manual bind loops. Session owns its
//! pool, row decoding and the existing flush/row-lock transaction boundaries.

use crate::backends::types::{DatabaseType, RowLockCapabilities};
use crate::orm::FieldCodecError;
use crate::orm::field_codec::database_value_to_query_value;
use crate::orm::inspection::FieldInfo;
use crate::orm::model::Model;
use crate::orm::query::{OrmQuery, QuerySet};
use crate::orm::query_types::DbBackend;
#[cfg(test)]
use reinhardt_query::QueryStatementBuilder;
use reinhardt_query::value::Value as RValue;
use reinhardt_query::{
	Alias, ColumnRef, Expr, ExprTrait, IntoIden, LockType, MySqlQueryBuilder, PostgresQueryBuilder,
	Query as RQuery, SelectStatement, SimpleExpr, SqliteQueryBuilder,
};
use serde_json::Value;
use sqlx::{AnyPool, Row};
use std::any::TypeId;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
#[cfg(test)]
use uuid::Uuid;

/// Session error types
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum SessionError {
	/// Database error occurred
	DatabaseError(String),
	/// Object not found in session
	ObjectNotFound(String),
	/// Serialization/deserialization error
	SerializationError(String),
	/// Model database field codec error
	FieldCodec(FieldCodecError),
	/// Invalid state
	InvalidState(String),
	/// Flush operation error
	FlushError(String),
}

impl std::fmt::Display for SessionError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::DatabaseError(msg) => write!(f, "Database error: {}", msg),
			Self::ObjectNotFound(msg) => write!(f, "Object not found: {}", msg),
			Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
			Self::FieldCodec(error) => write!(f, "Field codec error: {}", error),
			Self::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
			Self::FlushError(msg) => write!(f, "Flush error: {}", msg),
		}
	}
}

impl std::error::Error for SessionError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::FieldCodec(error) => Some(error),
			_ => None,
		}
	}
}

impl From<FieldCodecError> for SessionError {
	fn from(error: FieldCodecError) -> Self {
		Self::FieldCodec(error)
	}
}

/// Identity map entry storing tracked objects
struct IdentityEntry {
	/// The serialized object data
	data: Value,
	/// Canonical database field data used by type-erased flush processing.
	database_data: BTreeMap<String, crate::orm::DatabaseValue>,
	/// Type ID for runtime type checking
	type_id: TypeId,
	/// Model field metadata used by type-erased flush processing
	field_metadata: Vec<FieldInfo>,
	/// Rust model field name for the primary key.
	primary_key_field: String,
	/// Resolved database column name for the primary key.
	primary_key_column: String,
	/// Physical columns and typed values that identify the tracked row.
	primary_key_filters: Vec<(String, crate::orm::DatabaseValue)>,
	/// Nullable JSON fields whose model value is `None` and must be written as SQL NULL.
	sql_null_json_fields: HashSet<String>,
	/// Database-generated columns that must be omitted from INSERT/UPDATE writes.
	generated_fields: HashSet<String>,
	/// Whether the object must be inserted even when it has an assigned primary key.
	is_new: bool,
	/// Whether the object has been modified
	// Allow dead_code: dirty tracking flag set internally, read by future flush/commit logic
	#[allow(dead_code)]
	is_dirty: bool,
}

/// SQLAlchemy-style ORM session with identity map and unit of work.
///
/// A session tracks objects and writes them with [`Self::flush`], but it does
/// not own a database transaction. Use [`super::DatabaseConnection::atomic`]
/// for atomic ORM operations and discard or recreate a session when tracked
/// state must be abandoned.
#[derive(Clone)]
struct PendingDelete {
	table_name: &'static str,
	primary_key_filters: Vec<(String, crate::orm::DatabaseValue)>,
}

/// SQLAlchemy-style ORM session with identity map and unit of work
///
/// # Examples
///
/// ```no_run
/// use reinhardt_db::orm::session::Session;
/// use reinhardt_db::orm::query_types::DbBackend;
/// use sqlx::AnyPool;
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let pool = AnyPool::connect("sqlite::memory:").await?;
/// let session = Session::new(Arc::new(pool), DbBackend::Sqlite).await?;
///
/// // Session is ready for use
/// # Ok(())
/// # }
/// ```
pub struct Session {
	/// Connection pool
	// Allow dead_code: pool stored for session-scoped query execution and flushing.
	#[allow(dead_code)]
	pool: Arc<AnyPool>,
	/// Database backend type
	db_backend: DbBackend,
	/// Identity map: tracks objects by type and primary key
	identity_map: HashMap<String, IdentityEntry>,
	/// Set of object keys that have been modified
	dirty_objects: HashSet<String>,
	/// Objects marked for deletion with canonical primary-key carriers.
	deleted_objects: HashMap<String, PendingDelete>,
	/// Whether session is closed
	is_closed: bool,
	/// Counter for generating temporary keys for new objects
	new_object_counter: usize,
	/// Generated IDs from the last flush operation (table_name, generated_id)
	last_generated_ids: Vec<(String, i64)>,
}

impl Session {
	/// Create a new session with the given connection pool and database backend
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::session::Session;
	/// use reinhardt_db::orm::query_types::DbBackend;
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let session = Session::new(Arc::new(pool), DbBackend::Sqlite).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn new(pool: Arc<AnyPool>, db_backend: DbBackend) -> Result<Self, SessionError> {
		Ok(Self {
			pool,
			db_backend,
			identity_map: HashMap::new(),
			dirty_objects: HashSet::new(),
			deleted_objects: HashMap::new(),
			is_closed: false,
			new_object_counter: 0,
			last_generated_ids: Vec::new(),
		})
	}

	/// Add an object to the session for tracking
	///
	/// Objects with a primary key will be tracked for UPDATE operations.
	/// Objects without a primary key (None) will be tracked for INSERT operations.
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::session::Session;
	/// use reinhardt_db::orm::Model;
	/// use serde::{Serialize, Deserialize};
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	/// use reinhardt_db::orm::query_types::DbBackend;
	///
	/// #[derive(Serialize, Deserialize, Clone)]
	/// struct User {
	///     id: Option<i64>,
	///     name: String,
	/// }
	///
	/// # #[derive(Clone)]
	/// # struct UserFields;
	/// # impl reinhardt_db::orm::FieldSelector for UserFields {
	/// #     fn with_alias(self, _alias: &str) -> Self { self }
	/// # }
	/// #
	/// impl Model for User {
	///     type PrimaryKey = i64;
	/// #     type Fields = UserFields;
	/// #     type Objects = reinhardt_db::orm::Manager<Self>;
	///     fn table_name() -> &'static str { "users" }
	/// #     fn new_fields() -> Self::Fields { UserFields }
	///     fn primary_key(&self) -> Option<Self::PrimaryKey> { self.id }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// }
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let mut session = Session::new(Arc::new(pool), DbBackend::Sqlite).await?;
	///
	/// // Add existing object with PK (for UPDATE)
	/// let user = User { id: Some(1), name: "Alice".to_string() };
	/// session.add(user).await?;
	///
	/// // Add new object without PK (for INSERT)
	/// let new_user = User { id: None, name: "Bob".to_string() };
	/// session.add(new_user).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn add<T: Model + 'static>(&mut self, obj: T) -> Result<(), SessionError> {
		let is_new = obj.primary_key().is_none()
			|| (T::primary_key_uses_zero_sentinel()
				&& obj
					.primary_key()
					.is_some_and(|primary_key| primary_key.to_string() == "0"));
		self.add_with_state(obj, is_new).await
	}

	/// Add an object as a new row, including objects with an assigned primary key.
	pub async fn add_new<T: Model + 'static>(&mut self, obj: T) -> Result<(), SessionError> {
		self.add_with_state(obj, true).await
	}

	async fn add_with_state<T: Model + 'static>(
		&mut self,
		obj: T,
		is_new: bool,
	) -> Result<(), SessionError> {
		self.check_closed()?;
		let has_generated_primary_key = T::primary_key_uses_zero_sentinel()
			&& obj
				.primary_key()
				.is_some_and(|primary_key| primary_key.to_string() == "0");

		// Generate key based on whether object has a primary key
		let key = match obj.primary_key().filter(|_| !has_generated_primary_key) {
			Some(pk) => {
				// Existing object with PK - use standard key format
				format!("{}:{}", T::table_name(), pk)
			}
			None => {
				// New object without PK - use temporary key format
				let counter = self.new_object_counter;
				self.new_object_counter += 1;
				format!("{}:__new__{}", T::table_name(), counter)
			}
		};

		let data = serde_json::to_value(&obj)
			.map_err(|e| SessionError::SerializationError(e.to_string()))?;
		let database_data = encode_model_database_data(&obj)?;
		let field_metadata = T::field_metadata();
		let primary_key_filters = if obj.primary_key().is_some() && !has_generated_primary_key {
			primary_key_filters_for_model::<T>(&field_metadata, &database_data)?
		} else {
			Vec::new()
		};
		let primary_key_field = T::primary_key_field().to_string();
		let primary_key_column = primary_key_field_info(&field_metadata, T::primary_key_field())
			.and_then(|field| field.db_column.clone())
			.unwrap_or_else(|| primary_key_field.clone());
		let sql_null_json_fields = field_metadata
			.iter()
			.filter(|field| {
				field.nullable && is_structured_field(field) && obj.field_is_none(&field.name)
			})
			.map(|field| field.name.clone())
			.collect();

		self.identity_map.insert(
			key.clone(),
			IdentityEntry {
				data,
				database_data,
				type_id: TypeId::of::<T>(),
				field_metadata,
				primary_key_field,
				primary_key_column,
				primary_key_filters,
				sql_null_json_fields,
				generated_fields: T::generated_field_names()
					.iter()
					.map(|field| (*field).to_string())
					.collect(),
				is_new,
				is_dirty: true,
			},
		);

		self.dirty_objects.insert(key);

		Ok(())
	}

	/// Get an object by primary key
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::session::Session;
	/// use reinhardt_db::orm::Model;
	/// use serde::{Serialize, Deserialize};
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	/// use reinhardt_db::orm::query_types::DbBackend;
	///
	/// #[derive(Serialize, Deserialize, Clone)]
	/// struct User {
	///     id: Option<i64>,
	///     name: String,
	/// }
	///
	/// # #[derive(Clone)]
	/// # struct UserFields;
	/// # impl reinhardt_db::orm::FieldSelector for UserFields {
	/// #     fn with_alias(self, _alias: &str) -> Self { self }
	/// # }
	/// #
	/// impl Model for User {
	///     type PrimaryKey = i64;
	/// #     type Fields = UserFields;
	/// #     type Objects = reinhardt_db::orm::Manager<Self>;
	///     fn table_name() -> &'static str { "users" }
	/// #     fn new_fields() -> Self::Fields { UserFields }
	///     fn primary_key(&self) -> Option<Self::PrimaryKey> { self.id }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// }
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let mut session = Session::new(Arc::new(pool), DbBackend::Sqlite).await?;
	///
	/// let user: Option<User> = session.get(1).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn get<T: Model + 'static>(
		&mut self,
		id: T::PrimaryKey,
	) -> Result<Option<T>, SessionError> {
		self.check_closed()?;

		let key = format!("{}:{}", T::table_name(), id);

		// Check identity map first
		if let Some(entry) = self.identity_map.get(&key) {
			if entry.type_id != TypeId::of::<T>() {
				return Err(SessionError::InvalidState(
					"Type mismatch in identity map".to_string(),
				));
			}

			let json_null_fields = json_null_fields_for_data(
				&entry.data,
				&entry.field_metadata,
				&entry.sql_null_json_fields,
			);
			let obj: T =
				super::json::deserialize_model_value(entry.data.clone(), &json_null_fields)
					.map_err(|e| SessionError::SerializationError(e.to_string()))?;

			return Ok(Some(obj));
		}

		// Query database if not in identity map
		// Use field_metadata() to build the query and map results
		let field_metadata = T::field_metadata();
		if field_metadata.is_empty() {
			// No field metadata available - model might not use derive(Model) macro
			// Return None as we cannot query without field information
			return Ok(None);
		}

		// Build SELECT query using reinhardt_query
		let pk_field = primary_key_field_info(&field_metadata, T::primary_key_field());
		let pk_column = pk_field
			.and_then(|field| field.db_column.as_deref())
			.unwrap_or_else(|| {
				pk_field.map_or(T::primary_key_field(), |field| field.name.as_str())
			});
		let mut select_query = RQuery::select();
		select_query.from(Alias::new(T::table_name()));

		apply_any_model_projection::<T>(&mut select_query, self.db_backend, T::table_name())?;
		let primary_key = database_value_to_query_value(T::primary_key_database_value(&id)?);
		select_query.and_where(Expr::col(Alias::new(pk_column)).eq(Expr::val(primary_key)));
		let (sql, arguments) = prepare_any_select(&select_query, self.db_backend)?.into_parts();

		// Execute query
		let row = match sqlx::query_with(&sql, arguments)
			.fetch_optional(&*self.pool)
			.await
		{
			Ok(Some(row)) => row,
			Ok(None) => return Ok(None),
			Err(e) => {
				return Err(SessionError::DatabaseError(format!(
					"Failed to query database: {}",
					e
				)));
			}
		};

		let obj: T = deserialize_any_row(&row, &field_metadata)?;
		let mut sql_null_json_fields = HashSet::new();
		for field in field_metadata
			.iter()
			.filter(|field| field.nullable && is_structured_field(field))
		{
			if any_text_value(&row, field.db_column_name())?.is_none() {
				sql_null_json_fields.insert(field.name.clone());
			}
		}

		// Add to identity map
		let obj_data = serde_json::to_value(&obj)
			.map_err(|e| SessionError::SerializationError(e.to_string()))?;
		let database_data = encode_model_database_data(&obj)?;
		let primary_key_filters =
			primary_key_filters_for_model::<T>(&field_metadata, &database_data)?;

		self.identity_map.insert(
			key.clone(),
			IdentityEntry {
				data: obj_data,
				database_data,
				type_id: TypeId::of::<T>(),
				field_metadata: field_metadata.clone(),
				primary_key_field: T::primary_key_field().to_string(),
				primary_key_column: primary_key_field_info(&field_metadata, T::primary_key_field())
					.and_then(|field| field.db_column.clone())
					.unwrap_or_else(|| T::primary_key_field().to_string()),
				primary_key_filters,
				sql_null_json_fields,
				generated_fields: T::generated_field_names()
					.iter()
					.map(|field| (*field).to_string())
					.collect(),
				is_new: false,
				is_dirty: false,
			},
		);

		Ok(Some(obj))
	}

	/// Get all objects of a given type from the database
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::session::Session;
	/// use reinhardt_db::orm::Model;
	/// use serde::{Serialize, Deserialize};
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	/// use reinhardt_db::orm::query_types::DbBackend;
	///
	/// #[derive(Serialize, Deserialize, Clone)]
	/// struct User {
	///     id: Option<i64>,
	///     name: String,
	/// }
	///
	/// # #[derive(Clone)]
	/// # struct UserFields;
	/// # impl reinhardt_db::orm::FieldSelector for UserFields {
	/// #     fn with_alias(self, _alias: &str) -> Self { self }
	/// # }
	/// #
	/// impl Model for User {
	///     type PrimaryKey = i64;
	/// #     type Fields = UserFields;
	/// #     type Objects = reinhardt_db::orm::Manager<Self>;
	///     fn table_name() -> &'static str { "users" }
	/// #     fn new_fields() -> Self::Fields { UserFields }
	///     fn primary_key(&self) -> Option<Self::PrimaryKey> { self.id }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// }
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("postgres://localhost/test").await?;
	/// let mut session = Session::new(Arc::new(pool), DbBackend::Postgres).await?;
	///
	/// let users: Vec<User> = session.list_all().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn list_all<T: Model + 'static>(&self) -> Result<Vec<T>, SessionError> {
		self.check_closed()?;

		// Use field_metadata() to build the query and map results
		let field_metadata = T::field_metadata();
		if field_metadata.is_empty() {
			// No field metadata available - return empty list
			return Ok(Vec::new());
		}

		let mut statement = RQuery::select();
		statement.from(Alias::new(T::table_name()));
		apply_any_model_projection::<T>(&mut statement, self.db_backend, T::table_name())?;
		let (sql, arguments) = prepare_any_select(&statement, self.db_backend)?.into_parts();
		let rows = sqlx::query_with(&sql, arguments)
			.fetch_all(&*self.pool)
			.await
			.map_err(|error| {
				SessionError::DatabaseError(format!("Failed to query database: {error}"))
			})?;
		rows.iter()
			.map(|row| deserialize_any_row(row, &field_metadata))
			.collect()
	}

	/// Execute a model-shaped [`QuerySet`] using this session's configured pool.
	///
	/// Unlike the global query-set execution helpers, this method always uses
	/// the pool and backend owned by the session. It is therefore suitable for
	/// request-scoped queries whose connection is selected by the caller.
	pub async fn list<T>(&self, queryset: &QuerySet<T>) -> Result<Vec<T>, SessionError>
	where
		T: Model + serde::de::DeserializeOwned + 'static,
	{
		queryset
			.ensure_not_locking_without_transaction()
			.map_err(|error| SessionError::DatabaseError(error.to_string()))?;
		self.check_closed()?;
		let mut connection = self
			.pool
			.acquire()
			.await
			.map_err(|error| SessionError::DatabaseError(error.to_string()))?;
		self.list_with_connection(queryset, &mut connection).await
	}

	/// Execute a model-shaped [`QuerySet`] through a caller-owned connection.
	///
	/// The connection may be a transaction connection, keeping the read on the
	/// same database transaction as a subsequent [`Self::flush_with_connection`].
	pub async fn list_with_connection<T>(
		&self,
		queryset: &QuerySet<T>,
		connection: &mut sqlx::AnyConnection,
	) -> Result<Vec<T>, SessionError>
	where
		T: Model + serde::de::DeserializeOwned + 'static,
	{
		self.list_with_connection_inner(queryset, connection, false)
			.await
	}

	/// Execute a model-shaped [`QuerySet`] and lock matching rows until the
	/// caller-owned transaction completes.
	pub async fn list_with_connection_for_update<T>(
		&self,
		queryset: &QuerySet<T>,
		connection: &mut sqlx::AnyConnection,
	) -> Result<Vec<T>, SessionError>
	where
		T: Model + serde::de::DeserializeOwned + 'static,
	{
		self.list_with_connection_inner(queryset, connection, true)
			.await
	}

	async fn list_with_connection_inner<T>(
		&self,
		queryset: &QuerySet<T>,
		connection: &mut sqlx::AnyConnection,
		lock_rows: bool,
	) -> Result<Vec<T>, SessionError>
	where
		T: Model + serde::de::DeserializeOwned + 'static,
	{
		self.check_closed()?;
		let fields = T::field_metadata();
		if fields.is_empty() {
			return Ok(Vec::new());
		}
		if queryset.is_empty_result() {
			return Ok(Vec::new());
		}

		let (database_type, capabilities) = match self.db_backend {
			DbBackend::Postgres => (DatabaseType::Postgres, RowLockCapabilities::postgres()),
			// The session backend does not identify MySQL versus MariaDB or expose
			// the server version. Keep the portable unqualified `FOR UPDATE` form
			// until the connection can report whether explicit lock targets exist.
			DbBackend::Mysql => (
				DatabaseType::Mysql,
				RowLockCapabilities {
					targets: false,
					..RowLockCapabilities::mysql()
				},
			),
			DbBackend::Sqlite => (DatabaseType::Sqlite, RowLockCapabilities::unsupported()),
		};
		queryset
			.validate_select_for_update(capabilities, database_type)
			.map_err(|error| SessionError::DatabaseError(error.to_string()))?;
		if lock_rows && !capabilities.update {
			return Err(SessionError::DatabaseError(
				"SELECT FOR UPDATE is not supported by this backend or server version".to_owned(),
			));
		}
		if lock_rows && self.db_backend == DbBackend::Postgres && queryset.has_right_join() {
			return Err(SessionError::DatabaseError(
				"SELECT FOR UPDATE does not support RIGHT JOIN mutation scopes on PostgreSQL"
					.to_owned(),
			));
		}
		if lock_rows {
			queryset
				.validate_row_lock_source()
				.map_err(|error| SessionError::DatabaseError(error.to_string()))?;
		}

		let mut locked_queryset = queryset.clone();
		if lock_rows {
			locked_queryset.lock_scope_subqueries();
		}
		let mut statement = locked_queryset
			.build_full_model_select_statement_for_backend(database_type)
			.map_err(|error| SessionError::DatabaseError(error.to_string()))?;
		let root_alias = locked_queryset.root_alias().to_owned();
		apply_any_model_projection::<T>(&mut statement, self.db_backend, &root_alias)?;
		if lock_rows {
			if self.db_backend == DbBackend::Postgres {
				self.lock_nullable_relation_rows(&locked_queryset, &statement, connection)
					.await?;
			}
			statement.clear_distinct();
			statement.lock(LockType::Update);
			if capabilities.targets {
				let mut lock_tables = vec![Alias::new(root_alias)];
				lock_tables.extend(
					locked_queryset
						.inner_relation_aliases_for_lock()
						.into_iter()
						.map(Alias::new),
				);
				statement.lock_tables(lock_tables);
			}
		}
		let (sql, arguments) = prepare_any_select(&statement, self.db_backend)?.into_parts();
		let rows = sqlx::query_with(&sql, arguments)
			.fetch_all(&mut *connection)
			.await
			.map_err(|error| SessionError::DatabaseError(error.to_string()))?;
		rows.iter()
			.map(|row| deserialize_any_row::<T>(row, &fields))
			.collect()
	}

	async fn lock_nullable_relation_rows<T>(
		&self,
		queryset: &QuerySet<T>,
		source_statement: &SelectStatement,
		connection: &mut sqlx::AnyConnection,
	) -> Result<(), SessionError>
	where
		T: Model + serde::de::DeserializeOwned + 'static,
	{
		for (table, alias, column) in queryset.nullable_filter_relations_for_lock() {
			let mut subquery = source_statement.clone();
			subquery.clear_selects();
			subquery.column((Alias::new(&alias), Alias::new(&column)));

			// Lock nullable relation rows through a single-table statement so the
			// backend never applies FOR UPDATE to the nullable side of an outer join.
			let mut statement = RQuery::select();
			statement
				.column(ColumnRef::table_asterisk(Alias::new(&alias)))
				.from_as(Alias::new(&table), Alias::new(&alias))
				.and_where(
					Expr::col((Alias::new(&alias), Alias::new(&column))).in_subquery(subquery),
				)
				.lock(LockType::Update);
			let (sql, arguments) = prepare_any_select(&statement, self.db_backend)?.into_parts();
			sqlx::query_with(&sql, arguments)
				.fetch_all(&mut *connection)
				.await
				.map_err(|error| SessionError::DatabaseError(error.to_string()))?;
		}

		Ok(())
	}

	/// Create a query for the given model type
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::session::Session;
	/// use reinhardt_db::orm::Model;
	/// use serde::{Serialize, Deserialize};
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	/// use reinhardt_db::orm::query_types::DbBackend;
	///
	/// #[derive(Serialize, Deserialize, Clone)]
	/// struct User {
	///     id: Option<i64>,
	///     name: String,
	/// }
	///
	/// # #[derive(Clone)]
	/// # struct UserFields;
	/// # impl reinhardt_db::orm::FieldSelector for UserFields {
	/// #     fn with_alias(self, _alias: &str) -> Self { self }
	/// # }
	/// #
	/// impl Model for User {
	///     type PrimaryKey = i64;
	/// #     type Fields = UserFields;
	/// #     type Objects = reinhardt_db::orm::Manager<Self>;
	///     fn table_name() -> &'static str { "users" }
	/// #     fn new_fields() -> Self::Fields { UserFields }
	///     fn primary_key(&self) -> Option<Self::PrimaryKey> { self.id }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// }
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let session = Session::new(Arc::new(pool), DbBackend::Sqlite).await?;
	///
	/// let query = session.query::<User>();
	/// # Ok(())
	/// # }
	/// ```
	pub fn query<T: Model>(&self) -> OrmQuery {
		OrmQuery::new()
	}

	/// Flush all pending changes to the database
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::session::Session;
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	/// use reinhardt_db::orm::query_types::DbBackend;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let mut session = Session::new(Arc::new(pool), DbBackend::Sqlite).await?;
	///
	/// // Add/modify objects...
	/// session.flush().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn flush(&mut self) -> Result<(), SessionError> {
		self.check_closed()?;
		let mut connection = self
			.pool
			.acquire()
			.await
			.map_err(|error| SessionError::DatabaseError(error.to_string()))?;
		self.flush_with_connection(&mut connection).await
	}

	/// Flush tracked changes through a caller-owned connection.
	///
	/// The connection may be a transaction connection, keeping the writes on
	/// the same database transaction as a preceding [`Self::list_with_connection`].
	pub async fn flush_with_connection(
		&mut self,
		connection: &mut sqlx::AnyConnection,
	) -> Result<(), SessionError> {
		self.check_closed()?;

		// Clear any previously generated IDs
		self.last_generated_ids.clear();

		// Determine database backend from pool
		let backend = self.get_backend();

		// Process dirty objects (INSERT/UPDATE)
		for key in &self.dirty_objects.clone() {
			if let Some(entry) = self.identity_map.get(key) {
				// Parse the identity key to get table name and primary key
				let Some((table_name, _identity)) = key.split_once(':') else {
					continue;
				};
				let primary_key_field = entry.primary_key_field.clone();
				let primary_key_column = entry.primary_key_column.clone();

				{
					let obj = &entry.database_data;
					if !entry.is_new {
						// UPDATE existing record
						let mut update_stmt =
							RQuery::update().table(Alias::new(table_name)).to_owned();
						let mut has_update_values = false;

						// Set all columns except primary key and auto-managed datetime fields
						for (col_name, col_value) in obj {
							if col_name == &primary_key_field {
								continue;
							}
							let field_info = find_field_info(&entry.field_metadata, col_name);
							let column_name = flush_column_name(col_name, field_info);
							if should_skip_flush_column(col_name, column_name, field_info) {
								continue;
							}
							if entry.generated_fields.contains(col_name) {
								continue;
							}
							// Skip null auto-managed datetime fields to let database defaults remain.
							if matches!(col_value, crate::orm::DatabaseValue::Null)
								&& is_auto_managed_datetime_column(col_name, column_name)
							{
								continue;
							}
							update_stmt.value(
								Alias::new(column_name),
								database_value_to_query_value(col_value.clone()),
							);
							has_update_values = true;
						}

						if !has_update_values {
							continue;
						}

						// Add WHERE clauses for every primary-key component.
						for (column, value) in &entry.primary_key_filters {
							update_stmt.and_where(
								Expr::col(Alias::new(column))
									.eq(Expr::val(database_value_to_query_value(value.clone()))),
							);
						}

						// Build and execute SQL
						let prepared =
							prepare_any_built(build_any_update(&update_stmt, backend)?, backend)?;

						self.execute_prepared(connection, prepared).await?;
					} else {
						// INSERT new record
						let mut insert_stmt = RQuery::insert()
							.into_table(Alias::new(table_name))
							.to_owned();

						let mut columns = Vec::new();
						let mut values_vec: Vec<RValue> = Vec::new();

						for (col_name, col_value) in obj {
							let field_info = find_field_info(&entry.field_metadata, col_name);
							let column_name = flush_column_name(col_name, field_info);
							let is_assigned_primary_key = entry
								.primary_key_filters
								.iter()
								.any(|(column, _)| column == column_name);
							if (col_name == &primary_key_field
								|| should_skip_flush_column(col_name, column_name, field_info))
								&& !is_assigned_primary_key
							{
								continue;
							}
							if entry.generated_fields.contains(col_name) {
								continue;
							}
							// Skip null datetime fields to let database DEFAULT apply
							// (e.g., created_at, updated_at with DEFAULT CURRENT_TIMESTAMP)
							if matches!(col_value, crate::orm::DatabaseValue::Null)
								&& is_auto_managed_datetime_column(col_name, column_name)
							{
								continue;
							}
							columns.push(Alias::new(column_name));
							values_vec.push(database_value_to_query_value(col_value.clone()));
						}

						if columns.is_empty() {
							return Err(SessionError::FlushError(format!(
								"Cannot insert {table_name} because no writable fields remain after filtering generated and defaulted columns"
							)));
						}

						insert_stmt.columns(columns);
						insert_stmt.values(values_vec).map_err(|e| {
							SessionError::FlushError(format!(
								"Failed to build INSERT values: {}",
								e
							))
						})?;

						// Add RETURNING clause for PostgreSQL to get generated ID
						if backend == DbBackend::Postgres {
							insert_stmt.returning_col(Alias::new(&primary_key_column));
						}

						// Build and execute SQL
						let prepared =
							prepare_any_built(build_any_insert(&insert_stmt, backend)?, backend)?;

						// Execute and get generated ID if available
						if backend == DbBackend::Postgres {
							let row = self.execute_returning(connection, prepared).await?;
							// Extract the generated ID
							let generated_id: i64 =
								row.try_get(primary_key_column.as_str()).map_err(|e| {
									SessionError::FlushError(format!("Failed to extract ID: {}", e))
								})?;

							// Track the generated ID for retrieval after flush
							self.last_generated_ids
								.push((table_name.to_string(), generated_id));

							// Update the identity map
							self.update_identity_map_with_generated_id(
								key,
								table_name,
								&primary_key_field,
								generated_id,
							)?;
						} else {
							self.execute_prepared(connection, prepared).await?;
						}
					}
				}
			}
		}

		self.dirty_objects.clear();

		// Process deleted objects (DELETE)
		for (key, pending) in self.deleted_objects.clone() {
			// Build DELETE statement
			let mut delete_stmt = RQuery::delete()
				.from_table(Alias::new(pending.table_name))
				.to_owned();

			for (column, value) in &pending.primary_key_filters {
				delete_stmt.and_where(
					Expr::col(Alias::new(column))
						.eq(Expr::val(database_value_to_query_value(value.clone()))),
				);
			}

			// Build and execute SQL
			let prepared = prepare_any_built(build_any_delete(&delete_stmt, backend)?, backend)?;

			self.execute_prepared(connection, prepared).await?;

			// Remove from identity map
			self.identity_map.remove(&key);
		}

		self.deleted_objects.clear();

		Ok(())
	}

	/// Update identity map with generated ID from RETURNING clause
	///
	/// This method is called after executing an INSERT with RETURNING clause
	/// to update the identity map entry with the generated primary key value.
	///
	/// # Arguments
	///
	/// * `old_key` - The current identity key (e.g., "table_name:null")
	/// * `table_name` - The name of the table
	/// * `generated_id` - The generated primary key value from the database
	fn update_identity_map_with_generated_id(
		&mut self,
		old_key: &str,
		table_name: &str,
		primary_key_field: &str,
		generated_id: i64,
	) -> Result<(), SessionError> {
		if let Some(mut entry) = self.identity_map.remove(old_key) {
			// JSON update
			if let Some(obj) = entry.data.as_object_mut() {
				obj.insert(
					primary_key_field.to_string(),
					serde_json::Value::from(generated_id),
				);
			}
			entry.database_data.insert(
				primary_key_field.to_string(),
				crate::orm::DatabaseValue::I64(generated_id),
			);
			entry.primary_key_filters = vec![(
				entry.primary_key_column.clone(),
				crate::orm::DatabaseValue::I64(generated_id),
			)];

			entry.is_new = false;
			entry.is_dirty = false;
			let new_key = format!("{}:{}", table_name, generated_id);
			self.identity_map.insert(new_key, entry);
			self.dirty_objects.remove(old_key);

			Ok(())
		} else {
			Err(SessionError::InvalidState(
				"Entry not found in identity map".to_string(),
			))
		}
	}

	/// Get database backend type from pool
	fn get_backend(&self) -> DbBackend {
		// Return the backend type that was provided during Session creation
		self.db_backend
	}

	/// Get the IDs generated during the last flush operation
	///
	/// Returns a slice of (table_name, generated_id) tuples for all objects
	/// that were inserted with auto-generated primary keys during the last flush.
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::session::Session;
	/// use reinhardt_db::orm::query_types::DbBackend;
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("postgres://localhost/test").await?;
	/// let mut session = Session::new(Arc::new(pool), DbBackend::Postgres).await?;
	///
	/// // ... add objects and flush ...
	///
	/// // Get the generated IDs
	/// for (table_name, id) in session.get_generated_ids() {
	///     println!("Generated ID {} for table {}", id, table_name);
	/// }
	/// # Ok(())
	/// # }
	/// ```
	pub fn get_generated_ids(&self) -> &[(String, i64)] {
		&self.last_generated_ids
	}

	/// Execute the exact generated statement and companion arguments.
	async fn execute_prepared(
		&self,
		connection: &mut sqlx::AnyConnection,
		prepared: PreparedAnyQuery,
	) -> Result<(), SessionError> {
		let (sql, arguments) = prepared.into_parts();
		sqlx::query_with(&sql, arguments)
			.execute(&mut *connection)
			.await
			.map_err(|error| SessionError::FlushError(error.to_string()))?;
		Ok(())
	}

	/// Execute a prepared PostgreSQL INSERT with RETURNING on the same connection.
	async fn execute_returning(
		&self,
		connection: &mut sqlx::AnyConnection,
		prepared: PreparedAnyQuery,
	) -> Result<sqlx::any::AnyRow, SessionError> {
		let (sql, arguments) = prepared.into_parts();
		sqlx::query_with(&sql, arguments)
			.fetch_one(&mut *connection)
			.await
			.map_err(|error| SessionError::FlushError(error.to_string()))
	}

	/// Close the session
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::session::Session;
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	/// use reinhardt_db::orm::query_types::DbBackend;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let mut session = Session::new(Arc::new(pool), DbBackend::Sqlite).await?;
	///
	/// session.close().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn close(mut self) -> Result<(), SessionError> {
		if self.is_closed {
			return Ok(());
		}

		self.is_closed = true;
		Ok(())
	}

	/// Delete an object from the session
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::session::Session;
	/// use reinhardt_db::orm::Model;
	/// use serde::{Serialize, Deserialize};
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	/// use reinhardt_db::orm::query_types::DbBackend;
	///
	/// #[derive(Serialize, Deserialize, Clone)]
	/// struct User {
	///     id: Option<i64>,
	///     name: String,
	/// }
	///
	/// # #[derive(Clone)]
	/// # struct UserFields;
	/// # impl reinhardt_db::orm::FieldSelector for UserFields {
	/// #     fn with_alias(self, _alias: &str) -> Self { self }
	/// # }
	/// #
	/// impl Model for User {
	///     type PrimaryKey = i64;
	/// #     type Fields = UserFields;
	/// #     type Objects = reinhardt_db::orm::Manager<Self>;
	///     fn table_name() -> &'static str { "users" }
	/// #     fn new_fields() -> Self::Fields { UserFields }
	///     fn primary_key(&self) -> Option<Self::PrimaryKey> { self.id }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// }
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let mut session = Session::new(Arc::new(pool), DbBackend::Sqlite).await?;
	///
	/// let user = User { id: Some(1), name: "Alice".to_string() };
	/// session.delete(user).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn delete<T: Model + 'static>(&mut self, obj: T) -> Result<(), SessionError> {
		self.check_closed()?;

		let pk = obj
			.primary_key()
			.ok_or_else(|| SessionError::InvalidState("Object has no primary key".to_string()))?;

		let key = format!("{}:{}", T::table_name(), pk);
		let field_metadata = T::field_metadata();
		let database_data = obj.encode_database_fields()?;
		let primary_key_filters =
			primary_key_filters_for_model::<T>(&field_metadata, &database_data)?;

		// Mark for deletion
		self.deleted_objects.insert(
			key.clone(),
			PendingDelete {
				table_name: T::table_name(),
				primary_key_filters,
			},
		);

		// Remove from dirty set if present
		self.dirty_objects.remove(&key);

		Ok(())
	}

	/// Check if the session is closed
	fn check_closed(&self) -> Result<(), SessionError> {
		if self.is_closed {
			Err(SessionError::InvalidState("Session is closed".to_string()))
		} else {
			Ok(())
		}
	}

	/// Get the number of objects in the identity map
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::session::Session;
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	/// use reinhardt_db::orm::query_types::DbBackend;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let session = Session::new(Arc::new(pool), DbBackend::Sqlite).await?;
	///
	/// let count = session.identity_count();
	/// # Ok(())
	/// # }
	/// ```
	pub fn identity_count(&self) -> usize {
		self.identity_map.len()
	}

	/// Get the number of dirty objects
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::session::Session;
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	/// use reinhardt_db::orm::query_types::DbBackend;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let session = Session::new(Arc::new(pool), DbBackend::Sqlite).await?;
	///
	/// let count = session.dirty_count();
	/// # Ok(())
	/// # }
	/// ```
	pub fn dirty_count(&self) -> usize {
		self.dirty_objects.len()
	}

	/// Check if session is closed
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_db::orm::session::Session;
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	/// use reinhardt_db::orm::query_types::DbBackend;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let session = Session::new(Arc::new(pool), DbBackend::Sqlite).await?;
	///
	/// let closed = session.is_closed();
	/// # Ok(())
	/// # }
	/// ```
	pub fn is_closed(&self) -> bool {
		self.is_closed
	}
}

/// Any compatibility is explicit at this consumer boundary; the companion
/// preserves the SQL and Values produced by the checked statement.
type PreparedAnyQuery = reinhardt_query_sqlx::PreparedQuery<sqlx::any::AnyArguments<'static>>;

fn prepare_any_select(
	statement: &SelectStatement,
	backend: DbBackend,
) -> Result<PreparedAnyQuery, SessionError> {
	prepare_any_built(build_any_select(statement, backend)?, backend)
}

fn prepare_any_built(
	built: (String, reinhardt_query::Values),
	backend: DbBackend,
) -> Result<PreparedAnyQuery, SessionError> {
	let backend = match backend {
		DbBackend::Postgres => reinhardt_query_sqlx::AnyBackend::Postgres,
		DbBackend::Mysql => reinhardt_query_sqlx::AnyBackend::MySql,
		DbBackend::Sqlite => reinhardt_query_sqlx::AnyBackend::Sqlite,
	};
	reinhardt_query_sqlx::prepare_any_with_text_codecs(built, backend)
		.map_err(|error| SessionError::DatabaseError(error.to_string()))
}

fn build_any_select(
	statement: &reinhardt_query::SelectStatement,
	backend: DbBackend,
) -> Result<(String, reinhardt_query::Values), SessionError> {
	let statement = statement
		.try_map_value_expressions(|value| any_parameter_expression(value.clone(), backend))?;
	match backend {
		DbBackend::Postgres => PostgresQueryBuilder.build_select_checked(&statement),
		DbBackend::Mysql => MySqlQueryBuilder.build_select_checked(&statement),
		DbBackend::Sqlite => SqliteQueryBuilder.build_select_checked(&statement),
	}
	.map_err(|error| SessionError::DatabaseError(error.to_string()))
}

fn build_any_insert(
	statement: &reinhardt_query::InsertStatement,
	backend: DbBackend,
) -> Result<(String, reinhardt_query::Values), SessionError> {
	let statement = statement
		.try_map_value_expressions(|value| any_parameter_expression(value.clone(), backend))?;
	match backend {
		DbBackend::Postgres => PostgresQueryBuilder.build_insert_checked(&statement),
		DbBackend::Mysql => MySqlQueryBuilder.build_insert_checked(&statement),
		DbBackend::Sqlite => SqliteQueryBuilder.build_insert_checked(&statement),
	}
	.map_err(|error| SessionError::DatabaseError(error.to_string()))
}

fn build_any_update(
	statement: &reinhardt_query::UpdateStatement,
	backend: DbBackend,
) -> Result<(String, reinhardt_query::Values), SessionError> {
	let statement = statement
		.try_map_value_expressions(|value| any_parameter_expression(value.clone(), backend))?;
	match backend {
		DbBackend::Postgres => PostgresQueryBuilder.build_update_checked(&statement),
		DbBackend::Mysql => MySqlQueryBuilder.build_update_checked(&statement),
		DbBackend::Sqlite => SqliteQueryBuilder.build_update_checked(&statement),
	}
	.map_err(|error| SessionError::DatabaseError(error.to_string()))
}

fn build_any_delete(
	statement: &reinhardt_query::DeleteStatement,
	backend: DbBackend,
) -> Result<(String, reinhardt_query::Values), SessionError> {
	let statement = statement
		.try_map_value_expressions(|value| any_parameter_expression(value.clone(), backend))?;
	match backend {
		DbBackend::Postgres => PostgresQueryBuilder.build_delete_checked(&statement),
		DbBackend::Mysql => MySqlQueryBuilder.build_delete_checked(&statement),
		DbBackend::Sqlite => SqliteQueryBuilder.build_delete_checked(&statement),
	}
	.map_err(|error| SessionError::DatabaseError(error.to_string()))
}

fn any_parameter_expression(value: RValue, backend: DbBackend) -> Result<SimpleExpr, SessionError> {
	if value.is_null() {
		return Ok(Expr::val(value).into_simple_expr());
	}
	let cast = if backend == DbBackend::Postgres {
		postgres_parameter_cast(&value)
	} else {
		None
	};
	let (value, cast) = match value {
		RValue::Array(kind, Some(values)) if backend == DbBackend::Postgres => {
			validate_any_array(&kind, &values, backend)?;
			let name = match kind {
				reinhardt_query::value::ArrayType::String => "_text",
				reinhardt_query::value::ArrayType::Int => "_int4",
				reinhardt_query::value::ArrayType::BigInt => "_int8",
				reinhardt_query::value::ArrayType::Bool => "_bool",
				reinhardt_query::value::ArrayType::Float => "_float4",
				reinhardt_query::value::ArrayType::Double => "_float8",
				reinhardt_query::value::ArrayType::Uuid => "_uuid",
				_ => {
					return Err(SessionError::DatabaseError(
						"unsupported PostgreSQL Any array element type".into(),
					));
				}
			};
			let literal = postgres_array_literal(&values).ok_or_else(|| {
				SessionError::DatabaseError("unsupported PostgreSQL Any array value".into())
			})?;
			(literal.into(), Some(name))
		}
		RValue::Array(kind, Some(values)) => {
			validate_any_array(&kind, &values, backend)?;
			(
				super::execution::array_values_to_json(&values)
					.to_string()
					.into(),
				None,
			)
		}
		#[cfg(feature = "pgvector")]
		RValue::Vector(Some(values)) => {
			if values.iter().any(|value| !value.is_finite()) {
				return Err(SessionError::DatabaseError(
					"Any vector values must be finite".into(),
				));
			}
			let literal = serde_json::to_string(&values)
				.map_err(|error| SessionError::DatabaseError(error.to_string()))?;
			(literal.into(), cast)
		}
		value => (value, cast),
	};
	let expression = Expr::val(value);
	Ok(match cast {
		Some(name) => expression.cast_as(name),
		None => expression.into_simple_expr(),
	})
}

fn validate_any_array(
	kind: &reinhardt_query::ArrayType,
	values: &[RValue],
	backend: DbBackend,
) -> Result<(), SessionError> {
	use reinhardt_query::ArrayType;
	for (index, value) in values.iter().enumerate() {
		// The canonical untyped NULL inherits its declared array element type.
		let matches_kind = matches!(value, RValue::Int(None))
			|| matches!(
				(kind, value),
				(ArrayType::Bool, RValue::Bool(_))
					| (ArrayType::TinyInt, RValue::TinyInt(_))
					| (ArrayType::SmallInt, RValue::SmallInt(_))
					| (ArrayType::Int, RValue::Int(_))
					| (ArrayType::BigInt, RValue::BigInt(_))
					| (ArrayType::TinyUnsigned, RValue::TinyUnsigned(_))
					| (ArrayType::SmallUnsigned, RValue::SmallUnsigned(_))
					| (ArrayType::Unsigned, RValue::Unsigned(_))
					| (ArrayType::BigUnsigned, RValue::BigUnsigned(_))
					| (ArrayType::Float, RValue::Float(_))
					| (ArrayType::Double, RValue::Double(_))
					| (ArrayType::String, RValue::String(_))
					| (ArrayType::Char, RValue::Char(_))
					| (ArrayType::Bytes, RValue::Bytes(_))
					| (ArrayType::ChronoDate, RValue::ChronoDate(_))
					| (ArrayType::ChronoTime, RValue::ChronoTime(_))
					| (ArrayType::ChronoDateTime, RValue::ChronoDateTime(_))
					| (ArrayType::ChronoDateTimeUtc, RValue::ChronoDateTimeUtc(_))
					| (
						ArrayType::ChronoDateTimeLocal,
						RValue::ChronoDateTimeLocal(_)
					) | (
					ArrayType::ChronoDateTimeWithTimeZone,
					RValue::ChronoDateTimeWithTimeZone(_)
				) | (ArrayType::Uuid, RValue::Uuid(_))
					| (ArrayType::Json, RValue::Json(_))
					| (ArrayType::Decimal, RValue::Decimal(_))
					| (ArrayType::BigDecimal, RValue::BigDecimal(_))
			);
		if !matches_kind {
			return Err(SessionError::DatabaseError(format!(
				"Any {backend:?} array element {} does not match declared {kind:?} type",
				index + 1
			)));
		}
		if backend != DbBackend::Postgres
			&& matches!(value,
			RValue::Float(Some(value)) if !value.is_finite())
		{
			return Err(SessionError::DatabaseError(format!(
				"Any {backend:?} Float array element {} cannot be represented as finite JSON",
				index + 1
			)));
		}
		if backend != DbBackend::Postgres
			&& matches!(value,
			RValue::Double(Some(value)) if !value.is_finite())
		{
			return Err(SessionError::DatabaseError(format!(
				"Any {backend:?} Double array element {} cannot be represented as finite JSON",
				index + 1
			)));
		}
	}
	Ok(())
}

fn projection_function(name: &'static str, arguments: Vec<SimpleExpr>) -> SimpleExpr {
	SimpleExpr::FunctionCall(name.into_iden(), arguments)
}

fn any_field_projection(backend: DbBackend, field: &FieldInfo, column: SimpleExpr) -> SimpleExpr {
	let field_type = field.field_type.as_str();
	if is_temporal_field_type(field_type) {
		let (function, mask, column) = match backend {
			DbBackend::Postgres
				if field_type.contains("DateTimeField")
					&& field.storage_kind
						== Some(crate::orm::DatabaseStorageKind::NaiveDateTime) =>
			{
				("TO_CHAR", "YYYY-MM-DD\"T\"HH24:MI:SS.US", column)
			}
			DbBackend::Postgres if field_type.contains("DateTimeField") => (
				"TO_CHAR",
				"YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"",
				projection_function(
					"TIMEZONE",
					vec![Expr::val("UTC").into_simple_expr(), column],
				),
			),
			DbBackend::Postgres if field_type.contains("DateField") => {
				("TO_CHAR", "YYYY-MM-DD", column)
			}
			DbBackend::Postgres => ("TO_CHAR", "HH24:MI:SS.US", column),
			DbBackend::Mysql if field_type.contains("DateTimeField") => {
				("DATE_FORMAT", "%Y-%m-%dT%H:%i:%s.%fZ", column)
			}
			DbBackend::Mysql if field_type.contains("DateField") => {
				("DATE_FORMAT", "%Y-%m-%d", column)
			}
			DbBackend::Mysql => ("TIME_FORMAT", "%H:%i:%s.%f", column),
			DbBackend::Sqlite => return column,
		};
		return projection_function(function, vec![column, Expr::val(mask).into_simple_expr()]);
	}
	if backend == DbBackend::Postgres && is_array_field(field) {
		return projection_function("ARRAY_TO_JSON", vec![column]).cast_as_text();
	}
	if backend == DbBackend::Postgres && is_hstore_field(field) {
		return projection_function("HSTORE_TO_JSON", vec![column]).cast_as_text();
	}
	if is_structured_field(field)
		|| field_type.contains("UuidField")
		|| field_type.contains("UUIDField")
		|| field_type.contains("DecimalField")
	{
		return column.cast_as_text();
	}
	if field_type.contains("BooleanField") && backend != DbBackend::Postgres {
		return column.cast_as_signed_integer();
	}
	column
}

fn describe_row_context(row: &sqlx::any::AnyRow, table_name: &str, primary_key: &str) -> String {
	if let Ok(value) = row.try_get::<i64, _>(primary_key) {
		return format!("{table_name}:{primary_key}={value}");
	}
	if let Ok(value) = row.try_get::<i32, _>(primary_key) {
		return format!("{table_name}:{primary_key}={value}");
	}
	if let Ok(value) = row.try_get::<String, _>(primary_key) {
		return format!("{table_name}:{primary_key}={value}");
	}
	table_name.to_owned()
}

fn apply_any_model_projection<T: Model>(
	statement: &mut SelectStatement,
	backend: DbBackend,
	root_alias: &str,
) -> Result<Vec<FieldInfo>, SessionError> {
	let fields = T::field_metadata();
	statement.clear_selects();
	for field in &fields {
		let column_name = field.db_column_name();
		let column = Expr::col((Alias::new(root_alias), Alias::new(column_name)));
		let expression = any_field_projection(backend, field, column.into_simple_expr());

		statement.expr_as(expression, Alias::new(column_name));
	}
	Ok(fields)
}

fn deserialize_any_row<T>(row: &sqlx::any::AnyRow, fields: &[FieldInfo]) -> Result<T, SessionError>
where
	T: Model + serde::de::DeserializeOwned,
{
	let mut json_map = serde_json::Map::new();
	let mut sql_null_json_fields = HashSet::new();
	for field in fields {
		let column_name = field.db_column_name();
		let serialization_error = |detail: String| {
			SessionError::SerializationError(format!(
				"table `{}`, field `{}`, column `{}`: {detail}",
				T::table_name(),
				field.name,
				column_name
			))
		};
		let field_type = field.field_type.as_str();
		let value = if field_type.contains("BigIntegerField") || field_type.contains("BigAutoField")
		{
			row.try_get::<Option<i64>, _>(column_name)
				.map(|value| value.map(Value::from))
				.map_err(|error| serialization_error(error.to_string()))?
		} else if field_type.contains("IntegerField") || field_type.contains("AutoField") {
			row.try_get::<Option<i32>, _>(column_name)
				.map(|value| value.map(Value::from))
				.map_err(|error| serialization_error(error.to_string()))?
		} else if field_type.contains("FloatField") {
			if field.storage_kind == Some(crate::orm::DatabaseStorageKind::F32) {
				row.try_get::<Option<f32>, _>(column_name)
					.map(|value| value.map(|value| Value::from(f64::from(value))))
					.map_err(|error| serialization_error(error.to_string()))?
			} else {
				row.try_get::<Option<f64>, _>(column_name)
					.map(|value| value.map(Value::from))
					.map_err(|error| serialization_error(error.to_string()))?
			}
		} else if field_type.contains("BooleanField") {
			backend_bool_value(row, column_name, field, serialization_error)?.map(Value::Bool)
		} else if field_type.contains("BinaryField")
			|| field.storage_kind == Some(crate::orm::DatabaseStorageKind::Bytes)
		{
			row.try_get::<Option<Vec<u8>>, _>(column_name)
				.map(|value| value.map(Value::from))
				.map_err(|error| serialization_error(error.to_string()))?
		} else if is_structured_field(field) {
			let primary_key = primary_key_field_info(fields, T::primary_key_field())
				.map_or(T::primary_key_field(), FieldInfo::db_column_name);
			let context = describe_row_context(row, T::table_name(), primary_key);
			match decode_json_field_value(
				row,
				T::table_name(),
				&context,
				&field.name,
				column_name,
				field.nullable,
			)? {
				DecodedJsonFieldValue::SqlNull => {
					sql_null_json_fields.insert(field.name.clone());
					None
				}
				DecodedJsonFieldValue::Json(value) => Some(value),
			}
		} else {
			any_text_value(row, column_name)
				.map(|value| value.map(Value::from))
				.map_err(|error| serialization_error(error.to_string()))?
		};

		let value = match value {
			Some(value) => value,
			None if field.nullable => Value::Null,
			None => return Err(serialization_error("unexpected SQL NULL".to_owned())),
		};
		json_map.insert(field.name.clone(), value);
	}

	let data = Value::Object(json_map);
	let json_null_fields = json_null_fields_for_data(&data, fields, &sql_null_json_fields);
	let native_json_fields = fields
		.iter()
		.filter(|field| is_structured_field(field))
		.map(|field| field.name.clone())
		.collect();
	super::json::deserialize_model_row(data, json_null_fields, native_json_fields)
		.map_err(SessionError::FieldCodec)
}

/// SQLx Any may represent MySQL TEXT as bytes even after a CHAR projection.
/// Decode the complete UTF-8 value without bounding its length or truncating it.
fn any_text_value(row: &sqlx::any::AnyRow, column: &str) -> Result<Option<String>, SessionError> {
	match row.try_get::<Option<String>, _>(column) {
		Ok(value) => Ok(value),
		Err(string_error) => {
			let bytes = row
				.try_get::<Option<Vec<u8>>, _>(column)
				.map_err(|bytes_error| {
					SessionError::SerializationError(format!(
						"cannot decode text column {column}: string: {string_error}; bytes: {bytes_error}"
					))
				})?;
			bytes
				.map(|bytes| {
					String::from_utf8(bytes).map_err(|error| {
						SessionError::SerializationError(format!(
							"invalid UTF-8 in text column {column}: {}",
							error.utf8_error()
						))
					})
				})
				.transpose()
		}
	}
}

fn backend_bool_value<F>(
	row: &sqlx::any::AnyRow,
	column_name: &str,
	field: &FieldInfo,
	serialization_error: F,
) -> Result<Option<bool>, SessionError>
where
	F: Fn(String) -> SessionError,
{
	match row.try_get::<Option<i64>, _>(column_name) {
		Ok(Some(0)) => Ok(Some(false)),
		Ok(Some(1)) => Ok(Some(true)),
		Ok(Some(value)) => Err(serialization_error(format!(
			"boolean integer must be 0 or 1, got {value}"
		))),
		Ok(None) => Ok(None),
		Err(integer_error) => row
			.try_get::<Option<bool>, _>(column_name)
			.map_err(|bool_error| {
				serialization_error(format!(
					"cannot decode boolean field {}: integer: {integer_error}; boolean fallback: {bool_error}",
					field.name
				))
			}),
	}
}

fn find_field_info<'a>(field_metadata: &'a [FieldInfo], field_name: &str) -> Option<&'a FieldInfo> {
	field_metadata.iter().find(|field| field.name == field_name)
}

fn primary_key_field_info<'a>(
	field_metadata: &'a [FieldInfo],
	model_primary_key_field: &str,
) -> Option<&'a FieldInfo> {
	field_metadata
		.iter()
		.find(|field| field.primary_key)
		.or_else(|| find_field_info(field_metadata, model_primary_key_field))
}

fn encode_model_database_data<T: Model>(
	model: &T,
) -> Result<BTreeMap<String, crate::orm::DatabaseValue>, SessionError> {
	model
		.encode_database_fields()
		.map_err(SessionError::FieldCodec)
}

fn primary_key_filters_for_model<T: Model>(
	field_metadata: &[FieldInfo],
	database_data: &BTreeMap<String, crate::orm::DatabaseValue>,
) -> Result<Vec<(String, crate::orm::DatabaseValue)>, SessionError> {
	let field_names = T::composite_primary_key()
		.map(|key| key.fields().to_vec())
		.unwrap_or_else(|| vec![T::primary_key_field().to_owned()]);

	field_names
		.into_iter()
		.map(|field_name| {
			let value = database_data
				.get(&field_name)
				.filter(|value| !matches!(value, crate::orm::DatabaseValue::Null))
				.cloned()
				.ok_or_else(|| {
					SessionError::FieldCodec(FieldCodecError::Serialization(format!(
						"encoded {} fields must contain a non-null primary key '{}'",
						T::table_name(),
						field_name
					)))
				})?;
			let column =
				flush_column_name(&field_name, find_field_info(field_metadata, &field_name))
					.to_owned();
			Ok((column, value))
		})
		.collect()
}

fn flush_column_name<'a>(field_name: &'a str, field_info: Option<&'a FieldInfo>) -> &'a str {
	field_info
		.and_then(|field| field.db_column.as_deref())
		.unwrap_or(field_name)
}

fn should_skip_flush_column(
	_field_name: &str,
	_column_name: &str,
	field_info: Option<&FieldInfo>,
) -> bool {
	field_info.map(|field| field.primary_key).unwrap_or(false)
		|| field_info
			.map(|field| {
				field.attributes.contains_key("relation_managed")
					&& !field.attributes.contains_key("fk_id_field")
			})
			.unwrap_or(false)
}

fn is_auto_managed_datetime_column(field_name: &str, column_name: &str) -> bool {
	field_name == "created_at"
		|| field_name == "updated_at"
		|| field_name.ends_with("_date")
		|| field_name.ends_with("_time")
		|| field_name.ends_with("_at")
		|| column_name == "created_at"
		|| column_name == "updated_at"
		|| column_name.ends_with("_date")
		|| column_name.ends_with("_time")
		|| column_name.ends_with("_at")
}

fn is_json_field_type(field_type: &str) -> bool {
	super::json::is_json_field_type(field_type)
}

fn is_array_field(field: &FieldInfo) -> bool {
	field.storage_kind != Some(crate::orm::DatabaseStorageKind::Bytes)
		&& field.field_type.contains("ArrayField")
}

fn is_hstore_field(field: &FieldInfo) -> bool {
	field.field_type.contains("HStoreField")
}

fn is_vector_field(field: &FieldInfo) -> bool {
	field.field_type.contains("VectorField")
}

fn is_json_or_array_field(field: &FieldInfo) -> bool {
	field.storage_kind != Some(crate::orm::DatabaseStorageKind::Bytes)
		&& (is_json_field_type(&field.field_type) || is_array_field(field))
}

fn is_structured_field(field: &FieldInfo) -> bool {
	is_json_or_array_field(field) || is_hstore_field(field) || is_vector_field(field)
}

fn is_temporal_field_type(field_type: &str) -> bool {
	field_type.contains("DateTimeField")
		|| field_type.contains("DateField")
		|| field_type.contains("TimeField")
}

fn json_null_fields_for_data(
	data: &Value,
	field_metadata: &[FieldInfo],
	sql_null_json_fields: &HashSet<String>,
) -> HashSet<String> {
	let Some(values) = data.as_object() else {
		return HashSet::new();
	};
	field_metadata
		.iter()
		.filter(|field| {
			field.nullable
				&& is_structured_field(field)
				&& values.get(&field.name).map(Value::is_null).unwrap_or(false)
				&& !sql_null_json_fields.contains(&field.name)
		})
		.map(|field| field.name.clone())
		.collect()
}

enum DecodedJsonFieldValue {
	SqlNull,
	Json(Value),
}

fn decode_json_field_value(
	row: &sqlx::any::AnyRow,
	table_name: &str,
	row_context: &str,
	field_name: &str,
	column_name: &str,
	nullable: bool,
) -> Result<DecodedJsonFieldValue, SessionError> {
	if nullable {
		if let Ok(value) = row.try_get::<Option<String>, _>(column_name) {
			return value
				.map(|value| {
					parse_json_field_text(&value, table_name, row_context, field_name, column_name)
						.map(DecodedJsonFieldValue::Json)
				})
				.unwrap_or(Ok(DecodedJsonFieldValue::SqlNull));
		}
		if let Ok(value) = row.try_get::<Option<Vec<u8>>, _>(column_name) {
			return value
				.map(|value| {
					parse_json_field_bytes(&value, table_name, row_context, field_name, column_name)
						.map(DecodedJsonFieldValue::Json)
				})
				.unwrap_or(Ok(DecodedJsonFieldValue::SqlNull));
		}
		return Ok(DecodedJsonFieldValue::SqlNull);
	}

	if let Ok(value) = row.try_get::<String, _>(column_name) {
		return parse_json_field_text(&value, table_name, row_context, field_name, column_name)
			.map(DecodedJsonFieldValue::Json);
	}
	if let Ok(value) = row.try_get::<Vec<u8>, _>(column_name) {
		return parse_json_field_bytes(&value, table_name, row_context, field_name, column_name)
			.map(DecodedJsonFieldValue::Json);
	}

	Err(SessionError::SerializationError(format!(
		"Failed to hydrate JSON field {}.{} for row {} from column '{}': value could not be decoded as JSON",
		table_name, field_name, row_context, column_name
	)))
}

fn parse_json_field_text(
	value: &str,
	table_name: &str,
	row_context: &str,
	field_name: &str,
	column_name: &str,
) -> Result<Value, SessionError> {
	serde_json::from_str(value).map_err(|e| {
		let field_path = [table_name, field_name].join(".");
		SessionError::SerializationError(format!(
			"Failed to hydrate JSON field {field_path} for row {row_context} from column '{column_name}': {e}"
		))
	})
}

fn parse_json_field_bytes(
	value: &[u8],
	table_name: &str,
	row_context: &str,
	field_name: &str,
	column_name: &str,
) -> Result<Value, SessionError> {
	serde_json::from_slice(value).map_err(|e| {
		let field_path = [table_name, field_name].join(".");
		SessionError::SerializationError(format!(
			"Failed to hydrate JSON field {field_path} for row {row_context} from column '{column_name}': {e}"
		))
	})
}

/// Convert JSON value to reinhardt_query Value
#[cfg(test)]
fn json_to_reinhardt_query_value(value: &Value) -> RValue {
	match value {
		Value::Null => RValue::Int(None),
		Value::Bool(b) => RValue::Bool(Some(*b)),
		Value::Number(n) => {
			if let Some(i) = n.as_i64() {
				RValue::BigInt(Some(i))
			} else if let Some(f) = n.as_f64() {
				RValue::Double(Some(f))
			} else {
				RValue::Int(None)
			}
		}
		Value::String(s) => {
			// Try to parse as UUID first
			if let Ok(uuid) = Uuid::parse_str(s) {
				return RValue::Uuid(Some(Box::new(uuid)));
			}
			RValue::String(Some(Box::new(s.clone())))
		}
		Value::Array(_) | Value::Object(_) => RValue::Json(Some(Box::new(value.clone()))),
	}
}

#[cfg(test)]
fn json_to_reinhardt_query_value_for_field(
	value: &Value,
	field_info: Option<&FieldInfo>,
	field_is_none: bool,
) -> RValue {
	if field_info.is_some_and(is_json_or_array_field) {
		if field_is_none {
			RValue::Json(None)
		} else {
			RValue::Json(Some(Box::new(value.clone())))
		}
	} else if value.is_null() {
		null_reinhardt_query_value_for_field(field_info)
	} else {
		json_to_reinhardt_query_value(value)
	}
}

#[cfg(test)]
fn null_reinhardt_query_value_for_field(field_info: Option<&FieldInfo>) -> RValue {
	let Some(field_type) = field_info.map(|field| field.field_type.as_str()) else {
		return RValue::Int(None);
	};
	if field_type.contains("BooleanField") {
		RValue::Bool(None)
	} else if field_type.contains("BigIntegerField") {
		RValue::BigInt(None)
	} else if field_type.contains("IntegerField") {
		RValue::Int(None)
	} else if field_type.contains("FloatField") || field_type.contains("DecimalField") {
		RValue::Double(None)
	} else if field_type.contains("BinaryField") {
		RValue::Bytes(None)
	} else if field_type.contains("UuidField") || field_type.contains("UUIDField") {
		RValue::Uuid(None)
	} else {
		RValue::String(None)
	}
}

fn postgres_parameter_cast(value: &RValue) -> Option<&'static str> {
	match value {
		RValue::Decimal(_) | RValue::BigDecimal(_) => Some("numeric"),
		RValue::Uuid(_) => Some("uuid"),
		RValue::ChronoDateTimeUtc(_)
		| RValue::ChronoDateTimeLocal(_)
		| RValue::ChronoDateTimeWithTimeZone(_) => Some("timestamptz"),
		RValue::ChronoDateTime(_) => Some("timestamp"),
		RValue::ChronoDate(_) => Some("date"),
		RValue::ChronoTime(_) => Some("time"),
		RValue::Json(_) => Some("jsonb"),
		#[cfg(feature = "pgvector")]
		RValue::Vector(_) => Some("vector"),

		_ => None,
	}
}

fn postgres_array_literal(values: &[RValue]) -> Option<String> {
	let elements = values
		.iter()
		.map(postgres_array_element)
		.collect::<Option<Vec<_>>>()?;
	Some(format!("{{{}}}", elements.join(",")))
}

fn postgres_array_element(value: &RValue) -> Option<String> {
	match value {
		RValue::String(Some(value)) => Some(postgres_array_quote(value)),
		RValue::Int(Some(value)) => Some(value.to_string()),
		RValue::BigInt(Some(value)) => Some(value.to_string()),
		RValue::Bool(Some(value)) => Some(value.to_string()),
		RValue::Float(Some(value)) => Some(value.to_string()),
		RValue::Double(Some(value)) => Some(value.to_string()),
		RValue::Uuid(Some(value)) => Some(value.to_string()),
		RValue::String(None)
		| RValue::Int(None)
		| RValue::BigInt(None)
		| RValue::Bool(None)
		| RValue::Float(None)
		| RValue::Double(None)
		| RValue::Uuid(None) => Some("NULL".to_string()),
		_ => None,
	}
}

fn postgres_array_quote(value: &str) -> String {
	format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::orm::Manager;
	use crate::orm::json::Json;
	use rstest::*;
	use serde::{Deserialize, Serialize};
	use serial_test::serial;
	use sqlx::Any;
	use std::collections::HashMap;

	#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
	struct TestUser {
		id: Option<i64>,
		name: String,
		email: String,
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

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn new_fields() -> Self::Fields {
			TestUserFields
		}

		fn field_metadata() -> Vec<FieldInfo> {
			vec![
				test_field_info("id", "reinhardt.orm.models.BigIntegerField", false, true),
				test_field_info("name", "reinhardt.orm.models.CharField", false, false),
				test_field_info("email", "reinhardt.orm.models.CharField", false, false),
			]
		}
	}

	#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
	struct JsonScalarModel {
		id: Option<i64>,
		external_id: String,
		name_json: Json<String>,
		flag_json: Json<bool>,
		optional_json: Option<Json<serde_json::Value>>,
		publish_date: Option<String>,
	}

	#[derive(Debug, Clone)]
	struct JsonScalarModelFields;

	impl crate::orm::model::FieldSelector for JsonScalarModelFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for JsonScalarModel {
		type PrimaryKey = i64;
		type Fields = JsonScalarModelFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"json_scalar_models"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn new_fields() -> Self::Fields {
			JsonScalarModelFields
		}

		fn field_metadata() -> Vec<FieldInfo> {
			vec![
				test_field_info("id", "reinhardt.orm.models.BigIntegerField", false, true),
				test_field_info(
					"external_id",
					"reinhardt.orm.models.CharField",
					false,
					false,
				),
				test_field_info("name_json", "reinhardt.orm.models.JsonField", false, false),
				test_field_info("flag_json", "reinhardt.orm.models.JsonField", false, false),
				test_field_info(
					"optional_json",
					"reinhardt.orm.models.JsonField",
					true,
					false,
				),
				test_field_info(
					"publish_date",
					"reinhardt.orm.models.CharField",
					true,
					false,
				),
			]
		}

		fn field_is_none(&self, field_name: &str) -> bool {
			match field_name {
				"id" => self.id.is_none(),
				"optional_json" => self.optional_json.is_none(),
				"publish_date" => self.publish_date.is_none(),
				_ => false,
			}
		}
	}

	#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
	struct ArraySessionModel {
		id: Option<i64>,
		tags: Vec<String>,
	}

	#[derive(Debug, Clone)]
	struct ArraySessionModelFields;

	impl crate::orm::model::FieldSelector for ArraySessionModelFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for ArraySessionModel {
		type PrimaryKey = i64;
		type Fields = ArraySessionModelFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"array_session_models"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn new_fields() -> Self::Fields {
			ArraySessionModelFields
		}

		fn field_metadata() -> Vec<FieldInfo> {
			vec![
				test_field_info("id", "reinhardt.orm.models.BigIntegerField", false, true),
				typed_test_field_info(
					"tags",
					"reinhardt.orm.models.ArrayField",
					crate::orm::DatabaseStorageKind::Json,
				),
			]
		}
	}

	#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
	struct TemporalSessionModel {
		id: Option<i64>,
		published_on: chrono::NaiveDate,
		starts_at: chrono::NaiveTime,
		published_at: chrono::DateTime<chrono::Utc>,
	}

	#[derive(Debug, Clone)]
	struct TemporalSessionModelFields;

	impl crate::orm::model::FieldSelector for TemporalSessionModelFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for TemporalSessionModel {
		type PrimaryKey = i64;
		type Fields = TemporalSessionModelFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"temporal_session_models"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn new_fields() -> Self::Fields {
			TemporalSessionModelFields
		}

		fn field_metadata() -> Vec<FieldInfo> {
			vec![
				test_field_info("id", "reinhardt.orm.models.BigIntegerField", false, true),
				typed_test_field_info(
					"published_on",
					"reinhardt.orm.models.DateField",
					crate::orm::DatabaseStorageKind::Date,
				),
				typed_test_field_info(
					"starts_at",
					"reinhardt.orm.models.TimeField",
					crate::orm::DatabaseStorageKind::Time,
				),
				typed_test_field_info(
					"published_at",
					"reinhardt.orm.models.DateTimeField",
					crate::orm::DatabaseStorageKind::DateTime,
				),
			]
		}
	}

	#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
	struct DecimalSessionModel {
		id: Option<i64>,
		amount: rust_decimal::Decimal,
	}

	#[derive(Debug, Clone)]
	struct DecimalSessionModelFields;

	impl crate::orm::model::FieldSelector for DecimalSessionModelFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for DecimalSessionModel {
		type PrimaryKey = i64;
		type Fields = DecimalSessionModelFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"decimal_session_models"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn new_fields() -> Self::Fields {
			DecimalSessionModelFields
		}

		fn field_metadata() -> Vec<FieldInfo> {
			vec![
				test_field_info("id", "reinhardt.orm.models.BigIntegerField", false, true),
				typed_test_field_info(
					"amount",
					"reinhardt.orm.models.DecimalField",
					crate::orm::DatabaseStorageKind::Decimal,
				),
			]
		}
	}

	fn test_field_info(
		name: &str,
		field_type: &str,
		nullable: bool,
		primary_key: bool,
	) -> FieldInfo {
		FieldInfo {
			name: name.to_string(),
			field_type: field_type.to_string(),
			storage_kind: None,
			domain: None,
			nullable,
			primary_key,
			unique: false,
			blank: false,
			editable: true,
			default: None,
			db_default: None,
			db_column: None,
			choices: None,
			attributes: HashMap::new(),
		}
	}

	fn typed_test_field_info(
		name: &str,
		field_type: &str,
		storage_kind: crate::orm::DatabaseStorageKind,
	) -> FieldInfo {
		let mut field = test_field_info(name, field_type, false, false);
		field.storage_kind = Some(storage_kind);
		field
	}

	#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
	struct GeneratedOnlyUser {
		id: Option<i64>,
		full_name: String,
	}

	#[derive(Debug, Clone)]
	struct GeneratedOnlyUserFields;

	impl crate::orm::model::FieldSelector for GeneratedOnlyUserFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for GeneratedOnlyUser {
		type PrimaryKey = i64;
		type Fields = GeneratedOnlyUserFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"generated_only_users"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn new_fields() -> Self::Fields {
			GeneratedOnlyUserFields
		}

		fn generated_field_names() -> &'static [&'static str] {
			&["full_name"]
		}
	}

	// Create test pool using SQLite in-memory database
	async fn create_test_pool() -> Arc<AnyPool> {
		use sqlx::pool::PoolOptions;

		// Initialize SQLx drivers (idempotent operation)
		sqlx::any::install_default_drivers();

		// Use shared in-memory database so all connections see the same data
		// The "mode=memory" and "cache=shared" ensure the database persists across connections
		let pool = PoolOptions::<Any>::new()
			.min_connections(1)
			.max_connections(5)
			.connect("sqlite:file:test_session_db?mode=memory&cache=shared")
			.await
			.expect("Failed to create test pool");

		// Create the users table for testing
		sqlx::query(
			"CREATE TABLE IF NOT EXISTS users (
				id INTEGER PRIMARY KEY,
				name TEXT NOT NULL,
				email TEXT NOT NULL
			)",
		)
		.execute(&pool)
		.await
		.expect("Failed to create users table");

		Arc::new(pool)
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn list_executes_queryset_filters_on_the_session_pool() {
		// Arrange
		let pool = create_test_pool().await;
		sqlx::query("DELETE FROM users")
			.execute(&*pool)
			.await
			.expect("test rows should be cleared");
		for (id, name) in [(1_i64, "outside"), (2_i64, "inside")] {
			sqlx::query("INSERT INTO users (id, name, email) VALUES (?, ?, ?)")
				.bind(id)
				.bind(name)
				.bind(format!("{name}@example.com"))
				.execute(&*pool)
				.await
				.expect("test row should be inserted");
		}
		let session = Session::new(pool, DbBackend::Sqlite)
			.await
			.expect("session should use the configured pool");
		let queryset = QuerySet::<TestUser>::new()
			.filter(crate::orm::query::Filter::new(
				"id",
				crate::orm::query::FilterOperator::Eq,
				crate::orm::query::FilterValue::Integer(2),
			))
			.limit(1);

		// Act
		let users = session
			.list(&queryset)
			.await
			.expect("session list should execute the queryset");

		// Assert
		assert_eq!(
			users,
			vec![TestUser {
				id: Some(2),
				name: "inside".to_owned(),
				email: "inside@example.com".to_owned(),
			}]
		);
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn list_returns_empty_without_querying_none_queryset(_init_drivers: ()) {
		// Arrange
		let pool = Arc::new(
			sqlx::pool::PoolOptions::<Any>::new()
				.max_connections(1)
				.connect("sqlite::memory:")
				.await
				.expect("test pool should initialize"),
		);
		let session = Session::new(pool, DbBackend::Sqlite)
			.await
			.expect("session should initialize");

		// Act
		let users = session
			.list(&QuerySet::<TestUser>::new().none())
			.await
			.expect("none queryset should not access the missing table");

		// Assert
		assert_eq!(users, Vec::<TestUser>::new());
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn list_with_connection_for_update_rejects_unsupported_backend(_init_drivers: ()) {
		// Arrange
		let pool = Arc::new(
			sqlx::pool::PoolOptions::<Any>::new()
				.max_connections(1)
				.connect("sqlite::memory:")
				.await
				.expect("test pool should initialize"),
		);
		let session = Session::new(pool.clone(), DbBackend::Sqlite)
			.await
			.expect("session should initialize");
		let mut connection = pool.acquire().await.expect("connection should acquire");

		// Act
		let result = session
			.list_with_connection_for_update(&QuerySet::<TestUser>::new(), &mut connection)
			.await;

		// Assert
		assert_eq!(
			result,
			Err(SessionError::DatabaseError(
				"SELECT FOR UPDATE is not supported by this backend or server version".to_owned(),
			))
		);
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn list_preserves_temporal_field_precision(_init_drivers: ()) {
		// Arrange
		let pool = create_temporal_test_pool().await;
		sqlx::query(
			"INSERT INTO temporal_session_models \
			 (id, published_on, starts_at, published_at) \
			 VALUES (1, '2026-07-18', '08:09:10.123456', '2026-07-18T08:09:10.123456Z')",
		)
		.execute(&*pool)
		.await
		.expect("temporal row should insert");
		let session = Session::new(pool, DbBackend::Sqlite)
			.await
			.expect("session should initialize");

		// Act
		let rows = session
			.list(&QuerySet::<TemporalSessionModel>::new())
			.await
			.expect("session list should preserve temporal values");

		// Assert
		assert_eq!(
			rows,
			vec![TemporalSessionModel {
				id: Some(1),
				published_on: chrono::NaiveDate::from_ymd_opt(2026, 7, 18)
					.expect("date should be valid"),
				starts_at: chrono::NaiveTime::from_hms_micro_opt(8, 9, 10, 123_456)
					.expect("time should be valid"),
				published_at: chrono::DateTime::parse_from_rfc3339("2026-07-18T08:09:10.123456Z",)
					.expect("timestamp should be valid")
					.with_timezone(&chrono::Utc),
			}]
		);
	}

	async fn create_json_scalar_test_pool() -> Arc<AnyPool> {
		use sqlx::pool::PoolOptions;

		sqlx::any::install_default_drivers();

		let pool = PoolOptions::<Any>::new()
			.min_connections(1)
			.max_connections(5)
			.connect("sqlite:file:test_session_json_scalar_db?mode=memory&cache=shared")
			.await
			.expect("Failed to create JSON scalar test pool");

		sqlx::query(
			"CREATE TABLE IF NOT EXISTS json_scalar_models (
				id INTEGER PRIMARY KEY,
				external_id TEXT NOT NULL,
				name_json TEXT NOT NULL,
				flag_json TEXT NOT NULL,
				optional_json TEXT NULL,
				publish_date TEXT NULL
			)",
		)
		.execute(&pool)
		.await
		.expect("Failed to create json_scalar_models table");

		sqlx::query("DELETE FROM json_scalar_models")
			.execute(&pool)
			.await
			.expect("Failed to clear json_scalar_models table");

		Arc::new(pool)
	}

	async fn create_array_session_test_pool() -> Arc<AnyPool> {
		use sqlx::pool::PoolOptions;

		sqlx::any::install_default_drivers();

		let pool = PoolOptions::<Any>::new()
			.min_connections(1)
			.max_connections(5)
			.connect("sqlite:file:test_session_array_db?mode=memory&cache=shared")
			.await
			.expect("Failed to create array session test pool");

		sqlx::query(
			"CREATE TABLE IF NOT EXISTS array_session_models (
				id INTEGER PRIMARY KEY,
				tags TEXT NOT NULL
			)",
		)
		.execute(&pool)
		.await
		.expect("Failed to create array_session_models table");

		sqlx::query("DELETE FROM array_session_models")
			.execute(&pool)
			.await
			.expect("Failed to clear array_session_models table");

		Arc::new(pool)
	}

	async fn create_temporal_test_pool() -> Arc<AnyPool> {
		use sqlx::pool::PoolOptions;

		sqlx::any::install_default_drivers();

		let pool = PoolOptions::<Any>::new()
			.min_connections(1)
			.max_connections(5)
			.connect("sqlite:file:test_session_temporal_db?mode=memory&cache=shared")
			.await
			.expect("Failed to create temporal test pool");

		sqlx::query(
			"CREATE TABLE IF NOT EXISTS temporal_session_models (
				id INTEGER PRIMARY KEY,
				published_on TEXT NOT NULL,
				starts_at TEXT NOT NULL,
				published_at TEXT NOT NULL
			)",
		)
		.execute(&pool)
		.await
		.expect("Failed to create temporal_session_models table");

		sqlx::query("DELETE FROM temporal_session_models")
			.execute(&pool)
			.await
			.expect("Failed to clear temporal_session_models table");

		Arc::new(pool)
	}

	async fn create_decimal_session_test_pool() -> Arc<AnyPool> {
		use sqlx::pool::PoolOptions;

		sqlx::any::install_default_drivers();

		let pool = PoolOptions::<Any>::new()
			.min_connections(1)
			.max_connections(5)
			.connect("sqlite:file:test_session_decimal_db?mode=memory&cache=shared")
			.await
			.expect("Failed to create decimal session test pool");

		sqlx::query(
			"CREATE TABLE IF NOT EXISTS decimal_session_models (
				id INTEGER PRIMARY KEY,
				amount TEXT NOT NULL
			)",
		)
		.execute(&pool)
		.await
		.expect("Failed to create decimal_session_models table");

		sqlx::query("DELETE FROM decimal_session_models")
			.execute(&pool)
			.await
			.expect("Failed to clear decimal_session_models table");

		Arc::new(pool)
	}

	/// Initialize SQLx drivers (required for AnyPool)
	#[fixture]
	fn init_drivers() {
		sqlx::any::install_default_drivers();
	}

	#[tokio::test]

	async fn test_session_creation() {
		let pool = create_test_pool().await;
		let session = Session::new(pool, DbBackend::Sqlite).await;

		let session = session.unwrap();
		assert!(!session.is_closed());
		assert_eq!(session.identity_count(), 0);
		assert_eq!(session.dirty_count(), 0);
	}

	#[tokio::test]

	async fn test_session_add_object() {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool, DbBackend::Sqlite).await.unwrap();

		let user = TestUser {
			id: Some(1),
			name: "Alice".to_string(),
			email: "alice@example.com".to_string(),
		};

		let result = session.add(user).await;
		assert!(result.is_ok());
		assert_eq!(session.identity_count(), 1);
		assert_eq!(session.dirty_count(), 1);
	}

	#[tokio::test]

	async fn test_session_get_from_identity_map() {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool, DbBackend::Sqlite).await.unwrap();

		let user = TestUser {
			id: Some(1),
			name: "Bob".to_string(),
			email: "bob@example.com".to_string(),
		};

		session.add(user.clone()).await.unwrap();

		let retrieved: Option<TestUser> = session.get(1).await.unwrap();
		assert!(retrieved.is_some());
		assert_eq!(retrieved.unwrap(), user);
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_session_flush_clears_dirty(_init_drivers: ()) {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool, DbBackend::Sqlite).await.unwrap();

		let user = TestUser {
			id: Some(1),
			name: "Charlie".to_string(),
			email: "charlie@example.com".to_string(),
		};

		session.add(user).await.unwrap();
		assert_eq!(session.dirty_count(), 1);

		session.flush().await.unwrap();
		assert_eq!(session.dirty_count(), 0);
		assert_eq!(session.identity_count(), 1);
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_session_flush_generated_only_update_is_noop(_init_drivers: ()) {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool, DbBackend::Sqlite).await.unwrap();

		session
			.add(GeneratedOnlyUser {
				id: Some(7),
				full_name: "Computed".to_string(),
			})
			.await
			.unwrap();
		assert_eq!(session.dirty_count(), 1);

		session.flush().await.unwrap();

		assert_eq!(session.dirty_count(), 0);
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_session_flush_generated_only_insert_errors(_init_drivers: ()) {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool, DbBackend::Sqlite).await.unwrap();

		session
			.add(GeneratedOnlyUser {
				id: None,
				full_name: "Computed".to_string(),
			})
			.await
			.unwrap();

		let error = session
			.flush()
			.await
			.expect_err("generated-only insert should fail before rendering empty SQL");

		assert_eq!(
			error.to_string(),
			"Flush error: Cannot insert generated_only_users because no writable fields remain after filtering generated and defaulted columns"
		);
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_session_delete_object(_init_drivers: ()) {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool, DbBackend::Sqlite).await.unwrap();

		let user = TestUser {
			id: Some(1),
			name: "Dave".to_string(),
			email: "dave@example.com".to_string(),
		};

		session.add(user.clone()).await.unwrap();
		session.flush().await.unwrap();

		session.delete(user).await.unwrap();
		session.flush().await.unwrap();

		let retrieved: Option<TestUser> = session.get(1).await.unwrap();
		assert!(retrieved.is_none());
	}

	#[tokio::test]

	async fn test_session_close() {
		let pool = create_test_pool().await;
		let session = Session::new(pool, DbBackend::Sqlite).await.unwrap();

		assert!(!session.is_closed());

		session.close().await.unwrap();
	}

	#[tokio::test]

	async fn test_session_operations_after_close() {
		let pool = create_test_pool().await;
		let session = Session::new(pool, DbBackend::Sqlite).await.unwrap();

		let _user = TestUser {
			id: Some(1),
			name: "Grace".to_string(),
			email: "grace@example.com".to_string(),
		};

		session.close().await.unwrap();

		// Cannot use session after close since it consumes self
		// This test verifies the API design
	}

	#[tokio::test]

	async fn test_session_multiple_objects() {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool, DbBackend::Sqlite).await.unwrap();

		for i in 1..=5 {
			let user = TestUser {
				id: Some(i),
				name: format!("User{}", i),
				email: format!("user{}@example.com", i),
			};
			session.add(user).await.unwrap();
		}

		assert_eq!(session.identity_count(), 5);
		assert_eq!(session.dirty_count(), 5);
	}

	#[tokio::test]

	async fn test_session_delete_removes_from_dirty() {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool, DbBackend::Sqlite).await.unwrap();

		let user = TestUser {
			id: Some(1),
			name: "Henry".to_string(),
			email: "henry@example.com".to_string(),
		};

		session.add(user.clone()).await.unwrap();
		assert_eq!(session.dirty_count(), 1);

		session.delete(user).await.unwrap();
		assert_eq!(session.dirty_count(), 0);
	}

	#[tokio::test]

	async fn test_session_query_creation() {
		let pool = create_test_pool().await;
		let session = Session::new(pool, DbBackend::Sqlite).await.unwrap();

		let _query = session.query::<TestUser>();
	}

	#[tokio::test]
	async fn test_session_add_without_pk_succeeds() {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool, DbBackend::Sqlite).await.unwrap();

		let user = TestUser {
			id: None,
			name: "NewUser".to_string(),
			email: "newuser@example.com".to_string(),
		};

		// Objects without PK can be added (for INSERT operations)
		let result = session.add(user).await;
		assert!(result.is_ok());
	}

	// ──────────────────────────────────────────────────────────────
	// Additional session tests - SessionError Display
	// ──────────────────────────────────────────────────────────────

	#[test]
	fn test_session_error_database_error_display() {
		let err = SessionError::DatabaseError("connection failed".to_string());
		assert_eq!(err.to_string(), "Database error: connection failed");
	}

	#[test]
	fn test_session_error_object_not_found_display() {
		let err = SessionError::ObjectNotFound("user:123".to_string());
		assert_eq!(err.to_string(), "Object not found: user:123");
	}

	#[test]
	fn test_session_error_serialization_error_display() {
		let err = SessionError::SerializationError("invalid json".to_string());
		assert_eq!(err.to_string(), "Serialization error: invalid json");
	}

	#[test]
	fn test_session_error_invalid_state_display() {
		let err = SessionError::InvalidState("session closed".to_string());
		assert_eq!(err.to_string(), "Invalid state: session closed");
	}

	#[test]
	fn test_session_error_flush_error_display() {
		let err = SessionError::FlushError("failed to write".to_string());
		assert_eq!(err.to_string(), "Flush error: failed to write");
	}

	#[test]
	fn test_session_error_debug() {
		let err = SessionError::DatabaseError("test".to_string());
		let debug_str = format!("{:?}", err);
		assert!(debug_str.contains("DatabaseError"));
		assert!(debug_str.contains("test"));
	}

	#[test]
	fn test_session_error_clone() {
		let err = SessionError::ObjectNotFound("key".to_string());
		let cloned = err.clone();
		assert_eq!(err.to_string(), cloned.to_string());
	}

	#[test]
	fn test_session_error_is_std_error() {
		let err: Box<dyn std::error::Error> =
			Box::new(SessionError::DatabaseError("test".to_string()));
		assert!(err.to_string().contains("Database error"));
	}

	#[test]
	fn test_parse_json_field_text_error_includes_context() {
		let err = super::parse_json_field_text(
			"{invalid",
			"writing_projects",
			"writing_projects:id=7",
			"style_settings",
			"style_settings",
		)
		.unwrap_err();

		match err {
			SessionError::SerializationError(message) => {
				let json_error = serde_json::from_str::<Value>("{invalid").unwrap_err();
				assert_eq!(
					message,
					format!(
						"Failed to hydrate JSON field writing_projects.style_settings \
						 for row writing_projects:id=7 from column 'style_settings': {json_error}"
					)
				);
			}
			other => panic!("expected serialization error, got {other:?}"),
		}
	}

	#[test]
	fn test_should_skip_flush_column_keeps_regular_id_suffix_fields() {
		let field = test_field_info(
			"external_id",
			"reinhardt.orm.models.CharField",
			false,
			false,
		);

		assert!(!super::should_skip_flush_column(
			"external_id",
			"external_id",
			Some(&field)
		));
	}

	#[test]
	fn test_should_skip_flush_column_skips_relation_managed_fields() {
		let mut field = test_field_info(
			"author_id",
			"reinhardt.orm.models.IntegerField",
			false,
			false,
		);
		field.attributes.insert(
			"relation_managed".to_string(),
			crate::orm::fields::FieldKwarg::Bool(true),
		);

		assert!(super::should_skip_flush_column(
			"author_id",
			"author_id",
			Some(&field)
		));
	}

	#[test]
	fn test_should_skip_flush_column_keeps_relation_id_storage_fields() {
		let mut field = test_field_info(
			"author_id",
			"reinhardt.orm.models.IntegerField",
			false,
			false,
		);
		field.attributes.insert(
			"relation_managed".to_string(),
			crate::orm::fields::FieldKwarg::Bool(true),
		);
		field.attributes.insert(
			"fk_id_field".to_string(),
			crate::orm::fields::FieldKwarg::Bool(true),
		);

		assert!(!super::should_skip_flush_column(
			"author_id",
			"author_id",
			Some(&field)
		));
	}

	// ──────────────────────────────────────────────────────────────
	// json_to_reinhardt_query_value tests
	// ──────────────────────────────────────────────────────────────

	#[test]
	fn test_json_to_reinhardt_query_value_string() {
		use serde_json::json;
		let value = json!("hello world");
		let rq_value = super::json_to_reinhardt_query_value(&value);

		let debug_str = format!("{:?}", rq_value);
		assert!(debug_str.contains("hello world") || debug_str.contains("String"));
	}

	#[test]
	fn test_json_to_reinhardt_query_value_integer() {
		use serde_json::json;
		let value = json!(42);
		let rq_value = super::json_to_reinhardt_query_value(&value);

		let debug_str = format!("{:?}", rq_value);
		assert!(debug_str.contains("42") || debug_str.contains("Int"));
	}

	#[test]
	fn test_json_to_reinhardt_query_value_float() {
		use serde_json::json;
		let value = json!(2.5);
		let rq_value = super::json_to_reinhardt_query_value(&value);

		let debug_str = format!("{:?}", rq_value);
		assert!(debug_str.contains("2.5") || debug_str.contains("Double"));
	}

	#[test]
	fn test_json_to_reinhardt_query_value_bool_true() {
		use serde_json::json;
		let value = json!(true);
		let rq_value = super::json_to_reinhardt_query_value(&value);

		let debug_str = format!("{:?}", rq_value);
		assert!(debug_str.contains("true") || debug_str.contains("Bool"));
	}

	#[test]
	fn test_json_to_reinhardt_query_value_bool_false() {
		use serde_json::json;
		let value = json!(false);
		let rq_value = super::json_to_reinhardt_query_value(&value);

		let debug_str = format!("{:?}", rq_value);
		assert!(debug_str.contains("false") || debug_str.contains("Bool"));
	}

	#[test]
	fn test_json_to_reinhardt_query_value_null() {
		use serde_json::json;
		let value = json!(null);
		let rq_value = super::json_to_reinhardt_query_value(&value);

		// Should produce some value (null representation)
		let debug_str = format!("{:?}", rq_value);
		assert!(!debug_str.is_empty());
	}

	#[test]
	fn test_json_to_reinhardt_query_value_array() {
		use serde_json::json;
		let value = json!([1, 2, 3]);
		let rq_value = super::json_to_reinhardt_query_value(&value);

		let debug_str = format!("{:?}", rq_value);
		assert!(debug_str.contains("Json"));
	}

	#[test]
	fn test_json_to_reinhardt_query_value_object() {
		use serde_json::json;
		let value = json!({"name": "test", "count": 42});
		let rq_value = super::json_to_reinhardt_query_value(&value);

		let debug_str = format!("{:?}", rq_value);
		assert!(debug_str.contains("Json"));
	}

	#[test]
	fn test_json_field_scalar_values_bind_as_json() {
		use serde_json::json;

		let field = test_field_info("name_json", "reinhardt.orm.models.JsonField", false, false);
		let value = json!("draft");
		let rq_value = super::json_to_reinhardt_query_value_for_field(&value, Some(&field), false);

		assert!(matches!(rq_value, RValue::Json(Some(json)) if *json == value));
	}

	#[test]
	fn test_non_json_scalar_values_keep_primitive_binding() {
		use serde_json::json;

		let field = test_field_info("name", "reinhardt.orm.models.CharField", false, false);
		let value = json!("draft");
		let rq_value = super::json_to_reinhardt_query_value_for_field(&value, Some(&field), false);

		assert!(matches!(rq_value, RValue::String(Some(text)) if text.as_ref() == "draft"));
	}

	#[test]
	fn test_non_nullable_json_null_binds_as_json_null() {
		use serde_json::json;

		let field = test_field_info("payload", "reinhardt.orm.models.JsonField", false, false);
		let value = json!(null);
		let rq_value = super::json_to_reinhardt_query_value_for_field(&value, Some(&field), false);

		assert!(matches!(rq_value, RValue::Json(Some(json)) if json.is_null()));
	}

	#[test]
	fn test_nullable_json_none_binds_as_sql_null() {
		use serde_json::json;

		let field = test_field_info("payload", "reinhardt.orm.models.JsonField", true, false);
		let value = json!(null);
		let rq_value = super::json_to_reinhardt_query_value_for_field(&value, Some(&field), true);

		assert!(matches!(rq_value, RValue::Json(None)));
	}

	#[rstest]
	fn test_nullable_non_json_null_uses_field_specific_rvalue() {
		use serde_json::json;

		let char_field = test_field_info("nickname", "reinhardt.orm.models.CharField", true, false);
		let bool_field =
			test_field_info("enabled", "reinhardt.orm.models.BooleanField", true, false);
		let bigint_field = test_field_info(
			"counter",
			"reinhardt.orm.models.BigIntegerField",
			true,
			false,
		);
		let float_field = test_field_info("ratio", "reinhardt.orm.models.FloatField", true, false);

		assert!(matches!(
			super::json_to_reinhardt_query_value_for_field(&json!(null), Some(&char_field), false,),
			RValue::String(None)
		));
		assert!(matches!(
			super::json_to_reinhardt_query_value_for_field(&json!(null), Some(&bool_field), false,),
			RValue::Bool(None)
		));
		assert!(matches!(
			super::json_to_reinhardt_query_value_for_field(
				&json!(null),
				Some(&bigint_field),
				false,
			),
			RValue::BigInt(None)
		));
		assert!(matches!(
			super::json_to_reinhardt_query_value_for_field(&json!(null), Some(&float_field), false,),
			RValue::Double(None)
		));
	}

	#[rstest]
	fn postgres_json_casts_and_nulls_preserve_final_slots() {
		// Arrange
		let statement = RQuery::update()
			.table("items")
			.value(
				"payload",
				RValue::Json(Some(Box::new(serde_json::json!({"stage": "draft' $10"})))),
			)
			.value("optional", RValue::Json(None))
			.value("name", "bound' $99")
			.and_where(Expr::col("id").eq(7))
			.to_owned();
		// Act
		let (sql, values) = build_any_update(&statement, DbBackend::Postgres).unwrap();
		// Assert
		assert_eq!(
			sql,
			"UPDATE \"items\" SET \"payload\" = CAST($1 AS \"jsonb\"), \"optional\" = NULL, \"name\" = $2 WHERE \"id\" = $3"
		);
		assert_eq!(
			values.0,
			vec![
				RValue::Json(Some(Box::new(serde_json::json!({"stage": "draft' $10"})))),
				"bound' $99".into(),
				7.into()
			]
		);
	}

	#[rstest]
	fn postgres_temporal_and_uuid_parameters_are_structural() {
		// Arrange
		let date = chrono::NaiveDate::from_ymd_opt(2026, 7, 18).unwrap();
		let time = chrono::NaiveTime::from_hms_micro_opt(8, 9, 10, 123_456).unwrap();
		let naive = date.and_time(time);
		let values = vec![
			RValue::from(Uuid::nil()),
			RValue::from(naive.and_utc()),
			RValue::from(naive),
			RValue::from(date),
			RValue::from(time),
		];
		let mut statement = RQuery::select();
		for value in &values {
			statement.expr(Expr::val(value.clone()));
		}
		// Act
		let (sql, actual) = build_any_select(&statement, DbBackend::Postgres).unwrap();
		// Assert
		assert_eq!(
			sql,
			"SELECT CAST($1 AS \"uuid\"), CAST($2 AS \"timestamptz\"), CAST($3 AS \"timestamp\"), CAST($4 AS \"date\"), CAST($5 AS \"time\")"
		);
		assert_eq!(actual.0, values);
	}

	#[cfg(feature = "pgvector")]
	#[rstest]
	fn postgres_vectors_are_structural_and_null_does_not_bind() {
		// Arrange
		let statement = RQuery::update()
			.table("items")
			.value(
				"embedding",
				RValue::Vector(Some(Box::new(vec![1.0, 2.0, 3.0]))),
			)
			.value("optional_embedding", RValue::Vector(None))
			.to_owned();
		// Act
		let (sql, values) = build_any_update(&statement, DbBackend::Postgres).unwrap();
		// Assert
		assert_eq!(
			sql,
			"UPDATE \"items\" SET \"embedding\" = CAST($1 AS \"vector\"), \"optional_embedding\" = NULL"
		);
		assert_eq!(values.0, vec!["[1.0,2.0,3.0]".into()]);
	}

	#[rstest]
	fn postgres_arrays_are_structural_with_declared_catalog_types() {
		use reinhardt_query::ArrayType;
		// Arrange
		let statement = RQuery::update()
			.table("items")
			.value(
				"labels",
				RValue::Array(
					ArrayType::String,
					Some(Box::new(vec!["alpha".into(), RValue::Int(None)])),
				),
			)
			.value(
				"ranks",
				RValue::Array(ArrayType::Int, Some(Box::new(vec![7.into()]))),
			)
			.value(
				"owner_ids",
				RValue::Array(ArrayType::Uuid, Some(Box::new(vec![Uuid::nil().into()]))),
			)
			.to_owned();
		// Act
		let (sql, values) = build_any_update(&statement, DbBackend::Postgres).unwrap();
		// Assert
		assert_eq!(
			sql,
			"UPDATE \"items\" SET \"labels\" = CAST($1 AS \"_text\"), \"ranks\" = CAST($2 AS \"_int4\"), \"owner_ids\" = CAST($3 AS \"_uuid\")"
		);
		assert_eq!(
			values.0,
			vec![
				"{\"alpha\",NULL}".into(),
				"{7}".into(),
				"{00000000-0000-0000-0000-000000000000}".into()
			]
		);
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_session_binds_postgres_arrays_as_native_arrays() {
		use reinhardt_query::ArrayType;
		use testcontainers::{GenericImage, ImageExt, core::WaitFor, runners::AsyncRunner};
		// Arrange
		let container = GenericImage::new("postgres", "17-alpine")
			.with_wait_for(WaitFor::message_on_stderr(
				"database system is ready to accept connections",
			))
			.with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust")
			.start()
			.await
			.unwrap();
		let port = container.get_host_port_ipv4(5432).await.unwrap();
		sqlx::any::install_default_drivers();
		let pool = sqlx::pool::PoolOptions::<sqlx::Any>::new()
			.max_connections(1)
			.connect(&format!("postgres://postgres@localhost:{port}/postgres"))
			.await
			.unwrap();
		let array = RValue::Array(
			ArrayType::String,
			Some(Box::new(vec!["alpha".into(), "beta".into()])),
		);
		let statement = RQuery::select()
			.expr_as(
				projection_function(
					"ARRAY_TO_STRING",
					vec![
						Expr::val(array).into_simple_expr(),
						Expr::val(",").into_simple_expr(),
					],
				),
				"joined",
			)
			.to_owned();
		// Act
		let (sql, arguments) = prepare_any_select(&statement, DbBackend::Postgres)
			.unwrap()
			.into_parts();
		let row = sqlx::query_with(&sql, arguments)
			.fetch_one(&pool)
			.await
			.unwrap();
		// Assert
		assert_eq!(row.try_get::<String, _>("joined").unwrap(), "alpha,beta");
	}

	#[rstest]
	#[case(DbBackend::Postgres)]
	#[case(DbBackend::Mysql)]
	#[case(DbBackend::Sqlite)]
	fn any_array_type_mismatch_is_redacted(#[case] backend: DbBackend) {
		// Arrange
		let statement = RQuery::select()
			.expr(Expr::val(RValue::Array(
				reinhardt_query::ArrayType::Int,
				Some(Box::new(vec!["sensitive' data".into()])),
			)))
			.to_owned();
		// Act
		let error = prepare_any_select(&statement, backend).err().unwrap();
		// Assert
		assert_eq!(
			error.to_string(),
			format!(
				"Database error: Any {backend:?} array element 1 does not match declared Int type"
			)
		);
		assert!(!error.to_string().contains("sensitive"));
	}

	#[rstest]
	#[case(
		DbBackend::Mysql,
		reinhardt_query::ArrayType::Float,
		RValue::Float(Some(f32::NAN)),
		"Float"
	)]
	#[case(
		DbBackend::Sqlite,
		reinhardt_query::ArrayType::Float,
		RValue::Float(Some(f32::NAN)),
		"Float"
	)]
	#[case(
		DbBackend::Mysql,
		reinhardt_query::ArrayType::Double,
		RValue::Double(Some(f64::INFINITY)),
		"Double"
	)]
	#[case(
		DbBackend::Sqlite,
		reinhardt_query::ArrayType::Double,
		RValue::Double(Some(f64::INFINITY)),
		"Double"
	)]
	fn any_json_array_rejects_nonfinite_elements(
		#[case] backend: DbBackend,
		#[case] kind: reinhardt_query::ArrayType,
		#[case] value: RValue,
		#[case] name: &str,
	) {
		// Arrange
		let statement = RQuery::select()
			.expr(Expr::val(RValue::Array(kind, Some(Box::new(vec![value])))))
			.to_owned();
		// Act
		let error = prepare_any_select(&statement, backend).err().unwrap();
		// Assert: a nonfinite number must not silently become JSON null.
		assert_eq!(
			error.to_string(),
			format!(
				"Database error: Any {backend:?} {name} array element 1 cannot be represented as finite JSON"
			)
		);
	}

	#[cfg(feature = "pgvector")]
	#[rstest]
	#[case(DbBackend::Postgres)]
	#[case(DbBackend::Mysql)]
	#[case(DbBackend::Sqlite)]
	fn any_nonfinite_vector_is_rejected(#[case] backend: DbBackend) {
		// Arrange
		let statement = RQuery::select()
			.expr(Expr::val(RValue::Vector(Some(Box::new(vec![f32::NAN])))))
			.to_owned();
		// Act
		let error = prepare_any_select(&statement, backend).err().unwrap();
		// Assert
		assert_eq!(
			error.to_string(),
			"Database error: Any vector values must be finite"
		);
	}

	#[test]
	fn test_json_to_reinhardt_query_value_negative_integer() {
		use serde_json::json;
		let value = json!(-100);
		let rq_value = super::json_to_reinhardt_query_value(&value);

		let debug_str = format!("{:?}", rq_value);
		assert!(debug_str.contains("-100") || debug_str.contains("Int"));
	}

	#[test]
	fn test_json_to_reinhardt_query_value_large_integer() {
		use serde_json::json;
		let value = json!(9223372036854775807i64); // i64::MAX
		let rq_value = super::json_to_reinhardt_query_value(&value);

		// Should handle large integers
		let debug_str = format!("{:?}", rq_value);
		assert!(!debug_str.is_empty());
	}

	// ──────────────────────────────────────────────────────────────
	// DbBackend tests
	// ──────────────────────────────────────────────────────────────

	#[tokio::test]
	async fn test_session_get_backend() {
		let pool = create_test_pool().await;
		let session = Session::new(pool, DbBackend::Sqlite).await.unwrap();

		assert_eq!(session.get_backend(), DbBackend::Sqlite);
	}

	// ──────────────────────────────────────────────────────────────
	// Generated unsigned argument boundary tests exercise the real companion encoder.

	#[rstest]
	#[case(42)]
	#[case(i64::MAX as u64)]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn any_unsigned_values_keep_their_exact_signed_range(#[case] value: u64) {
		// Arrange
		let pool = create_test_pool().await;
		let statement = RQuery::select()
			.expr_as(Expr::val(RValue::BigUnsigned(Some(value))), "value")
			.to_owned();
		// Act
		let (sql, arguments) = prepare_any_select(&statement, DbBackend::Sqlite)
			.unwrap()
			.into_parts();
		let row = sqlx::query_with(&sql, arguments)
			.fetch_one(&*pool)
			.await
			.unwrap();
		// Assert
		assert_eq!(
			row.try_get::<i64, _>("value").unwrap(),
			i64::try_from(value).unwrap()
		);
	}

	#[rstest]
	#[case((i64::MAX as u64) + 1)]
	#[case(u64::MAX)]
	fn any_unsigned_overflow_fails_without_clamping_or_values(#[case] value: u64) {
		// Arrange
		let statement = RQuery::select()
			.expr(Expr::val(RValue::BigUnsigned(Some(value))))
			.to_owned();
		// Act
		let error = prepare_any_select(&statement, DbBackend::Sqlite)
			.err()
			.unwrap();
		// Assert
		assert_eq!(
			error.to_string(),
			"Database error: cannot encode BigUnsigned argument 1 for sqlite/any: unsigned integer exceeds signed 64-bit range"
		);
		assert!(!error.to_string().contains(&value.to_string()));
	}

	#[rstest]
	fn test_insert_values_error_maps_to_flush_error() {
		// Arrange
		// Create an InsertStatement with 2 columns but provide 1 value to trigger mismatch error
		let mut insert_stmt = RQuery::insert()
			.into_table(Alias::new("test_table"))
			.to_owned();
		insert_stmt.columns(vec![Alias::new("col_a"), Alias::new("col_b")]);
		let mismatched_values = vec![RValue::String(Some(Box::new("only_one".to_string())))];

		// Act
		let result: Result<(), SessionError> = insert_stmt
			.values(mismatched_values)
			.map(|_| ())
			.map_err(|e| SessionError::FlushError(format!("Failed to build INSERT values: {}", e)));

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert!(
			matches!(err, SessionError::FlushError(ref msg) if msg.contains("Failed to build INSERT values"))
		);
		assert!(err.to_string().contains("Flush error:"));
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_session_flush_insert_new_object_without_pk(_init_drivers: ()) {
		// Arrange
		// Test flush with a new object (no primary key) to exercise the INSERT path
		let pool = create_test_pool().await;
		let mut session = Session::new(pool, DbBackend::Sqlite).await.unwrap();

		let user = TestUser {
			id: None,
			name: "NewUser".to_string(),
			email: "newuser@example.com".to_string(),
		};

		// Act
		session.add(user).await.unwrap();
		let flush_result = session.flush().await;

		// Assert
		assert!(flush_result.is_ok());
		assert_eq!(session.dirty_count(), 0);
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_session_flush_roundtrips_scalar_json_fields(_init_drivers: ()) {
		// Arrange
		let pool = create_json_scalar_test_pool().await;
		let mut session = Session::new(pool.clone(), DbBackend::Sqlite).await.unwrap();
		let model = JsonScalarModel {
			id: None,
			external_id: "external-1".to_string(),
			name_json: Json::new("draft".to_string()),
			flag_json: Json::new(true),
			optional_json: Some(Json::new(serde_json::json!({ "stage": "draft" }))),
			publish_date: Some("2026-07-01".to_string()),
		};

		// Act
		session.add(model).await.unwrap();
		session.flush().await.unwrap();

		// Assert
		let row = sqlx::query(
			"SELECT external_id, name_json, flag_json, optional_json, publish_date \
			 FROM json_scalar_models WHERE id = 1",
		)
		.fetch_one(&*pool)
		.await
		.unwrap();
		let stored_external_id: String = row.try_get("external_id").unwrap();
		let stored_name: String = row.try_get("name_json").unwrap();
		let stored_flag: String = row.try_get("flag_json").unwrap();
		let stored_optional: String = row.try_get("optional_json").unwrap();
		let stored_publish_date: String = row.try_get("publish_date").unwrap();
		assert_eq!(stored_external_id, "external-1");
		assert_eq!(stored_name, "\"draft\"");
		assert_eq!(stored_flag, "true");
		assert_eq!(stored_optional, "{\"stage\":\"draft\"}");
		assert_eq!(stored_publish_date, "2026-07-01");

		let loaded: JsonScalarModel = session.get(1).await.unwrap().unwrap();
		assert_eq!(loaded.name_json.as_inner(), "draft");
		assert!(*loaded.flag_json.as_inner());
		assert_eq!(loaded.external_id, "external-1");
		assert_eq!(
			loaded.optional_json.unwrap().as_inner(),
			&serde_json::json!({ "stage": "draft" })
		);
		assert_eq!(loaded.publish_date.as_deref(), Some("2026-07-01"));
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_session_hydrates_array_fields(_init_drivers: ()) {
		let pool = create_array_session_test_pool().await;
		sqlx::query(
			"INSERT INTO array_session_models (id, tags) VALUES (1, '[\"alpha\",\"beta\"]')",
		)
		.execute(&*pool)
		.await
		.expect("array row should insert");

		let mut session = Session::new(pool, DbBackend::Sqlite)
			.await
			.expect("session should initialize");
		let expected = ArraySessionModel {
			id: Some(1),
			tags: vec!["alpha".to_string(), "beta".to_string()],
		};

		let loaded = session
			.get::<ArraySessionModel>(1)
			.await
			.expect("session get should succeed")
			.expect("array row should exist");
		assert_eq!(loaded, expected);

		let all = session
			.list_all::<ArraySessionModel>()
			.await
			.expect("session list_all should succeed");
		assert_eq!(all, vec![expected]);
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_session_hydrates_decimal_fields(_init_drivers: ()) {
		// Arrange
		let pool = create_decimal_session_test_pool().await;
		sqlx::query(
			"INSERT INTO decimal_session_models (id, amount) VALUES (1, '9007199254740993.01')",
		)
		.execute(&*pool)
		.await
		.expect("decimal row should insert");
		let mut session = Session::new(pool, DbBackend::Sqlite)
			.await
			.expect("session should initialize");
		let expected = DecimalSessionModel {
			id: Some(1),
			amount: rust_decimal::Decimal::new(900_719_925_474_099_301, 2),
		};

		// Act
		let loaded = session
			.get::<DecimalSessionModel>(1)
			.await
			.expect("session get should succeed")
			.expect("decimal row should exist");
		let all = session
			.list_all::<DecimalSessionModel>()
			.await
			.expect("session list_all should succeed");

		// Assert
		assert_eq!(loaded, expected);
		assert_eq!(all, vec![expected]);
	}

	#[rstest]
	fn postgres_array_selects_are_json_text() {
		let field = typed_test_field_info(
			"tags",
			"reinhardt.orm.models.ArrayField",
			crate::orm::DatabaseStorageKind::Json,
		);

		assert_eq!(
			RQuery::select()
				.expr(any_field_projection(
					DbBackend::Postgres,
					&field,
					Expr::col("tags").into_simple_expr()
				))
				.to_string(PostgresQueryBuilder),
			"SELECT CAST(ARRAY_TO_JSON(\"tags\") AS TEXT)"
		);
		assert_eq!(
			RQuery::select()
				.expr(any_field_projection(
					DbBackend::Sqlite,
					&field,
					Expr::col("tags").into_simple_expr()
				))
				.to_string(SqliteQueryBuilder),
			"SELECT CAST(\"tags\" AS TEXT)"
		);
	}

	#[rstest]
	fn postgres_vector_selects_are_text_for_json_decoding() {
		let mut field = test_field_info(
			"embedding",
			"reinhardt.orm.models.VectorField",
			false,
			false,
		);
		field.storage_kind = None;

		assert_eq!(
			RQuery::select()
				.expr(any_field_projection(
					DbBackend::Postgres,
					&field,
					Expr::col("embedding").into_simple_expr()
				))
				.to_string(PostgresQueryBuilder),
			"SELECT CAST(\"embedding\" AS TEXT)"
		);
	}

	#[rstest]
	fn postgres_naive_datetime_projection_preserves_wall_clock_value() {
		// Arrange
		let field = typed_test_field_info(
			"created_at",
			"reinhardt.orm.models.DateTimeField",
			crate::orm::DatabaseStorageKind::NaiveDateTime,
		);
		let statement = RQuery::select()
			.expr(any_field_projection(
				DbBackend::Postgres,
				&field,
				Expr::col("created_at").into_simple_expr(),
			))
			.to_owned();
		// Act
		let (sql, values) = PostgresQueryBuilder
			.build_select_checked(&statement)
			.unwrap();
		// Assert
		assert_eq!(sql, "SELECT TO_CHAR(\"created_at\", $1)");
		assert_eq!(values.0, vec![RValue::from("YYYY-MM-DD\"T\"HH24:MI:SS.US")]);
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_session_hydrates_temporal_fields(_init_drivers: ()) {
		let pool = create_temporal_test_pool().await;
		sqlx::query(
			"INSERT INTO temporal_session_models \
			 (id, published_on, starts_at, published_at) \
			 VALUES (1, '2026-07-18', '08:09:10.123456', '2026-07-18T08:09:10Z')",
		)
		.execute(&*pool)
		.await
		.expect("temporal row should insert");

		let mut session = Session::new(pool, DbBackend::Sqlite)
			.await
			.expect("session should initialize");

		let expected = TemporalSessionModel {
			id: Some(1),
			published_on: chrono::NaiveDate::from_ymd_opt(2026, 7, 18)
				.expect("date should be valid"),
			starts_at: chrono::NaiveTime::from_hms_micro_opt(8, 9, 10, 123_456)
				.expect("time should be valid"),
			published_at: chrono::DateTime::parse_from_rfc3339("2026-07-18T08:09:10Z")
				.expect("timestamp should be valid")
				.with_timezone(&chrono::Utc),
		};

		let loaded = session
			.get::<TemporalSessionModel>(1)
			.await
			.expect("session get should succeed")
			.expect("temporal row should exist");
		assert_eq!(loaded, expected);

		let all = session
			.list_all::<TemporalSessionModel>()
			.await
			.expect("session list_all should succeed");
		assert_eq!(all, vec![expected]);
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_session_flush_updates_nullable_json_and_id_suffix_fields(_init_drivers: ()) {
		// Arrange
		let pool = create_json_scalar_test_pool().await;
		sqlx::query(
			"INSERT INTO json_scalar_models \
			 (id, external_id, name_json, flag_json, optional_json, publish_date) \
			 VALUES (1, 'external-1', '\"draft\"', 'true', '{\"stage\":\"draft\"}', '2026-07-01')",
		)
		.execute(&*pool)
		.await
		.unwrap();

		let mut session = Session::new(pool.clone(), DbBackend::Sqlite).await.unwrap();
		let model = JsonScalarModel {
			id: Some(1),
			external_id: "external-2".to_string(),
			name_json: Json::new("revised".to_string()),
			flag_json: Json::new(false),
			optional_json: None,
			publish_date: Some("2026-07-08".to_string()),
		};

		// Act
		session.add(model).await.unwrap();
		session.flush().await.unwrap();

		// Assert
		let row = sqlx::query(
			"SELECT external_id, name_json, flag_json, optional_json, publish_date \
			 FROM json_scalar_models WHERE id = 1",
		)
		.fetch_one(&*pool)
		.await
		.unwrap();
		let stored_external_id: String = row.try_get("external_id").unwrap();
		let stored_name: String = row.try_get("name_json").unwrap();
		let stored_flag: String = row.try_get("flag_json").unwrap();
		let stored_optional: Option<String> = row.try_get("optional_json").unwrap();
		let stored_publish_date: String = row.try_get("publish_date").unwrap();

		assert_eq!(stored_external_id, "external-2");
		assert_eq!(stored_name, "\"revised\"");
		assert_eq!(stored_flag, "false");
		assert_eq!(stored_optional, None);
		assert_eq!(stored_publish_date, "2026-07-08");
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_session_preserves_json_null_distinct_from_sql_null(_init_drivers: ()) {
		let pool = create_json_scalar_test_pool().await;
		let mut session = Session::new(pool.clone(), DbBackend::Sqlite).await.unwrap();
		let json_null = JsonScalarModel {
			id: None,
			external_id: "json-null".to_string(),
			name_json: Json::new("draft".to_string()),
			flag_json: Json::new(true),
			optional_json: Some(Json::new(serde_json::Value::Null)),
			publish_date: None,
		};
		let sql_null = JsonScalarModel {
			id: None,
			external_id: "sql-null".to_string(),
			name_json: Json::new("draft".to_string()),
			flag_json: Json::new(true),
			optional_json: None,
			publish_date: None,
		};

		session.add(json_null).await.unwrap();
		session.add(sql_null).await.unwrap();
		session.flush().await.unwrap();

		let rows =
			sqlx::query("SELECT external_id, optional_json FROM json_scalar_models ORDER BY id")
				.fetch_all(&*pool)
				.await
				.unwrap();
		let stored_json_null: Option<String> = rows
			.iter()
			.find(|row| row.try_get::<String, _>("external_id").unwrap() == "json-null")
			.unwrap()
			.try_get("optional_json")
			.unwrap();
		let stored_sql_null: Option<String> = rows
			.iter()
			.find(|row| row.try_get::<String, _>("external_id").unwrap() == "sql-null")
			.unwrap()
			.try_get("optional_json")
			.unwrap();
		assert_eq!(stored_json_null.as_deref(), Some("null"));
		assert_eq!(stored_sql_null, None);

		let reader = Session::new(pool, DbBackend::Sqlite).await.unwrap();
		let loaded = reader.list_all::<JsonScalarModel>().await.unwrap();
		let loaded_json_null = loaded
			.iter()
			.find(|model| model.external_id == "json-null")
			.unwrap();
		let loaded_sql_null = loaded
			.iter()
			.find(|model| model.external_id == "sql-null")
			.unwrap();
		assert_eq!(
			loaded_json_null.optional_json.as_ref().unwrap().as_inner(),
			&serde_json::Value::Null
		);
		assert_eq!(loaded_sql_null.optional_json, None);
	}
}
