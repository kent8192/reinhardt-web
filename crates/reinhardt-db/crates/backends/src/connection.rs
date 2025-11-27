//! Database connection management

use std::sync::Arc;

use crate::{
	backend::DatabaseBackend,
	error::Result,
	query_builder::{DeleteBuilder, InsertBuilder, SelectBuilder, UpdateBuilder},
};

#[cfg(feature = "postgres")]
use crate::dialect::PostgresBackend;

#[cfg(feature = "sqlite")]
use crate::dialect::SqliteBackend;

#[cfg(feature = "mysql")]
use crate::dialect::MySqlBackend;

/// Database connection wrapper
#[derive(Clone)]
pub struct DatabaseConnection {
	backend: Arc<dyn DatabaseBackend>,
}

/// Injectable implementation for DatabaseConnection
///
/// DatabaseConnection must be explicitly registered in the DI context using
/// `InjectionContextBuilder::singleton()`. It cannot be auto-injected because
/// it requires runtime configuration (connection URL, pool settings, etc.).
///
/// # Example
///
/// ```ignore
/// use reinhardt_di::{InjectionContext, SingletonScope};
/// use reinhardt_db::backends::DatabaseConnection;
/// use std::sync::Arc;
///
/// // Create and configure the connection
/// let db = DatabaseConnection::connect_postgres("postgres://localhost/mydb")
///     .await
///     .expect("Failed to connect to database");
///
/// // Register in DI context
/// let singleton_scope = Arc::new(SingletonScope::new());
/// let ctx = InjectionContext::builder(singleton_scope)
///     .singleton(db)
///     .build();
/// ```
#[cfg(feature = "di")]
#[async_trait::async_trait]
impl reinhardt_di::Injectable for DatabaseConnection {
	async fn inject(ctx: &reinhardt_di::InjectionContext) -> reinhardt_di::DiResult<Self> {
		// Try singleton scope first (primary expected location)
		if let Some(conn) = ctx.get_singleton::<Self>() {
			return Ok(std::sync::Arc::try_unwrap(conn).unwrap_or_else(|arc| (*arc).clone()));
		}

		// Try request scope as fallback
		if let Some(conn) = ctx.get_request::<Self>() {
			return Ok(std::sync::Arc::try_unwrap(conn).unwrap_or_else(|arc| (*arc).clone()));
		}

		// Not registered - provide helpful error
		Err(reinhardt_di::DiError::NotRegistered {
			type_name: std::any::type_name::<Self>().to_string(),
			hint: "Use InjectionContextBuilder::singleton(db_connection) to register a \
			       DatabaseConnection. Create it with DatabaseConnection::connect_postgres(), \
			       connect_sqlite(), or connect_mysql()."
				.to_string(),
		})
	}

	async fn inject_uncached(ctx: &reinhardt_di::InjectionContext) -> reinhardt_di::DiResult<Self> {
		// For DatabaseConnection, inject_uncached behaves the same as inject
		// because we don't support creating new connections on demand
		Self::inject(ctx).await
	}
}

impl DatabaseConnection {
	pub fn new(backend: Arc<dyn DatabaseBackend>) -> Self {
		Self { backend }
	}

	#[cfg(feature = "postgres")]
	pub async fn connect_postgres(url: &str) -> Result<Self> {
		Self::connect_postgres_with_pool_size(url, None).await
	}

	#[cfg(feature = "postgres")]
	pub async fn connect_postgres_with_pool_size(
		url: &str,
		pool_size: Option<u32>,
	) -> Result<Self> {
		use sqlx::postgres::PgPoolOptions;
		use std::time::Duration;

		// Priority: explicit argument > environment variable > default
		let max_connections = pool_size
			.or_else(|| {
				std::env::var("DATABASE_POOL_MAX_CONNECTIONS")
					.ok()
					.and_then(|v| v.parse::<u32>().ok())
			})
			.unwrap_or(20); // Increased default from 10 to 20 for better concurrency

		let pool = PgPoolOptions::new()
			.max_connections(max_connections)
			.min_connections(1) // Maintain at least 1 connection
			.acquire_timeout(Duration::from_secs(10)) // Increased from 3s to 10s for busy pools
			.idle_timeout(Some(Duration::from_secs(10))) // Close idle connections after 10s
			.max_lifetime(Some(Duration::from_secs(30 * 60))) // Close connections after 30 minutes
			.connect(url)
			.await?;

		Ok(Self {
			backend: Arc::new(PostgresBackend::new(pool)),
		})
	}

	/// Connect to PostgreSQL with automatic database creation if it doesn't exist.
	///
	/// This method first attempts to connect to the specified database. If the connection
	/// fails due to the database not existing, it will:
	/// 1. Connect to the default `postgres` database
	/// 2. Create the target database
	/// 3. Reconnect to the newly created database
	///
	/// # Arguments
	///
	/// * `url` - PostgreSQL connection URL (e.g., "postgres://user:pass@localhost/mydb")
	///
	/// # Example
	///
	/// ```no_run
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// use reinhardt_db::backends::connection::DatabaseConnection;
	///
	/// // Will create 'mydb' if it doesn't exist
	/// let conn = DatabaseConnection::connect_postgres_or_create(
	///     "postgres://postgres@localhost/mydb"
	/// ).await?;
	/// # Ok(())
	/// # }
	/// ```
	#[cfg(feature = "postgres")]
	pub async fn connect_postgres_or_create(url: &str) -> Result<Self> {
		Self::connect_postgres_or_create_with_pool_size(url, None).await
	}

	/// Connect to PostgreSQL with automatic database creation and custom pool size.
	///
	/// See [`connect_postgres_or_create`] for details on automatic database creation.
	#[cfg(feature = "postgres")]
	pub async fn connect_postgres_or_create_with_pool_size(
		url: &str,
		pool_size: Option<u32>,
	) -> Result<Self> {
		// First try normal connection
		match Self::connect_postgres_with_pool_size(url, pool_size).await {
			Ok(conn) => return Ok(conn),
			Err(e) => {
				// Check if error indicates database doesn't exist
				let error_str = format!("{:?}", e);
				if !error_str.contains("does not exist")
					&& !error_str.contains("database")
					&& !error_str.contains("3D000")
				{
					// Not a "database doesn't exist" error, propagate it
					return Err(e);
				}
				// Database doesn't exist, try to create it
			}
		}

		// Parse the URL to extract database name
		let (admin_url, db_name) = Self::parse_postgres_url_for_creation(url)?;

		// Connect to default postgres database
		use sqlx::postgres::PgPoolOptions;
		use std::time::Duration;

		let admin_pool = PgPoolOptions::new()
			.max_connections(1)
			.acquire_timeout(Duration::from_secs(10))
			.connect(&admin_url)
			.await
			.map_err(|e| {
				crate::error::DatabaseError::ConnectionError(format!(
					"Failed to connect to postgres database for auto-creation: {}",
					e
				))
			})?;

		// Create the database
		let create_sql = format!("CREATE DATABASE \"{}\"", db_name);
		sqlx::query(&create_sql)
			.execute(&admin_pool)
			.await
			.map_err(|e| {
				crate::error::DatabaseError::QueryError(format!(
					"Failed to create database '{}': {}",
					db_name, e
				))
			})?;

		// Close admin connection
		admin_pool.close().await;

		// Now connect to the newly created database
		Self::connect_postgres_with_pool_size(url, pool_size).await
	}

	/// Parse a PostgreSQL URL and return an admin URL (pointing to 'postgres' db) and the target database name.
	#[cfg(feature = "postgres")]
	fn parse_postgres_url_for_creation(url: &str) -> Result<(String, String)> {
		// Parse URL like: postgres://user:pass@host:port/dbname?params
		// We need to extract dbname and create a URL pointing to 'postgres' database

		// Handle both postgres:// and postgresql:// schemes
		let url_without_scheme = url
			.strip_prefix("postgres://")
			.or_else(|| url.strip_prefix("postgresql://"))
			.ok_or_else(|| {
				crate::error::DatabaseError::ConnectionError(
					"Invalid PostgreSQL URL: must start with postgres:// or postgresql://"
						.to_string(),
				)
			})?;

		// Split at '?' to separate query params
		let (path_part, query_part) = match url_without_scheme.find('?') {
			Some(pos) => (&url_without_scheme[..pos], Some(&url_without_scheme[pos..])),
			None => (url_without_scheme, None),
		};

		// Find the last '/' which separates host:port from database name
		let last_slash_pos = path_part.rfind('/').ok_or_else(|| {
			crate::error::DatabaseError::ConnectionError(
				"Invalid PostgreSQL URL: no database name found".to_string(),
			)
		})?;

		let host_part = &path_part[..last_slash_pos];
		let db_name = &path_part[last_slash_pos + 1..];

		if db_name.is_empty() {
			return Err(crate::error::DatabaseError::ConnectionError(
				"Invalid PostgreSQL URL: database name is empty".to_string(),
			));
		}

		// Construct admin URL with 'postgres' database
		let admin_url = match query_part {
			Some(params) => format!("postgres://{}/postgres{}", host_part, params),
			None => format!("postgres://{}/postgres", host_part),
		};

		Ok((admin_url, db_name.to_string()))
	}

	#[cfg(feature = "sqlite")]
	pub async fn connect_sqlite(url: &str) -> Result<Self> {
		use sqlx::SqlitePool;
		let pool = SqlitePool::connect(url).await?;
		Ok(Self {
			backend: Arc::new(SqliteBackend::new(pool)),
		})
	}

	#[cfg(feature = "sqlite")]
	pub fn from_sqlite_pool(pool: sqlx::SqlitePool) -> Self {
		Self {
			backend: Arc::new(SqliteBackend::new(pool)),
		}
	}

	#[cfg(feature = "mysql")]
	pub async fn connect_mysql(url: &str) -> Result<Self> {
		use sqlx::MySqlPool;
		let pool = MySqlPool::connect(url).await?;
		Ok(Self {
			backend: Arc::new(MySqlBackend::new(pool)),
		})
	}

	/// Connect to MongoDB database
	///
	/// # Arguments
	///
	/// * `url` - MongoDB connection string (e.g., "mongodb://localhost:27017")
	/// * `database` - Database name to use
	///
	/// # Example
	///
	/// ```no_run
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// use reinhardt_db::backends::connection::DatabaseConnection;
	///
	/// let connection = DatabaseConnection::connect_mongodb(
	///     "mongodb://localhost:27017",
	///     "mydb"
	/// ).await?;
	/// # Ok(())
	/// # }
	/// ```
	#[cfg(feature = "mongodb-backend")]
	pub async fn connect_mongodb(url: &str, database: &str) -> Result<Self> {
		use crate::drivers::mongodb::MongoDBBackend;
		let backend = MongoDBBackend::connect(url).await?.with_database(database);
		Ok(Self {
			backend: Arc::new(backend),
		})
	}

	pub fn backend(&self) -> Arc<dyn DatabaseBackend> {
		self.backend.clone()
	}

	/// Get the database type
	pub fn database_type(&self) -> crate::types::DatabaseType {
		self.backend.database_type()
	}

	pub fn insert(&self, table: impl Into<String>) -> InsertBuilder {
		InsertBuilder::new(self.backend.clone(), table)
	}

	pub fn update(&self, table: impl Into<String>) -> UpdateBuilder {
		UpdateBuilder::new(self.backend.clone(), table)
	}

	pub fn select(&self) -> SelectBuilder {
		SelectBuilder::new(self.backend.clone())
	}

	pub fn delete(&self, table: impl Into<String>) -> DeleteBuilder {
		DeleteBuilder::new(self.backend.clone(), table)
	}

	pub async fn execute(
		&self,
		sql: &str,
		params: Vec<crate::types::QueryValue>,
	) -> Result<crate::types::QueryResult> {
		self.backend.execute(sql, params).await
	}

	pub async fn fetch_one(
		&self,
		sql: &str,
		params: Vec<crate::types::QueryValue>,
	) -> Result<crate::types::Row> {
		self.backend.fetch_one(sql, params).await
	}

	pub async fn fetch_all(
		&self,
		sql: &str,
		params: Vec<crate::types::QueryValue>,
	) -> Result<Vec<crate::types::Row>> {
		self.backend.fetch_all(sql, params).await
	}

	pub async fn fetch_optional(
		&self,
		sql: &str,
		params: Vec<crate::types::QueryValue>,
	) -> Result<Option<crate::types::Row>> {
		self.backend.fetch_optional(sql, params).await
	}

	/// Begin a database transaction and return a dedicated executor
	///
	/// This method acquires a dedicated database connection and begins a
	/// transaction on it. All queries executed through the returned
	/// `TransactionExecutor` are guaranteed to run on the same physical
	/// connection, ensuring proper transaction isolation.
	///
	/// # Returns
	///
	/// A boxed `TransactionExecutor` that holds the dedicated connection
	/// and provides methods for executing queries within the transaction.
	///
	/// # Example
	///
	/// ```no_run
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// use reinhardt_db::backends::connection::DatabaseConnection;
	///
	/// let conn = DatabaseConnection::connect_postgres("postgres://localhost/mydb").await?;
	/// let mut tx = conn.begin().await?;
	///
	/// tx.execute("INSERT INTO users (name) VALUES ($1)", vec!["Alice".into()]).await?;
	/// tx.commit().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn begin(&self) -> Result<Box<dyn crate::types::TransactionExecutor>> {
		self.backend.begin().await
	}

	/// Begin a transaction with a specific isolation level
	///
	/// # Examples
	///
	/// ```no_run
	/// # async fn example() -> reinhardt_backends::error::Result<()> {
	/// use reinhardt_backends::connection::DatabaseConnection;
	/// use reinhardt_backends::types::IsolationLevel;
	///
	/// let conn = DatabaseConnection::connect_postgres("postgres://localhost/mydb").await?;
	/// let mut tx = conn.begin_with_isolation(IsolationLevel::Serializable).await?;
	///
	/// tx.execute("INSERT INTO users (name) VALUES ($1)", vec!["Alice".into()]).await?;
	/// tx.commit().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn begin_with_isolation(
		&self,
		level: crate::types::IsolationLevel,
	) -> Result<Box<dyn crate::types::TransactionExecutor>> {
		self.backend.begin_with_isolation(level).await
	}
}
