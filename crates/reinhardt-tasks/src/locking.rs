//! Distributed task locking mechanism
//!
//! This module provides locking primitives for distributed task systems,
//! preventing multiple workers from executing the same task simultaneously.

use crate::{TaskId, TaskResult};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Distributed lock trait for task synchronization
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_tasks::{TaskLock, TaskId};
/// use async_trait::async_trait;
/// use std::time::Duration;
///
/// struct MyLock;
///
/// #[async_trait]
/// impl TaskLock for MyLock {
///     async fn acquire(&self, task_id: TaskId, ttl: Duration) -> reinhardt_tasks::TaskResult<bool> {
///         // Acquire lock implementation
///         Ok(true)
///     }
///
///     async fn release(&self, task_id: TaskId) -> reinhardt_tasks::TaskResult<()> {
///         // Release lock implementation
///         Ok(())
///     }
///
///     async fn is_locked(&self, task_id: TaskId) -> reinhardt_tasks::TaskResult<bool> {
///         // Check lock status
///         Ok(false)
///     }
/// }
/// ```
#[async_trait]
pub trait TaskLock: Send + Sync {
	/// Acquire a lock for a task
	///
	/// Returns `true` if lock was acquired, `false` if already locked by another worker
	async fn acquire(&self, task_id: TaskId, ttl: Duration) -> TaskResult<bool>;

	/// Release a lock for a task
	async fn release(&self, task_id: TaskId) -> TaskResult<()>;

	/// Check if a task is locked
	async fn is_locked(&self, task_id: TaskId) -> TaskResult<bool>;

	/// Extend the TTL of an existing lock
	async fn extend(&self, task_id: TaskId, ttl: Duration) -> TaskResult<bool> {
		if self.is_locked(task_id).await? {
			self.release(task_id).await?;
			self.acquire(task_id, ttl).await
		} else {
			Ok(false)
		}
	}
}

/// In-memory task lock for single-process testing
///
/// # Examples
///
/// ```rust
/// use reinhardt_tasks::{MemoryTaskLock, TaskLock, TaskId};
/// use std::time::Duration;
///
/// # async fn example() -> reinhardt_tasks::TaskResult<()> {
/// let lock = MemoryTaskLock::new();
/// let task_id = TaskId::new();
///
/// // Acquire lock
/// let acquired = lock.acquire(task_id, Duration::from_secs(60)).await?;
/// assert!(acquired);
///
/// // Check if locked
/// let is_locked = lock.is_locked(task_id).await?;
/// assert!(is_locked);
///
/// // Release lock
/// lock.release(task_id).await?;
/// # Ok(())
/// # }
/// ```
pub struct MemoryTaskLock {
	locks: Arc<RwLock<std::collections::HashMap<TaskId, i64>>>,
}

impl MemoryTaskLock {
	/// Create a new in-memory task lock
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::MemoryTaskLock;
	///
	/// let lock = MemoryTaskLock::new();
	/// ```
	pub fn new() -> Self {
		Self {
			locks: Arc::new(RwLock::new(std::collections::HashMap::new())),
		}
	}

	/// Clean up expired locks
	async fn cleanup_expired(&self) {
		let mut locks = self.locks.write().await;
		let now = chrono::Utc::now().timestamp();
		locks.retain(|_, &mut expiry| expiry > now);
	}
}

impl Default for MemoryTaskLock {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl TaskLock for MemoryTaskLock {
	async fn acquire(&self, task_id: TaskId, ttl: Duration) -> TaskResult<bool> {
		self.cleanup_expired().await;

		let mut locks = self.locks.write().await;
		let now = chrono::Utc::now().timestamp();
		let expiry = now + ttl.as_secs() as i64;

		if let Some(&existing_expiry) = locks.get(&task_id)
			&& existing_expiry > now {
				return Ok(false);
			}

		locks.insert(task_id, expiry);
		Ok(true)
	}

	async fn release(&self, task_id: TaskId) -> TaskResult<()> {
		let mut locks = self.locks.write().await;
		locks.remove(&task_id);
		Ok(())
	}

	async fn is_locked(&self, task_id: TaskId) -> TaskResult<bool> {
		self.cleanup_expired().await;

		let locks = self.locks.read().await;
		let now = chrono::Utc::now().timestamp();

		Ok(locks
			.get(&task_id)
			.map(|&expiry| expiry > now)
			.unwrap_or(false))
	}
}

#[cfg(feature = "redis-backend")]
/// Redis-based distributed task lock
///
/// # Examples
///
/// ```no_run
/// use reinhardt_tasks::{RedisTaskLock, TaskLock, TaskId};
/// use std::time::Duration;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let lock = RedisTaskLock::new("redis://127.0.0.1/").await?;
/// let task_id = TaskId::new();
///
/// // Acquire distributed lock
/// let acquired = lock.acquire(task_id, Duration::from_secs(30)).await?;
/// if acquired {
///     // Execute task
///     // ...
///     lock.release(task_id).await?;
/// }
/// # Ok(())
/// # }
/// ```
pub struct RedisTaskLock {
	connection: Arc<redis::aio::ConnectionManager>,
	key_prefix: String,
}

#[cfg(feature = "redis-backend")]
impl RedisTaskLock {
	/// Create a new Redis-based task lock
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_tasks::RedisTaskLock;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let lock = RedisTaskLock::new("redis://localhost/").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
		let client = redis::Client::open(redis_url)?;
		let connection = redis::aio::ConnectionManager::new(client).await?;

		Ok(Self {
			connection: Arc::new(connection),
			key_prefix: "reinhardt:locks:".to_string(),
		})
	}

	/// Create a Redis task lock with custom key prefix
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_tasks::RedisTaskLock;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let lock = RedisTaskLock::with_prefix(
	///     "redis://localhost/",
	///     "myapp:locks:".to_string()
	/// ).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn with_prefix(
		redis_url: &str,
		key_prefix: String,
	) -> Result<Self, redis::RedisError> {
		let client = redis::Client::open(redis_url)?;
		let connection = redis::aio::ConnectionManager::new(client).await?;

		Ok(Self {
			connection: Arc::new(connection),
			key_prefix,
		})
	}

	fn lock_key(&self, task_id: TaskId) -> String {
		format!("{}task:{}", self.key_prefix, task_id)
	}
}

#[cfg(feature = "redis-backend")]
#[async_trait]
impl TaskLock for RedisTaskLock {
	async fn acquire(&self, task_id: TaskId, ttl: Duration) -> TaskResult<bool> {
		use crate::TaskError;
		use redis::AsyncCommands;

		let mut conn = (*self.connection).clone();
		let key = self.lock_key(task_id);

		let result: Result<bool, redis::RedisError> = conn.set_nx(&key, "1").await;

		match result {
			Ok(acquired) => {
				if acquired {
					let _: Result<(), redis::RedisError> =
						conn.expire(&key, ttl.as_secs() as i64).await;
				}
				Ok(acquired)
			}
			Err(e) => Err(TaskError::ExecutionFailed(format!(
				"Failed to acquire lock: {}",
				e
			))),
		}
	}

	async fn release(&self, task_id: TaskId) -> TaskResult<()> {
		use redis::AsyncCommands;

		let mut conn = (*self.connection).clone();
		let key = self.lock_key(task_id);

		let _: Result<(), redis::RedisError> = conn.del(&key).await;
		Ok(())
	}

	async fn is_locked(&self, task_id: TaskId) -> TaskResult<bool> {
		use crate::TaskError;
		use redis::AsyncCommands;

		let mut conn = (*self.connection).clone();
		let key = self.lock_key(task_id);

		let result: Result<bool, redis::RedisError> = conn.exists(&key).await;

		result.map_err(|e| TaskError::ExecutionFailed(format!("Failed to check lock: {}", e)))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::Duration;

	#[tokio::test]
	async fn test_memory_lock_acquire() {
		let lock = MemoryTaskLock::new();
		let task_id = TaskId::new();

		let acquired = lock
			.acquire(task_id, Duration::from_secs(60))
			.await
			.unwrap();
		assert!(acquired);
	}

	#[tokio::test]
	async fn test_memory_lock_already_locked() {
		let lock = MemoryTaskLock::new();
		let task_id = TaskId::new();

		lock.acquire(task_id, Duration::from_secs(60))
			.await
			.unwrap();
		let acquired = lock
			.acquire(task_id, Duration::from_secs(60))
			.await
			.unwrap();
		assert!(!acquired);
	}

	#[tokio::test]
	async fn test_memory_lock_release() {
		let lock = MemoryTaskLock::new();
		let task_id = TaskId::new();

		lock.acquire(task_id, Duration::from_secs(60))
			.await
			.unwrap();
		lock.release(task_id).await.unwrap();

		let is_locked = lock.is_locked(task_id).await.unwrap();
		assert!(!is_locked);
	}

	#[tokio::test]
	async fn test_memory_lock_expiry() {
		let lock = MemoryTaskLock::new();
		let task_id = TaskId::new();

		lock.acquire(task_id, Duration::from_millis(100))
			.await
			.unwrap();

		tokio::time::sleep(Duration::from_millis(200)).await;

		let is_locked = lock.is_locked(task_id).await.unwrap();
		assert!(!is_locked);
	}

	#[tokio::test]
	async fn test_memory_lock_extend() {
		let lock = MemoryTaskLock::new();
		let task_id = TaskId::new();

		lock.acquire(task_id, Duration::from_secs(60))
			.await
			.unwrap();

		let extended = lock
			.extend(task_id, Duration::from_secs(120))
			.await
			.unwrap();
		assert!(extended);

		let is_locked = lock.is_locked(task_id).await.unwrap();
		assert!(is_locked);
	}

	#[cfg(feature = "redis-backend")]
	mod redis_tests {
		use super::*;
		use serial_test::serial;
		use testcontainers::{
			GenericImage,
			core::{ContainerPort, WaitFor},
			runners::AsyncRunner,
		};

		async fn setup_redis() -> testcontainers::ContainerAsync<GenericImage> {
			let redis_image = GenericImage::new("redis", "7-alpine")
				.with_exposed_port(ContainerPort::Tcp(6379))
				.with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"));

			redis_image
				.start()
				.await
				.expect("Failed to start Redis container")
		}

		#[tokio::test]
		#[serial(redis_lock)]
		async fn test_redis_lock_acquire() {
			let container = setup_redis().await;
			let port = container
				.get_host_port_ipv4(6379)
				.await
				.expect("Failed to get port");
			let redis_url = format!("redis://127.0.0.1:{}/", port);

			let lock = RedisTaskLock::new(&redis_url)
				.await
				.expect("Failed to connect to Redis");
			let task_id = TaskId::new();

			let acquired = lock
				.acquire(task_id, Duration::from_secs(60))
				.await
				.unwrap();
			assert!(acquired);
		}

		#[tokio::test]
		#[serial(redis_lock)]
		async fn test_redis_lock_already_locked() {
			let container = setup_redis().await;
			let port = container
				.get_host_port_ipv4(6379)
				.await
				.expect("Failed to get port");
			let redis_url = format!("redis://127.0.0.1:{}/", port);

			let lock = RedisTaskLock::new(&redis_url)
				.await
				.expect("Failed to connect to Redis");
			let task_id = TaskId::new();

			lock.acquire(task_id, Duration::from_secs(60))
				.await
				.unwrap();
			let acquired = lock
				.acquire(task_id, Duration::from_secs(60))
				.await
				.unwrap();
			assert!(!acquired);
		}

		#[tokio::test]
		#[serial(redis_lock)]
		async fn test_redis_lock_release() {
			let container = setup_redis().await;
			let port = container
				.get_host_port_ipv4(6379)
				.await
				.expect("Failed to get port");
			let redis_url = format!("redis://127.0.0.1:{}/", port);

			let lock = RedisTaskLock::new(&redis_url)
				.await
				.expect("Failed to connect to Redis");
			let task_id = TaskId::new();

			lock.acquire(task_id, Duration::from_secs(60))
				.await
				.unwrap();
			lock.release(task_id).await.unwrap();

			let is_locked = lock.is_locked(task_id).await.unwrap();
			assert!(!is_locked);
		}
	}
}
