use crate::connection::DatabaseConnection;
use crate::{Model, QuerySet};
use sea_query::{Alias, Expr, ExprTrait, InsertStatement, Query, SelectStatement, UpdateStatement};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global database connection state
static DB: once_cell::sync::OnceCell<Arc<RwLock<Option<DatabaseConnection>>>> =
	once_cell::sync::OnceCell::new();

/// Initialize the global database connection
///
/// # Arguments
///
/// * `url` - Database connection URL
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use reinhardt_orm::manager::init_database;
///
/// init_database("postgres://localhost/mydb").await.unwrap();
/// # }
/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
/// ```
pub async fn init_database(url: &str) -> reinhardt_core::exception::Result<()> {
	init_database_with_pool_size(url, None).await
}

/// Initialize the global database connection with a specific pool size
///
/// # Arguments
///
/// * `url` - Database connection URL
/// * `pool_size` - Maximum number of connections in the pool (None = use default)
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use reinhardt_orm::manager::init_database_with_pool_size;
///
/// // Use larger pool for high-concurrency tests
/// init_database_with_pool_size("postgres://localhost/mydb", Some(50)).await.unwrap();
/// # }
/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
/// ```
pub async fn init_database_with_pool_size(
	url: &str,
	pool_size: Option<u32>,
) -> reinhardt_core::exception::Result<()> {
	let conn = DatabaseConnection::connect_with_pool_size(url, pool_size).await?;
	DB.get_or_init(|| Arc::new(RwLock::new(Some(conn))));
	Ok(())
}

/// Reinitialize the global database connection (for testing)
///
/// This function replaces the existing database connection with a new one.
/// Useful for test scenarios where each test needs a fresh connection pool.
///
/// # Arguments
///
/// * `url` - Database connection URL
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use reinhardt_orm::manager::reinitialize_database;
///
/// reinitialize_database("postgres://localhost/mydb").await.unwrap();
/// # }
/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
/// ```
pub async fn reinitialize_database(url: &str) -> reinhardt_core::exception::Result<()> {
	reinitialize_database_with_pool_size(url, None).await
}

/// Reinitialize the global database connection with a specific pool size (for testing)
///
/// # Arguments
///
/// * `url` - Database connection URL
/// * `pool_size` - Maximum number of connections in the pool (None = use default)
///
/// # Examples
///
/// ```no_run
/// # async fn example() {
/// use reinhardt_orm::manager::reinitialize_database_with_pool_size;
///
/// // Use larger pool for concurrent tests
/// reinitialize_database_with_pool_size("postgres://localhost/mydb", Some(30)).await.unwrap();
/// # }
/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
/// ```
pub async fn reinitialize_database_with_pool_size(
	url: &str,
	pool_size: Option<u32>,
) -> reinhardt_core::exception::Result<()> {
	let conn = DatabaseConnection::connect_with_pool_size(url, pool_size).await?;

	if let Some(db_cell) = DB.get() {
		// Replace existing connection
		let mut guard = db_cell.write().await;
		*guard = Some(conn);
	} else {
		// First time initialization
		DB.get_or_init(|| Arc::new(RwLock::new(Some(conn))));
	}

	Ok(())
}

/// Get a reference to the global database connection
pub async fn get_connection() -> reinhardt_core::exception::Result<DatabaseConnection> {
	let db = DB.get().ok_or_else(|| {
		reinhardt_core::exception::Error::Database("Database not initialized".to_string())
	})?;
	let guard = db.read().await;
	guard.clone().ok_or_else(|| {
		reinhardt_core::exception::Error::Database("Database connection not available".to_string())
	})
}

/// Model manager (similar to Django's Manager)
/// Provides an interface for database operations
pub struct Manager<M: Model> {
	_marker: PhantomData<M>,
}

impl<M: Model> Manager<M> {
	pub fn new() -> Self {
		Self {
			_marker: PhantomData,
		}
	}

	/// Get all records
	pub fn all(&self) -> QuerySet<M> {
		QuerySet::new()
	}

	/// Filter records
	pub fn filter(
		&self,
		field: &str,
		operator: crate::query::FilterOperator,
		value: crate::query::FilterValue,
	) -> QuerySet<M> {
		let filter = crate::query::Filter::new(field.to_string(), operator, value);
		QuerySet::new().filter(filter)
	}

	/// Get a single record by primary key
	/// Returns a QuerySet filtered by the primary key field
	pub fn get(&self, pk: M::PrimaryKey) -> QuerySet<M> {
		let pk_field = M::primary_key_field();
		let pk_value = pk.to_string();

		let filter = crate::query::Filter::new(
			pk_field.to_string(),
			crate::query::FilterOperator::Eq,
			crate::query::FilterValue::String(pk_value),
		);
		QuerySet::new().filter(filter)
	}

	/// Create a new record using SeaQuery for SQL injection protection
	pub async fn create(&self, model: &M) -> reinhardt_core::exception::Result<M> {
		let conn = get_connection().await?;
		self.create_with_conn(&conn, model).await
	}

	/// Create a new record with an explicit database connection
	///
	/// This method allows using a specific connection, which is essential for
	/// transaction support. When operations are performed within a transaction,
	/// the same connection must be used throughout.
	///
	/// # Arguments
	///
	/// * `conn` - The database connection to use
	/// * `model` - The model to create
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_orm::{Model, Manager, TransactionScope};
	/// # async fn example<M: Model>(manager: Manager<M>, model: &M) -> reinhardt_core::exception::Result<()> {
	/// use reinhardt_orm::manager::get_connection;
	///
	/// let conn = get_connection().await?;
	/// let tx = TransactionScope::begin(&conn).await?;
	///
	/// // Create within transaction
	/// let created = manager.create_with_conn(&conn, model).await?;
	///
	/// tx.commit().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn create_with_conn(
		&self,
		conn: &DatabaseConnection,
		model: &M,
	) -> reinhardt_core::exception::Result<M> {
		let json = serde_json::to_value(model)
			.map_err(|e| reinhardt_core::exception::Error::Database(e.to_string()))?;

		// Extract fields and values from model
		let obj = json.as_object().ok_or_else(|| {
			reinhardt_core::exception::Error::Database("Model must serialize to object".to_string())
		})?;

		// Build SeaQuery INSERT statement
		let mut stmt = Query::insert();
		stmt.into_table(Alias::new(M::table_name()));

		// Filter out null values (e.g., id field when creating new records)
		let (fields, values): (Vec<_>, Vec<_>) = obj
			.iter()
			.filter(|(_, v)| !v.is_null())
			.map(|(k, v)| {
				(
					Alias::new(k.as_str()),
					Expr::value(Self::json_to_sea_value(v)),
				)
			})
			.unzip();

		stmt.columns(fields);
		stmt.values_panic(values);

		// Add RETURNING * support
		stmt.returning(Query::returning().columns([sea_query::Asterisk]));

		use sea_query::PostgresQueryBuilder;
		let sql = stmt.to_string(PostgresQueryBuilder);

		let row = conn.query_one(&sql, vec![]).await?;
		let value = serde_json::to_value(&row.data)
			.map_err(|e| reinhardt_core::exception::Error::Database(e.to_string()))?;
		serde_json::from_value(value)
			.map_err(|e| reinhardt_core::exception::Error::Database(e.to_string()))
	}

	/// Convert serde_json::Value to sea_query::Value for parameter binding
	fn json_to_sea_value(v: &serde_json::Value) -> sea_query::Value {
		match v {
			serde_json::Value::Null => sea_query::Value::Int(None),
			serde_json::Value::Bool(b) => sea_query::Value::Bool(Some(*b)),
			serde_json::Value::Number(n) => {
				if let Some(i) = n.as_i64() {
					sea_query::Value::BigInt(Some(i))
				} else if let Some(f) = n.as_f64() {
					sea_query::Value::Double(Some(f))
				} else {
					sea_query::Value::Int(None)
				}
			}
			serde_json::Value::String(s) => sea_query::Value::String(Some(s.clone())),
			serde_json::Value::Array(arr) => {
				// Use sea-query's Array type for PostgreSQL arrays
				let values: Vec<sea_query::Value> =
					arr.iter().map(Self::json_to_sea_value).collect();
				sea_query::Value::Array(sea_query::ArrayType::String, Some(Box::new(values)))
			}
			serde_json::Value::Object(_obj) => {
				// Use sea-query's Json type for PostgreSQL JSONB/JSON columns
				sea_query::Value::Json(Some(v.clone()))
			}
		}
	}

	/// Serialize a JSON value to SQL-compatible string representation
	#[allow(dead_code)]
	fn serialize_value(v: &serde_json::Value) -> String {
		match v {
			serde_json::Value::Null => "NULL".to_string(),
			serde_json::Value::Bool(b) => b.to_string().to_uppercase(),
			serde_json::Value::Number(n) => n.to_string(),
			serde_json::Value::String(s) => {
				// Escape single quotes and wrap in quotes
				format!("'{}'", s.replace('\'', "''"))
			}
			serde_json::Value::Array(arr) => {
				// Convert to PostgreSQL array syntax: ARRAY['a', 'b', 'c']
				let items: Vec<String> = arr.iter().map(Self::serialize_value).collect();
				format!("ARRAY[{}]", items.join(", "))
			}
			serde_json::Value::Object(obj) => {
				// Convert to JSON string for JSONB columns
				let json_str = serde_json::to_string(obj).unwrap_or_else(|_| "{}".to_string());
				format!("'{}'::jsonb", json_str.replace('\'', "''"))
			}
		}
	}

	/// Update an existing record using SeaQuery for SQL injection protection
	pub async fn update(&self, model: &M) -> reinhardt_core::exception::Result<M> {
		let conn = get_connection().await?;
		self.update_with_conn(&conn, model).await
	}

	/// Update an existing record with an explicit database connection
	///
	/// This method allows using a specific connection, which is essential for
	/// transaction support.
	///
	/// # Arguments
	///
	/// * `conn` - The database connection to use
	/// * `model` - The model to update (must have primary key set)
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_orm::{Model, Manager, TransactionScope};
	/// # async fn example<M: Model>(manager: Manager<M>, model: &M) -> reinhardt_core::exception::Result<()> {
	/// use reinhardt_orm::manager::get_connection;
	///
	/// let conn = get_connection().await?;
	/// let tx = TransactionScope::begin(&conn).await?;
	///
	/// // Update within transaction
	/// let updated = manager.update_with_conn(&conn, model).await?;
	///
	/// tx.commit().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn update_with_conn(
		&self,
		conn: &DatabaseConnection,
		model: &M,
	) -> reinhardt_core::exception::Result<M> {
		let pk = model.primary_key().ok_or_else(|| {
			reinhardt_core::exception::Error::Database("Model must have primary key".to_string())
		})?;

		let json = serde_json::to_value(model)
			.map_err(|e| reinhardt_core::exception::Error::Database(e.to_string()))?;

		let obj = json.as_object().ok_or_else(|| {
			reinhardt_core::exception::Error::Database("Model must serialize to object".to_string())
		})?;

		// Build SeaQuery UPDATE statement
		let mut stmt = Query::update();
		stmt.table(Alias::new(M::table_name()));

		// Add SET clauses for all fields except primary key
		for (k, v) in obj
			.iter()
			.filter(|(k, _)| k.as_str() != M::primary_key_field())
		{
			stmt.value(Alias::new(k.as_str()), Self::json_to_sea_value(v));
		}

		// Add WHERE clause for primary key
		// Convert primary key to sea_query::Value for type safety
		let pk_value = sea_query::Value::String(Some(pk.to_string()));
		stmt.and_where(Expr::col(Alias::new(M::primary_key_field())).eq(pk_value));

		// Add RETURNING * support
		stmt.returning(Query::returning().columns([sea_query::Asterisk]));

		use sea_query::PostgresQueryBuilder;
		let sql = stmt.to_string(PostgresQueryBuilder);

		let row = conn.query_one(&sql, vec![]).await?;
		let value = serde_json::to_value(&row.data)
			.map_err(|e| reinhardt_core::exception::Error::Database(e.to_string()))?;
		serde_json::from_value(value)
			.map_err(|e| reinhardt_core::exception::Error::Database(e.to_string()))
	}

	/// Delete a record using SeaQuery for SQL injection protection
	pub async fn delete(&self, pk: M::PrimaryKey) -> reinhardt_core::exception::Result<()> {
		let conn = get_connection().await?;
		self.delete_with_conn(&conn, pk).await
	}

	/// Delete a record with an explicit database connection
	///
	/// This method allows using a specific connection, which is essential for
	/// transaction support.
	///
	/// # Arguments
	///
	/// * `conn` - The database connection to use
	/// * `pk` - The primary key of the record to delete
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_orm::{Model, Manager, TransactionScope};
	/// # async fn example<M: Model>(manager: Manager<M>, pk: M::PrimaryKey) -> reinhardt_core::exception::Result<()> {
	/// use reinhardt_orm::manager::get_connection;
	///
	/// let conn = get_connection().await?;
	/// let tx = TransactionScope::begin(&conn).await?;
	///
	/// // Delete within transaction
	/// manager.delete_with_conn(&conn, pk).await?;
	///
	/// tx.commit().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn delete_with_conn(
		&self,
		conn: &DatabaseConnection,
		pk: M::PrimaryKey,
	) -> reinhardt_core::exception::Result<()> {
		// Build SeaQuery DELETE statement
		let mut stmt = Query::delete();
		stmt.from_table(Alias::new(M::table_name()))
			.and_where(Expr::col(Alias::new(M::primary_key_field())).eq(pk.to_string()));

		use sea_query::PostgresQueryBuilder;
		let sql = stmt.to_string(PostgresQueryBuilder);

		conn.execute(&sql, vec![]).await?;
		Ok(())
	}

	/// Count records using SeaQuery
	pub async fn count(&self) -> reinhardt_core::exception::Result<i64> {
		let conn = get_connection().await?;
		self.count_with_conn(&conn).await
	}

	/// Count records with an explicit database connection
	///
	/// This method allows using a specific connection, which is essential for
	/// verifying data within a transaction before commit/rollback.
	///
	/// # Arguments
	///
	/// * `conn` - The database connection to use
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_orm::{Model, Manager, TransactionScope};
	/// # async fn example<M: Model>(manager: Manager<M>) -> reinhardt_core::exception::Result<()> {
	/// use reinhardt_orm::manager::get_connection;
	///
	/// let conn = get_connection().await?;
	/// let tx = TransactionScope::begin(&conn).await?;
	///
	/// // Count within transaction (sees uncommitted data)
	/// let count = manager.count_with_conn(&conn).await?;
	///
	/// tx.commit().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn count_with_conn(
		&self,
		conn: &DatabaseConnection,
	) -> reinhardt_core::exception::Result<i64> {
		// Build SeaQuery SELECT COUNT(*) statement with explicit alias
		let stmt = Query::select()
			.from(Alias::new(M::table_name()))
			.expr_as(
				sea_query::Func::count(Expr::col(sea_query::Asterisk)),
				Alias::new("count"),
			)
			.to_owned();

		use sea_query::PostgresQueryBuilder;
		let sql = stmt.to_string(PostgresQueryBuilder);

		let row = conn.query_one(&sql, vec![]).await?;
		row.get::<i64>("count").ok_or_else(|| {
			reinhardt_core::exception::Error::Database("Failed to get count".to_string())
		})
	}

	/// Bulk create multiple records using SeaQuery (similar to Django's bulk_create())
	pub fn bulk_create_query(&self, models: &[M]) -> Option<InsertStatement> {
		if models.is_empty() {
			return None;
		}

		// Convert all models to JSON and extract field names from first model
		let json_values: Vec<serde_json::Value> = models
			.iter()
			.filter_map(|m| serde_json::to_value(m).ok())
			.collect();

		if json_values.is_empty() {
			return None;
		}

		// Get field names from first model
		let first_obj = json_values[0].as_object()?;

		let fields: Vec<_> = first_obj.keys().map(|k| Alias::new(k.as_str())).collect();

		// Build SeaQuery INSERT statement
		let mut stmt = Query::insert();
		stmt.into_table(Alias::new(M::table_name())).columns(fields);

		// Add value rows for each model
		for val in &json_values {
			if let Some(obj) = val.as_object() {
				let values: Vec<sea_query::Expr> = first_obj
					.keys()
					.map(|field| {
						obj.get(field)
							.map(|v| Expr::value(Self::json_to_sea_value(v)))
							.unwrap_or(Expr::value(sea_query::Value::Int(None)))
					})
					.collect();
				stmt.values_panic(values);
			}
		}

		Some(stmt.to_owned())
	}

	/// Generate bulk create SQL (convenience method)
	pub fn bulk_create_sql(&self, models: &[M]) -> String {
		if let Some(stmt) = self.bulk_create_query(models) {
			use sea_query::PostgresQueryBuilder;
			stmt.to_string(PostgresQueryBuilder)
		} else {
			String::new()
		}
	}

	/// Generate UPDATE query for QuerySet
	pub fn update_queryset(
		&self,
		queryset: &QuerySet<M>,
		updates: &[(&str, &str)],
	) -> (String, Vec<String>) {
		queryset.update_sql(updates)
	}

	/// Generate DELETE query for QuerySet
	pub fn delete_queryset(&self, queryset: &QuerySet<M>) -> (String, Vec<String>) {
		queryset.delete_sql()
	}

	/// Get or create a record (Django's get_or_create)
	/// Returns (model, created) where created is true if a new record was created
	///
	/// Django equivalent:
	/// ```python
	/// obj, created = Model.objects.get_or_create(
	///     field1=value1,
	///     defaults={'field2': value2}
	/// )
	/// ```
	pub async fn get_or_create(
		&self,
		lookup_fields: HashMap<String, String>,
		defaults: Option<HashMap<String, String>>,
	) -> reinhardt_core::exception::Result<(M, bool)> {
		let conn = get_connection().await?;

		// Try to find existing record
		let (select_sql, _) =
			self.get_or_create_sql(&lookup_fields, &defaults.clone().unwrap_or_default());

		if let Ok(Some(row)) = conn.query_optional(&select_sql, vec![]).await {
			let value = serde_json::to_value(&row.data)
				.map_err(|e| reinhardt_core::exception::Error::Database(e.to_string()))?;
			let model: M = serde_json::from_value(value)
				.map_err(|e| reinhardt_core::exception::Error::Database(e.to_string()))?;
			return Ok((model, false));
		}

		// Record not found, create new one
		let mut all_fields = lookup_fields.clone();
		if let Some(defs) = defaults {
			all_fields.extend(defs);
		}

		let fields: Vec<String> = all_fields.keys().cloned().collect();
		let values: Vec<String> = all_fields.values().map(|v| format!("'{}'", v)).collect();

		let insert_sql = format!(
			"INSERT INTO {} ({}) VALUES ({}) RETURNING *",
			M::table_name(),
			fields.join(", "),
			values.join(", ")
		);

		let row = conn.query_one(&insert_sql, vec![]).await?;
		let value = serde_json::to_value(&row.data)
			.map_err(|e| reinhardt_core::exception::Error::Database(e.to_string()))?;
		let model: M = serde_json::from_value(value)
			.map_err(|e| reinhardt_core::exception::Error::Database(e.to_string()))?;

		Ok((model, true))
	}

	/// Bulk create multiple records efficiently (Django's bulk_create)
	/// Inserts multiple records in a single query for performance
	///
	/// Django equivalent:
	/// ```python
	/// Model.objects.bulk_create([
	///     Model(field1=value1),
	///     Model(field2=value2),
	/// ])
	/// ```
	///
	/// Options:
	/// - batch_size: Split into multiple batches if needed
	/// - ignore_conflicts: Skip records that would violate constraints
	/// - update_conflicts: Update existing records instead of failing
	pub async fn bulk_create(
		&self,
		models: Vec<M>,
		batch_size: Option<usize>,
		ignore_conflicts: bool,
		_update_conflicts: bool,
	) -> reinhardt_core::exception::Result<Vec<M>> {
		if models.is_empty() {
			return Ok(vec![]);
		}

		let conn = get_connection().await?;
		let batch_size = batch_size.unwrap_or(models.len());
		let mut results = Vec::new();

		for chunk in models.chunks(batch_size) {
			// Extract fields from first model
			let json = serde_json::to_value(&chunk[0])
				.map_err(|e| reinhardt_core::exception::Error::Database(e.to_string()))?;
			let obj = json.as_object().ok_or_else(|| {
				reinhardt_core::exception::Error::Database(
					"Model must serialize to object".to_string(),
				)
			})?;
			let field_names: Vec<String> = obj.keys().cloned().collect();

			// Extract values for all models in chunk
			let value_rows: Vec<Vec<String>> = chunk
				.iter()
				.map(|model| {
					let json = serde_json::to_value(model).unwrap();
					let obj = json.as_object().unwrap();
					field_names
						.iter()
						.map(|field| {
							let val = &obj[field];
							if val.is_string() {
								val.as_str().unwrap().to_string()
							} else {
								val.to_string()
							}
						})
						.collect()
				})
				.collect();

			let sql = self.bulk_create_sql_detailed(&field_names, &value_rows, ignore_conflicts);

			// Execute and get results
			if ignore_conflicts {
				conn.execute(&sql, vec![]).await?;
				// Note: Can't get RETURNING with DO NOTHING, skip results
				// Return empty vec for ignored conflicts
			} else {
				let sql_with_returning = sql + " RETURNING *";
				let rows = conn.query(&sql_with_returning, vec![]).await?;
				for row in rows {
					let value = serde_json::to_value(&row.data)
						.map_err(|e| reinhardt_core::exception::Error::Database(e.to_string()))?;
					let model: M = serde_json::from_value(value)
						.map_err(|e| reinhardt_core::exception::Error::Database(e.to_string()))?;
					results.push(model);
				}
			}
		}

		Ok(results)
	}

	/// Bulk update multiple records efficiently (Django's bulk_update)
	/// Updates specified fields for multiple records in optimized queries
	///
	/// Django equivalent:
	/// ```python
	/// Model.objects.bulk_update(
	///     [obj1, obj2, obj3],
	///     ['field1', 'field2'],
	///     batch_size=100
	/// )
	/// ```
	pub async fn bulk_update(
		&self,
		models: Vec<M>,
		fields: Vec<String>,
		batch_size: Option<usize>,
	) -> reinhardt_core::exception::Result<usize> {
		if models.is_empty() || fields.is_empty() {
			return Ok(0);
		}

		let conn = get_connection().await?;
		let batch_size = batch_size.unwrap_or(models.len());
		let mut total_updated = 0;

		for chunk in models.chunks(batch_size) {
			// Build updates structure
			let updates: Vec<(M::PrimaryKey, HashMap<String, String>)> = chunk
				.iter()
				.filter_map(|model| {
					let pk = model.primary_key()?.clone();
					let json = serde_json::to_value(model).ok()?;
					let obj = json.as_object()?;

					let mut field_map = HashMap::new();
					for field in &fields {
						if let Some(val) = obj.get(field) {
							let val_str = if val.is_string() {
								val.as_str().unwrap().to_string()
							} else {
								val.to_string()
							};
							field_map.insert(field.clone(), val_str);
						}
					}

					Some((pk, field_map))
				})
				.collect();

			if !updates.is_empty() {
				let sql = self.bulk_update_sql_detailed(&updates, &fields);
				let rows_affected = conn.execute(&sql, vec![]).await?;
				total_updated += rows_affected as usize;
			}
		}

		Ok(total_updated)
	}

	/// Get or create - SQL generation using SeaQuery (for testing)
	pub fn get_or_create_queries(
		&self,
		lookup_fields: &HashMap<String, String>,
		defaults: &HashMap<String, String>,
	) -> (SelectStatement, InsertStatement) {
		// Generate SELECT query with SeaQuery
		let mut select_stmt = Query::select();
		select_stmt
			.from(Alias::new(M::table_name()))
			.column(sea_query::Asterisk);

		for (k, v) in lookup_fields.iter() {
			select_stmt.and_where(Expr::col(Alias::new(k.as_str())).eq(v.as_str()));
		}

		// Generate INSERT query with SeaQuery
		let mut insert_fields = lookup_fields.clone();
		insert_fields.extend(defaults.clone());

		let mut insert_stmt = Query::insert();
		insert_stmt.into_table(Alias::new(M::table_name()));

		let columns: Vec<_> = insert_fields
			.keys()
			.map(|k| Alias::new(k.as_str()))
			.collect();
		let values: Vec<sea_query::Expr> = insert_fields
			.values()
			.map(|v| Expr::val(v.clone()))
			.collect();

		insert_stmt.columns(columns);
		insert_stmt.values_panic(values);

		(select_stmt.to_owned(), insert_stmt.to_owned())
	}

	/// Get or create - SQL generation (convenience method for testing)
	pub fn get_or_create_sql(
		&self,
		lookup_fields: &HashMap<String, String>,
		defaults: &HashMap<String, String>,
	) -> (String, String) {
		let (select_stmt, insert_stmt) = self.get_or_create_queries(lookup_fields, defaults);
		use sea_query::PostgresQueryBuilder;
		(
			select_stmt.to_string(PostgresQueryBuilder),
			insert_stmt.to_string(PostgresQueryBuilder),
		)
	}

	/// Bulk create - SQL generation only (for testing)
	pub fn bulk_create_sql_detailed(
		&self,
		field_names: &[String],
		value_rows: &[Vec<String>],
		ignore_conflicts: bool,
	) -> String {
		if value_rows.is_empty() {
			return String::new();
		}

		let values_clause: Vec<String> = value_rows
			.iter()
			.map(|row| {
				let values = row
					.iter()
					.map(|v| format!("'{}'", v))
					.collect::<Vec<_>>()
					.join(", ");
				format!("({})", values)
			})
			.collect();

		let mut sql = format!(
			"INSERT INTO {} ({}) VALUES {}",
			M::table_name(),
			field_names.join(", "),
			values_clause.join(", ")
		);

		if ignore_conflicts {
			sql.push_str(" ON CONFLICT DO NOTHING");
		}

		sql
	}

	/// Bulk update using SeaQuery - SQL generation (for testing)
	pub fn bulk_update_query_detailed(
		&self,
		updates: &[(M::PrimaryKey, HashMap<String, String>)],
		fields: &[String],
	) -> Option<UpdateStatement>
	where
		M::PrimaryKey: std::fmt::Display + Clone,
	{
		if updates.is_empty() || fields.is_empty() {
			return None;
		}

		let mut stmt = Query::update();
		stmt.table(Alias::new(M::table_name()));

		// Generate CASE statements for each field
		for field in fields {
			// Build CASE expression for this field
			let mut case_expr = sea_query::CaseStatement::new();

			for (pk, field_map) in updates.iter() {
				if let Some(value) = field_map.get(field) {
					// WHEN id = pk THEN 'value'
					case_expr = case_expr.case(
						Expr::col(Alias::new("id")).eq(pk.to_string()),
						Expr::val(value.clone()),
					);
				}
			}

			// field = CASE ... END
			let case_simple_expr: sea_query::SimpleExpr = case_expr.into();
			stmt.value(Alias::new(field.as_str()), case_simple_expr);
		}

		// WHERE id IN (...)
		let ids: Vec<sea_query::Value> = updates
			.iter()
			.map(|(pk, _)| sea_query::Value::String(Some(pk.to_string())))
			.collect();

		stmt.and_where(Expr::col(Alias::new("id")).is_in(ids));

		Some(stmt.to_owned())
	}

	/// Bulk update - SQL generation (convenience method for testing)
	pub fn bulk_update_sql_detailed(
		&self,
		updates: &[(M::PrimaryKey, HashMap<String, String>)],
		fields: &[String],
	) -> String
	where
		M::PrimaryKey: std::fmt::Display + Clone,
	{
		if let Some(stmt) = self.bulk_update_query_detailed(updates, fields) {
			use sea_query::PostgresQueryBuilder;
			stmt.to_string(PostgresQueryBuilder)
		} else {
			String::new()
		}
	}
}

impl<M: Model> Default for Manager<M> {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_core::validators::TableName;
	use serde::{Deserialize, Serialize};

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct TestUser {
		id: Option<i64>,
		name: String,
		email: String,
	}

	const TEST_USER_TABLE: TableName = TableName::new_const("test_user");

	impl crate::Model for TestUser {
		type PrimaryKey = i64;

		fn table_name() -> &'static str {
			TEST_USER_TABLE.as_str()
		}

		fn primary_key(&self) -> Option<&Self::PrimaryKey> {
			self.id.as_ref()
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}
	}

	#[test]
	fn test_get_or_create_sql() {
		let manager = Manager::<TestUser>::new();
		let mut lookup = HashMap::new();
		lookup.insert("email".to_string(), "test@example.com".to_string());

		let mut defaults = HashMap::new();
		defaults.insert("name".to_string(), "Test User".to_string());

		let (select_sql, insert_sql) = manager.get_or_create_sql(&lookup, &defaults);

		// SeaQuery uses quoted identifiers and TestUser table is "test_user"
		assert!(select_sql.contains("SELECT") && select_sql.contains("FROM"));
		assert!(select_sql.contains("test_user"));
		assert!(select_sql.contains("email"));
		assert!(select_sql.contains("test@example.com"));
		assert!(insert_sql.contains("INSERT"));
		assert!(insert_sql.contains("test_user"));
		assert!(insert_sql.contains("email"));
		assert!(insert_sql.contains("name"));
	}

	#[test]
	fn test_bulk_create_sql() {
		let manager = Manager::<TestUser>::new();
		let fields = vec!["name".to_string(), "email".to_string()];
		let values = vec![
			vec!["Alice".to_string(), "alice@example.com".to_string()],
			vec!["Bob".to_string(), "bob@example.com".to_string()],
		];

		let sql = manager.bulk_create_sql_detailed(&fields, &values, false);

		// SeaQuery uses quoted identifiers and TestUser table is "test_user"
		assert!(sql.contains("INSERT"));
		assert!(sql.contains("test_user"));
		assert!(sql.contains("name"));
		assert!(sql.contains("email"));
		assert!(sql.contains("Alice"));
		assert!(sql.contains("alice@example.com"));
		assert!(sql.contains("Bob"));
		assert!(sql.contains("bob@example.com"));
	}

	#[test]
	fn test_bulk_create_sql_with_conflict() {
		let manager = Manager::<TestUser>::new();
		let fields = vec!["name".to_string(), "email".to_string()];
		let values = vec![vec!["Alice".to_string(), "alice@example.com".to_string()]];

		let sql = manager.bulk_create_sql_detailed(&fields, &values, true);

		assert!(sql.contains("ON CONFLICT DO NOTHING"));
	}

	#[test]
	fn test_bulk_update_sql() {
		let manager = Manager::<TestUser>::new();

		let mut updates = Vec::new();
		let mut user1_fields = HashMap::new();
		user1_fields.insert("name".to_string(), "Alice Updated".to_string());
		user1_fields.insert("email".to_string(), "alice_new@example.com".to_string());
		updates.push((1i64, user1_fields));

		let mut user2_fields = HashMap::new();
		user2_fields.insert("name".to_string(), "Bob Updated".to_string());
		user2_fields.insert("email".to_string(), "bob_new@example.com".to_string());
		updates.push((2i64, user2_fields));

		let fields = vec!["name".to_string(), "email".to_string()];
		let sql = manager.bulk_update_sql_detailed(&updates, &fields);

		// SeaQuery uses quoted identifiers and TestUser table is "test_user"
		assert!(sql.contains("UPDATE"));
		assert!(sql.contains("test_user"));
		assert!(sql.contains("SET"));
		assert!(sql.contains("name"));
		assert!(sql.contains("CASE"));
		assert!(sql.contains("email"));
		assert!(sql.contains("Alice Updated"));
		assert!(sql.contains("Bob Updated"));
		assert!(sql.contains("WHERE"));
	}

	#[test]
	fn test_bulk_create_empty() {
		let manager = Manager::<TestUser>::new();
		let fields: Vec<String> = vec![];
		let values: Vec<Vec<String>> = vec![];

		let sql = manager.bulk_create_sql_detailed(&fields, &values, false);
		assert!(sql.is_empty());
	}

	#[test]
	fn test_bulk_update_empty() {
		let manager = Manager::<TestUser>::new();
		let updates: Vec<(i64, HashMap<String, String>)> = vec![];
		let fields = vec!["name".to_string()];

		let sql = manager.bulk_update_sql_detailed(&updates, &fields);
		assert!(sql.is_empty());
	}
}
