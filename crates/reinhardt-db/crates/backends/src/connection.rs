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

impl DatabaseConnection {
	pub fn new(backend: Arc<dyn DatabaseBackend>) -> Self {
		Self { backend }
	}

	#[cfg(feature = "postgres")]
	pub async fn connect_postgres(url: &str) -> Result<Self> {
		use sqlx::PgPool;
		let pool = PgPool::connect(url).await?;
		Ok(Self {
			backend: Arc::new(PostgresBackend::new(pool)),
		})
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
}
