//! # Database Engine
//!
//! SQLAlchemy-inspired database engine with connection pooling.
//!
//! This module is inspired by SQLAlchemy's engine implementation
//! Copyright 2005-2025 SQLAlchemy authors and contributors
//! Licensed under MIT License. See THIRD-PARTY-NOTICES for details.

use backends::{DatabaseError, DatabaseType, Row as DbRow, connection::DatabaseConnection};

#[cfg(feature = "pooling")]
use deadpool_sqlx::{Pool, Runtime};

use sqlx::{Any, AnyPool, AssertSqlSafe};

/// Database engine configuration
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Database URL (e.g., "sqlite::memory:", "postgres://...")
    pub url: String,

    /// Connection pool size (min)
    pub pool_min_size: u32,

    /// Connection pool size (max)
    pub pool_max_size: u32,

    /// Connection timeout in seconds
    pub pool_timeout: u64,

    /// Enable echo (log all SQL)
    pub echo: bool,

    /// Enable query result caching
    pub query_cache_size: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            url: "sqlite::memory:".to_string(),
            pool_min_size: 1,
            pool_max_size: 10,
            pool_timeout: 30,
            echo: false,
            query_cache_size: 500,
        }
    }
}

impl EngineConfig {
    /// Create new config with URL
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }
    /// Set pool sizes
    pub fn with_pool_size(mut self, min: u32, max: u32) -> Self {
        self.pool_min_size = min;
        self.pool_max_size = max;
        self
    }
    /// Enable SQL echo
    pub fn with_echo(mut self, echo: bool) -> Self {
        self.echo = echo;
        self
    }
    /// Set query cache size
    pub fn with_cache_size(mut self, size: usize) -> Self {
        self.query_cache_size = size;
        self
    }
}

/// Database engine - manages connections and execution
pub struct Engine {
    pool: AnyPool,
    config: EngineConfig,
}

impl Engine {
    /// Create a new engine from config
    ///
    pub async fn from_config(config: EngineConfig) -> Result<Self, sqlx::Error> {
        let pool = AnyPool::connect(&config.url).await?;

        Ok(Self { pool, config })
    }
    /// Create a new engine from URL
    ///
    /// # Examples
    ///
    /// ```
    /// let instance = Type::new();
    /// ```
    pub async fn new(url: impl Into<String>) -> Result<Self, sqlx::Error> {
        Self::from_config(EngineConfig::new(url)).await
    }
    /// Get a connection from the pool
    ///
    pub async fn connect(&self) -> Result<sqlx::pool::PoolConnection<Any>, sqlx::Error> {
        self.pool.acquire().await
    }
    /// Execute a SQL statement
    pub async fn execute(&self, sql: &str) -> Result<u64, sqlx::Error> {
        if self.config.echo {
            println!("SQL: {}", sql);
        }

        let result = sqlx::query(AssertSqlSafe(sql)).execute(&self.pool).await?;
        Ok(result.rows_affected())
    }
    /// Execute a query and return results
    ///
    pub async fn fetch_all(&self, sql: &str) -> Result<Vec<sqlx::any::AnyRow>, sqlx::Error> {
        if self.config.echo {
            println!("SQL: {}", sql);
        }

        sqlx::query(AssertSqlSafe(sql)).fetch_all(&self.pool).await
    }
    /// Execute a query and return a single result
    ///
    pub async fn fetch_one(&self, sql: &str) -> Result<sqlx::any::AnyRow, sqlx::Error> {
        if self.config.echo {
            println!("SQL: {}", sql);
        }

        sqlx::query(AssertSqlSafe(sql)).fetch_one(&self.pool).await
    }
    /// Execute a query and return an optional result
    ///
    pub async fn fetch_optional(
        &self,
        sql: &str,
    ) -> Result<Option<sqlx::any::AnyRow>, sqlx::Error> {
        if self.config.echo {
            println!("SQL: {}", sql);
        }

        sqlx::query(AssertSqlSafe(sql))
            .fetch_optional(&self.pool)
            .await
    }
    /// Begin a transaction
    ///
    pub async fn begin(&self) -> Result<sqlx::Transaction<'_, Any>, sqlx::Error> {
        self.pool.begin().await
    }
    /// Get the engine configuration
    ///
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }
    /// Get reference to the connection pool
    ///
    pub fn pool(&self) -> &AnyPool {
        &self.pool
    }
    /// Clone this engine (shares the connection pool)
    ///
    pub fn clone_ref(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            config: self.config.clone(),
        }
    }
}
/// Create a new database engine
///
pub async fn create_engine(url: impl Into<String>) -> Result<Engine, sqlx::Error> {
    Engine::new(url).await
}
/// Create a new database engine with configuration
///
pub async fn create_engine_with_config(config: EngineConfig) -> Result<Engine, sqlx::Error> {
    Engine::from_config(config).await
}

/// Database engine using DatabaseConnection (multi-database support)
pub struct DatabaseEngine {
    connection: DatabaseConnection,
    db_type: DatabaseType,
    config: EngineConfig,
}

impl DatabaseEngine {
    /// Create a new database engine from DatabaseConnection
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use reinhardt_orm::engine::DatabaseEngine;
    /// use reinhardt_db::backends::{DatabaseConnection, DatabaseType};
    ///
    /// let connection = DatabaseConnection::connect_postgres("postgres://localhost/mydb").await?;
    /// let engine = DatabaseEngine::new(connection, DatabaseType::Postgres);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(connection: DatabaseConnection, db_type: DatabaseType) -> Self {
        Self {
            connection,
            db_type,
            config: EngineConfig::default(),
        }
    }

    /// Create a new database engine with configuration
    pub fn with_config(
        connection: DatabaseConnection,
        db_type: DatabaseType,
        config: EngineConfig,
    ) -> Self {
        Self {
            connection,
            db_type,
            config,
        }
    }

    /// Create a new PostgreSQL engine
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use reinhardt_orm::engine::DatabaseEngine;
    ///
    /// let engine = DatabaseEngine::from_postgres("postgres://localhost/mydb").await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "postgres")]
    pub async fn from_postgres(url: &str) -> Result<Self, DatabaseError> {
        let connection = DatabaseConnection::connect_postgres(url).await?;
        Ok(Self::new(connection, DatabaseType::Postgres))
    }

    /// Create a new SQLite engine
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use reinhardt_orm::engine::DatabaseEngine;
    ///
    /// let engine = DatabaseEngine::from_sqlite(":memory:").await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sqlite")]
    pub async fn from_sqlite(url: &str) -> Result<Self, DatabaseError> {
        let connection = DatabaseConnection::connect_sqlite(url).await?;
        Ok(Self::new(connection, DatabaseType::Sqlite))
    }

    /// Create a new MySQL engine
    #[cfg(feature = "mysql")]
    pub async fn from_mysql(url: &str) -> Result<Self, DatabaseError> {
        let connection = DatabaseConnection::connect_mysql(url).await?;
        Ok(Self::new(connection, DatabaseType::Mysql))
    }

    /// Get reference to the database connection
    pub fn connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    /// Get the database type
    pub fn database_type(&self) -> DatabaseType {
        self.db_type
    }

    /// Get the engine configuration
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Execute a SQL statement
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use reinhardt_orm::engine::DatabaseEngine;
    ///
    /// let engine = DatabaseEngine::from_sqlite(":memory:").await?;
    /// let rows_affected = engine.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute(&self, sql: &str) -> Result<u64, DatabaseError> {
        if self.config.echo {
            println!("SQL: {}", sql);
        }

        let result = self.connection.execute(sql, vec![]).await?;
        Ok(result.rows_affected)
    }

    /// Execute a query and return all results
    pub async fn fetch_all(&self, sql: &str) -> Result<Vec<DbRow>, DatabaseError> {
        if self.config.echo {
            println!("SQL: {}", sql);
        }

        self.connection.fetch_all(sql, vec![]).await
    }

    /// Execute a query and return a single result
    pub async fn fetch_one(&self, sql: &str) -> Result<DbRow, DatabaseError> {
        if self.config.echo {
            println!("SQL: {}", sql);
        }

        self.connection.fetch_one(sql, vec![]).await
    }

    /// Execute a query and return an optional result
    pub async fn fetch_optional(&self, sql: &str) -> Result<Option<DbRow>, DatabaseError> {
        if self.config.echo {
            println!("SQL: {}", sql);
        }

        let rows = self.connection.fetch_all(sql, vec![]).await?;
        Ok(rows.into_iter().next())
    }

    /// Clone this engine (shares the database connection)
    pub fn clone_ref(&self) -> Self {
        Self {
            connection: self.connection.clone(),
            db_type: self.db_type,
            config: self.config.clone(),
        }
    }
}

/// Create a new database engine from PostgreSQL URL
#[cfg(feature = "postgres")]
pub async fn create_database_engine_postgres(url: &str) -> Result<DatabaseEngine, DatabaseError> {
    DatabaseEngine::from_postgres(url).await
}

/// Create a new database engine from SQLite URL
#[cfg(feature = "sqlite")]
pub async fn create_database_engine_sqlite(url: &str) -> Result<DatabaseEngine, DatabaseError> {
    DatabaseEngine::from_sqlite(url).await
}

/// Create a new database engine from MySQL URL
#[cfg(feature = "mysql")]
pub async fn create_database_engine_mysql(url: &str) -> Result<DatabaseEngine, DatabaseError> {
    DatabaseEngine::from_mysql(url).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Requires sqlx database driver to be installed"]
    async fn test_engine_creation() {
        let engine = create_engine("sqlite::memory:")
            .await
            .expect("Failed to create engine");

        assert_eq!(engine.config().url, "sqlite::memory:");
    }

    #[tokio::test]
    #[ignore = "Requires sqlx database driver to be installed"]
    async fn test_engine_execute() {
        let engine = create_engine("sqlite::memory:")
            .await
            .expect("Failed to create engine");

        let result = engine
            .execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "Requires sqlx database driver to be installed"]
    async fn test_engine_query() {
        let engine = create_engine("sqlite::memory:")
            .await
            .expect("Failed to create engine");

        engine
            .execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
            .await
            .unwrap();

        engine
            .execute("INSERT INTO users (id, name) VALUES (1, 'Alice')")
            .await
            .unwrap();

        let rows = engine
            .fetch_all("SELECT * FROM users")
            .await
            .expect("Query failed");

        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    #[ignore = "Requires sqlx database driver to be installed"]
    async fn test_engine_with_config() {
        let config = EngineConfig::new("sqlite::memory:")
            .with_pool_size(2, 5)
            .with_echo(true);

        let engine = create_engine_with_config(config)
            .await
            .expect("Failed to create engine");

        assert_eq!(engine.config().pool_min_size, 2);
        assert_eq!(engine.config().pool_max_size, 5);
        assert!(engine.config().echo);
    }

    #[tokio::test]
    #[ignore = "Requires sqlx database driver to be installed"]
    async fn test_transaction() {
        let engine = create_engine("sqlite::memory:")
            .await
            .expect("Failed to create engine");

        engine
            .execute("CREATE TABLE accounts (id INTEGER PRIMARY KEY, balance INTEGER)")
            .await
            .unwrap();

        let mut tx = engine.begin().await.expect("Failed to begin transaction");

        sqlx::query("INSERT INTO accounts (id, balance) VALUES (1, 100)")
            .execute(&mut *tx)
            .await
            .unwrap();

        tx.commit().await.expect("Failed to commit");

        let rows = engine.fetch_all("SELECT * FROM accounts").await.unwrap();
        assert_eq!(rows.len(), 1);
    }
}
