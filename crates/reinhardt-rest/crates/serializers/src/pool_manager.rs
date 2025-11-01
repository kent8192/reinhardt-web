//! Connection pool management for serializers
//!
//! This module provides connection pool integration for ORM operations
//! in serializers, enabling efficient database access in high-concurrency scenarios.

#[cfg(feature = "django-compat")]
use crate::SerializerError;
#[cfg(feature = "django-compat")]
use reinhardt_pool::{ConnectionPool, PoolConfig};
#[cfg(feature = "django-compat")]
use std::sync::{Arc, RwLock};

/// Global connection pool manager
///
/// Provides a singleton pattern for managing database connection pools
/// across the serializer subsystem.
///
/// # Examples
///
/// ```ignore
/// use reinhardt_serializers::pool_manager::ConnectionPoolManager;
/// use reinhardt_pool::{ConnectionPool, PoolConfig};
///
/// // Initialize the pool
/// let config = PoolConfig::default()
///     .max_connections(10)
///     .acquire_timeout(Duration::from_secs(5));
/// let pool = ConnectionPool::new_postgres("postgresql://...", config).await?;
/// ConnectionPoolManager::set_pool(Arc::new(pool));
///
/// // Acquire connections in serializers
/// let conn = ConnectionPoolManager::acquire().await?;
/// ```
pub struct ConnectionPoolManager {
	#[cfg(feature = "django-compat")]
	pool: Option<Arc<ConnectionPool<sqlx::Postgres>>>,
}

impl std::fmt::Debug for ConnectionPoolManager {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ConnectionPoolManager")
			.field("pool", &"<ConnectionPool>")
			.finish()
	}
}

impl ConnectionPoolManager {
	/// Create a new connection pool manager
	pub fn new() -> Self {
		Self {
			#[cfg(feature = "django-compat")]
			pool: None,
		}
	}

	/// Get the global instance
	#[cfg(feature = "django-compat")]
	fn instance() -> &'static RwLock<ConnectionPoolManager> {
		static INSTANCE: once_cell::sync::Lazy<RwLock<ConnectionPoolManager>> =
			once_cell::sync::Lazy::new(|| RwLock::new(ConnectionPoolManager::new()));
		&INSTANCE
	}

	/// Set the connection pool
	///
	/// # Examples
	///
	/// ```ignore
	/// let pool = ConnectionPool::new_postgres(url, config).await?;
	/// ConnectionPoolManager::set_pool(Arc::new(pool));
	/// ```
	#[cfg(feature = "django-compat")]
	pub fn set_pool(pool: Arc<ConnectionPool<sqlx::Postgres>>) {
		let instance = Self::instance();
		let mut manager = instance.write().expect("Failed to acquire write lock");
		manager.pool = Some(pool);
	}

	/// Get the current connection pool
	#[cfg(feature = "django-compat")]
	pub fn get_pool() -> Option<Arc<ConnectionPool<sqlx::Postgres>>> {
		let instance = Self::instance();
		let manager = instance.read().expect("Failed to acquire read lock");
		manager.pool.clone()
	}

	/// Acquire a connection from the pool
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The pool is not initialized
	/// - Failed to acquire a connection (timeout, pool exhausted, etc.)
	///
	/// # Examples
	///
	/// ```ignore
	/// let conn = ConnectionPoolManager::acquire().await?;
	/// // Use the connection...
	/// // Connection is automatically returned to pool when dropped
	/// ```
	#[cfg(feature = "django-compat")]
	pub async fn acquire()
	-> Result<reinhardt_pool::PooledConnection<sqlx::Postgres>, SerializerError> {
		let pool = Self::get_pool().ok_or_else(|| SerializerError::Other {
			message: "Connection pool not initialized".to_string(),
		})?;

		pool.acquire().await.map_err(|e| SerializerError::Other {
			message: format!("Failed to acquire connection from pool: {}", e),
		})
	}

	/// Check if the pool is initialized
	#[cfg(feature = "django-compat")]
	pub fn is_initialized() -> bool {
		Self::get_pool().is_some()
	}

	/// Clear the connection pool
	///
	/// Useful for testing or reconfiguration scenarios.
	#[cfg(feature = "django-compat")]
	pub fn clear() {
		let instance = Self::instance();
		let mut manager = instance.write().expect("Failed to acquire write lock");
		manager.pool = None;
	}
}

impl Default for ConnectionPoolManager {
	fn default() -> Self {
		Self::new()
	}
}

/// Helper function to create a default pool configuration
///
/// # Examples
///
/// ```ignore
/// let config = default_pool_config();
/// let pool = ConnectionPool::new_postgres(url, config).await?;
/// ```
#[cfg(feature = "django-compat")]
pub fn default_pool_config() -> PoolConfig {
	PoolConfig::default()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	#[cfg(feature = "django-compat")]
	fn test_pool_manager_not_initialized() {
		// Clear any existing pool
		ConnectionPoolManager::clear();

		// Should return false when not initialized
		assert!(!ConnectionPoolManager::is_initialized());
	}

	#[test]
	#[cfg(feature = "django-compat")]
	fn test_default_pool_config() {
		let config = default_pool_config();
		assert_eq!(config.max_connections, 10);
		assert_eq!(config.min_connections, 1);
	}

	#[test]
	fn test_pool_manager_creation() {
		let manager = ConnectionPoolManager::new();
		let _ = manager; // Use the manager to suppress warning
	}

	#[test]
	fn test_pool_manager_default() {
		let manager = ConnectionPoolManager::default();
		let _ = manager;
	}
}
