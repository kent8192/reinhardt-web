//! Connection pool implementation

use super::config::PoolConfig;
use super::errors::{PoolError, PoolResult};
use super::events::{PoolEvent, PoolEventListener};
use sqlx::{Database, MySql, Pool, Postgres, Sqlite};
use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

fn generate_connection_id() -> String {
	uuid::Uuid::now_v7().to_string()
}

struct PoolEventHub {
	listeners: RwLock<Vec<Arc<dyn PoolEventListener>>>,
	listener_count: AtomicUsize,
}

impl PoolEventHub {
	fn new() -> Self {
		Self {
			listeners: RwLock::new(Vec::new()),
			listener_count: AtomicUsize::new(0),
		}
	}

	fn has_listeners(&self) -> bool {
		self.listener_count.load(Ordering::Acquire) > 0
	}

	async fn add_listener(&self, listener: Arc<dyn PoolEventListener>) {
		let mut listeners = self.listeners.write().await;
		listeners.push(listener);
		self.listener_count
			.store(listeners.len(), Ordering::Release);
	}

	async fn emit_event(&self, event: PoolEvent) {
		if !self.has_listeners() {
			return;
		}

		let listeners = self.listeners.read().await;
		for listener in listeners.iter() {
			listener.on_event(event.clone()).await;
		}
	}
}

/// A database connection pool
pub struct ConnectionPool<DB: Database> {
	pool: Pool<DB>,
	config: PoolConfig,
	url: String,
	events: Arc<PoolEventHub>,
	first_connect_fired: Arc<AtomicBool>,
}

impl ConnectionPool<Postgres> {
	/// Create a new PostgreSQL connection pool
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::pool::{ConnectionPool, PoolConfig};
	///
	/// # async fn example() {
	/// let config = PoolConfig::default();
	/// // For doctest purposes, using SQLite in-memory instead of PostgreSQL
	/// let pool = ConnectionPool::new_sqlite("sqlite::memory:", config).await.unwrap();
	/// assert!(pool.url().contains("memory"));
	/// assert_eq!(pool.config().max_connections, 10);
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn new_postgres(url: &str, config: PoolConfig) -> PoolResult<Self> {
		config.validate().map_err(PoolError::Config)?;

		let pool = sqlx::postgres::PgPoolOptions::new()
			.min_connections(config.min_connections)
			.max_connections(config.max_connections)
			.acquire_timeout(config.acquire_timeout)
			.idle_timeout(config.idle_timeout)
			.max_lifetime(config.max_lifetime)
			.test_before_acquire(config.test_before_acquire)
			.connect(url)
			.await?;

		Ok(Self {
			pool,
			config,
			url: url.to_string(),
			events: Arc::new(PoolEventHub::new()),
			first_connect_fired: Arc::new(AtomicBool::new(false)),
		})
	}
}

impl ConnectionPool<MySql> {
	/// Create a new MySQL connection pool
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::pool::{ConnectionPool, PoolConfig};
	///
	/// # async fn example() {
	/// let config = PoolConfig::default();
	/// // For doctest purposes, using SQLite in-memory instead of MySQL
	/// let pool = ConnectionPool::new_sqlite("sqlite::memory:", config).await.unwrap();
	/// assert!(pool.url().contains("memory"));
	/// assert_eq!(pool.config().max_connections, 10);
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn new_mysql(url: &str, config: PoolConfig) -> PoolResult<Self> {
		config.validate().map_err(PoolError::Config)?;

		let pool = sqlx::mysql::MySqlPoolOptions::new()
			.min_connections(config.min_connections)
			.max_connections(config.max_connections)
			.acquire_timeout(config.acquire_timeout)
			.idle_timeout(config.idle_timeout)
			.max_lifetime(config.max_lifetime)
			.test_before_acquire(config.test_before_acquire)
			.connect(url)
			.await?;

		Ok(Self {
			pool,
			config,
			url: url.to_string(),
			events: Arc::new(PoolEventHub::new()),
			first_connect_fired: Arc::new(AtomicBool::new(false)),
		})
	}
}

impl ConnectionPool<Sqlite> {
	/// Create a new SQLite connection pool
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::pool::{ConnectionPool, PoolConfig};
	///
	/// # async fn example() {
	/// let config = PoolConfig::default();
	/// // Using in-memory SQLite for doctest
	/// let pool = ConnectionPool::new_sqlite("sqlite::memory:", config).await.unwrap();
	/// assert!(pool.url().contains("memory"));
	/// assert_eq!(pool.config().max_connections, 10);
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn new_sqlite(url: &str, config: PoolConfig) -> PoolResult<Self> {
		config.validate().map_err(PoolError::Config)?;

		let pool = sqlx::sqlite::SqlitePoolOptions::new()
			.min_connections(config.min_connections)
			.max_connections(config.max_connections)
			.acquire_timeout(config.acquire_timeout)
			.idle_timeout(config.idle_timeout)
			.max_lifetime(config.max_lifetime)
			.test_before_acquire(config.test_before_acquire)
			.connect(url)
			.await?;

		Ok(Self {
			pool,
			config,
			url: url.to_string(),
			events: Arc::new(PoolEventHub::new()),
			first_connect_fired: Arc::new(AtomicBool::new(false)),
		})
	}
}

impl<DB> ConnectionPool<DB>
where
	DB: sqlx::Database,
{
	/// Add an event listener
	///
	pub async fn add_listener(&self, listener: Arc<dyn PoolEventListener>) {
		self.events.add_listener(listener).await;
	}

	/// Emit an event to all listeners
	pub(crate) async fn emit_event(&self, event: PoolEvent) {
		self.events.emit_event(event).await;
	}
	/// Acquire a connection from the pool with event emission
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::pool::{ConnectionPool, PoolConfig};
	///
	/// # async fn example() {
	/// let config = PoolConfig::default();
	/// // For doctest purposes, using SQLite in-memory instead of PostgreSQL
	/// let pool = ConnectionPool::new_sqlite("sqlite::memory:", config)
	///     .await
	///     .unwrap();
	///
	/// // Acquire a connection
	/// let conn = pool.acquire().await.unwrap();
	/// let id = conn.connection_id();
	/// assert!(!id.is_empty());
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn acquire(&self) -> PoolResult<PooledConnection<DB>> {
		// Check if this is the first connection
		let is_first = !self.first_connect_fired.swap(true, Ordering::SeqCst);

		let conn = self.pool.acquire().await?;
		let connection_id = OnceLock::new();

		if self.events.has_listeners() {
			let id = connection_id.get_or_init(generate_connection_id).clone();

			if is_first {
				// Emit first_connect event (using ConnectionCreated as proxy)
				self.emit_event(PoolEvent::connection_created(id.clone()))
					.await;
			}

			// Emit checkout event
			self.emit_event(PoolEvent::connection_acquired(id)).await;
		}

		Ok(PooledConnection {
			conn: ManuallyDrop::new(conn),
			events: self.events.clone(),
			connection_id,
		})
	}
	/// Get the underlying pool
	///
	pub fn inner(&self) -> &Pool<DB> {
		&self.pool
	}
	/// Get pool configuration
	///
	pub fn config(&self) -> &PoolConfig {
		&self.config
	}
	/// Close the pool
	///
	pub async fn close(&self) {
		self.pool.close().await;
	}
	/// Get the database URL with password masked for safe display
	///
	/// Returns the database URL with any password replaced by `***`
	/// to prevent credential exposure in logs and debug output.
	/// Use `url_raw()` when the actual password is needed for reconnection.
	pub fn url(&self) -> String {
		crate::pool::pool::mask_url_password(&self.url)
	}

	/// Get the raw database URL including credentials
	///
	/// This method returns the unmasked URL containing the actual password.
	/// Use with caution - prefer `url()` for logging and display purposes.
	// Allow dead_code: preserved for internal use by reconnection logic (e.g., `recreate()`)
	#[allow(dead_code)]
	pub(crate) fn url_raw(&self) -> &str {
		&self.url
	}
}

// Database-specific recreate implementations
impl ConnectionPool<Postgres> {
	/// Recreate the pool with the same configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::pool::{ConnectionPool, PoolConfig};
	///
	/// # async fn example() {
	/// let config = PoolConfig::default();
	/// // For doctest purposes, using SQLite in-memory instead of PostgreSQL
	/// let mut pool = ConnectionPool::new_sqlite("sqlite::memory:", config)
	///     .await
	///     .unwrap();
	///
	/// // Recreate the pool
	/// pool.recreate().await.unwrap();
	/// assert_eq!(pool.config().max_connections, 10);
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn recreate(&mut self) -> PoolResult<()> {
		// Close existing pool
		self.pool.close().await;

		// Create new pool with same configuration
		let new_pool = sqlx::postgres::PgPoolOptions::new()
			.min_connections(self.config.min_connections)
			.max_connections(self.config.max_connections)
			.acquire_timeout(self.config.acquire_timeout)
			.idle_timeout(self.config.idle_timeout)
			.max_lifetime(self.config.max_lifetime)
			.test_before_acquire(self.config.test_before_acquire)
			.connect(&self.url)
			.await?;

		self.pool = new_pool;
		self.first_connect_fired.store(false, Ordering::SeqCst);

		Ok(())
	}
}

impl ConnectionPool<MySql> {
	/// Recreate the pool with the same configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::pool::{ConnectionPool, PoolConfig};
	///
	/// # async fn example() {
	/// let config = PoolConfig::default();
	/// // For doctest purposes, using SQLite in-memory instead of MySQL
	/// let mut pool = ConnectionPool::new_sqlite("sqlite::memory:", config)
	///     .await
	///     .unwrap();
	///
	/// // Recreate the pool
	/// pool.recreate().await.unwrap();
	/// assert_eq!(pool.config().max_connections, 10);
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn recreate(&mut self) -> PoolResult<()> {
		// Close existing pool
		self.pool.close().await;

		// Create new pool with same configuration
		let new_pool = sqlx::mysql::MySqlPoolOptions::new()
			.min_connections(self.config.min_connections)
			.max_connections(self.config.max_connections)
			.acquire_timeout(self.config.acquire_timeout)
			.idle_timeout(self.config.idle_timeout)
			.max_lifetime(self.config.max_lifetime)
			.test_before_acquire(self.config.test_before_acquire)
			.connect(&self.url)
			.await?;

		self.pool = new_pool;
		self.first_connect_fired.store(false, Ordering::SeqCst);

		Ok(())
	}
}

impl ConnectionPool<Sqlite> {
	/// Recreate the pool with the same configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::pool::{ConnectionPool, PoolConfig};
	///
	/// # async fn example() {
	/// let config = PoolConfig::default();
	/// let mut pool = ConnectionPool::new_sqlite("sqlite::memory:", config)
	///     .await
	///     .unwrap();
	///
	/// // Recreate the pool
	/// pool.recreate().await.unwrap();
	/// assert_eq!(pool.config().max_connections, 10);
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn recreate(&mut self) -> PoolResult<()> {
		// Close existing pool
		self.pool.close().await;

		// Create new pool with same configuration
		let new_pool = sqlx::sqlite::SqlitePoolOptions::new()
			.min_connections(self.config.min_connections)
			.max_connections(self.config.max_connections)
			.acquire_timeout(self.config.acquire_timeout)
			.idle_timeout(self.config.idle_timeout)
			.max_lifetime(self.config.max_lifetime)
			.test_before_acquire(self.config.test_before_acquire)
			.connect(&self.url)
			.await?;

		self.pool = new_pool;
		self.first_connect_fired.store(false, Ordering::SeqCst);

		Ok(())
	}
}

/// A pooled connection wrapper with event emission
pub struct PooledConnection<DB: sqlx::Database> {
	// Wrapped in ManuallyDrop so we can take ownership in Drop.
	// When no tokio runtime is available, we detach the connection
	// to avoid sqlx's PoolConnection::Drop calling rt::spawn().
	conn: ManuallyDrop<sqlx::pool::PoolConnection<DB>>,
	events: Arc<PoolEventHub>,
	connection_id: OnceLock<String>,
}

impl<DB: sqlx::Database> PooledConnection<DB> {
	/// Documentation for `inner`
	///
	pub fn inner(&mut self) -> &mut sqlx::pool::PoolConnection<DB> {
		&mut self.conn
	}
	/// Get the unique identifier for this connection
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::pool::{ConnectionPool, PoolConfig};
	///
	/// # async fn example() {
	/// let config = PoolConfig::default();
	/// // For doctest purposes, using SQLite in-memory instead of PostgreSQL
	/// let pool = ConnectionPool::new_sqlite("sqlite::memory:", config)
	///     .await
	///     .unwrap();
	///
	/// let mut conn = pool.acquire().await.unwrap();
	/// let id = conn.connection_id();
	/// assert!(!id.is_empty());
	/// assert!(id.len() > 10);
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub fn connection_id(&self) -> &str {
		self.connection_id
			.get_or_init(generate_connection_id)
			.as_str()
	}
	/// Invalidate this connection (hard invalidation - connection is unusable)
	///
	pub async fn invalidate(self, reason: String) {
		if self.events.has_listeners() {
			let connection_id = self
				.connection_id
				.get_or_init(generate_connection_id)
				.clone();
			self.events
				.emit_event(PoolEvent::connection_invalidated(connection_id, reason))
				.await;
		}
		// Connection will be dropped and not returned to pool
	}
	/// Soft invalidate this connection (can complete current operation)
	///
	pub async fn soft_invalidate(&mut self) {
		if self.events.has_listeners() {
			let connection_id = self
				.connection_id
				.get_or_init(generate_connection_id)
				.clone();
			self.events
				.emit_event(PoolEvent::connection_soft_invalidated(connection_id))
				.await;
		}
	}
	/// Reset this connection
	///
	pub async fn reset(&mut self) {
		if self.events.has_listeners() {
			let connection_id = self
				.connection_id
				.get_or_init(generate_connection_id)
				.clone();
			self.events
				.emit_event(PoolEvent::connection_reset(connection_id))
				.await;
		}
	}
}

impl<DB: sqlx::Database> Drop for PooledConnection<DB> {
	fn drop(&mut self) {
		// SAFETY: ManuallyDrop::take is called exactly once (in drop).
		let conn = unsafe { ManuallyDrop::take(&mut self.conn) };

		match tokio::runtime::Handle::try_current() {
			Ok(handle) => {
				// Runtime available: drop the connection normally (returns to pool)
				// and emit the connection-returned event.
				drop(conn);

				if self.events.has_listeners() {
					let events = self.events.clone();
					let connection_id = self
						.connection_id
						.get_or_init(generate_connection_id)
						.clone();

					handle.spawn(async move {
						events
							.emit_event(PoolEvent::connection_returned(connection_id))
							.await;
					});
				}
			}
			Err(_) => {
				// No runtime available: prevent sqlx's PoolConnection::Drop
				// from running, as it calls crate::rt::spawn() which panics
				// without a tokio runtime. The connection is intentionally
				// leaked to avoid the panic.
				std::mem::forget(conn);
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;
	use std::sync::Mutex;

	struct RecordingListener {
		events: Arc<Mutex<Vec<&'static str>>>,
	}

	#[async_trait::async_trait]
	impl PoolEventListener for RecordingListener {
		async fn on_event(&self, event: PoolEvent) {
			let name = match event {
				PoolEvent::ConnectionAcquired { .. } => "acquired",
				PoolEvent::ConnectionReturned { .. } => "returned",
				PoolEvent::ConnectionCreated { .. } => "created",
				PoolEvent::ConnectionClosed { .. } => "closed",
				PoolEvent::ConnectionTestFailed { .. } => "test_failed",
				PoolEvent::ConnectionInvalidated { .. } => "invalidated",
				PoolEvent::ConnectionSoftInvalidated { .. } => "soft_invalidated",
				PoolEvent::ConnectionReset { .. } => "reset",
			};
			self.events
				.lock()
				.expect("events mutex should not be poisoned")
				.push(name);
		}
	}

	#[rstest]
	fn handle_try_current_returns_err_outside_runtime() {
		// Arrange
		// Run on a fresh thread to avoid inheriting any runtime context
		// from the test runner's worker thread.

		// Act & Assert
		let handle = std::thread::spawn(|| {
			let result = tokio::runtime::Handle::try_current();
			assert!(
				result.is_err(),
				"Handle::try_current() should return Err outside a runtime"
			);
		});
		handle.join().expect("thread should not panic");
	}

	#[rstest]
	#[tokio::test]
	async fn handle_try_current_returns_ok_inside_runtime() {
		// Arrange & Act
		let result = tokio::runtime::Handle::try_current();

		// Assert
		assert!(
			result.is_ok(),
			"Handle::try_current() should return Ok inside a runtime"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn drop_pooled_connection_inside_runtime_does_not_panic() {
		// Arrange
		let config = PoolConfig::default();
		let pool = ConnectionPool::new_sqlite("sqlite::memory:", config)
			.await
			.unwrap();

		// Act
		let conn = pool.acquire().await.unwrap();

		// Assert
		// Dropping within an active runtime should work without panic
		drop(conn);
	}

	#[rstest]
	fn drop_pooled_connection_outside_runtime_does_not_panic() {
		// Arrange
		// Create a Tokio runtime and acquire a pooled connection inside it.
		let rt = tokio::runtime::Runtime::new().expect("failed to create Tokio runtime");

		let (pool, conn) = rt.block_on(async {
			let config = PoolConfig::default();
			let pool = ConnectionPool::new_sqlite("sqlite::memory:", config)
				.await
				.expect("failed to create ConnectionPool");

			let conn = pool.acquire().await.expect("failed to acquire connection");

			(pool, conn)
		});

		// Drop the runtime so there is no active Tokio runtime.
		drop(rt);

		// Act & Assert
		// Dropping the connection outside any runtime should not panic.
		drop(conn);

		// Also drop the pool to ensure cleanup does not panic outside a runtime.
		drop(pool);
	}

	#[tokio::test]
	async fn pool_events_are_emitted_when_listener_registered() {
		// Arrange
		let events = Arc::new(Mutex::new(Vec::new()));
		let listener = Arc::new(RecordingListener {
			events: events.clone(),
		});
		let pool = ConnectionPool::new_sqlite("sqlite::memory:", PoolConfig::default())
			.await
			.expect("failed to create ConnectionPool");
		pool.add_listener(listener).await;

		// Act
		let conn = pool.acquire().await.expect("failed to acquire connection");
		drop(conn);
		tokio::task::yield_now().await;

		// Assert
		let recorded = events.lock().expect("events mutex should not be poisoned");
		assert_eq!(
			recorded.as_slice(),
			["created", "acquired", "returned"],
			"pool listener should observe the first acquire and return events"
		);
	}
}
