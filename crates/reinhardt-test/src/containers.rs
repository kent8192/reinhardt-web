//! TestContainers integration for database testing
//!
//! Provides automatic Docker container management for testing with real databases.
//! Containers are automatically started and cleaned up during tests.
//!
//! # Features
//!
//! - **PostgreSQL**: Full-featured PostgreSQL container with customizable credentials
//! - **MySQL**: MySQL container with customizable credentials
//! - **Redis**: Redis container for cache/session testing
//! - **SQLite**: In-memory and temporary file database URLs
//!
//! # Container Types
//!
//! ## PostgresContainer
//!
//! ```ignore
//! use reinhardt_test::containers::PostgresContainer;
//!
//! let container = PostgresContainer::new();
//! let url = container.connection_url();
//! // Use url for database connection
//! ```
//!
//! ## MySqlContainer
//!
//! ```ignore
//! use reinhardt_test::containers::MySqlContainer;
//!
//! let container = MySqlContainer::new();
//! let url = container.connection_url();
//! ```
//!
//! ## RedisContainer
//!
//! ```ignore
//! use reinhardt_test::containers::RedisContainer;
//!
//! let container = RedisContainer::new();
//! let url = container.connection_url();
//! ```
//!
//! # Helper Functions
//!
//! ## Quick Start Functions
//!
//! ```ignore
//! use reinhardt_test::containers::{start_postgres, start_redis};
//!
//! let (pg_container, pg_url) = start_postgres();
//! let (redis_container, redis_url) = start_redis();
//! ```
//!
//! ## Test Wrapper Functions
//!
//! ```ignore
//! use reinhardt_test::containers::with_postgres;
//!
//! #[tokio::test]
//! async fn my_test() {
//!     with_postgres(|db| async move {
//!         let url = db.connection_url();
//!         // Use database...
//!         Ok(())
//!     }).await.unwrap();
//! }
//! ```
//!
//! ## SQLite Helpers
//!
//! ```ignore
//! use reinhardt_test::containers::sqlite;
//!
//! let memory_url = sqlite::memory_url();
//! let temp_url = sqlite::temp_file_url("my_test");
//! ```

use testcontainers::runners::{AsyncRunner, SyncRunner};
use testcontainers::{Container, ContainerAsync, GenericImage, ImageExt};
use testcontainers_modules::mysql::Mysql;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::redis::Redis as RedisImage;

/// Common interface for database test containers
#[async_trait::async_trait]
pub trait TestDatabase: Send + Sync {
	/// Get the database connection URL
	fn connection_url(&self) -> String;

	/// Get the database type (postgres, mysql, etc.)
	fn database_type(&self) -> &'static str;

	/// Wait for the database to be ready
	async fn wait_ready(&self) -> Result<(), Box<dyn std::error::Error>>;
}

/// PostgreSQL test container
pub struct PostgresContainer {
	#[allow(dead_code)]
	container: Container<Postgres>,
	host: String,
	port: u16,
	database: String,
	username: String,
	password: String,
}

/// Helper function to start a PostgreSQL container with default credentials
///
/// This is provided for compatibility with existing test code.
/// Returns a tuple of (container, connection_url).
///
/// Default credentials:
/// - Username: postgres
/// - Password: postgres
/// - Database: test
pub fn start_postgres() -> (PostgresContainer, String) {
	let container = PostgresContainer::new();
	let url = container.connection_url();
	(container, url)
}

/// Helper function to start a PostgreSQL container with custom credentials
///
/// Returns a tuple of (container, connection_url).
pub fn start_postgres_with_credentials(
	username: &str,
	password: &str,
	database: &str,
) -> (PostgresContainer, String) {
	let container = PostgresContainer::with_credentials(username, password, database);
	let url = container.connection_url();
	(container, url)
}

impl Default for PostgresContainer {
	fn default() -> Self {
		Self::new()
	}
}

impl PostgresContainer {
	/// Create a new PostgreSQL container with default settings
	pub fn new() -> Self {
		Self::with_credentials("postgres", "postgres", "test")
	}
	/// Create a PostgreSQL container with custom credentials
	pub fn with_credentials(username: &str, password: &str, database: &str) -> Self {
		let image = Postgres::default()
			.with_env_var("POSTGRES_USER", username)
			.with_env_var("POSTGRES_PASSWORD", password)
			.with_env_var("POSTGRES_DB", database);

		let container = SyncRunner::start(image).expect("Failed to start PostgreSQL container");
		let port = container.get_host_port_ipv4(5432).unwrap();

		Self {
			container,
			host: "localhost".to_string(),
			port,
			database: database.to_string(),
			username: username.to_string(),
			password: password.to_string(),
		}
	}
	/// Get the container port
	pub fn port(&self) -> u16 {
		self.port
	}
}

#[async_trait::async_trait]
impl TestDatabase for PostgresContainer {
	fn connection_url(&self) -> String {
		format!(
			"postgres://{}:{}@{}:{}/{}",
			self.username, self.password, self.host, self.port, self.database
		)
	}

	fn database_type(&self) -> &'static str {
		"postgres"
	}

	async fn wait_ready(&self) -> Result<(), Box<dyn std::error::Error>> {
		// Try to connect to ensure database is ready
		let url = self.connection_url();
		let pool = sqlx::postgres::PgPool::connect(&url).await?;
		sqlx::query("SELECT 1").execute(&pool).await?;
		pool.close().await;
		Ok(())
	}
}

/// MySQL test container
pub struct MySqlContainer {
	#[allow(dead_code)]
	container: Container<Mysql>,
	host: String,
	port: u16,
	database: String,
	username: String,
	password: String,
}

impl Default for MySqlContainer {
	fn default() -> Self {
		Self::new()
	}
}

impl MySqlContainer {
	/// Create a new MySQL container with default settings
	pub fn new() -> Self {
		Self::with_credentials("root", "test", "test")
	}
	/// Create a MySQL container with custom credentials
	pub fn with_credentials(username: &str, password: &str, database: &str) -> Self {
		let image = Mysql::default()
			.with_env_var("MYSQL_ROOT_PASSWORD", password)
			.with_env_var("MYSQL_DATABASE", database);

		let container = SyncRunner::start(image).expect("Failed to start MySQL container");
		let port = container.get_host_port_ipv4(3306).unwrap();

		Self {
			container,
			host: "localhost".to_string(),
			port,
			database: database.to_string(),
			username: username.to_string(),
			password: password.to_string(),
		}
	}
	/// Get the container port
	pub fn port(&self) -> u16 {
		self.port
	}
}

#[async_trait::async_trait]
impl TestDatabase for MySqlContainer {
	fn connection_url(&self) -> String {
		format!(
			"mysql://{}:{}@{}:{}/{}",
			self.username, self.password, self.host, self.port, self.database
		)
	}

	fn database_type(&self) -> &'static str {
		"mysql"
	}

	async fn wait_ready(&self) -> Result<(), Box<dyn std::error::Error>> {
		// Try to connect to ensure database is ready
		let url = self.connection_url();
		let pool = sqlx::mysql::MySqlPool::connect(&url).await?;
		sqlx::query("SELECT 1").execute(&pool).await?;
		pool.close().await;
		Ok(())
	}
}

/// Redis test container
pub struct RedisContainer {
	#[allow(dead_code)]
	container: ContainerAsync<RedisImage>,
	host: String,
	port: u16,
}

/// Helper function to start a Redis container (alias for RedisContainer::new)
///
/// This is provided for compatibility with existing test code.
/// Returns a tuple of (container, connection_url).
pub async fn start_redis() -> (RedisContainer, String) {
	let container = RedisContainer::new().await;
	let url = container.connection_url();
	(container, url)
}

impl RedisContainer {
	/// Create a new Redis container
	pub async fn new() -> Self {
		let image = RedisImage::default();
		let container = AsyncRunner::start(image)
			.await
			.expect("Failed to start Redis container");
		let port = container.get_host_port_ipv4(6379).await.unwrap();

		Self {
			container,
			host: "localhost".to_string(),
			port,
		}
	}
	/// Get the connection URL for Redis
	pub fn connection_url(&self) -> String {
		format!("redis://{}:{}", self.host, self.port)
	}
	/// Get the container port
	pub fn port(&self) -> u16 {
		self.port
	}
}

/// Memcached test container
pub struct MemcachedContainer {
	#[allow(dead_code)]
	container: ContainerAsync<GenericImage>,
	host: String,
	port: u16,
}

/// Helper function to start a Memcached container
///
/// Returns a tuple of (container, connection_url).
pub async fn start_memcached() -> (MemcachedContainer, String) {
	let container = MemcachedContainer::new().await;
	let url = container.connection_url();
	(container, url)
}

impl MemcachedContainer {
	/// Create a new Memcached container
	pub async fn new() -> Self {
		// Start Memcached container without WaitFor (we'll handle it manually)
		let image = GenericImage::new("memcached", "1.6-alpine");

		let container = AsyncRunner::start(image)
			.await
			.expect("Failed to start Memcached container");
		let port = container.get_host_port_ipv4(11211).await.unwrap();

		let instance = Self {
			container,
			host: "localhost".to_string(),
			port,
		};

		// Wait for Memcached to be fully ready with set/get test
		// This has its own retry logic with exponential backoff
		instance
			.wait_ready()
			.await
			.expect("Failed to wait for Memcached to be ready");

		instance
	}

	/// Get the connection URL for Memcached
	pub fn connection_url(&self) -> String {
		format!("{}:{}", self.host, self.port)
	}

	/// Get the container port
	pub fn port(&self) -> u16 {
		self.port
	}

	/// Wait for Memcached to be ready by performing actual set/get operations
	///
	/// This method implements exponential backoff retry logic and tests
	/// actual Memcached operations (set/get) instead of just connection checks.
	/// This ensures Memcached is fully initialized and ready to handle requests.
	pub async fn wait_ready(&self) -> Result<(), Box<dyn std::error::Error>> {
		use memcache_async::ascii::Protocol;
		use std::time::Duration;
		use tokio::net::TcpStream;
		use tokio::time::sleep;
		use tokio_util::compat::TokioAsyncReadCompatExt;

		let max_attempts = 10;
		let mut attempt = 0;
		let base_delay = Duration::from_millis(100);
		let test_key = "__wait_ready_test__".to_string();
		let test_value = b"ready";

		while attempt < max_attempts {
			match TcpStream::connect(format!("{}:{}", self.host, self.port)).await {
				Ok(stream) => {
					let compat_stream = stream.compat();
					let mut proto = Protocol::new(compat_stream);

					// Test actual set operation
					if let Ok(()) = proto.set(&test_key, test_value, 10).await {
						// Test actual get operation
						if let Ok(retrieved) = proto.get(&test_key).await
							&& retrieved == test_value
						{
							// Success! Clean up test key
							let _ = proto.delete(&test_key).await;
							// Small delay to ensure cleanup completes
							sleep(Duration::from_millis(50)).await;
							return Ok(());
						}
					}

					// Operations failed, retry with backoff
					attempt += 1;
					let delay = base_delay * 2_u32.pow(attempt.min(5));
					sleep(delay).await;
				}
				Err(e) => {
					// Connection failed
					attempt += 1;
					if attempt >= max_attempts {
						return Err(format!(
							"Memcached not ready after {} attempts: {}",
							max_attempts, e
						)
						.into());
					}

					// Exponential backoff: 100ms, 200ms, 400ms, 800ms, 1600ms, 3200ms...
					let delay = base_delay * 2_u32.pow(attempt.min(5));
					sleep(delay).await;
				}
			}
		}

		Err("Memcached not ready: set/get test failed after maximum retry attempts".into())
	}
}

/// Helper function to run a test with a Memcached container
pub async fn with_memcached<F, Fut>(f: F) -> Result<(), Box<dyn std::error::Error>>
where
	F: FnOnce(MemcachedContainer) -> Fut,
	Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
	let container = MemcachedContainer::new().await;
	f(container).await
}

/// Helper function to run a test with a database container
///
/// # Example
/// ```ignore
/// use reinhardt_test::containers::{with_postgres, PostgresContainer};
///
/// #[tokio::test]
/// async fn test_with_database() {
///     with_postgres(|db| async move {
///         let url = db.connection_url();
///         // Use database...
///         Ok(())
///     }).await.unwrap();
/// }
/// ```
pub async fn with_postgres<F, Fut>(f: F) -> Result<(), Box<dyn std::error::Error>>
where
	F: FnOnce(PostgresContainer) -> Fut,
	Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
	let container = PostgresContainer::new();
	container.wait_ready().await?;
	f(container).await
}
/// Helper function to run a test with a MySQL container
pub async fn with_mysql<F, Fut>(f: F) -> Result<(), Box<dyn std::error::Error>>
where
	F: FnOnce(MySqlContainer) -> Fut,
	Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
	let container = MySqlContainer::new();
	container.wait_ready().await?;
	f(container).await
}
/// Helper function to run a test with a Redis container
pub async fn with_redis<F, Fut>(f: F) -> Result<(), Box<dyn std::error::Error>>
where
	F: FnOnce(RedisContainer) -> Fut,
	Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
	let container = RedisContainer::new().await;
	f(container).await
}

/// SQLite test helpers
pub mod sqlite {
	/// Get a SQLite in-memory database URL for testing
	///
	/// This returns a connection URL for an in-memory SQLite database,
	/// which is useful for fast tests that don't require a real database container.
	///
	/// # Example
	/// ```ignore
	/// use reinhardt_test::containers::sqlite::memory_url;
	///
	/// let url = memory_url();
	/// assert_eq!(url, "sqlite::memory:");
	/// ```
	pub fn memory_url() -> &'static str {
		"sqlite::memory:"
	}

	/// Get a SQLite temporary file database URL for testing
	///
	/// Creates a temporary file-based SQLite database. The file is automatically
	/// cleaned up when the test completes (if using proper cleanup).
	///
	/// # Example
	/// ```ignore
	/// use reinhardt_test::containers::sqlite::temp_file_url;
	///
	/// let url = temp_file_url("test_db");
	/// // Use the database...
	/// ```
	pub fn temp_file_url(name: &str) -> String {
		format!("sqlite:/tmp/{}.db", name)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_postgres_container() {
		with_postgres(|db| async move {
			let url = db.connection_url();
			assert!(url.starts_with("postgres://"));
			assert_eq!(db.database_type(), "postgres");
			Ok(())
		})
		.await
		.unwrap();
	}

	#[tokio::test]
	async fn test_mysql_container() {
		with_mysql(|db| async move {
			let url = db.connection_url();
			assert!(url.starts_with("mysql://"));
			assert_eq!(db.database_type(), "mysql");
			Ok(())
		})
		.await
		.unwrap();
	}

	#[tokio::test]
	async fn test_redis_container() {
		with_redis(|redis| async move {
			let url = redis.connection_url();
			assert!(url.starts_with("redis://"));
			Ok(())
		})
		.await
		.unwrap();
	}
}
