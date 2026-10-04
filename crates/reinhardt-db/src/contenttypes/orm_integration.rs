//! ORM Integration Module
//!
//! This module provides integration between ContentTypes and reinhardt-orm.
//! It provides an ORM-style query builder and pool-backed ContentType operations
//! for SQLite. These interfaces generate SQLite SQL regardless of the `AnyPool`
//! driver; PostgreSQL and MySQL pools are not supported.
//! These interfaces execute through the pool and do not own a database transaction.

#[cfg(feature = "database")]
use reinhardt_query::prelude::{
	Alias, BinOper, Cond, Expr, Func, Order, Query, QueryStatementBuilder, SqliteQueryBuilder,
};
#[cfg(feature = "database")]
use reinhardt_query::value::Values;
#[cfg(feature = "database")]
use sqlx::{AnyPool, Row};
#[cfg(feature = "database")]
use std::sync::Arc;

#[cfg(feature = "database")]
use super::ContentType;
#[cfg(feature = "database")]
use super::persistence::{PersistenceError, bind_query_values};

/// ORM-compatible ContentType query builder
///
/// Provides an API similar to reinhardt-orm's Query interface,
/// building type-safe queries for ContentType.
///
/// Only SQLite-backed pools are supported. Queries use `SqliteQueryBuilder`
/// regardless of the `AnyPool` driver. The PostgreSQL and MySQL support in
/// `ContentTypePersistence` does not extend to this query builder.
///
/// ## Example
///
/// ```rust,no_run
/// use reinhardt_db::contenttypes::orm_integration::ContentTypeQuery;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let pool = sqlx::AnyPool::connect("sqlite::memory:").await?;
///
/// let query = ContentTypeQuery::new(pool.into());
/// let results = query
///     .filter_app_label("auth")
///     .order_by_model()
///     .all()
///     .await?;
///
/// for ct in results {
///     println!("{}.{}", ct.app_label, ct.model);
/// }
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "database")]
#[derive(Clone)]
pub struct ContentTypeQuery {
	pool: Arc<AnyPool>,
	filters: Vec<ContentTypeFilter>,
	order_by: Vec<OrderBy>,
	limit: Option<u64>,
	offset: Option<u64>,
}

#[cfg(feature = "database")]
#[derive(Clone)]
enum ContentTypeFilter {
	AppLabel(String),
	Model(String),
	Id(i64),
}

#[cfg(feature = "database")]
#[derive(Clone)]
enum OrderBy {
	AppLabel(OrderDirection),
	Model(OrderDirection),
	Id(OrderDirection),
}

#[cfg(feature = "database")]
#[derive(Clone)]
enum OrderDirection {
	Asc,
	Desc,
}

#[cfg(feature = "database")]
impl ContentTypeQuery {
	/// Create a new query builder for a SQLite-backed pool.
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::contenttypes::orm_integration::ContentTypeQuery;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = sqlx::AnyPool::connect("sqlite::memory:").await?;
	/// let query = ContentTypeQuery::new(Arc::new(pool));
	/// # Ok(())
	/// # }
	/// ```
	pub fn new(pool: Arc<AnyPool>) -> Self {
		Self {
			pool,
			filters: Vec::new(),
			order_by: Vec::new(),
			limit: None,
			offset: None,
		}
	}

	/// Filter by app_label
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::contenttypes::orm_integration::ContentTypeQuery;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = sqlx::AnyPool::connect("sqlite::memory:").await?;
	/// let query = ContentTypeQuery::new(Arc::new(pool))
	///     .filter_app_label("auth");
	/// # Ok(())
	/// # }
	/// ```
	pub fn filter_app_label(mut self, app_label: impl Into<String>) -> Self {
		self.filters
			.push(ContentTypeFilter::AppLabel(app_label.into()));
		self
	}

	/// Filter by model
	pub fn filter_model(mut self, model: impl Into<String>) -> Self {
		self.filters.push(ContentTypeFilter::Model(model.into()));
		self
	}

	/// Filter by ID
	pub fn filter_id(mut self, id: i64) -> Self {
		self.filters.push(ContentTypeFilter::Id(id));
		self
	}

	/// Sort by app_label in ascending order
	pub fn order_by_app_label(mut self) -> Self {
		self.order_by.push(OrderBy::AppLabel(OrderDirection::Asc));
		self
	}

	/// Sort by model in ascending order
	pub fn order_by_model(mut self) -> Self {
		self.order_by.push(OrderBy::Model(OrderDirection::Asc));
		self
	}

	/// Sort by ID in ascending order
	pub fn order_by_id(mut self) -> Self {
		self.order_by.push(OrderBy::Id(OrderDirection::Asc));
		self
	}

	/// Sort by app_label in descending order
	pub fn order_by_app_label_desc(mut self) -> Self {
		self.order_by.push(OrderBy::AppLabel(OrderDirection::Desc));
		self
	}

	/// Sort by model in descending order
	pub fn order_by_model_desc(mut self) -> Self {
		self.order_by.push(OrderBy::Model(OrderDirection::Desc));
		self
	}

	/// Sort by ID in descending order
	pub fn order_by_id_desc(mut self) -> Self {
		self.order_by.push(OrderBy::Id(OrderDirection::Desc));
		self
	}

	/// Limit the number of results
	pub fn limit(mut self, limit: u64) -> Self {
		self.limit = Some(limit);
		self
	}

	/// Set result offset
	pub fn offset(mut self, offset: u64) -> Self {
		self.offset = Some(offset);
		self
	}

	/// Build the query, returning parameterized SQL and bound values
	fn build_query(&self) -> (String, Values) {
		let mut query = Query::select()
			.columns([
				Alias::new("id"),
				Alias::new("app_label"),
				Alias::new("model"),
			])
			.from(Alias::new("django_content_type"))
			.to_owned();

		// Apply filters
		for filter in &self.filters {
			let condition = match filter {
				ContentTypeFilter::AppLabel(app_label) => Cond::all().add(
					Expr::col(Alias::new("app_label"))
						.binary(BinOper::Equal, Expr::val(app_label.clone())),
				),
				ContentTypeFilter::Model(model) => Cond::all().add(
					Expr::col(Alias::new("model")).binary(BinOper::Equal, Expr::val(model.clone())),
				),
				ContentTypeFilter::Id(id) => Cond::all()
					.add(Expr::col(Alias::new("id")).binary(BinOper::Equal, Expr::val(*id))),
			};
			query.cond_where(condition);
		}

		// Apply sort order
		for order in &self.order_by {
			match order {
				OrderBy::AppLabel(direction) => {
					query.order_by(
						Alias::new("app_label"),
						match direction {
							OrderDirection::Asc => Order::Asc,
							OrderDirection::Desc => Order::Desc,
						},
					);
				}
				OrderBy::Model(direction) => {
					query.order_by(
						Alias::new("model"),
						match direction {
							OrderDirection::Asc => Order::Asc,
							OrderDirection::Desc => Order::Desc,
						},
					);
				}
				OrderBy::Id(direction) => {
					query.order_by(
						Alias::new("id"),
						match direction {
							OrderDirection::Asc => Order::Asc,
							OrderDirection::Desc => Order::Desc,
						},
					);
				}
			}
		}

		// Apply limit/offset
		if let Some(limit) = self.limit {
			query.limit(limit);
		}
		if let Some(offset) = self.offset {
			query.offset(offset);
		}

		query.build(SqliteQueryBuilder)
	}

	/// Retrieve all results
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::contenttypes::orm_integration::ContentTypeQuery;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = sqlx::AnyPool::connect("sqlite::memory:").await?;
	/// let results = ContentTypeQuery::new(Arc::new(pool))
	///     .filter_app_label("auth")
	///     .all()
	///     .await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn all(&self) -> Result<Vec<ContentType>, PersistenceError> {
		let (sql, values) = self.build_query();
		let rows = bind_query_values(sqlx::query(&sql), &values)
			.fetch_all(&*self.pool)
			.await
			.map_err(|e| {
				PersistenceError::DatabaseError(format!("Failed to execute query: {}", e))
			})?;

		let mut results = Vec::new();
		for row in rows {
			let id: i64 = row
				.try_get("id")
				.map_err(|e| PersistenceError::DatabaseError(format!("Invalid id: {}", e)))?;
			let app_label: String = row.try_get("app_label").map_err(|e| {
				PersistenceError::DatabaseError(format!("Invalid app_label: {}", e))
			})?;
			let model: String = row
				.try_get("model")
				.map_err(|e| PersistenceError::DatabaseError(format!("Invalid model: {}", e)))?;

			results.push(ContentType {
				id: Some(id),
				app_label,
				model,
			});
		}

		Ok(results)
	}

	/// Retrieve the first result
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::contenttypes::orm_integration::ContentTypeQuery;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = sqlx::AnyPool::connect("sqlite::memory:").await?;
	/// let result = ContentTypeQuery::new(Arc::new(pool))
	///     .filter_app_label("auth")
	///     .filter_model("User")
	///     .first()
	///     .await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn first(&self) -> Result<Option<ContentType>, PersistenceError> {
		let mut query = self.clone();
		query.limit = Some(1);

		let results = query.all().await?;
		Ok(results.into_iter().next())
	}

	/// Get the count of results
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::contenttypes::orm_integration::ContentTypeQuery;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = sqlx::AnyPool::connect("sqlite::memory:").await?;
	/// let count = ContentTypeQuery::new(Arc::new(pool))
	///     .filter_app_label("auth")
	///     .count()
	///     .await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn count(&self) -> Result<i64, PersistenceError> {
		let mut count_query = Query::select()
			.expr(Func::count(Expr::col(Alias::new("id")).into()))
			.from(Alias::new("django_content_type"))
			.to_owned();

		// Apply filters only (order and limit are unnecessary)
		for filter in &self.filters {
			let condition = match filter {
				ContentTypeFilter::AppLabel(app_label) => Cond::all().add(
					Expr::col(Alias::new("app_label"))
						.binary(BinOper::Equal, Expr::val(app_label.clone())),
				),
				ContentTypeFilter::Model(model) => Cond::all().add(
					Expr::col(Alias::new("model")).binary(BinOper::Equal, Expr::val(model.clone())),
				),
				ContentTypeFilter::Id(id) => Cond::all()
					.add(Expr::col(Alias::new("id")).binary(BinOper::Equal, Expr::val(*id))),
			};
			count_query.cond_where(condition);
		}

		let (sql, values) = count_query.build(SqliteQueryBuilder);
		let row = bind_query_values(sqlx::query(&sql), &values)
			.fetch_one(&*self.pool)
			.await
			.map_err(|e| PersistenceError::DatabaseError(format!("Failed to count: {}", e)))?;

		let count: i64 = row
			.try_get(0)
			.map_err(|e| PersistenceError::DatabaseError(format!("Invalid count: {}", e)))?;

		Ok(count)
	}

	/// Check if results exist
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::contenttypes::orm_integration::ContentTypeQuery;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = sqlx::AnyPool::connect("sqlite::memory:").await?;
	/// let exists = ContentTypeQuery::new(Arc::new(pool))
	///     .filter_app_label("auth")
	///     .exists()
	///     .await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn exists(&self) -> Result<bool, PersistenceError> {
		let count = self.count().await?;
		Ok(count > 0)
	}
}

/// Pool-backed ContentType operations without transaction ownership.
///
/// Only SQLite-backed pools are supported. This context and its queries generate
/// SQL with `SqliteQueryBuilder`, and [`Self::create`] retrieves the generated ID
/// with SQLite's `last_insert_rowid()` on the same acquired connection as the
/// insert. PostgreSQL and MySQL pools are unsupported,
/// even though `ContentTypePersistence` supports those backends.
///
/// The historical name is retained for compatibility. This context stores a pool;
/// it does not begin or own a database transaction and has no commit or rollback
/// boundary. Each operation executes independently through the pool, using its
/// normal autocommit behavior. An error or dropping the context does not roll back
/// preceding writes.
///
/// Queries returned by [`Self::query`] also execute through the pool. Opening a
/// transaction separately on a pool connection does not enlist these operations
/// in that transaction. Callers requiring atomic changes must use an API that
/// executes on an owned database transaction.
#[cfg(feature = "database")]
pub struct ContentTypeTransaction {
	pool: Arc<AnyPool>,
}

#[cfg(feature = "database")]
impl ContentTypeTransaction {
	/// Create a SQLite pool-backed context without acquiring a connection or beginning a transaction.
	pub fn new(pool: Arc<AnyPool>) -> Self {
		Self { pool }
	}

	/// Get an independent SQLite query builder using the same pool.
	///
	/// The builder does not share a transaction or snapshot with this context and
	/// can be used after the context is dropped.
	pub fn query(&self) -> ContentTypeQuery {
		ContentTypeQuery::new(self.pool.clone())
	}

	/// Create a ContentType through the SQLite pool using autocommit.
	///
	/// The insert and generated-ID lookup use one acquired connection, including
	/// when the pool allows multiple connections or callers create concurrently.
	///
	/// Successful writes are not rolled back when the context is dropped or a
	/// subsequent operation fails. This method does not provide atomicity with
	/// other context operations.
	pub async fn create(
		&self,
		app_label: impl Into<String>,
		model: impl Into<String>,
	) -> Result<ContentType, PersistenceError> {
		let app_label = app_label.into();
		let model = model.into();

		let stmt = Query::insert()
			.into_table(Alias::new("django_content_type"))
			.columns([Alias::new("app_label"), Alias::new("model")])
			.values(vec![app_label.clone().into(), model.clone().into()])
			.expect("Failed to build insert statement")
			.to_owned();
		let (sql, values) = stmt.build(SqliteQueryBuilder);
		// Keep the insert and its connection-local ID lookup on the same connection.
		let mut connection = self.pool.acquire().await.map_err(|e| {
			PersistenceError::DatabaseError(format!("Failed to create content type: {}", e))
		})?;
		bind_query_values(sqlx::query(&sql), &values)
			.execute(&mut *connection)
			.await
			.map_err(|e| {
				PersistenceError::DatabaseError(format!("Failed to create content type: {}", e))
			})?;

		// Get the last inserted ID using SQLite's last_insert_rowid()
		let id_row = sqlx::query("SELECT last_insert_rowid() as id")
			.fetch_one(&mut *connection)
			.await
			.map_err(|e| {
				PersistenceError::DatabaseError(format!("Failed to get last insert ID: {}", e))
			})?;

		let id: i64 = id_row
			.try_get("id")
			.map_err(|e| PersistenceError::DatabaseError(format!("Failed to extract ID: {}", e)))?;

		Ok(ContentType {
			id: Some(id),
			app_label,
			model,
		})
	}

	/// Delete a ContentType through the SQLite pool using autocommit.
	///
	/// Successful deletes are not rolled back when the context is dropped or a
	/// subsequent operation fails.
	pub async fn delete(&self, id: i64) -> Result<(), PersistenceError> {
		let stmt = Query::delete()
			.from_table(Alias::new("django_content_type"))
			.cond_where(
				Cond::all().add(Expr::col(Alias::new("id")).binary(BinOper::Equal, Expr::val(id))),
			)
			.to_owned();
		let (sql, values) = stmt.build(SqliteQueryBuilder);
		bind_query_values(sqlx::query(&sql), &values)
			.execute(&*self.pool)
			.await
			.map_err(|e| {
				PersistenceError::DatabaseError(format!("Failed to delete content type: {}", e))
			})?;

		Ok(())
	}
}

#[cfg(all(test, feature = "database"))]
mod tests {
	use super::*;
	use crate::contenttypes::persistence::{ContentTypePersistence, ContentTypePersistenceBackend};
	use rstest::{fixture, rstest};
	use std::sync::Once;

	static INIT_DRIVERS: Once = Once::new();

	fn init_drivers() {
		INIT_DRIVERS.call_once(|| {
			sqlx::any::install_default_drivers();
		});
	}

	#[fixture]
	async fn setup_test_db() -> Arc<AnyPool> {
		init_drivers();

		// Use in-memory SQLite with shared cache mode and single connection
		let db_url = "sqlite::memory:?mode=rwc&cache=shared";

		// Create pool with single connection
		use sqlx::pool::PoolOptions;
		let pool = PoolOptions::new()
			.min_connections(1)
			.max_connections(1)
			.connect(db_url)
			.await
			.expect("Failed to connect");

		// Create table
		let persistence = ContentTypePersistence::from_pool(pool.clone().into(), db_url);
		persistence
			.create_table()
			.await
			.expect("Failed to create table");

		pool.into()
	}

	struct MultiConnectionTestDb {
		persistence: ContentTypePersistence,
		pool: Arc<AnyPool>,
		_directory: tempfile::TempDir,
	}

	#[fixture]
	async fn setup_multi_connection_db() -> MultiConnectionTestDb {
		init_drivers();
		let directory = tempfile::tempdir().expect("Failed to create SQLite directory");
		let path = directory.path().join("contenttypes.sqlite");
		let url = format!("sqlite://{}?mode=rwc", path.display());
		let pool = Arc::new(
			sqlx::any::AnyPoolOptions::new()
				.min_connections(2)
				.max_connections(2)
				.acquire_timeout(std::time::Duration::from_secs(5))
				.connect(&url)
				.await
				.expect("Failed to open two-connection SQLite pool"),
		);
		let persistence = ContentTypePersistence::from_pool(pool.clone(), &url);
		persistence
			.create_table()
			.await
			.expect("Failed to create content type table");
		MultiConnectionTestDb {
			persistence,
			pool,
			_directory: directory,
		}
	}

	#[rstest]
	#[tokio::test]
	async fn test_content_type_context_create_ids_with_two_connections(
		#[future] setup_multi_connection_db: MultiConnectionTestDb,
	) {
		// Arrange
		let database = setup_multi_connection_db.await;
		let context = ContentTypeTransaction::new(database.pool.clone());
		let mut created = Vec::new();

		// Act
		for index in 0..8 {
			created.push(
				context
					.create("affinity", format!("Sequential{index}"))
					.await
					.expect("Failed to create content type"),
			);
		}
		let stored = database
			.persistence
			.load_all()
			.await
			.expect("Failed to load content types");

		// Assert
		assert_eq!(created, stored);
		assert_eq!(
			created
				.iter()
				.map(|content_type| content_type.id)
				.collect::<Vec<_>>(),
			(1..=8).map(Some).collect::<Vec<_>>(),
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_content_type_context_concurrent_create_ids_with_two_connections(
		#[future] setup_multi_connection_db: MultiConnectionTestDb,
	) {
		// Arrange
		let database = setup_multi_connection_db.await;
		let context = ContentTypeTransaction::new(database.pool.clone());

		// Act
		let (first, second) = tokio::try_join!(
			context.create("affinity", "First"),
			context.create("affinity", "Second"),
		)
		.expect("Failed to create content types concurrently");
		let mut stored = database
			.persistence
			.load_all()
			.await
			.expect("Failed to load content types");
		stored.sort_by(|left, right| left.model.cmp(&right.model));

		// Assert
		let mut ids = [first.id, second.id];
		ids.sort();
		assert_eq!(ids, [Some(1), Some(2)]);
		assert_eq!(stored, vec![first, second]);
	}

	#[rstest]
	#[tokio::test]
	async fn test_content_type_context_duplicate_returns_both_connections(
		#[future] setup_multi_connection_db: MultiConnectionTestDb,
	) {
		// Arrange
		let database = setup_multi_connection_db.await;
		let context = ContentTypeTransaction::new(database.pool.clone());
		let created = context
			.create("affinity", "Unique")
			.await
			.expect("Failed to create content type");

		// Act
		let error = context
			.create("affinity", "Unique")
			.await
			.expect_err("Duplicate content type should fail");
		let stored = database
			.persistence
			.load_all()
			.await
			.expect("Failed to load content types");
		let _connections = tokio::time::timeout(std::time::Duration::from_secs(5), async {
			let first = database.pool.acquire().await?;
			let second = database.pool.acquire().await?;
			Ok::<_, sqlx::Error>((first, second))
		})
		.await
		.expect("Both connections should return to the pool after an error")
		.expect("Failed to acquire returned connections");

		// Assert
		assert!(matches!(error, PersistenceError::DatabaseError(_)));
		assert_eq!(stored, vec![created]);
		assert_eq!(database.pool.size(), 2);
	}

	#[rstest]
	#[tokio::test]
	async fn test_content_type_context_create_persists_after_drop(
		#[future] setup_test_db: Arc<AnyPool>,
	) {
		// Arrange
		let pool = setup_test_db.await;
		let persistence = ContentTypePersistence::from_pool(pool.clone(), "sqlite::memory:");

		// Act
		let created = {
			let context = ContentTypeTransaction::new(pool);
			context
				.create("affinity", "CommittedWithoutTransaction")
				.await
				.expect("Failed to create content type")
		};
		let stored = persistence
			.get("affinity", "CommittedWithoutTransaction")
			.await
			.expect("Failed to retrieve content type");

		// Assert
		assert_eq!(stored, Some(created));
	}

	#[rstest]
	#[tokio::test]
	async fn test_content_type_context_delete_persists_after_drop(
		#[future] setup_test_db: Arc<AnyPool>,
	) {
		// Arrange
		let pool = setup_test_db.await;
		let persistence = ContentTypePersistence::from_pool(pool.clone(), "sqlite::memory:");
		let created = persistence
			.get_or_create("affinity", "DeletedWithoutTransaction")
			.await
			.expect("Failed to create content type");

		// Act
		{
			let context = ContentTypeTransaction::new(pool);
			context
				.delete(created.id.expect("Missing content type ID"))
				.await
				.expect("Failed to delete content type");
		}
		let stored = persistence
			.get("affinity", "DeletedWithoutTransaction")
			.await
			.expect("Failed to retrieve content type");

		// Assert
		assert_eq!(stored, None);
	}

	#[rstest]
	#[tokio::test]
	async fn test_content_type_context_error_does_not_roll_back_writes(
		#[future] setup_test_db: Arc<AnyPool>,
	) {
		// Arrange
		let pool = setup_test_db.await;
		let persistence = ContentTypePersistence::from_pool(pool.clone(), "sqlite::memory:");

		// Act
		let (created, error) = {
			let context = ContentTypeTransaction::new(pool);
			let created = context
				.create("affinity", "KeptAfterError")
				.await
				.expect("Failed to create content type");
			let deleted = context
				.create("affinity", "DeletedBeforeError")
				.await
				.expect("Failed to create content type");
			context
				.delete(deleted.id.expect("Missing content type ID"))
				.await
				.expect("Failed to delete content type");
			let error = context
				.create("affinity", "KeptAfterError")
				.await
				.expect_err("Duplicate content type should fail");
			(created, error)
		};
		let remaining = persistence
			.load_all()
			.await
			.expect("Failed to load content types");

		// Assert
		assert!(matches!(error, PersistenceError::DatabaseError(_)));
		assert_eq!(remaining, vec![created]);
	}

	#[rstest]
	#[tokio::test]
	async fn test_content_type_context_query_survives_context_drop(
		#[future] setup_test_db: Arc<AnyPool>,
	) {
		// Arrange
		let pool = setup_test_db.await;

		// Act
		let (created, query) = {
			let context = ContentTypeTransaction::new(pool);
			let created = context
				.create("affinity", "IndependentQuery")
				.await
				.expect("Failed to create content type");
			(created, context.query())
		};
		let results = query.all().await.expect("Failed to query content types");

		// Assert
		assert_eq!(results, vec![created]);
	}

	#[tokio::test]
	async fn test_content_type_query_all() {
		let pool = setup_test_db().await;

		// Create test data
		let context = ContentTypeTransaction::new(pool.clone());
		context
			.create("auth", "User")
			.await
			.expect("Failed to create");
		context
			.create("auth", "Group")
			.await
			.expect("Failed to create");

		// Execute query
		let query = ContentTypeQuery::new(pool);
		let results = query.all().await.expect("Failed to execute query");

		assert_eq!(results.len(), 2);
	}

	#[tokio::test]
	async fn test_content_type_query_filter() {
		let pool = setup_test_db().await;

		// Create test data
		let context = ContentTypeTransaction::new(pool.clone());
		context
			.create("auth", "User")
			.await
			.expect("Failed to create");
		context
			.create("blog", "Post")
			.await
			.expect("Failed to create");

		// Filter query
		let query = ContentTypeQuery::new(pool);
		let results = query
			.filter_app_label("auth")
			.all()
			.await
			.expect("Failed to execute query");

		assert_eq!(results.len(), 1);
		assert_eq!(results[0].app_label, "auth");
	}

	#[tokio::test]
	async fn test_content_type_query_order_by() {
		let pool = setup_test_db().await;

		// Create test data
		let context = ContentTypeTransaction::new(pool.clone());
		context
			.create("blog", "Post")
			.await
			.expect("Failed to create");
		context
			.create("auth", "User")
			.await
			.expect("Failed to create");

		// Query with sorting
		let query = ContentTypeQuery::new(pool);
		let results = query
			.order_by_app_label()
			.all()
			.await
			.expect("Failed to execute query");

		assert_eq!(results.len(), 2);
		assert_eq!(results[0].app_label, "auth");
		assert_eq!(results[1].app_label, "blog");
	}

	#[tokio::test]
	async fn test_content_type_query_limit_offset() {
		let pool = setup_test_db().await;

		// Create test data
		let context = ContentTypeTransaction::new(pool.clone());
		context
			.create("app1", "Model1")
			.await
			.expect("Failed to create");
		context
			.create("app2", "Model2")
			.await
			.expect("Failed to create");
		context
			.create("app3", "Model3")
			.await
			.expect("Failed to create");

		// Query with limit/offset
		let query = ContentTypeQuery::new(pool);
		let results = query
			.order_by_id()
			.limit(2)
			.offset(1)
			.all()
			.await
			.expect("Failed to execute query");

		assert_eq!(results.len(), 2);
	}

	#[tokio::test]
	async fn test_content_type_query_first() {
		let pool = setup_test_db().await;

		// Create test data
		let context = ContentTypeTransaction::new(pool.clone());
		context
			.create("auth", "User")
			.await
			.expect("Failed to create");

		// first()
		let query = ContentTypeQuery::new(pool);
		let result = query
			.filter_app_label("auth")
			.first()
			.await
			.expect("Failed to execute query");

		assert!(result.is_some());
		assert_eq!(result.unwrap().model, "User");
	}

	#[tokio::test]
	async fn test_content_type_query_count() {
		let pool = setup_test_db().await;

		// Create test data
		let context = ContentTypeTransaction::new(pool.clone());
		context
			.create("auth", "User")
			.await
			.expect("Failed to create");
		context
			.create("auth", "Group")
			.await
			.expect("Failed to create");
		context
			.create("blog", "Post")
			.await
			.expect("Failed to create");

		// count()
		let query = ContentTypeQuery::new(pool);
		let count = query
			.filter_app_label("auth")
			.count()
			.await
			.expect("Failed to count");

		assert_eq!(count, 2);
	}

	#[tokio::test]
	async fn test_content_type_query_exists() {
		let pool = setup_test_db().await;

		// Create test data
		let context = ContentTypeTransaction::new(pool.clone());
		context
			.create("auth", "User")
			.await
			.expect("Failed to create");

		// exists()
		let query = ContentTypeQuery::new(pool.clone());
		let exists = query
			.filter_app_label("auth")
			.exists()
			.await
			.expect("Failed to check existence");

		assert!(exists);

		// Non-existent case
		let query2 = ContentTypeQuery::new(pool);
		let not_exists = query2
			.filter_app_label("nonexistent")
			.exists()
			.await
			.expect("Failed to check existence");

		assert!(!not_exists);
	}

	#[tokio::test]
	async fn test_content_type_context_create() {
		let pool = setup_test_db().await;

		let context = ContentTypeTransaction::new(pool.clone());
		let ct = context
			.create("shop", "Product")
			.await
			.expect("Failed to create");

		assert_eq!(ct.app_label, "shop");
		assert_eq!(ct.model, "Product");
		assert!(ct.id.is_some());
	}

	#[tokio::test]
	async fn test_content_type_context_delete() {
		let pool = setup_test_db().await;

		let context = ContentTypeTransaction::new(pool.clone());
		let ct = context
			.create("temp", "Model")
			.await
			.expect("Failed to create");
		let id = ct.id.unwrap();

		// Delete
		context.delete(id).await.expect("Failed to delete");

		// Verify deletion
		let query = ContentTypeQuery::new(pool);
		let result = query.filter_id(id).first().await.expect("Failed to query");

		assert!(result.is_none());
	}

	#[tokio::test]
	async fn test_content_type_query_multiple_filters() {
		let pool = setup_test_db().await;

		let context = ContentTypeTransaction::new(pool.clone());
		context
			.create("auth", "User")
			.await
			.expect("Failed to create");
		context
			.create("auth", "Group")
			.await
			.expect("Failed to create");

		// Multiple filters
		let query = ContentTypeQuery::new(pool);
		let results = query
			.filter_app_label("auth")
			.filter_model("User")
			.all()
			.await
			.expect("Failed to execute query");

		assert_eq!(results.len(), 1);
		assert_eq!(results[0].model, "User");
	}

	#[tokio::test]
	async fn test_content_type_query_order_desc() {
		let pool = setup_test_db().await;

		let context = ContentTypeTransaction::new(pool.clone());
		context
			.create("app1", "Model1")
			.await
			.expect("Failed to create");
		context
			.create("app2", "Model2")
			.await
			.expect("Failed to create");

		// Descending sort
		let query = ContentTypeQuery::new(pool);
		let results = query
			.order_by_app_label_desc()
			.all()
			.await
			.expect("Failed to execute query");

		assert_eq!(results[0].app_label, "app2");
		assert_eq!(results[1].app_label, "app1");
	}
}
