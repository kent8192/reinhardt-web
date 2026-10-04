use super::connection::{DatabaseBackend, DatabaseConnection};
use super::inspection::FieldInfo;
use super::{Model, QuerySet};
use reinhardt_query::prelude::{
	Alias, ColumnRef, Condition, DeleteStatement, Expr, ExprTrait, Func, InsertStatement,
	MySqlQueryBuilder, PostgresQueryBuilder, Query, QueryBuilder, SelectStatement,
	SqliteQueryBuilder, UpdateStatement, Values,
};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Build SQL with values from an INSERT statement based on database backend
fn build_insert_sql(stmt: &InsertStatement, backend: DatabaseBackend) -> (String, Values) {
	match backend {
		DatabaseBackend::Postgres => PostgresQueryBuilder.build_insert(stmt),
		DatabaseBackend::MySql => MySqlQueryBuilder.build_insert(stmt),
		DatabaseBackend::Sqlite => SqliteQueryBuilder.build_insert(stmt),
	}
}

/// Build SQL with values from an UPDATE statement based on database backend
fn build_update_sql(stmt: &UpdateStatement, backend: DatabaseBackend) -> (String, Values) {
	match backend {
		DatabaseBackend::Postgres => PostgresQueryBuilder.build_update(stmt),
		DatabaseBackend::MySql => MySqlQueryBuilder.build_update(stmt),
		DatabaseBackend::Sqlite => SqliteQueryBuilder.build_update(stmt),
	}
}

/// Build SQL with values from a SELECT statement based on database backend
fn build_select_sql(stmt: &SelectStatement, backend: DatabaseBackend) -> (String, Values) {
	match backend {
		DatabaseBackend::Postgres => PostgresQueryBuilder.build_select(stmt),
		DatabaseBackend::MySql => MySqlQueryBuilder.build_select(stmt),
		DatabaseBackend::Sqlite => SqliteQueryBuilder.build_select(stmt),
	}
}

/// Convert a SELECT statement to SQL string based on database backend
fn select_to_string(stmt: &SelectStatement, backend: DatabaseBackend) -> String {
	build_select_sql(stmt, backend).0
}

/// Convert an INSERT statement to SQL string based on database backend
fn insert_to_string(stmt: &InsertStatement, backend: DatabaseBackend) -> String {
	build_insert_sql(stmt, backend).0
}

/// Build SQL with values from a DELETE statement based on database backend
fn build_delete_sql(stmt: &DeleteStatement, backend: DatabaseBackend) -> (String, Values) {
	match backend {
		DatabaseBackend::Postgres => PostgresQueryBuilder.build_delete(stmt),
		DatabaseBackend::MySql => MySqlQueryBuilder.build_delete(stmt),
		DatabaseBackend::Sqlite => SqliteQueryBuilder.build_delete(stmt),
	}
}

fn validate_bulk_batch_size(batch_size: Option<usize>) -> reinhardt_core::exception::Result<()> {
	if batch_size == Some(0) {
		return Err(reinhardt_core::exception::Error::Validation(
			"batch_size must be greater than zero".into(),
		));
	}
	Ok(())
}

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
/// use reinhardt_db::orm::manager::init_database;
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
/// use reinhardt_db::orm::manager::init_database_with_pool_size;
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
/// use reinhardt_db::orm::manager::reinitialize_database;
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
/// use reinhardt_db::orm::manager::reinitialize_database_with_pool_size;
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
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			_marker: PhantomData,
		}
	}

	/// Get all records
	pub fn all(&self) -> QuerySet<M> {
		QuerySet::new()
	}

	/// Filter records by a typed filter expression.
	///
	/// Accepts any value convertible into a [`FilterCondition`](super::query::FilterCondition).
	/// The intended call style is the fluent builder produced by the
	/// `#[model]`-generated field accessors (`FieldRef::eq()` / `.gt()` / ...)
	/// or a composite condition built with `.and()`, `.or()`, and `.not()`.
	///
	/// # Examples
	///
	/// ```ignore
	/// // Typed builder (recommended):
	/// User::objects()
	///     .filter(User::field_email().eq("alice@example.com"))
	///     .all()
	///     .await?;
	///
	/// // Raw Filter (when the field name is dynamic):
	/// use reinhardt_db::orm::{Filter, FilterOperator, FilterValue};
	/// User::objects()
	///     .filter(Filter::new("email", FilterOperator::Eq, FilterValue::String("alice@example.com".to_string())))
	///     .all()
	///     .await?;
	/// ```
	pub fn filter(&self, filter: impl Into<super::query::FilterCondition>) -> QuerySet<M> {
		QuerySet::new().filter(filter)
	}

	/// Get a single record by primary key
	/// Returns a QuerySet filtered by the primary key field
	pub fn get(&self, pk: M::PrimaryKey) -> QuerySet<M> {
		let pk_field = M::primary_key_field();
		let pk_value = M::primary_key_filter_value(pk);

		let filter = super::query::Filter::new(
			pk_field.to_string(),
			super::query::FilterOperator::Eq,
			pk_value,
		);
		QuerySet::new().filter(filter)
	}

	/// Set LIMIT clause
	///
	/// Limits the number of records returned by the QuerySet.
	/// Corresponds to Django's `QuerySet[:n]`.
	///
	/// # Examples
	///
	/// ```ignore
	/// let users = User::objects().limit(10).all().await?;
	/// ```
	pub fn limit(&self, limit: usize) -> QuerySet<M> {
		QuerySet::new().limit(limit)
	}

	/// Set ORDER BY clause
	///
	/// Sorts the QuerySet results by the specified fields.
	/// Use "-" prefix for descending order (e.g., "-created_at").
	/// Corresponds to Django's QuerySet.order_by().
	///
	/// # Examples
	///
	/// ```ignore
	/// // Ascending by name
	/// let users = User::objects().order_by(&["name"]).all().await?;
	///
	/// // Descending by created_at
	/// let users = User::objects().order_by(&["-created_at"]).all().await?;
	///
	/// // Multiple fields
	/// let users = User::objects().order_by(&["department", "-salary"]).all().await?;
	/// ```
	pub fn order_by(&self, fields: &[&str]) -> QuerySet<M> {
		QuerySet::new().order_by(fields)
	}

	/// Add annotation to QuerySet
	///
	/// Adds a computed field to each record using SQL expressions or aggregations.
	/// Corresponds to Django's QuerySet.annotate().
	///
	/// # Examples
	///
	/// ```ignore
	/// use reinhardt_db::orm::annotation::{Annotation, AnnotationValue};
	/// use reinhardt_db::orm::aggregation::Aggregate;
	///
	/// let users = User::objects()
	///     .annotate(Annotation::new("total_orders",
	///         AnnotationValue::Aggregate(Aggregate::count(Some("orders")))))
	///     .all()
	///     .await?;
	/// ```
	pub fn annotate(&self, annotation: super::annotation::Annotation) -> QuerySet<M> {
		QuerySet::new().annotate(annotation)
	}

	/// Defer loading of specified fields
	///
	/// Excludes the specified fields from the initial query, loading them only when accessed.
	/// Corresponds to Django's QuerySet.defer().
	///
	/// # Examples
	///
	/// ```ignore
	/// let users = User::objects().defer(&["bio", "profile_picture"]).all().await?;
	/// ```
	pub fn defer(&self, fields: &[&str]) -> QuerySet<M> {
		QuerySet::new().defer(fields)
	}

	/// Load only specified fields
	///
	/// Loads only the specified fields, excluding all others.
	/// Corresponds to Django's QuerySet.only().
	///
	/// # Examples
	///
	/// ```ignore
	/// let users = User::objects().only(&["id", "username"]).all().await?;
	/// ```
	pub fn only(&self, fields: &[&str]) -> QuerySet<M> {
		QuerySet::new().only(fields)
	}

	/// Select specific fields (values)
	///
	/// Returns records with only the specified fields.
	/// Corresponds to Django's QuerySet.values().
	///
	/// # Examples
	///
	/// ```ignore
	/// let user_data = User::objects().values(&["id", "username", "email"]).all().await?;
	/// ```
	pub fn values(&self, fields: &[&str]) -> QuerySet<M> {
		QuerySet::new().values(fields)
	}

	/// Eager load related objects using JOIN
	///
	/// Performs SQL JOINs to load related objects in a single query.
	/// Corresponds to Django's QuerySet.select_related().
	///
	/// # Examples
	///
	/// ```ignore
	/// let posts = Post::objects().select_related(&["author", "category"]).all().await?;
	/// ```
	pub fn select_related(&self, fields: &[&str]) -> QuerySet<M> {
		QuerySet::new().select_related(fields)
	}

	/// Set OFFSET clause
	///
	/// Skips the specified number of records before returning results.
	/// Corresponds to Django's QuerySet slicing `[offset:]`.
	///
	/// # Examples
	///
	/// ```ignore
	/// let users = User::objects().offset(20).all().await?;
	/// ```
	pub fn offset(&self, offset: usize) -> QuerySet<M> {
		QuerySet::new().offset(offset)
	}

	/// Paginate results (LIMIT + OFFSET)
	///
	/// Convenience method that combines LIMIT and OFFSET for pagination.
	/// Corresponds to Django's Paginator.
	///
	/// # Examples
	///
	/// ```ignore
	/// let users = User::objects().paginate(3, 10).all().await?;  // page 3, 10 items per page
	/// ```
	pub fn paginate(&self, page: usize, page_size: usize) -> QuerySet<M> {
		QuerySet::new().paginate(page, page_size)
	}

	/// Prefetch related objects using separate queries
	///
	/// Performs separate queries to load related objects, reducing N+1 queries.
	/// Corresponds to Django's QuerySet.prefetch_related().
	///
	/// # Examples
	///
	/// ```ignore
	/// let posts = Post::objects().prefetch_related(&["comments", "tags"]).all().await?;
	/// ```
	pub fn prefetch_related(&self, fields: &[&str]) -> QuerySet<M> {
		QuerySet::new().prefetch_related(fields)
	}

	/// Select specific fields (values_list)
	///
	/// Alias for `values()`. Returns records with only the specified fields.
	/// Corresponds to Django's QuerySet.values_list().
	///
	/// # Examples
	///
	/// ```ignore
	/// let user_data = User::objects().values_list(&["id", "username"]).all().await?;
	/// ```
	pub fn values_list(&self, fields: &[&str]) -> QuerySet<M> {
		QuerySet::new().values_list(fields)
	}

	/// Filter by array overlap (PostgreSQL)
	///
	/// Filters rows where the array field overlaps with the provided values.
	/// Uses the `&&` operator in PostgreSQL.
	///
	/// # Examples
	///
	/// ```ignore
	/// let posts = Post::objects().filter_array_overlap("tags", &["rust", "web"]).all().await?;
	/// ```
	pub fn filter_array_overlap(&self, field: &str, values: &[&str]) -> QuerySet<M> {
		QuerySet::new().filter_array_overlap(field, values)
	}

	/// Filter by array contains (PostgreSQL)
	///
	/// Filters rows where the array field contains all provided values.
	/// Uses the `@>` operator in PostgreSQL.
	///
	/// # Examples
	///
	/// ```ignore
	/// let posts = Post::objects().filter_array_contains("tags", &["rust", "web"]).all().await?;
	/// ```
	pub fn filter_array_contains(&self, field: &str, values: &[&str]) -> QuerySet<M> {
		QuerySet::new().filter_array_contains(field, values)
	}

	/// Filter by JSONB contains (PostgreSQL)
	///
	/// Filters rows where the JSONB field contains the provided JSON.
	/// Uses the `@>` operator in PostgreSQL.
	///
	/// # Examples
	///
	/// ```ignore
	/// let users = User::objects().filter_jsonb_contains("metadata", r#"{"role": "admin"}"#).all().await?;
	/// ```
	pub fn filter_jsonb_contains(&self, field: &str, json: &str) -> QuerySet<M> {
		QuerySet::new().filter_jsonb_contains(field, json)
	}

	/// Filter by JSONB key exists (PostgreSQL)
	///
	/// Filters rows where the JSONB field has the specified key.
	/// Uses the `?` operator in PostgreSQL.
	///
	/// # Examples
	///
	/// ```ignore
	/// let users = User::objects().filter_jsonb_key_exists("metadata", "email").all().await?;
	/// ```
	pub fn filter_jsonb_key_exists(&self, field: &str, key: &str) -> QuerySet<M> {
		QuerySet::new().filter_jsonb_key_exists(field, key)
	}

	/// Filter by range contains (PostgreSQL)
	///
	/// Filters rows where the range field contains the provided value.
	/// Uses the `@>` operator in PostgreSQL.
	///
	/// # Examples
	///
	/// ```ignore
	/// let events = Event::objects().filter_range_contains("date_range", "2024-01-15").all().await?;
	/// ```
	pub fn filter_range_contains(&self, field: &str, value: &str) -> QuerySet<M> {
		QuerySet::new().filter_range_contains(field, value)
	}

	/// Filter by IN subquery
	///
	/// Filters rows where the field value is in the result of a subquery.
	///
	/// # Examples
	///
	/// ```ignore
	/// let authors = Author::objects()
	///     .filter_in_subquery("id", |subq: QuerySet<Book>| {
	///         subq.filter(Filter::new("price", FilterOperator::Gt, FilterValue::Int(1500)))
	///             .values(&["author_id"])
	///     })
	///     .all()
	///     .await?;
	/// ```
	pub fn filter_in_subquery<R: super::Model, F>(&self, field: &str, subquery_fn: F) -> QuerySet<M>
	where
		F: FnOnce(QuerySet<R>) -> QuerySet<R>,
	{
		QuerySet::new().filter_in_subquery(field, subquery_fn)
	}

	/// Filter by NOT IN subquery
	///
	/// Filters rows where the field value is not in the result of a subquery.
	///
	/// # Examples
	///
	/// ```ignore
	/// let authors = Author::objects()
	///     .filter_not_in_subquery("id", |subq: QuerySet<Book>| {
	///         subq.filter(Filter::new("status", FilterOperator::Eq, FilterValue::String("archived".into())))
	///             .values(&["author_id"])
	///     })
	///     .all()
	///     .await?;
	/// ```
	pub fn filter_not_in_subquery<R: super::Model, F>(
		&self,
		field: &str,
		subquery_fn: F,
	) -> QuerySet<M>
	where
		F: FnOnce(QuerySet<R>) -> QuerySet<R>,
	{
		QuerySet::new().filter_not_in_subquery(field, subquery_fn)
	}

	/// Filter by EXISTS subquery
	///
	/// Filters rows where the subquery returns at least one row.
	///
	/// # Examples
	///
	/// ```ignore
	/// let authors = Author::objects()
	///     .filter_exists(|subq: QuerySet<Book>| {
	///         subq.filter(Filter::new("author_id", FilterOperator::Eq, FilterValue::FieldRef(F::new("authors.id"))))
	///     })
	///     .all()
	///     .await?;
	/// ```
	pub fn filter_exists<R: super::Model, F>(&self, subquery_fn: F) -> QuerySet<M>
	where
		F: FnOnce(QuerySet<R>) -> QuerySet<R>,
	{
		QuerySet::new().filter_exists(subquery_fn)
	}

	/// Filter by NOT EXISTS subquery
	///
	/// Filters rows where the subquery returns no rows.
	///
	/// # Examples
	///
	/// ```ignore
	/// let authors = Author::objects()
	///     .filter_not_exists(|subq: QuerySet<Book>| {
	///         subq.filter(Filter::new("author_id", FilterOperator::Eq, FilterValue::FieldRef(F::new("authors.id"))))
	///     })
	///     .all()
	///     .await?;
	/// ```
	pub fn filter_not_exists<R: super::Model, F>(&self, subquery_fn: F) -> QuerySet<M>
	where
		F: FnOnce(QuerySet<R>) -> QuerySet<R>,
	{
		QuerySet::new().filter_not_exists(subquery_fn)
	}

	/// Add Common Table Expression (WITH clause)
	///
	/// Adds a CTE that can be referenced in the main query.
	///
	/// # Examples
	///
	/// ```ignore
	/// let high_earners = CTE::new("high_earners", "SELECT * FROM employees WHERE salary > 100000");
	/// let results = Employee::objects()
	///     .with_cte(high_earners)
	///     .all()
	///     .await?;
	/// ```
	pub fn with_cte(&self, cte: super::cte::CTE) -> QuerySet<M> {
		QuerySet::new().with_cte(cte)
	}

	/// Full-text search (PostgreSQL)
	///
	/// Filters rows using PostgreSQL's full-text search.
	///
	/// # Examples
	///
	/// ```ignore
	/// let articles = Article::objects()
	///     .full_text_search("search_vector", "rust programming")
	///     .all()
	///     .await?;
	/// ```
	pub fn full_text_search(&self, field: &str, query: &str) -> QuerySet<M> {
		QuerySet::new().full_text_search(field, query)
	}

	/// Annotate with subquery
	///
	/// Adds a scalar subquery to the SELECT clause.
	///
	/// # Examples
	///
	/// ```ignore
	/// let authors = Author::objects()
	///     .annotate_subquery::<Book, _>("book_count", |subq| {
	///         subq.filter("author_id", FilterOperator::Eq, FilterValue::OuterRef(OuterRef::new("authors.id")))
	///             .values(&["COUNT(*)"])
	///     })
	///     .all()
	///     .await?;
	/// ```
	pub fn annotate_subquery<R, F>(&self, name: &str, builder: F) -> QuerySet<M>
	where
		R: super::Model + 'static,
		F: FnOnce(QuerySet<R>) -> QuerySet<R>,
	{
		QuerySet::new().annotate_subquery(name, builder)
	}

	/// Get a record by composite primary key
	///
	/// Retrieves a single object using all fields of a composite primary key.
	///
	/// # Examples
	///
	/// ```ignore
	/// let mut pk_values = HashMap::new();
	/// pk_values.insert("post_id".to_string(), PkValue::Int(1));
	/// pk_values.insert("tag_id".to_string(), PkValue::Int(5));
	/// let post_tag = PostTag::objects().get_composite(&pk_values).await?;
	/// ```
	pub async fn get_composite(
		&self,
		pk_values: &std::collections::HashMap<String, super::composite_pk::PkValue>,
	) -> reinhardt_core::exception::Result<M>
	where
		M: Clone + serde::de::DeserializeOwned,
	{
		QuerySet::new().get_composite(pk_values).await
	}

	/// Create a new record using reinhardt-query for SQL injection protection
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
	/// # use reinhardt_db::orm::{Model, Manager, TransactionScope};
	/// # async fn example<M: Model>(manager: Manager<M>, model: &M) -> reinhardt_core::exception::Result<()> {
	/// use reinhardt_db::orm::manager::get_connection;
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

		// Build reinhardt-query INSERT statement
		let mut stmt = Query::insert();
		stmt.into_table(Alias::new(M::table_name()));

		// Get the primary key field name to filter out auto-increment fields
		let pk_field = M::primary_key_field();

		// Filter out primary key fields and null datetime fields.
		// Null datetime fields are skipped to let database DEFAULT apply
		// (e.g., created_at, updated_at with DEFAULT CURRENT_TIMESTAMP).
		let (fields, values): (Vec<_>, Vec<_>) = obj
			.iter()
			.filter(|(k, v)| {
				let key = k.as_str();
				// Exclude primary key field if it's null or 0 (auto-increment)
				if key == pk_field {
					if v.is_null() {
						return false;
					}
					if let Some(n) = v.as_i64() {
						return n != 0;
					}
				}
				// Skip null datetime fields to let database DEFAULT apply
				if v.is_null()
					&& (key == "created_at"
						|| key == "updated_at"
						|| key.ends_with("_date")
						|| key.ends_with("_time")
						|| key.ends_with("_at"))
				{
					return false;
				}
				true
			})
			.map(|(k, v)| {
				// Convert null values to SQL NULL for proper insertion
				let value = if v.is_null() {
					reinhardt_query::value::Value::Int(None)
				} else {
					Self::json_to_sea_value(v)
				};
				(Alias::new(k.as_str()), value)
			})
			.unzip();

		stmt.columns(fields);
		stmt.values_panic(values);

		// Add RETURNING clause with explicit column names from JSON object
		// Note: Using Asterisk in columns() may not work correctly with reinhardt-query
		let all_columns: Vec<_> = obj.keys().map(|k| Alias::new(k.as_str())).collect();
		stmt.returning(all_columns);

		let (sql, values) = build_insert_sql(&stmt, conn.backend());
		let values: Vec<_> = values
			.0
			.into_iter()
			.map(Self::sea_value_to_query_value)
			.collect();

		let row = conn.query_one(&sql, values).await?;

		// row.data is already serde_json::Value::Object so deserialize directly
		serde_json::from_value(row.data.clone())
			.map_err(|e| reinhardt_core::exception::Error::Database(e.to_string()))
	}

	/// Convert serde_json::Value to reinhardt_query::value::Value for parameter binding
	fn json_to_sea_value(v: &serde_json::Value) -> reinhardt_query::value::Value {
		match v {
			serde_json::Value::Null => reinhardt_query::value::Value::Int(None),
			serde_json::Value::Bool(b) => reinhardt_query::value::Value::Bool(Some(*b)),
			serde_json::Value::Number(n) => {
				if let Some(i) = n.as_i64() {
					reinhardt_query::value::Value::BigInt(Some(i))
				} else if let Some(f) = n.as_f64() {
					reinhardt_query::value::Value::Double(Some(f))
				} else {
					reinhardt_query::value::Value::Int(None)
				}
			}
			serde_json::Value::String(s) => {
				// 1. Try to parse as UUID (format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx)
				//    UUIDs are often serialized as strings via serde
				if let Ok(uuid) = Uuid::parse_str(s) {
					return reinhardt_query::value::Value::Uuid(Some(Box::new(uuid)));
				}

				// 2. Try to parse as ISO 8601 datetime (chrono::DateTime<Utc>)
				// This handles timestamps serialized by serde_json from chrono::DateTime

				// 2.1 Try RFC3339 strict format first (e.g., "2024-01-01T00:00:00+00:00")
				if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
					return reinhardt_query::value::Value::ChronoDateTimeUtc(Some(Box::new(
						dt.with_timezone(&chrono::Utc),
					)));
				}

				// 2.2 Try chrono's FromStr trait for DateTime<Utc>
				//    This handles formats like "2024-01-01T00:00:00Z" with optional subseconds
				if let Ok(dt) = s.parse::<chrono::DateTime<chrono::Utc>>() {
					return reinhardt_query::value::Value::ChronoDateTimeUtc(Some(Box::new(dt)));
				}

				// 2.3 Try parsing with FixedOffset timezone then convert to UTC
				//    Handles formats like "2024-01-01T00:00:00.123456789+00:00"
				if let Ok(dt) = s.parse::<chrono::DateTime<chrono::FixedOffset>>() {
					return reinhardt_query::value::Value::ChronoDateTimeUtc(Some(Box::new(
						dt.with_timezone(&chrono::Utc),
					)));
				}

				// Fallback: treat as regular string (non-datetime, non-UUID values)
				reinhardt_query::value::Value::String(Some(Box::new(s.clone())))
			}
			serde_json::Value::Array(arr) => {
				// Convert JSON array to reinhardt_query::value::Value array
				// Array(ArrayType, Option<Box<Vec<Value>>>)
				let values: Vec<reinhardt_query::value::Value> =
					arr.iter().map(|v| Self::json_to_sea_value(v)).collect();
				reinhardt_query::value::Value::Array(
					reinhardt_query::value::ArrayType::String,
					Some(Box::new(values)),
				)
			}
			serde_json::Value::Object(_obj) => {
				// Use reinhardt-query's Json type for PostgreSQL JSONB/JSON columns
				// Json expects Box<serde_json::Value>
				reinhardt_query::value::Value::Json(Some(Box::new(v.clone())))
			}
		}
	}

	/// Convert reinhardt_query::value::Value to QueryValue for database parameter binding
	fn sea_value_to_query_value(v: reinhardt_query::value::Value) -> super::connection::QueryValue {
		use super::connection::QueryValue;

		match v {
			reinhardt_query::value::Value::Bool(Some(b)) => QueryValue::Bool(b),
			reinhardt_query::value::Value::Bool(None) => QueryValue::Null,

			reinhardt_query::value::Value::TinyInt(Some(i)) => QueryValue::Int(i as i64),
			reinhardt_query::value::Value::TinyInt(None) => QueryValue::Null,
			reinhardt_query::value::Value::SmallInt(Some(i)) => QueryValue::Int(i as i64),
			reinhardt_query::value::Value::SmallInt(None) => QueryValue::Null,
			reinhardt_query::value::Value::Int(Some(i)) => QueryValue::Int(i as i64),
			reinhardt_query::value::Value::Int(None) => QueryValue::Null,
			reinhardt_query::value::Value::BigInt(Some(i)) => QueryValue::Int(i),
			reinhardt_query::value::Value::BigInt(None) => QueryValue::Null,

			reinhardt_query::value::Value::TinyUnsigned(Some(u)) => QueryValue::Int(u as i64),
			reinhardt_query::value::Value::TinyUnsigned(None) => QueryValue::Null,
			reinhardt_query::value::Value::SmallUnsigned(Some(u)) => QueryValue::Int(u as i64),
			reinhardt_query::value::Value::SmallUnsigned(None) => QueryValue::Null,
			reinhardt_query::value::Value::Unsigned(Some(u)) => QueryValue::Int(u as i64),
			reinhardt_query::value::Value::Unsigned(None) => QueryValue::Null,
			reinhardt_query::value::Value::BigUnsigned(Some(u)) => QueryValue::Int(u as i64),
			reinhardt_query::value::Value::BigUnsigned(None) => QueryValue::Null,

			reinhardt_query::value::Value::Float(Some(f)) => QueryValue::Float(f as f64),
			reinhardt_query::value::Value::Float(None) => QueryValue::Null,
			reinhardt_query::value::Value::Double(Some(f)) => QueryValue::Float(f),
			reinhardt_query::value::Value::Double(None) => QueryValue::Null,

			reinhardt_query::value::Value::String(Some(s)) => QueryValue::String((*s).clone()),
			reinhardt_query::value::Value::String(None) => QueryValue::Null,

			reinhardt_query::value::Value::Bytes(Some(b)) => QueryValue::Bytes((*b).clone()),
			reinhardt_query::value::Value::Bytes(None) => QueryValue::Null,

			// Timestamp handling
			// ChronoDateTime contains NaiveDateTime, convert to UTC
			reinhardt_query::value::Value::ChronoDateTime(Some(dt)) => {
				QueryValue::Timestamp(dt.and_utc())
			}
			reinhardt_query::value::Value::ChronoDateTime(None) => QueryValue::Null,
			reinhardt_query::value::Value::ChronoDateTimeUtc(Some(dt)) => {
				QueryValue::Timestamp(*dt)
			}
			reinhardt_query::value::Value::ChronoDateTimeUtc(None) => QueryValue::Null,

			// UUID handling
			reinhardt_query::value::Value::Uuid(Some(u)) => QueryValue::Uuid(*u),
			reinhardt_query::value::Value::Uuid(None) => QueryValue::Null,

			// JSON types - serialize to string
			reinhardt_query::value::Value::Json(Some(json)) => QueryValue::String(json.to_string()),
			reinhardt_query::value::Value::Json(None) => QueryValue::Null,

			// For complex types or unsupported types, convert to null
			// This is a safe fallback that won't cause runtime errors
			_ => QueryValue::Null,
		}
	}

	/// Serialize a JSON value to SQL-compatible string representation
	// Allow dead_code: internal helper for JSON-to-SQL serialization in manager operations
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

	fn primary_key_fields(field_metadata: &[FieldInfo]) -> Vec<String> {
		let Some(composite) = M::composite_primary_key() else {
			return vec![M::primary_key_field().to_owned()];
		};

		// Serialized values are keyed by logical names even when composite metadata
		// names physical columns. Prefer the marked model fields when available.
		let fields: Vec<_> = field_metadata
			.iter()
			.filter(|field| field.primary_key)
			.map(|field| field.name.clone())
			.collect();
		if fields.len() == composite.fields().len() {
			return fields;
		}

		composite
			.fields()
			.iter()
			.map(|name| {
				field_metadata
					.iter()
					.find(|field| field.name == *name || field.db_column_name() == name)
					.map_or_else(|| name.clone(), |field| field.name.clone())
			})
			.collect()
	}

	fn column_name<'a>(name: &'a str, metadata: &'a [FieldInfo]) -> &'a str {
		metadata
			.iter()
			.find(|field| field.name == name)
			.map_or(name, FieldInfo::db_column_name)
	}

	fn primary_key_condition_from_object(
		model: &M,
		obj: &serde_json::Map<String, serde_json::Value>,
	) -> reinhardt_core::exception::Result<Condition> {
		use super::composite_pk::PkValue;
		use reinhardt_core::exception::Error;
		use reinhardt_query::value::Value;

		let metadata = M::field_metadata();
		let fields = Self::primary_key_fields(&metadata);

		let mut condition = Condition::all();
		if M::composite_primary_key().is_some() {
			for field in &fields {
				if obj.get(field).is_none_or(serde_json::Value::is_null) {
					return Err(Error::Database(format!(
						"Model must have non-null primary key field '{field}'"
					)));
				}
			}
			let values = model.get_composite_pk_values();
			for field in fields {
				let column = Self::column_name(&field, &metadata);
				let value = values
					.get(column)
					.or_else(|| values.get(&field))
					.ok_or_else(|| {
						Error::Database(format!("Model must have primary key field '{field}'"))
					})?;
				// Composite key values already carry their storage types. In particular,
				// UUID-looking or numeric strings must remain text bindings.
				let value = match value {
					PkValue::String(value) => Value::String(Some(Box::new(value.clone()))),
					PkValue::Int(value) => Value::BigInt(Some(*value)),
					PkValue::Uint(value) => Value::BigUnsigned(Some(*value)),
					PkValue::Bool(value) => Value::Bool(Some(*value)),
				};
				condition = condition.add(Expr::col(Alias::new(column)).eq(value));
			}
		} else {
			let pk = model
				.primary_key()
				.ok_or_else(|| Error::Database("Model must have primary key".to_owned()))?;
			let value = M::primary_key_filter_value(pk);
			let value = QuerySet::<M>::filter_value_to_sea_value(&value);
			let column = Self::column_name(M::primary_key_field(), &metadata);
			condition = condition.add(Expr::col(Alias::new(column)).eq(value));
		}
		Ok(condition)
	}

	fn build_update_statement_from_object_with_returning(
		model: &M,
		obj: &serde_json::Map<String, serde_json::Value>,
		returning: bool,
	) -> reinhardt_core::exception::Result<UpdateStatement> {
		let metadata = M::field_metadata();
		let key_fields = Self::primary_key_fields(&metadata);
		let condition = Self::primary_key_condition_from_object(model, obj)?;
		let mut stmt = Query::update();
		stmt.table(Alias::new(M::table_name()));
		let mut has_assignment = false;
		for (field, value) in obj.iter().filter(|(field, _)| !key_fields.contains(field)) {
			let column = Alias::new(Self::column_name(field, &metadata));
			if value.is_null() {
				// An untyped NULL also works for timestamp and UUID columns.
				stmt.value_expr(column, Expr::cust("NULL"));
			} else {
				stmt.value(column, Self::json_to_sea_value(value));
			}
			has_assignment = true;
		}
		if !has_assignment {
			// Key-only models still need a valid statement and the complete predicate.
			let column = Alias::new(Self::column_name(&key_fields[0], &metadata));
			stmt.value_expr(column.clone(), Expr::col(column));
		}
		stmt.cond_where(condition);
		if returning {
			stmt.returning(
				obj.keys()
					.map(|field| Alias::new(Self::column_name(field, &metadata))),
			);
		}
		Ok(stmt)
	}

	/// Update an existing record using reinhardt-query for SQL injection protection
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
	/// # use reinhardt_db::orm::{Model, Manager, TransactionScope};
	/// # async fn example<M: Model>(manager: Manager<M>, model: &M) -> reinhardt_core::exception::Result<()> {
	/// use reinhardt_db::orm::manager::get_connection;
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
		let json = serde_json::to_value(model)
			.map_err(|e| reinhardt_core::exception::Error::Database(e.to_string()))?;
		let obj = json.as_object().ok_or_else(|| {
			reinhardt_core::exception::Error::Database("Model must serialize to object".to_owned())
		})?;
		let returning = conn.backend() != DatabaseBackend::MySql;
		let stmt = Self::build_update_statement_from_object_with_returning(model, obj, returning)?;
		let (sql, values) = build_update_sql(&stmt, conn.backend());
		let values = values
			.0
			.into_iter()
			.map(Self::sea_value_to_query_value)
			.collect();
		let metadata = M::field_metadata();
		let row = if returning {
			conn.query_one(&sql, values).await?
		} else {
			conn.execute(&sql, values).await?;
			let mut select = Query::select();
			select
				.from(Alias::new(M::table_name()))
				.columns(
					obj.keys()
						.map(|field| Alias::new(Self::column_name(field, &metadata))),
				)
				.cond_where(Self::primary_key_condition_from_object(model, obj)?)
				.limit(1);
			let (sql, values) = build_select_sql(&select, conn.backend());
			let values = values
				.0
				.into_iter()
				.map(Self::sea_value_to_query_value)
				.collect();
			conn.query_one(&sql, values).await?
		};
		// RETURNING and MySQL reloads yield physical columns; serde expects model fields.
		let data = obj
			.keys()
			.filter_map(|field| {
				row.data
					.get(Self::column_name(field, &metadata))
					.map(|value| (field.clone(), value.clone()))
			})
			.collect();
		serde_json::from_value(serde_json::Value::Object(data))
			.map_err(|e| reinhardt_core::exception::Error::Database(e.to_string()))
	}

	/// Delete a record using reinhardt-query for SQL injection protection
	pub async fn delete(&self, pk: M::PrimaryKey) -> reinhardt_core::exception::Result<()> {
		let conn = get_connection().await?;
		self.delete_with_conn(&conn, pk).await
	}

	fn build_delete_statement(pk: M::PrimaryKey) -> DeleteStatement {
		let primary_key_field = M::primary_key_field();
		let primary_key_column = M::field_metadata()
			.into_iter()
			.find(|field| field.name == primary_key_field)
			.map(|field| field.db_column_name().to_owned())
			.unwrap_or_else(|| primary_key_field.to_owned());
		let primary_key_value = M::primary_key_filter_value(pk);
		let primary_key_value = QuerySet::<M>::filter_value_to_sea_value(&primary_key_value);

		let mut stmt = Query::delete();
		stmt.from_table(Alias::new(M::table_name()))
			.and_where(Expr::col(Alias::new(primary_key_column)).eq(primary_key_value));
		stmt
	}

	/// Delete a record with an explicit database connection
	///
	/// This method allows using a specific connection, which is essential for
	/// transaction support. Primary-key columns are resolved from model field
	/// metadata, and values use the model's typed primary-key binding.
	///
	/// # Arguments
	///
	/// * `conn` - The database connection to use
	/// * `pk` - The primary key of the record to delete
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_db::orm::{Model, Manager, TransactionScope};
	/// # async fn example<M: Model>(manager: Manager<M>, pk: M::PrimaryKey) -> reinhardt_core::exception::Result<()> {
	/// use reinhardt_db::orm::manager::get_connection;
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
		let stmt = Self::build_delete_statement(pk);

		let (sql, values) = build_delete_sql(&stmt, conn.backend());
		let values: Vec<_> = values
			.0
			.into_iter()
			.map(Self::sea_value_to_query_value)
			.collect();

		conn.execute(&sql, values).await?;
		Ok(())
	}

	/// Count records using reinhardt-query
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
	/// # use reinhardt_db::orm::{Model, Manager, TransactionScope};
	/// # async fn example<M: Model>(manager: Manager<M>) -> reinhardt_core::exception::Result<()> {
	/// use reinhardt_db::orm::manager::get_connection;
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
		// Build reinhardt-query SELECT COUNT(*) statement with explicit alias
		let stmt = Query::select()
			.from(Alias::new(M::table_name()))
			.expr_as(Func::count(Expr::asterisk().into()), Alias::new("count"))
			.to_owned();

		let (sql, values) = build_select_sql(&stmt, conn.backend());
		let values: Vec<_> = values
			.0
			.into_iter()
			.map(Self::sea_value_to_query_value)
			.collect();

		let row = conn.query_one(&sql, values).await?;
		row.get::<i64>("count").ok_or_else(|| {
			reinhardt_core::exception::Error::Database("Failed to get count".to_string())
		})
	}

	/// Bulk create multiple records using reinhardt-query (similar to Django's bulk_create())
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

		// Build reinhardt-query INSERT statement
		let mut stmt = Query::insert();
		stmt.into_table(Alias::new(M::table_name())).columns(fields);

		// Add value rows for each model
		for val in &json_values {
			if let Some(obj) = val.as_object() {
				let values: Vec<reinhardt_query::value::Value> = first_obj
					.keys()
					.map(|field| {
						obj.get(field)
							.map(|v| {
								if v.is_null() {
									// Use untyped NULL to avoid PostgreSQL type mismatch errors
									reinhardt_query::value::Value::Int(None)
								} else {
									Self::json_to_sea_value(v)
								}
							})
							// Use untyped NULL for missing fields
							.unwrap_or(reinhardt_query::value::Value::Int(None))
					})
					.collect();
				stmt.values_panic(values);
			}
		}

		Some(stmt.to_owned())
	}

	/// Generate bulk create SQL (convenience method)
	///
	/// # Arguments
	///
	/// * `models` - Models to insert
	/// * `backend` - Database backend to generate SQL for
	pub fn bulk_create_sql(&self, models: &[M], backend: DatabaseBackend) -> String {
		if let Some(stmt) = self.bulk_create_query(models) {
			insert_to_string(&stmt, backend)
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
		use crate::orm::query::UpdateValue;
		use std::collections::HashMap;

		// Convert &[(&str, &str)] to HashMap<String, UpdateValue>
		let updates_map: HashMap<String, UpdateValue> = updates
			.iter()
			.map(|(key, value)| (key.to_string(), UpdateValue::String(value.to_string())))
			.collect();

		queryset.update_sql(&updates_map)
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
		let (select_sql, _) = self.get_or_create_sql(
			&lookup_fields,
			&defaults.clone().unwrap_or_default(),
			conn.backend(),
		);

		if let Ok(Some(row)) = conn.query_optional(&select_sql, vec![]).await {
			// row.data is already serde_json::Value::Object so deserialize directly
			let model: M = serde_json::from_value(row.data.clone())
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
		// row.data is already serde_json::Value::Object so deserialize directly
		let model: M = serde_json::from_value(row.data.clone())
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
	/// - batch_size: A positive size splits the input into batches; `None` uses a single batch
	/// - ignore_conflicts: Skip records that would violate constraints
	/// - update_conflicts: Update existing records instead of failing
	///
	/// PostgreSQL and SQLite return the inserted rows, including generated primary
	/// keys. MySQL executes the insert without `RETURNING` and returns the input
	/// models; generated keys and database defaults are not hydrated. When
	/// `ignore_conflicts` is enabled, all backends return an empty vector. MySQL
	/// uses `INSERT IGNORE` to skip conflicting rows.
	/// MySQL uses field metadata for `db_column` names and binary, datetime, and
	/// JSON bindings. Serialized null values remain SQL NULL.
	///
	/// Empty input returns an empty vector without accessing the database, regardless of batch size.
	///
	/// # Errors
	///
	/// Nonempty input with `batch_size: Some(0)` returns
	/// [`reinhardt_core::exception::Error::Validation`] before accessing the database.
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

		validate_bulk_batch_size(batch_size)?;
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
			// Exclude primary key field if it's Null (for auto-increment)
			let pk_field = M::primary_key_field();
			let field_names: Vec<String> = obj
				.iter()
				.filter_map(|(k, v)| {
					if k == pk_field && v.is_null() {
						None
					} else {
						Some(k.clone())
					}
				})
				.collect();

			// Extract values for all models in chunk
			let value_rows: Vec<Vec<serde_json::Value>> = chunk
				.iter()
				.map(|model| {
					let json = serde_json::to_value(model).unwrap();
					let obj = json.as_object().unwrap();
					field_names.iter().map(|field| obj[field].clone()).collect()
				})
				.collect();

			if conn.backend() == DatabaseBackend::MySql {
				let statement = Self::mysql_bulk_create_query(&field_names, &value_rows)?;
				let (sql, values) = build_insert_sql(&statement, DatabaseBackend::MySql);
				let sql = if ignore_conflicts {
					sql.replacen("INSERT INTO", "INSERT IGNORE INTO", 1)
				} else {
					sql
				};
				let values = values
					.0
					.into_iter()
					.map(Self::sea_value_to_query_value)
					.collect();
				conn.execute(&sql, values).await?;
				if !ignore_conflicts {
					results.extend(chunk.iter().cloned());
				}
				continue;
			}

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
					// row.data is already serde_json::Value::Object so deserialize directly
					let model: M = serde_json::from_value(row.data.clone())
						.map_err(|e| reinhardt_core::exception::Error::Database(e.to_string()))?;
					results.push(model);
				}
			}
		}

		Ok(results)
	}

	fn mysql_bulk_create_query(
		field_names: &[String],
		value_rows: &[Vec<serde_json::Value>],
	) -> reinhardt_core::exception::Result<InsertStatement> {
		let metadata = M::field_metadata();
		let mut statement = Query::insert();
		statement.into_table(Alias::new(M::table_name())).columns(
			field_names
				.iter()
				.map(|field| Alias::new(Self::column_name(field, &metadata))),
		);
		for row in value_rows {
			let values: Vec<reinhardt_query::value::Value> = row
				.iter()
				.zip(field_names)
				.map(|(value, name)| {
					let field_type = metadata
						.iter()
						.find(|field| field.name == *name)
						.and_then(|field| field.field_type.rsplit('.').next());
					if !value.is_null() && field_type == Some("BinaryField") {
						let bytes =
							serde_json::from_value::<Vec<u8>>(value.clone()).map_err(|error| {
								reinhardt_core::exception::Error::Database(format!(
									"Invalid binary value for field '{name}': {error}"
								))
							})?;
						return Ok(reinhardt_query::value::Value::Bytes(Some(Box::new(bytes))));
					}
					if !value.is_null()
						&& matches!(
							field_type,
							Some("JsonField" | "JSONField" | "JsonbField" | "JSONBField")
						) {
						// JSON string scalars need their quotes, even when their contents
						// resemble booleans, numbers, UUIDs, or timestamps.
						return Ok(value.to_string().into());
					}
					if field_type == Some("DateTimeField")
						&& let serde_json::Value::String(text) = value
					{
						let timestamp =
							chrono::DateTime::parse_from_rfc3339(text).map_err(|error| {
								reinhardt_core::exception::Error::Database(format!(
									"Invalid datetime value for field '{name}': {error}"
								))
							})?;
						return Ok(reinhardt_query::value::Value::ChronoDateTimeUtc(Some(
							Box::new(timestamp.with_timezone(&chrono::Utc)),
						)));
					}
					Ok(match value {
						// Preserve legacy text and JSON values instead of inferring UUID
						// or timestamp bindings from serialized strings.
						serde_json::Value::String(text) => text.clone().into(),
						serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
							value.to_string().into()
						}
						// QueryValue has no unsigned variant; numeric text retains the
						// full range when MySQL converts it to an unsigned column.
						serde_json::Value::Number(number)
							if number.is_u64() && number.as_i64().is_none() =>
						{
							number.to_string().into()
						}
						value => Self::json_to_sea_value(value),
					})
				})
				.collect::<reinhardt_core::exception::Result<_>>()?;
			statement.values_panic(values);
		}
		Ok(statement)
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
	///
	/// A positive `batch_size` splits the input into batches; `None` uses a single batch.
	/// Empty models or fields return zero without accessing the database, regardless of batch size.
	///
	/// # Errors
	///
	/// Nonempty models and fields with `batch_size: Some(0)` return
	/// [`reinhardt_core::exception::Error::Validation`] before accessing the database.
	pub async fn bulk_update(
		&self,
		models: Vec<M>,
		fields: Vec<String>,
		batch_size: Option<usize>,
	) -> reinhardt_core::exception::Result<usize> {
		if models.is_empty() || fields.is_empty() {
			return Ok(0);
		}

		validate_bulk_batch_size(batch_size)?;
		let conn = get_connection().await?;
		let batch_size = batch_size.unwrap_or(models.len());
		let mut total_updated = 0;

		for chunk in models.chunks(batch_size) {
			// Build updates structure
			let updates: Vec<(M::PrimaryKey, HashMap<String, serde_json::Value>)> = chunk
				.iter()
				.filter_map(|model| {
					let pk = model.primary_key()?.clone();
					let json = serde_json::to_value(model).ok()?;
					let obj = json.as_object()?;

					let mut field_map = HashMap::new();
					for field in &fields {
						if let Some(val) = obj.get(field) {
							field_map.insert(field.clone(), val.clone());
						}
					}

					Some((pk, field_map))
				})
				.collect();

			if !updates.is_empty() {
				let sql = self.bulk_update_sql_detailed(&updates, &fields, conn.backend());
				let rows_affected = conn.execute(&sql, vec![]).await?;
				total_updated += rows_affected as usize;
			}
		}

		Ok(total_updated)
	}

	/// Get or create - SQL generation using reinhardt-query (for testing)
	pub fn get_or_create_queries(
		&self,
		lookup_fields: &HashMap<String, String>,
		defaults: &HashMap<String, String>,
	) -> (SelectStatement, InsertStatement) {
		// Generate SELECT query with reinhardt-query
		let mut select_stmt = Query::select();
		select_stmt
			.from(Alias::new(M::table_name()))
			.column(ColumnRef::Asterisk);

		for (k, v) in lookup_fields.iter() {
			select_stmt.and_where(Expr::col(Alias::new(k.as_str())).eq(v.as_str()));
		}

		// Generate INSERT query with reinhardt-query
		let mut insert_fields = lookup_fields.clone();
		insert_fields.extend(defaults.clone());

		let mut insert_stmt = Query::insert();
		insert_stmt.into_table(Alias::new(M::table_name()));

		let columns: Vec<_> = insert_fields
			.keys()
			.map(|k| Alias::new(k.as_str()))
			.collect();
		let values: Vec<reinhardt_query::prelude::Expr> = insert_fields
			.values()
			.map(|v| Expr::val(v.clone()))
			.collect();

		insert_stmt.columns(columns);
		insert_stmt.values_panic(values);

		(select_stmt.to_owned(), insert_stmt.to_owned())
	}

	/// Get or create - SQL generation (convenience method for testing)
	///
	/// # Arguments
	///
	/// * `lookup_fields` - Fields to lookup
	/// * `defaults` - Default values for creation
	/// * `backend` - Database backend to generate SQL for
	pub fn get_or_create_sql(
		&self,
		lookup_fields: &HashMap<String, String>,
		defaults: &HashMap<String, String>,
		backend: DatabaseBackend,
	) -> (String, String) {
		let (select_stmt, insert_stmt) = self.get_or_create_queries(lookup_fields, defaults);
		(
			select_to_string(&select_stmt, backend),
			insert_to_string(&insert_stmt, backend),
		)
	}

	/// Bulk create - SQL generation only (for testing)
	pub fn bulk_create_sql_detailed(
		&self,
		field_names: &[String],
		value_rows: &[Vec<serde_json::Value>],
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
					.map(|v| match v {
						serde_json::Value::Null => "NULL".to_string(),
						serde_json::Value::Number(n) => n.to_string(),
						serde_json::Value::String(s) => {
							// SQL injection prevention: Escape single quotes
							format!("'{}'", s.replace("'", "''"))
						}
						serde_json::Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
						serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
							// Treat arrays and objects as JSON strings
							format!("'{}'", v.to_string().replace("'", "''"))
						}
					})
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

	/// Bulk update SQL generation using CASE expressions
	///
	/// Generates raw SQL because reinhardt-query's `UpdateStatement` does not support
	/// expression-based SET values (e.g., CASE WHEN ... END).
	pub fn bulk_update_sql_detailed(
		&self,
		updates: &[(M::PrimaryKey, HashMap<String, serde_json::Value>)],
		fields: &[String],
		_backend: DatabaseBackend,
	) -> String
	where
		M::PrimaryKey: std::fmt::Display + Clone,
	{
		if updates.is_empty() || fields.is_empty() {
			return String::new();
		}

		let table_name = M::table_name();
		let mut set_clauses = Vec::new();

		for field in fields {
			let mut when_clauses = Vec::new();

			for (pk, field_map) in updates.iter() {
				if let Some(value) = field_map.get(field) {
					let val_str = match value {
						serde_json::Value::Null => "NULL".to_string(),
						serde_json::Value::Bool(b) => b.to_string().to_uppercase(),
						serde_json::Value::Number(n) => n.to_string(),
						serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
						serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
							format!("'{}'", value.to_string().replace('\'', "''"))
						}
					};
					when_clauses.push(format!(
						"WHEN \"id\" = '{}' THEN {}",
						pk.to_string().replace('\'', "''"),
						val_str
					));
				}
			}

			if !when_clauses.is_empty() {
				set_clauses.push(format!(
					"\"{}\" = CASE {} END",
					field,
					when_clauses.join(" ")
				));
			}
		}

		let ids: Vec<String> = updates
			.iter()
			.map(|(pk, _)| format!("'{}'", pk.to_string().replace('\'', "''")))
			.collect();

		format!(
			"UPDATE \"{}\" SET {} WHERE \"id\" IN ({})",
			table_name,
			set_clauses.join(", "),
			ids.join(", ")
		)
	}
}

impl<M: Model> Default for Manager<M> {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::{Manager, build_delete_sql};
	use crate::orm::FieldSelector;
	use crate::orm::Model;
	use crate::orm::connection::DatabaseBackend;
	use crate::orm::fields::{BigIntegerField, BinaryField, CharField, DateTimeField, Field};
	use crate::orm::inspection::FieldInfo;
	use crate::orm::query::FilterValue;
	use rstest::{fixture, rstest};
	use serde::{Deserialize, Serialize};
	use serial_test::serial;
	use std::collections::HashMap;
	use std::fmt;
	use uuid::Uuid;

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	struct TestUser {
		id: Option<i64>,
		name: String,
		email: String,
	}

	impl TestUser {
		// Allow dead_code: test helper constructor for manager tests
		#[allow(dead_code)]
		fn new(name: String, email: String) -> Self {
			Self {
				id: None,
				name,
				email,
			}
		}
	}

	#[derive(Debug, Clone)]
	struct TestUserFields;

	impl FieldSelector for TestUserFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for TestUser {
		type PrimaryKey = i64;
		type Fields = TestUserFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"test_user"
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
	}

	#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
	struct TestBinaryRecord {
		id: Option<i64>,
		name: String,
		payload: Vec<u8>,
		optional_payload: Option<Vec<u8>>,
		json_data: serde_json::Value,
		optional_json_data: Option<serde_json::Value>,
		timestamp: chrono::DateTime<chrono::Utc>,
		optional_timestamp: Option<chrono::DateTime<chrono::Utc>>,
	}

	impl Model for TestBinaryRecord {
		type PrimaryKey = i64;
		type Fields = TestUserFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"test_binary_record"
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
			let mut id = BigIntegerField::new();
			id.base.primary_key = true;
			id.base.db_column = Some("record_id".to_owned());
			id.set_attributes_from_name("id");
			let mut name = CharField::new(255);
			name.base.db_column = Some("stored_name".to_owned());
			name.set_attributes_from_name("name");
			let mut payload = BinaryField::new();
			payload.set_attributes_from_name("payload");
			let mut optional_payload = BinaryField::new();
			optional_payload.base.null = true;
			optional_payload.set_attributes_from_name("optional_payload");
			let mut json_data = CharField::new(255);
			json_data.set_attributes_from_name("json_data");
			let mut json_data = FieldInfo::from_field(&json_data);
			json_data.field_type = "reinhardt.orm.models.JsonField".to_owned();
			let mut optional_json_data = json_data.clone();
			optional_json_data.name = "optional_json_data".to_owned();
			optional_json_data.nullable = true;
			let mut timestamp = DateTimeField::new();
			timestamp.set_attributes_from_name("timestamp");
			let mut optional_timestamp = DateTimeField::new();
			optional_timestamp.base.null = true;
			optional_timestamp.set_attributes_from_name("optional_timestamp");
			vec![
				FieldInfo::from_field(&id),
				FieldInfo::from_field(&name),
				FieldInfo::from_field(&payload),
				FieldInfo::from_field(&optional_payload),
				json_data,
				optional_json_data,
				FieldInfo::from_field(&timestamp),
				FieldInfo::from_field(&optional_timestamp),
			]
		}
	}

	#[rstest]
	#[case(vec![0, 255, 128, 39, 92], None)]
	#[case(vec![], Some(vec![]))]
	#[case(vec![0, 255], Some(vec![255, 0]))]
	fn mysql_bulk_create_query_binds_binary_fields(
		#[case] payload: Vec<u8>,
		#[case] optional_payload: Option<Vec<u8>>,
	) {
		use crate::orm::connection::QueryValue;
		// Arrange
		let fields = vec![
			"payload".to_owned(),
			"optional_payload".to_owned(),
			"json_data".to_owned(),
		];
		let rows = vec![vec![
			serde_json::json!(&payload),
			serde_json::json!(&optional_payload),
			serde_json::json!([0, 255]),
		]];
		// Act
		let statement =
			Manager::<TestBinaryRecord>::mysql_bulk_create_query(&fields, &rows).unwrap();
		let (sql, values) = super::build_insert_sql(&statement, DatabaseBackend::MySql);
		let params: Vec<_> = values
			.0
			.into_iter()
			.map(Manager::<TestBinaryRecord>::sea_value_to_query_value)
			.collect();
		// Assert
		let mut expected = vec![QueryValue::Bytes(payload)];
		let expected_sql = if let Some(optional_payload) = optional_payload {
			expected.push(QueryValue::Bytes(optional_payload));
			"INSERT INTO `test_binary_record` (`payload`, `optional_payload`, `json_data`) VALUES (?, ?, ?)"
		} else {
			"INSERT INTO `test_binary_record` (`payload`, `optional_payload`, `json_data`) VALUES (?, NULL, ?)"
		};
		expected.push(QueryValue::String("[0,255]".to_owned()));
		assert_eq!(sql, expected_sql);
		assert_eq!(params, expected);
	}

	#[rstest]
	fn mysql_bulk_create_query_uses_physical_columns() {
		// Arrange
		let fields = vec!["id".to_owned(), "name".to_owned(), "json_data".to_owned()];
		let rows = vec![vec![
			serde_json::json!(42),
			serde_json::json!("record"),
			serde_json::json!([]),
		]];
		// Act
		let statement =
			Manager::<TestBinaryRecord>::mysql_bulk_create_query(&fields, &rows).unwrap();
		let (sql, _) = super::build_insert_sql(&statement, DatabaseBackend::MySql);
		// Assert
		assert_eq!(
			sql,
			"INSERT INTO `test_binary_record` (`record_id`, `stored_name`, `json_data`) VALUES (?, ?, ?)"
		);
	}

	#[rstest]
	#[case(serde_json::json!([256]))]
	#[case(serde_json::json!([-1]))]
	#[case(serde_json::json!([1.5]))]
	#[case(serde_json::json!(["bytes"]))]
	fn mysql_bulk_create_query_rejects_invalid_binary(#[case] value: serde_json::Value) {
		// Arrange
		let expected = serde_json::from_value::<Vec<u8>>(value.clone()).unwrap_err();
		// Act
		let error = Manager::<TestBinaryRecord>::mysql_bulk_create_query(
			&["payload".to_owned()],
			&[vec![value]],
		)
		.unwrap_err();
		// Assert
		assert!(
			matches!(error, reinhardt_core::exception::Error::Database(message) if message == format!("Invalid binary value for field 'payload': {expected}"))
		);
	}

	#[rstest]
	#[case("2026-10-04T12:34:56.123456Z", None)]
	#[case("2026-10-04T12:34:56.123456+09:00", Some("2026-10-04T00:00:00Z"))]
	fn mysql_bulk_create_query_binds_datetime_fields(
		#[case] timestamp: &str,
		#[case] optional_timestamp: Option<&str>,
	) {
		use crate::orm::connection::QueryValue;
		// Arrange
		let fields = vec![
			"timestamp".to_owned(),
			"optional_timestamp".to_owned(),
			"name".to_owned(),
		];
		let rows = vec![vec![
			serde_json::json!(timestamp),
			serde_json::json!(optional_timestamp),
			serde_json::json!(timestamp),
		]];
		let expected_timestamp = chrono::DateTime::parse_from_rfc3339(timestamp)
			.unwrap()
			.with_timezone(&chrono::Utc);
		// Act
		let statement =
			Manager::<TestBinaryRecord>::mysql_bulk_create_query(&fields, &rows).unwrap();
		let (sql, values) = super::build_insert_sql(&statement, DatabaseBackend::MySql);
		let params: Vec<_> = values
			.0
			.into_iter()
			.map(Manager::<TestBinaryRecord>::sea_value_to_query_value)
			.collect();
		// Assert
		let mut expected = vec![QueryValue::Timestamp(expected_timestamp)];
		let expected_sql = if let Some(optional_timestamp) = optional_timestamp {
			expected.push(QueryValue::Timestamp(
				chrono::DateTime::parse_from_rfc3339(optional_timestamp)
					.unwrap()
					.with_timezone(&chrono::Utc),
			));
			"INSERT INTO `test_binary_record` (`timestamp`, `optional_timestamp`, `stored_name`) VALUES (?, ?, ?)"
		} else {
			"INSERT INTO `test_binary_record` (`timestamp`, `optional_timestamp`, `stored_name`) VALUES (?, NULL, ?)"
		};
		expected.push(QueryValue::String(timestamp.to_owned()));
		assert_eq!(sql, expected_sql);
		assert_eq!(params, expected);
	}

	#[rstest]
	fn mysql_bulk_create_query_rejects_invalid_datetime() {
		// Arrange
		let fields = vec!["timestamp".to_owned()];
		let rows = vec![vec![serde_json::json!("not-a-timestamp")]];
		// Act
		let error =
			Manager::<TestBinaryRecord>::mysql_bulk_create_query(&fields, &rows).unwrap_err();
		// Assert
		assert!(
			matches!(error, reinhardt_core::exception::Error::Database(message) if message.starts_with("Invalid datetime value for field 'timestamp': "))
		);
	}

	#[rstest]
	#[case(serde_json::json!("hello"), Some("\"hello\""))]
	#[case(serde_json::json!("null"), Some("\"null\""))]
	#[case(serde_json::json!("true"), Some("\"true\""))]
	#[case(serde_json::json!("42"), Some("\"42\""))]
	#[case(serde_json::json!("550e8400-e29b-41d4-a716-446655440000"), Some("\"550e8400-e29b-41d4-a716-446655440000\""))]
	#[case(serde_json::json!("2026-10-04T00:00:00Z"), Some("\"2026-10-04T00:00:00Z\""))]
	#[case(serde_json::json!(true), Some("true"))]
	#[case(serde_json::json!(42), Some("42"))]
	#[case(serde_json::json!(1.25), Some("1.25"))]
	#[case(serde_json::json!([1, "two"]), Some("[1,\"two\"]"))]
	#[case(serde_json::json!({"answer": 42}), Some("{\"answer\":42}"))]
	#[case(serde_json::Value::Null, None)]
	fn mysql_bulk_create_query_preserves_json_types(
		#[case] json_data: serde_json::Value,
		#[case] expected: Option<&str>,
	) {
		use crate::orm::connection::QueryValue;
		// Arrange
		let fields = vec!["json_data".to_owned(), "optional_json_data".to_owned()];
		let rows = vec![vec![json_data, serde_json::Value::Null]];
		// Act
		let statement =
			Manager::<TestBinaryRecord>::mysql_bulk_create_query(&fields, &rows).unwrap();
		let (sql, values) = super::build_insert_sql(&statement, DatabaseBackend::MySql);
		let params: Vec<_> = values
			.0
			.into_iter()
			.map(Manager::<TestBinaryRecord>::sea_value_to_query_value)
			.collect();
		// Assert
		if let Some(expected) = expected {
			assert_eq!(
				sql,
				"INSERT INTO `test_binary_record` (`json_data`, `optional_json_data`) VALUES (?, NULL)"
			);
			assert_eq!(params, vec![QueryValue::String(expected.to_owned())]);
		} else {
			assert_eq!(
				sql,
				"INSERT INTO `test_binary_record` (`json_data`, `optional_json_data`) VALUES (NULL, NULL)"
			);
			assert!(params.is_empty());
		}
	}

	#[rstest]
	fn mysql_bulk_create_query_preserves_text_and_numeric_values() {
		use crate::orm::connection::QueryValue;
		// Arrange
		let fields: Vec<String> = [
			"uuid_text",
			"timestamp_text",
			"array",
			"object",
			"unsigned",
			"signed",
			"float",
			"nullable",
			"boolean",
		]
		.into_iter()
		.map(str::to_owned)
		.collect();
		let rows = vec![vec![
			serde_json::json!("550e8400-e29b-41d4-a716-446655440000"),
			serde_json::json!("2026-10-04T00:00:00Z"),
			serde_json::json!([1, "O'Reilly"]),
			serde_json::json!({"nested": true}),
			serde_json::json!(u64::MAX),
			serde_json::json!(i64::MIN),
			serde_json::json!(1.25),
			serde_json::Value::Null,
			serde_json::json!(true),
		]];
		// Act
		let statement = Manager::<TestUser>::mysql_bulk_create_query(&fields, &rows).unwrap();
		let (sql, values) = super::build_insert_sql(&statement, DatabaseBackend::MySql);
		let params: Vec<_> = values
			.0
			.into_iter()
			.map(Manager::<TestUser>::sea_value_to_query_value)
			.collect();
		// Assert
		assert_eq!(
			sql,
			"INSERT INTO `test_user` (`uuid_text`, `timestamp_text`, `array`, `object`, `unsigned`, `signed`, `float`, `nullable`, `boolean`) VALUES (?, ?, ?, ?, ?, ?, ?, NULL, ?)"
		);
		assert_eq!(
			params,
			vec![
				QueryValue::String("550e8400-e29b-41d4-a716-446655440000".to_owned()),
				QueryValue::String("2026-10-04T00:00:00Z".to_owned()),
				QueryValue::String("[1,\"O'Reilly\"]".to_owned()),
				QueryValue::String("{\"nested\":true}".to_owned()),
				QueryValue::String(u64::MAX.to_string()),
				QueryValue::Int(i64::MIN),
				QueryValue::Float(1.25),
				QueryValue::Bool(true),
			]
		);
	}

	#[cfg(all(
		not(all(target_family = "wasm", target_os = "unknown")),
		any(feature = "mysql", feature = "postgres", feature = "sqlite")
	))]
	mod bulk_create_backend_tests {
		use super::{DatabaseBackend, Manager, TestUser};
		#[cfg(feature = "mysql")]
		use super::{Model, TestBinaryRecord};
		use crate::orm::connection::DatabaseConnection;
		use crate::orm::manager::DB;
		use rstest::{fixture, rstest};
		use serial_test::serial;
		use std::sync::Arc;
		use tokio::sync::RwLock;

		struct DatabaseScope {
			previous: Option<DatabaseConnection>,
		}

		impl DatabaseScope {
			async fn install(connection: DatabaseConnection) -> Self {
				let database = DB.get_or_init(|| Arc::new(RwLock::new(None)));
				let previous = database.write().await.replace(connection);
				Self { previous }
			}
		}

		impl Drop for DatabaseScope {
			fn drop(&mut self) {
				// All global operations are awaited, and these tests share a serial group.
				let mut database = DB.get().unwrap().try_write().unwrap();
				*database = self.previous.take();
			}
		}

		#[cfg(feature = "mysql")]
		#[fixture]
		async fn mysql_database() -> (
			testcontainers::ContainerAsync<testcontainers_modules::mysql::Mysql>,
			DatabaseConnection,
		) {
			use testcontainers::ImageExt;
			use testcontainers::runners::AsyncRunner;
			let container = testcontainers_modules::mysql::Mysql::default()
				.with_tag("8.0")
				.with_startup_timeout(std::time::Duration::from_secs(180))
				.start()
				.await
				.unwrap();
			let url = format!(
				"mysql://root@{}:{}/test",
				container.get_host().await.unwrap(),
				container.get_host_port_ipv4(3306).await.unwrap()
			);
			let connection = DatabaseConnection::connect(&url).await.unwrap();
			(container, connection)
		}

		#[cfg(feature = "postgres")]
		#[fixture]
		async fn postgres_database() -> (
			testcontainers::ContainerAsync<testcontainers_modules::postgres::Postgres>,
			DatabaseConnection,
		) {
			use testcontainers::runners::AsyncRunner;
			let container = testcontainers_modules::postgres::Postgres::default()
				.start()
				.await
				.unwrap();
			let url = format!(
				"postgres://postgres:postgres@{}:{}/postgres",
				container.get_host().await.unwrap(),
				container.get_host_port_ipv4(5432).await.unwrap()
			);
			let connection = DatabaseConnection::connect(&url).await.unwrap();
			(container, connection)
		}

		#[cfg(feature = "sqlite")]
		#[fixture]
		async fn sqlite_database() -> DatabaseConnection {
			DatabaseConnection::connect_with_pool_size("sqlite::memory:", Some(1))
				.await
				.unwrap()
		}

		async fn stored_users(connection: &DatabaseConnection) -> Vec<TestUser> {
			#[cfg(feature = "mysql")]
			if connection.backend() == DatabaseBackend::MySql {
				// Explicit SQLx types keep insert verification independent of MySQL's
				// existing bool-first row inference and hydration limitations.
				let backend = connection.inner().backend();
				let mysql = backend
					.as_any()
					.downcast_ref::<crate::backends::dialect::MySqlBackend>()
					.unwrap();
				return sqlx::query_as::<_, (i64, String, String)>(
					"SELECT id, name, email FROM test_user ORDER BY email",
				)
				.fetch_all(mysql.pool())
				.await
				.unwrap()
				.into_iter()
				.map(|(id, name, email)| TestUser {
					id: Some(id),
					name,
					email,
				})
				.collect();
			}
			connection
				.query(
					"SELECT id, name, email FROM test_user ORDER BY email",
					vec![],
				)
				.await
				.unwrap()
				.into_iter()
				.map(|row| serde_json::from_value(row.data).unwrap())
				.collect()
		}

		async fn assert_bulk_create_backend_cases(connection: DatabaseConnection) {
			// Arrange
			let _scope = DatabaseScope::install(connection.clone()).await;
			let primary_key = match connection.backend() {
				DatabaseBackend::MySql => "BIGINT PRIMARY KEY AUTO_INCREMENT",
				DatabaseBackend::Postgres => "BIGSERIAL PRIMARY KEY",
				DatabaseBackend::Sqlite => "INTEGER PRIMARY KEY AUTOINCREMENT",
			};
			connection
				.execute(
					&format!(
						"CREATE TABLE test_user (id {primary_key}, name TEXT NOT NULL, email VARCHAR(255) NOT NULL UNIQUE)"
					),
					vec![],
				)
				.await
				.unwrap();
			let manager = Manager::<TestUser>::new();

			for generated_keys in [false, true] {
				for ignore_conflicts in [false, true] {
					for batch_size in [None, Some(2)] {
						connection
							.execute("DELETE FROM test_user", vec![])
							.await
							.unwrap();
						let seed = TestUser {
							id: Some(9000),
							name: "existing".to_owned(),
							email: "existing@example.com".to_owned(),
						};
						if ignore_conflicts {
							connection.execute(
								"INSERT INTO test_user (id, name, email) VALUES (9000, 'existing', 'existing@example.com')",
								vec![],
							).await.unwrap();
						}
						let models: Vec<TestUser> =
							["plain", "O'Reilly", "back\\slash", "日本語", "last"]
								.into_iter()
								.enumerate()
								.map(|(index, name)| TestUser {
									id: if generated_keys {
										None
									} else {
										Some(100 + index as i64)
									},
									name: name.to_owned(),
									email: if ignore_conflicts && index == 1 {
										seed.email.clone()
									} else {
										format!("user-{index}@example.com")
									},
								})
								.collect();
						let mut expected = models.clone();
						if ignore_conflicts {
							expected.remove(1);
							expected.push(seed);
						}
						expected.sort_by(|left, right| left.email.cmp(&right.email));

						// Act
						let outcome = manager
							.bulk_create(models.clone(), batch_size, ignore_conflicts, false)
							.await;
						let stored = stored_users(&connection).await;
						let returned = outcome.unwrap_or_else(|error| {
							panic!("bulk_create failed: {error}; stored rows: {}", stored.len())
						});

						// Assert
						assert_eq!(
							stored
								.iter()
								.map(|model| (&model.name, &model.email))
								.collect::<Vec<_>>(),
							expected
								.iter()
								.map(|model| (&model.name, &model.email))
								.collect::<Vec<_>>()
						);
						if !generated_keys {
							assert_eq!(stored, expected);
						}
						for model in &stored {
							assert_ne!(model.id, None);
						}
						if ignore_conflicts {
							assert_eq!(returned, Vec::<TestUser>::new());
						} else if connection.backend() == DatabaseBackend::MySql {
							assert_eq!(returned, models);
						} else {
							let hydrated: Vec<TestUser> = models
								.iter()
								.map(|model| {
									stored
										.iter()
										.find(|row| row.email == model.email)
										.unwrap()
										.clone()
								})
								.collect();
							assert_eq!(returned, hydrated);
						}
					}
				}
			}

			for batch_size in [None, Some(1)] {
				// Arrange
				connection
					.execute("DELETE FROM test_user", vec![])
					.await
					.unwrap();
				connection
					.execute(
						"INSERT INTO test_user (id, name, email) VALUES (1, 'existing', 'existing@example.com')",
						vec![],
					)
					.await
					.unwrap();
				let models = vec![
					TestUser {
						id: Some(2),
						name: "first".to_owned(),
						email: "first@example.com".to_owned(),
					},
					TestUser {
						id: Some(1),
						name: "conflicting".to_owned(),
						email: "conflicting@example.com".to_owned(),
					},
				];
				// Act
				manager
					.bulk_create(models, batch_size, false, false)
					.await
					.unwrap_err();
				let stored = stored_users(&connection).await;
				// Assert
				let mut expected = vec![TestUser {
					id: Some(1),
					name: "existing".to_owned(),
					email: "existing@example.com".to_owned(),
				}];
				if batch_size.is_some() {
					// Completed batches stay committed; bulk_create does not own a transaction.
					expected.push(TestUser {
						id: Some(2),
						name: "first".to_owned(),
						email: "first@example.com".to_owned(),
					});
				}
				assert_eq!(stored, expected);
			}
		}

		#[cfg(feature = "mysql")]
		#[rstest]
		#[tokio::test]
		#[serial(orm_database)]
		async fn bulk_create_mysql_backend_cases(
			#[future] mysql_database: (
				testcontainers::ContainerAsync<testcontainers_modules::mysql::Mysql>,
				DatabaseConnection,
			),
		) {
			let (_container, connection) = mysql_database.await;
			assert_bulk_create_backend_cases(connection).await;
		}

		#[cfg(feature = "mysql")]
		#[rstest]
		#[tokio::test]
		#[serial(orm_database)]
		async fn bulk_create_mysql_typed_fields_and_renamed_columns(
			#[future] mysql_database: (
				testcontainers::ContainerAsync<testcontainers_modules::mysql::Mysql>,
				DatabaseConnection,
			),
		) {
			use reinhardt_query::prelude::{Alias, MySqlQueryBuilder, Order, Query, QueryBuilder};
			use reinhardt_query::types::ColumnDef;
			// Arrange
			let (_container, connection) = mysql_database.await;
			let _scope = DatabaseScope::install(connection.clone()).await;
			let table = Query::create_table()
				.table(TestBinaryRecord::table_name())
				.col(
					ColumnDef::new("record_id")
						.big_integer()
						.primary_key(true)
						.auto_increment(true),
				)
				.col(
					ColumnDef::new("stored_name")
						.string_len(255)
						.not_null(true)
						.unique(true),
				)
				.col(ColumnDef::new("payload").custom("BLOB").not_null(true))
				.col(ColumnDef::new("optional_payload").custom("BLOB"))
				.col(ColumnDef::new("json_data").json().not_null(true))
				.col(ColumnDef::new("optional_json_data").json())
				.col(
					ColumnDef::new("timestamp")
						.custom("DATETIME(6)")
						.not_null(true),
				)
				.col(ColumnDef::new("optional_timestamp").custom("TIMESTAMP(6)"))
				.to_owned();
			let (sql, _) = MySqlQueryBuilder.build_create_table(&table);
			connection.execute(&sql, vec![]).await.unwrap();
			let manager = Manager::<TestBinaryRecord>::new();
			let backend = connection.inner().backend();
			let mysql = backend
				.as_any()
				.downcast_ref::<crate::backends::dialect::MySqlBackend>()
				.unwrap();
			let timestamp = chrono::DateTime::parse_from_rfc3339("2026-10-04T12:34:56.123456Z")
				.unwrap()
				.with_timezone(&chrono::Utc);

			for generated_keys in [false, true] {
				for ignore_conflicts in [false, true] {
					for batch_size in [None, Some(2)] {
						let delete = Query::delete()
							.from_table(Alias::new(TestBinaryRecord::table_name()))
							.to_owned();
						let (sql, _) = MySqlQueryBuilder.build_delete(&delete);
						connection.execute(&sql, vec![]).await.unwrap();
						let models: Vec<TestBinaryRecord> = [
							serde_json::json!("hello"),
							serde_json::json!("null"),
							serde_json::json!("true"),
							serde_json::json!("42"),
							serde_json::json!(true),
							serde_json::json!(42),
							serde_json::json!(1.25),
							serde_json::json!([0, 255]),
							serde_json::json!({"answer": 42}),
						]
						.into_iter()
						.enumerate()
						.map(|(index, json_data)| TestBinaryRecord {
							id: if generated_keys {
								None
							} else {
								Some(100 + index as i64)
							},
							name: format!("record-{index}"),
							payload: match index % 3 {
								0 => vec![0, 255, 128, 39, 92],
								1 => vec![],
								_ => vec![255, 0],
							},
							optional_payload: match index % 3 {
								0 => None,
								1 => Some(vec![]),
								_ => Some(vec![0, 255]),
							},
							json_data,
							optional_json_data: if index % 2 == 0 {
								None
							} else {
								Some(serde_json::json!("null"))
							},
							timestamp: timestamp + chrono::Duration::microseconds(index as i64),
							optional_timestamp: if index % 2 == 0 {
								None
							} else {
								Some(timestamp)
							},
						})
						.collect();
						let mut input = models.clone();
						if ignore_conflicts {
							let mut conflict = models[0].clone();
							conflict.payload = vec![42];
							input.insert(1, conflict);
						}
						// Act
						let returned = manager
							.bulk_create(input, batch_size, ignore_conflicts, false)
							.await
							.unwrap();
						let select = Query::select()
							.columns(
								[
									"record_id",
									"stored_name",
									"payload",
									"optional_payload",
									"json_data",
									"optional_json_data",
									"timestamp",
									"optional_timestamp",
								]
								.into_iter()
								.map(Alias::new),
							)
							.from(Alias::new(TestBinaryRecord::table_name()))
							.order_by(Alias::new("stored_name"), Order::Asc)
							.to_owned();
						let (sql, _) = MySqlQueryBuilder.build_select(&select);
						let stored = sqlx::query_as::<
							_,
							(
								i64,
								String,
								Vec<u8>,
								Option<Vec<u8>>,
								sqlx::types::Json<serde_json::Value>,
								Option<sqlx::types::Json<serde_json::Value>>,
								chrono::NaiveDateTime,
								Option<chrono::DateTime<chrono::Utc>>,
							),
						>(&sql)
						.fetch_all(mysql.pool())
						.await
						.unwrap();
						// Assert
						assert_eq!(
							returned,
							if ignore_conflicts {
								vec![]
							} else {
								models.clone()
							}
						);
						assert_eq!(stored.len(), models.len());
						for (
							(
								id,
								name,
								payload,
								optional_payload,
								json_data,
								optional_json_data,
								timestamp,
								optional_timestamp,
							),
							expected,
						) in stored.into_iter().zip(&models)
						{
							assert_eq!(
								(&name, &payload, &optional_payload, &json_data.0),
								(
									&expected.name,
									&expected.payload,
									&expected.optional_payload,
									&expected.json_data
								)
							);
							assert_eq!(
								optional_json_data.map(|value| value.0),
								expected.optional_json_data
							);
							assert_eq!(timestamp, expected.timestamp.naive_utc());
							assert_eq!(optional_timestamp, expected.optional_timestamp);
							if let Some(expected_id) = expected.id {
								assert_eq!(id, expected_id);
							}
						}
					}
				}
			}
		}

		#[cfg(feature = "postgres")]
		#[rstest]
		#[tokio::test]
		#[serial(orm_database)]
		async fn bulk_create_postgres_backend_cases(
			#[future] postgres_database: (
				testcontainers::ContainerAsync<testcontainers_modules::postgres::Postgres>,
				DatabaseConnection,
			),
		) {
			let (_container, connection) = postgres_database.await;
			assert_bulk_create_backend_cases(connection).await;
		}

		#[cfg(feature = "sqlite")]
		#[rstest]
		#[tokio::test]
		#[serial(orm_database)]
		async fn bulk_create_sqlite_backend_cases(#[future] sqlite_database: DatabaseConnection) {
			assert_bulk_create_backend_cases(sqlite_database.await).await;
		}
	}

	#[fixture]
	fn bulk_users() -> Vec<TestUser> {
		vec![
			TestUser {
				id: Some(1),
				name: "Alice".into(),
				email: "alice@example.com".into(),
			},
			TestUser {
				id: Some(2),
				name: "Bob".into(),
				email: "bob@example.com".into(),
			},
		]
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_bulk_create_rejects_zero_batch_size_before_database_access(
		bulk_users: Vec<TestUser>,
	) {
		// Arrange
		let manager = TestUser::objects();

		// Act
		let error = manager
			.bulk_create(bulk_users, Some(0), false, false)
			.await
			.expect_err("zero batch size must be rejected");

		// Assert
		assert!(
			matches!(error, reinhardt_core::exception::Error::Validation(ref message)
			if message == "batch_size must be greater than zero")
		);
	}

	#[rstest]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_bulk_update_rejects_zero_batch_size_before_database_access(
		bulk_users: Vec<TestUser>,
	) {
		// Arrange
		let manager = TestUser::objects();

		// Act
		let error = manager
			.bulk_update(bulk_users, vec!["name".into()], Some(0))
			.await
			.expect_err("zero batch size must be rejected");

		// Assert
		assert!(
			matches!(error, reinhardt_core::exception::Error::Validation(ref message)
			if message == "batch_size must be greater than zero")
		);
	}

	#[rstest]
	#[case::zero(Some(0))]
	#[case::one(Some(1))]
	#[case::default(None)]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_bulk_create_empty_models_remain_noop(#[case] batch_size: Option<usize>) {
		// Arrange
		let manager = TestUser::objects();

		// Act
		let created = manager
			.bulk_create(vec![], batch_size, false, false)
			.await
			.expect("empty models must succeed without a database");

		// Assert
		assert_eq!(created, Vec::<TestUser>::new());
	}

	#[rstest]
	#[case::zero(Some(0))]
	#[case::one(Some(1))]
	#[case::default(None)]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_bulk_update_empty_models_remain_noop(#[case] batch_size: Option<usize>) {
		// Arrange
		let manager = TestUser::objects();

		// Act
		let updated = manager
			.bulk_update(vec![], vec!["name".into()], batch_size)
			.await
			.expect("empty models must succeed without a database");

		// Assert
		assert_eq!(updated, 0);
	}

	#[rstest]
	#[case::zero(Some(0))]
	#[case::one(Some(1))]
	#[case::default(None)]
	#[serial(sqlx_drivers)]
	#[tokio::test]
	async fn test_bulk_update_empty_fields_remain_noop(
		bulk_users: Vec<TestUser>,
		#[case] batch_size: Option<usize>,
	) {
		// Arrange
		let manager = TestUser::objects();

		// Act
		let updated = manager
			.bulk_update(bulk_users, vec![], batch_size)
			.await
			.expect("empty fields must succeed without a database");

		// Assert
		assert_eq!(updated, 0);
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct TestStringUser {
		id: String,
	}

	#[derive(Debug, Clone)]
	struct TestStringUserFields;

	impl FieldSelector for TestStringUserFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for TestStringUser {
		type PrimaryKey = String;
		type Fields = TestStringUserFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"test_string_user"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			Some(self.id.clone())
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = value;
		}

		fn new_fields() -> Self::Fields {
			TestStringUserFields
		}
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct TestUuidUser {
		id: Uuid,
	}

	#[derive(Debug, Clone)]
	struct TestUuidUserFields;

	impl FieldSelector for TestUuidUserFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for TestUuidUser {
		type PrimaryKey = Uuid;
		type Fields = TestUuidUserFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"test_uuid_user"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			Some(self.id)
		}

		fn primary_key_filter_value(pk: Self::PrimaryKey) -> FilterValue {
			FilterValue::Uuid(pk)
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = value;
		}

		fn primary_key_field() -> &'static str {
			"id"
		}

		fn new_fields() -> Self::Fields {
			TestUuidUserFields
		}
	}

	#[rstest]
	fn test_get_preserves_uuid_primary_key_binding() {
		// Arrange
		let id = Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000")
			.expect("UUID literal should be valid");

		// Act
		let query = TestUuidUser::objects().get(id);

		// Assert
		assert_eq!(query.filters().len(), 1);
		assert!(matches!(&query.filters()[0].value, FilterValue::Uuid(value) if *value == id));
	}

	#[rstest]
	fn test_get_preserves_default_numeric_primary_key_binding() {
		// Arrange and Act
		let query = TestUser::objects().get(42);

		// Assert
		assert_eq!(query.filters().len(), 1);
		assert!(matches!(query.filters()[0].value, FilterValue::Integer(42)));
	}

	#[rstest]
	fn test_delete_preserves_default_numeric_primary_key_binding() {
		// Arrange and Act
		let statement = Manager::<TestUser>::build_delete_statement(42);
		let (_sql, values) = build_delete_sql(&statement, DatabaseBackend::Postgres);

		// Assert
		assert_eq!(
			values.0,
			vec![reinhardt_query::value::Value::BigInt(Some(42))]
		);
	}

	#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
	struct NumericUserId(i64);

	impl fmt::Display for NumericUserId {
		fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
			self.0.fmt(formatter)
		}
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct NumericNewtypeUser {
		id: NumericUserId,
	}

	impl Model for NumericNewtypeUser {
		type PrimaryKey = NumericUserId;
		type Fields = TestUserFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"numeric_newtype_user"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			Some(self.id)
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = value;
		}

		fn new_fields() -> Self::Fields {
			TestUserFields
		}
	}

	#[rstest]
	fn test_manual_numeric_newtype_preserves_numeric_primary_key_binding() {
		// Arrange and Act
		let query = NumericNewtypeUser::objects().get(NumericUserId(42));
		let statement = Manager::<NumericNewtypeUser>::build_delete_statement(NumericUserId(42));
		let (_sql, values) = build_delete_sql(&statement, DatabaseBackend::Postgres);

		// Assert
		assert_eq!(query.filters().len(), 1);
		assert!(matches!(query.filters()[0].value, FilterValue::Integer(42)));
		assert_eq!(
			values.0,
			vec![reinhardt_query::value::Value::BigInt(Some(42))]
		);
	}

	#[rstest]
	#[case("01")]
	#[case("+1")]
	#[case("0001")]
	fn test_get_preserves_exact_custom_primary_key_string(#[case] id: &str) {
		// Arrange and Act
		let query = TestStringUser::objects().get(id.to_owned());

		// Assert
		assert_eq!(query.filters().len(), 1);
		let FilterValue::String(value) = &query.filters()[0].value else {
			panic!("custom primary key should use an exact string binding");
		};
		assert_eq!(value, id);
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct ExternalId(String);

	impl fmt::Display for ExternalId {
		fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
			formatter.write_str(&self.0)
		}
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct TypedKeyUser {
		external_id: ExternalId,
	}

	#[derive(Debug, Clone)]
	struct TypedKeyUserFields;

	impl FieldSelector for TypedKeyUserFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for TypedKeyUser {
		type PrimaryKey = ExternalId;
		type Fields = TypedKeyUserFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"typed_key_user"
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			Some(self.external_id.clone())
		}

		fn primary_key_filter_value(pk: Self::PrimaryKey) -> FilterValue {
			FilterValue::String(format!("external:{}", pk.0))
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.external_id = value;
		}

		fn primary_key_field() -> &'static str {
			"external_id"
		}

		fn new_fields() -> Self::Fields {
			TypedKeyUserFields
		}

		fn field_metadata() -> Vec<FieldInfo> {
			let mut field = CharField::new(64);
			field.base.primary_key = true;
			field.base.db_column = Some("external_key".to_owned());
			field.set_attributes_from_name(Self::primary_key_field());
			vec![FieldInfo::from_field(&field)]
		}
	}

	#[rstest]
	#[case(
		DatabaseBackend::Postgres,
		"DELETE FROM \"typed_key_user\" WHERE \"external_key\" = $1"
	)]
	#[case(
		DatabaseBackend::MySql,
		"DELETE FROM `typed_key_user` WHERE `external_key` = ?"
	)]
	#[case(
		DatabaseBackend::Sqlite,
		"DELETE FROM \"typed_key_user\" WHERE \"external_key\" = ?"
	)]
	fn delete_uses_primary_key_column_and_typed_binding(
		#[case] backend: DatabaseBackend,
		#[case] expected_sql: &str,
	) {
		// Arrange
		let primary_key = ExternalId("42".to_owned());

		// Act
		let statement = Manager::<TypedKeyUser>::build_delete_statement(primary_key);
		let (sql, values) = build_delete_sql(&statement, backend);

		// Assert
		assert_eq!(sql, expected_sql);
		assert_eq!(
			values.0,
			vec![reinhardt_query::value::Value::String(Some(Box::new(
				"external:42".to_owned()
			)))]
		);
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct CompositeUpdateModel {
		tenant_key: String,
		entry_key: String,
		body: String,
	}

	impl Model for CompositeUpdateModel {
		type PrimaryKey = String;
		type Fields = TestUserFields;
		type Objects = Manager<Self>;

		fn table_name() -> &'static str {
			"composite_entries"
		}
		fn primary_key_field() -> &'static str {
			"tenant_key"
		}
		fn primary_key(&self) -> Option<String> {
			Some(self.tenant_key.clone())
		}
		fn set_primary_key(&mut self, value: String) {
			self.tenant_key = value;
		}
		fn new_fields() -> Self::Fields {
			TestUserFields
		}
		fn composite_primary_key() -> Option<crate::orm::composite_pk::CompositePrimaryKey> {
			Some(
				crate::orm::composite_pk::CompositePrimaryKey::new(vec![
					"tenant".into(),
					"entry".into(),
				])
				.unwrap(),
			)
		}
		fn get_composite_pk_values(&self) -> HashMap<String, crate::orm::composite_pk::PkValue> {
			HashMap::from([
				("tenant".into(), self.tenant_key.clone().into()),
				("entry".into(), self.entry_key.clone().into()),
			])
		}
		fn field_metadata() -> Vec<FieldInfo> {
			[("tenant_key", "tenant"), ("entry_key", "entry")]
				.into_iter()
				.map(|(name, column)| {
					let mut field = CharField::new(64);
					field.set_attributes_from_name(name);
					let mut info = FieldInfo::from_field(&field);
					info.primary_key = true;
					info.db_column = Some(column.to_owned());
					info
				})
				.collect()
		}
	}

	#[rstest::fixture]
	fn composite_update_model() -> CompositeUpdateModel {
		CompositeUpdateModel {
			tenant_key: "00123".to_owned(),
			entry_key: "550e8400-e29b-41d4-a716-446655440000".to_owned(),
			body: "changed".to_owned(),
		}
	}

	#[rstest]
	#[case::postgres(
		DatabaseBackend::Postgres,
		"UPDATE \"composite_entries\" SET \"body\" = $1 WHERE (\"tenant\" = $2 AND \"entry\" = $3) RETURNING \"body\", \"entry\", \"tenant\""
	)]
	#[case::mysql(
		DatabaseBackend::MySql,
		"UPDATE `composite_entries` SET `body` = ? WHERE (`tenant` = ? AND `entry` = ?)"
	)]
	#[case::sqlite(
		DatabaseBackend::Sqlite,
		"UPDATE \"composite_entries\" SET \"body\" = ? WHERE (\"tenant\" = ? AND \"entry\" = ?) RETURNING \"body\", \"entry\", \"tenant\""
	)]
	fn update_matches_every_composite_key(
		composite_update_model: CompositeUpdateModel,
		#[case] backend: DatabaseBackend,
		#[case] expected_sql: &str,
	) {
		// Arrange
		let mut json = serde_json::to_value(&composite_update_model).unwrap();
		json.as_object_mut().unwrap().sort_keys();
		// Act
		let stmt =
			Manager::<CompositeUpdateModel>::build_update_statement_from_object_with_returning(
				&composite_update_model,
				json.as_object().unwrap(),
				backend != DatabaseBackend::MySql,
			)
			.unwrap();
		let (sql, values) = super::build_update_sql(&stmt, backend);
		// Assert: text keys retain their storage type and never enter SET.
		assert_eq!(sql, expected_sql);
		assert_eq!(
			values.0,
			vec![
				reinhardt_query::value::Value::from("changed"),
				reinhardt_query::value::Value::from("00123"),
				reinhardt_query::value::Value::from("550e8400-e29b-41d4-a716-446655440000"),
			]
		);
	}

	#[rstest]
	fn update_rejects_incomplete_composite_keys(
		composite_update_model: CompositeUpdateModel,
		#[values("tenant_key", "entry_key")] field: &str,
		#[values(false, true)] missing: bool,
	) {
		// Arrange
		let mut json = serde_json::to_value(&composite_update_model).unwrap();
		let obj = json.as_object_mut().unwrap();
		if missing {
			obj.remove(field);
		} else {
			obj.insert(field.to_owned(), serde_json::Value::Null);
		}
		// Act
		let error =
			Manager::<CompositeUpdateModel>::build_update_statement_from_object_with_returning(
				&composite_update_model,
				obj,
				true,
			)
			.unwrap_err();
		// Assert
		assert_eq!(
			error.to_string(),
			format!("Database error: Model must have non-null primary key field '{field}'")
		);
	}

	#[rstest]
	fn key_only_update_keeps_complete_predicate(composite_update_model: CompositeUpdateModel) {
		// Arrange
		let mut json = serde_json::to_value(&composite_update_model).unwrap();
		let obj = json.as_object_mut().unwrap();
		obj.remove("body");
		obj.sort_keys();
		// Act
		let stmt =
			Manager::<CompositeUpdateModel>::build_update_statement_from_object_with_returning(
				&composite_update_model,
				obj,
				true,
			)
			.unwrap();
		let (sql, values) = super::build_update_sql(&stmt, DatabaseBackend::Postgres);
		// Assert
		assert_eq!(
			sql,
			"UPDATE \"composite_entries\" SET \"tenant\" = \"tenant\" WHERE (\"tenant\" = $1 AND \"entry\" = $2) RETURNING \"entry\", \"tenant\""
		);
		assert_eq!(
			values.0,
			vec![
				reinhardt_query::value::Value::from("00123"),
				reinhardt_query::value::Value::from("550e8400-e29b-41d4-a716-446655440000"),
			]
		);
	}

	#[rstest]
	fn update_preserves_scalar_key_bindings() {
		// Arrange
		let numeric = TestUser {
			id: Some(42),
			name: "Alice".into(),
			email: "alice@example.com".into(),
		};
		let text = TestStringUser { id: "00123".into() };
		let uuid = TestUuidUser {
			id: Uuid::from_u128(1),
		};
		let custom = TypedKeyUser {
			external_id: ExternalId("42".into()),
		};
		macro_rules! bindings {
			($ty:ty, $model:expr) => {{
				let json = serde_json::to_value(&$model).unwrap();
				let stmt = Manager::<$ty>::build_update_statement_from_object_with_returning(
					&$model,
					json.as_object().unwrap(),
					true,
				)
				.unwrap();
				super::build_update_sql(&stmt, DatabaseBackend::Postgres)
					.1
					.0
			}};
		}
		// Act and Assert
		assert_eq!(
			bindings!(TestUser, numeric).last(),
			Some(&reinhardt_query::value::Value::BigInt(Some(42)))
		);
		assert_eq!(
			bindings!(TestStringUser, text),
			vec![reinhardt_query::value::Value::from("00123")]
		);
		assert_eq!(
			bindings!(TestUuidUser, uuid),
			vec![reinhardt_query::value::Value::from(Uuid::from_u128(1))]
		);
		assert_eq!(
			bindings!(TypedKeyUser, custom),
			vec![reinhardt_query::value::Value::from("external:42")]
		);
	}

	#[test]
	fn test_get_or_create_sql() {
		let manager = TestUser::objects();
		let mut lookup = HashMap::new();
		lookup.insert("email".to_string(), "test@example.com".to_string());

		let mut defaults = HashMap::new();
		defaults.insert("name".to_string(), "Test User".to_string());

		let (select_sql, insert_sql) =
			manager.get_or_create_sql(&lookup, &defaults, DatabaseBackend::Postgres);

		// reinhardt-query uses quoted identifiers and TestUser table is "test_user"
		assert!(select_sql.contains("SELECT") && select_sql.contains("FROM"));
		assert!(select_sql.contains("test_user"));
		assert!(select_sql.contains("email"));
		// reinhardt-query produces parameterized SQL with $1 placeholder instead of inline values
		assert!(select_sql.contains("$1"));
		assert!(insert_sql.contains("INSERT"));
		assert!(insert_sql.contains("test_user"));
		assert!(insert_sql.contains("email"));
		assert!(insert_sql.contains("name"));
	}

	#[test]
	fn test_bulk_create_sql() {
		use serde_json::json;
		let manager = TestUser::objects();
		let fields = vec!["name".to_string(), "email".to_string()];
		let values = vec![
			vec![json!("Alice"), json!("alice@example.com")],
			vec![json!("Bob"), json!("bob@example.com")],
		];

		let sql = manager.bulk_create_sql_detailed(&fields, &values, false);

		// reinhardt-query uses quoted identifiers and TestUser table is "test_user"
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
		use serde_json::json;
		let manager = TestUser::objects();
		let fields = vec!["name".to_string(), "email".to_string()];
		let values = vec![vec![json!("Alice"), json!("alice@example.com")]];

		let sql = manager.bulk_create_sql_detailed(&fields, &values, true);

		assert!(sql.contains("ON CONFLICT DO NOTHING"));
	}

	#[test]
	fn test_bulk_update_sql() {
		use serde_json::json;
		let manager = TestUser::objects();

		let mut updates = Vec::new();
		let mut user1_fields = HashMap::new();
		user1_fields.insert("name".to_string(), json!("Alice Updated"));
		user1_fields.insert("email".to_string(), json!("alice_new@example.com"));
		updates.push((1i64, user1_fields));

		let mut user2_fields = HashMap::new();
		user2_fields.insert("name".to_string(), json!("Bob Updated"));
		user2_fields.insert("email".to_string(), json!("bob_new@example.com"));
		updates.push((2i64, user2_fields));

		let fields = vec!["name".to_string(), "email".to_string()];
		let sql = manager.bulk_update_sql_detailed(&updates, &fields, DatabaseBackend::Postgres);

		// reinhardt-query uses quoted identifiers and TestUser table is "test_user"
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
		use serde_json::Value;
		let manager = TestUser::objects();
		let fields: Vec<String> = vec![];
		let values: Vec<Vec<Value>> = vec![];

		let sql = manager.bulk_create_sql_detailed(&fields, &values, false);
		assert!(sql.is_empty());
	}

	#[test]
	fn test_bulk_update_empty() {
		use serde_json::Value;
		let manager = TestUser::objects();
		let updates: Vec<(i64, HashMap<String, Value>)> = vec![];
		let fields = vec!["name".to_string()];

		let sql = manager.bulk_update_sql_detailed(&updates, &fields, DatabaseBackend::Postgres);
		assert!(sql.is_empty());
	}

	// ──────────────────────────────────────────────────────────────
	// Additional manager tests
	// ──────────────────────────────────────────────────────────────

	#[test]
	fn test_manager_new() {
		let manager = super::Manager::<TestUser>::new();
		// Manager is just a phantom type wrapper, so this just ensures it compiles
		let _ = manager;
	}

	#[test]
	fn test_manager_default() {
		let manager = super::Manager::<TestUser>::default();
		// Default should work the same as new
		let _ = manager;
	}

	#[test]
	fn test_get_or_create_sql_empty_lookup() {
		let manager = TestUser::objects();
		let lookup: HashMap<String, String> = HashMap::new();
		let defaults: HashMap<String, String> = HashMap::new();

		let (select_sql, insert_sql) =
			manager.get_or_create_sql(&lookup, &defaults, DatabaseBackend::Postgres);

		// Empty lookup still produces valid SQL structure
		assert!(select_sql.contains("SELECT") || select_sql.contains("select"));
		assert!(insert_sql.contains("INSERT") || insert_sql.contains("insert"));
	}

	#[test]
	fn test_get_or_create_sql_with_multiple_lookups() {
		let manager = TestUser::objects();
		let mut lookup = HashMap::new();
		lookup.insert("email".to_string(), "test@example.com".to_string());
		lookup.insert("name".to_string(), "Test User".to_string());

		let defaults: HashMap<String, String> = HashMap::new();

		let (select_sql, _insert_sql) =
			manager.get_or_create_sql(&lookup, &defaults, DatabaseBackend::Postgres);

		// Should have both conditions in WHERE clause
		assert!(select_sql.contains("email"));
		assert!(select_sql.contains("name"));
	}

	#[test]
	fn test_bulk_create_sql_single_row() {
		use serde_json::json;
		let manager = TestUser::objects();
		let fields = vec!["name".to_string()];
		let values = vec![vec![json!("SingleUser")]];

		let sql = manager.bulk_create_sql_detailed(&fields, &values, false);

		assert!(sql.contains("INSERT"));
		assert!(sql.contains("test_user"));
		assert!(sql.contains("SingleUser"));
	}

	#[test]
	fn test_bulk_update_sql_single_field() {
		use serde_json::json;
		let manager = TestUser::objects();

		let mut updates = Vec::new();
		let mut user1_fields = HashMap::new();
		user1_fields.insert("name".to_string(), json!("Updated Name"));
		updates.push((1i64, user1_fields));

		let fields = vec!["name".to_string()];
		let sql = manager.bulk_update_sql_detailed(&updates, &fields, DatabaseBackend::Postgres);

		assert!(sql.contains("UPDATE"));
		assert!(sql.contains("name"));
		assert!(sql.contains("Updated Name"));
		assert!(!sql.contains("email"));
	}

	#[test]
	fn test_json_to_sea_value_string() {
		use serde_json::json;
		let value = json!("hello");
		let sea_value = super::Manager::<TestUser>::json_to_sea_value(&value);

		// reinhardt-query Value should contain the string
		let debug_str = format!("{:?}", sea_value);
		assert!(debug_str.contains("hello") || debug_str.contains("String"));
	}

	#[test]
	fn test_json_to_sea_value_integer() {
		use serde_json::json;
		let value = json!(42);
		let sea_value = super::Manager::<TestUser>::json_to_sea_value(&value);

		let debug_str = format!("{:?}", sea_value);
		assert!(debug_str.contains("42") || debug_str.contains("Int"));
	}

	#[test]
	fn test_json_to_sea_value_float() {
		use serde_json::json;
		let value = json!(1.5);
		let sea_value = super::Manager::<TestUser>::json_to_sea_value(&value);

		let debug_str = format!("{:?}", sea_value);
		assert!(debug_str.contains("1.5") || debug_str.contains("Double"));
	}

	#[test]
	fn test_json_to_sea_value_bool() {
		use serde_json::json;
		let value = json!(true);
		let sea_value = super::Manager::<TestUser>::json_to_sea_value(&value);

		let debug_str = format!("{:?}", sea_value);
		assert!(debug_str.contains("true") || debug_str.contains("Bool"));
	}

	#[test]
	fn test_json_to_sea_value_null() {
		use serde_json::json;
		let value = json!(null);
		let sea_value = super::Manager::<TestUser>::json_to_sea_value(&value);

		// Null should be represented somehow
		let debug_str = format!("{:?}", sea_value);
		assert!(!debug_str.is_empty());
	}

	#[test]
	fn test_json_to_sea_value_array() {
		use serde_json::json;
		let value = json!([1, 2, 3]);
		let sea_value = super::Manager::<TestUser>::json_to_sea_value(&value);

		// Array should be converted (typically to JSON string)
		let debug_str = format!("{:?}", sea_value);
		assert!(!debug_str.is_empty());
	}

	#[test]
	fn test_json_to_sea_value_object() {
		use serde_json::json;
		let value = json!({"key": "value"});
		let sea_value = super::Manager::<TestUser>::json_to_sea_value(&value);

		// Object should be converted (typically to JSON string)
		let debug_str = format!("{:?}", sea_value);
		assert!(!debug_str.is_empty());
	}

	#[test]
	fn test_serialize_value_string() {
		use serde_json::json;
		let value = json!("test_string");
		let serialized = super::Manager::<TestUser>::serialize_value(&value);

		// Should return the string representation
		assert!(serialized.contains("test_string"));
	}

	#[test]
	fn test_serialize_value_number() {
		use serde_json::json;
		let value = json!(123);
		let serialized = super::Manager::<TestUser>::serialize_value(&value);

		assert!(serialized.contains("123"));
	}
}
