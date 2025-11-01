//! Redis-based task backend implementation

use crate::{
	Task, TaskExecutionError, TaskId, TaskStatus,
	result::{ResultBackend, TaskResultMetadata},
};
use async_trait::async_trait;
use redis::{AsyncCommands, RedisError, aio::ConnectionManager};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Task metadata for Redis storage
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskMetadata {
	id: TaskId,
	name: String,
	status: TaskStatus,
	created_at: i64,
	updated_at: i64,
}

/// Redis-based task backend
///
/// Stores tasks in Redis with status tracking.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_tasks::RedisBackend;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let backend = RedisBackend::new("redis://127.0.0.1/").await?;
/// # Ok(())
/// # }
/// ```
pub struct RedisBackend {
	connection: Arc<ConnectionManager>,
	key_prefix: String,
}

impl RedisBackend {
	/// Create a new Redis backend
	///
	/// # Arguments
	///
	/// * `redis_url` - Redis connection URL (e.g., "redis://127.0.0.1/")
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_tasks::RedisBackend;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let backend = RedisBackend::new("redis://localhost/").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn new(redis_url: &str) -> Result<Self, RedisError> {
		let client = redis::Client::open(redis_url)?;
		let connection = ConnectionManager::new(client).await?;

		Ok(Self {
			connection: Arc::new(connection),
			key_prefix: "reinhardt:tasks:".to_string(),
		})
	}

	/// Create a new Redis backend with custom key prefix
	///
	/// # Arguments
	///
	/// * `redis_url` - Redis connection URL
	/// * `key_prefix` - Custom key prefix for Redis keys
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_tasks::RedisBackend;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let backend = RedisBackend::with_prefix(
	///     "redis://localhost/",
	///     "myapp:tasks:".to_string()
	/// ).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn with_prefix(redis_url: &str, key_prefix: String) -> Result<Self, RedisError> {
		let client = redis::Client::open(redis_url)?;
		let connection = ConnectionManager::new(client).await?;

		Ok(Self {
			connection: Arc::new(connection),
			key_prefix,
		})
	}

	/// Get Redis key for a task
	fn task_key(&self, task_id: TaskId) -> String {
		format!("{}task:{}", self.key_prefix, task_id)
	}

	/// Get Redis key for task queue
	fn queue_key(&self) -> String {
		format!("{}queue", self.key_prefix)
	}
}

#[async_trait]
impl crate::backend::TaskBackend for RedisBackend {
	async fn enqueue(&self, task: Box<dyn Task>) -> Result<TaskId, TaskExecutionError> {
		let task_id = task.id();
		let task_name = task.name().to_string();

		let metadata = TaskMetadata {
			id: task_id,
			name: task_name,
			status: TaskStatus::Pending,
			created_at: chrono::Utc::now().timestamp(),
			updated_at: chrono::Utc::now().timestamp(),
		};

		let metadata_json = serde_json::to_string(&metadata)
			.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		let mut conn = (*self.connection).clone();

		// Store task metadata
		let _: () = conn
			.set(self.task_key(task_id), metadata_json)
			.await
			.map_err(|e: RedisError| TaskExecutionError::BackendError(e.to_string()))?;

		// Add to queue
		let _: () = conn
			.rpush(self.queue_key(), task_id.to_string())
			.await
			.map_err(|e: RedisError| TaskExecutionError::BackendError(e.to_string()))?;

		Ok(task_id)
	}

	async fn dequeue(&self) -> Result<Option<TaskId>, TaskExecutionError> {
		let mut conn = (*self.connection).clone();

		let task_id_str: Option<String> = conn
			.lpop(self.queue_key(), None)
			.await
			.map_err(|e: RedisError| TaskExecutionError::BackendError(e.to_string()))?;

		match task_id_str {
			Some(id_str) => {
				let task_id = id_str
					.parse()
					.map_err(|e: uuid::Error| TaskExecutionError::BackendError(e.to_string()))?;
				Ok(Some(task_id))
			}
			None => Ok(None),
		}
	}

	async fn get_status(&self, task_id: TaskId) -> Result<TaskStatus, TaskExecutionError> {
		let mut conn = (*self.connection).clone();

		let metadata_json: Option<String> = conn
			.get(self.task_key(task_id))
			.await
			.map_err(|e: RedisError| TaskExecutionError::BackendError(e.to_string()))?;

		match metadata_json {
			Some(json) => {
				let metadata: TaskMetadata = serde_json::from_str(&json)
					.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;
				Ok(metadata.status)
			}
			None => Err(TaskExecutionError::NotFound(task_id)),
		}
	}

	async fn update_status(
		&self,
		task_id: TaskId,
		status: TaskStatus,
	) -> Result<(), TaskExecutionError> {
		let mut conn = (*self.connection).clone();

		let metadata_json: Option<String> = conn
			.get(self.task_key(task_id))
			.await
			.map_err(|e: RedisError| TaskExecutionError::BackendError(e.to_string()))?;

		match metadata_json {
			Some(json) => {
				let mut metadata: TaskMetadata = serde_json::from_str(&json)
					.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

				metadata.status = status;
				metadata.updated_at = chrono::Utc::now().timestamp();

				let updated_json = serde_json::to_string(&metadata)
					.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

				let _: () = conn
					.set(self.task_key(task_id), updated_json)
					.await
					.map_err(|e: RedisError| TaskExecutionError::BackendError(e.to_string()))?;

				Ok(())
			}
			None => Err(TaskExecutionError::NotFound(task_id)),
		}
	}

	async fn get_task_data(
		&self,
		task_id: TaskId,
	) -> Result<Option<crate::registry::SerializedTask>, TaskExecutionError> {
		let mut conn = (*self.connection).clone();

		let metadata_json: Option<String> = conn
			.get(self.task_key(task_id))
			.await
			.map_err(|e: RedisError| TaskExecutionError::BackendError(e.to_string()))?;

		match metadata_json {
			Some(json) => {
				let metadata: TaskMetadata = serde_json::from_str(&json)
					.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

				// Return a placeholder serialized task
				// In production, task data should be stored separately
				Ok(Some(crate::registry::SerializedTask::new(
					metadata.name,
					"{}".to_string(),
				)))
			}
			None => Ok(None),
		}
	}

	fn backend_name(&self) -> &str {
		"redis"
	}
}

/// Redis-based result backend for task result persistence
///
/// # Examples
///
/// ```no_run
/// use reinhardt_tasks::{RedisResultBackend, ResultBackend, TaskResultMetadata, TaskId, TaskStatus};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let backend = RedisResultBackend::new("redis://127.0.0.1/").await?;
///
/// let metadata = TaskResultMetadata::new(
///     TaskId::new(),
///     TaskStatus::Success,
///     Some("Task completed".to_string()),
/// );
///
/// backend.store_result(metadata).await?;
/// # Ok(())
/// # }
/// ```
pub struct RedisResultBackend {
	connection: Arc<ConnectionManager>,
	key_prefix: String,
	default_ttl: i64,
}

impl RedisResultBackend {
	/// Create a new Redis result backend
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_tasks::RedisResultBackend;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let backend = RedisResultBackend::new("redis://localhost/").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn new(redis_url: &str) -> Result<Self, RedisError> {
		let client = redis::Client::open(redis_url)?;
		let connection = ConnectionManager::new(client).await?;

		Ok(Self {
			connection: Arc::new(connection),
			key_prefix: "reinhardt:results:".to_string(),
			default_ttl: 86400, // 24 hours
		})
	}

	/// Create a new Redis result backend with custom settings
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_tasks::RedisResultBackend;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let backend = RedisResultBackend::with_config(
	///     "redis://localhost/",
	///     "myapp:results:".to_string(),
	///     3600, // 1 hour TTL
	/// ).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn with_config(
		redis_url: &str,
		key_prefix: String,
		default_ttl: i64,
	) -> Result<Self, RedisError> {
		let client = redis::Client::open(redis_url)?;
		let connection = ConnectionManager::new(client).await?;

		Ok(Self {
			connection: Arc::new(connection),
			key_prefix,
			default_ttl,
		})
	}

	fn result_key(&self, task_id: TaskId) -> String {
		format!("{}task:{}", self.key_prefix, task_id)
	}
}

#[async_trait]
impl ResultBackend for RedisResultBackend {
	async fn store_result(&self, metadata: TaskResultMetadata) -> Result<(), TaskExecutionError> {
		let task_id = metadata.task_id();
		let metadata_json = serde_json::to_string(&metadata)
			.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;

		let mut conn = (*self.connection).clone();
		let key = self.result_key(task_id);

		let _: () = conn
			.set_ex(&key, metadata_json, self.default_ttl as u64)
			.await
			.map_err(|e: RedisError| TaskExecutionError::BackendError(e.to_string()))?;

		Ok(())
	}

	async fn get_result(
		&self,
		task_id: TaskId,
	) -> Result<Option<TaskResultMetadata>, TaskExecutionError> {
		let mut conn = (*self.connection).clone();
		let key = self.result_key(task_id);

		let metadata_json: Option<String> = conn
			.get(&key)
			.await
			.map_err(|e: RedisError| TaskExecutionError::BackendError(e.to_string()))?;

		match metadata_json {
			Some(json) => {
				let metadata: TaskResultMetadata = serde_json::from_str(&json)
					.map_err(|e| TaskExecutionError::BackendError(e.to_string()))?;
				Ok(Some(metadata))
			}
			None => Ok(None),
		}
	}

	async fn delete_result(&self, task_id: TaskId) -> Result<(), TaskExecutionError> {
		let mut conn = (*self.connection).clone();
		let key = self.result_key(task_id);

		let _: () = conn
			.del(&key)
			.await
			.map_err(|e: RedisError| TaskExecutionError::BackendError(e.to_string()))?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::backend::TaskBackend;
	use crate::{Task, TaskId, TaskPriority};
	use serial_test::serial;
	use testcontainers::{
		GenericImage,
		core::{ContainerPort, WaitFor},
		runners::AsyncRunner,
	};

	struct TestTask {
		id: TaskId,
		name: String,
	}

	impl Task for TestTask {
		fn id(&self) -> TaskId {
			self.id
		}

		fn name(&self) -> &str {
			&self.name
		}

		fn priority(&self) -> TaskPriority {
			TaskPriority::new(5)
		}
	}

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
	#[serial(redis)]
	async fn test_redis_backend_enqueue() {
		let container = setup_redis().await;
		let port = container
			.get_host_port_ipv4(6379)
			.await
			.expect("Failed to get port");
		let redis_url = format!("redis://127.0.0.1:{}/", port);

		let backend = RedisBackend::new(&redis_url)
			.await
			.expect("Failed to connect to Redis");

		let task = Box::new(TestTask {
			id: TaskId::new(),
			name: "test_task".to_string(),
		});

		let task_id = task.id();
		let result = backend.enqueue(task).await;
		assert!(result.is_ok());
		assert_eq!(result.unwrap(), task_id);
	}

	#[tokio::test]
	#[serial(redis)]
	async fn test_redis_backend_get_status() {
		let container = setup_redis().await;
		let port = container
			.get_host_port_ipv4(6379)
			.await
			.expect("Failed to get port");
		let redis_url = format!("redis://127.0.0.1:{}/", port);

		let backend = RedisBackend::new(&redis_url)
			.await
			.expect("Failed to connect to Redis");

		let task = Box::new(TestTask {
			id: TaskId::new(),
			name: "test_task".to_string(),
		});

		let task_id = task.id();
		backend.enqueue(task).await.expect("Failed to enqueue");

		let status = backend
			.get_status(task_id)
			.await
			.expect("Failed to get status");
		assert_eq!(status, TaskStatus::Pending);
	}

	#[tokio::test]
	#[serial(redis)]
	async fn test_redis_backend_not_found() {
		let container = setup_redis().await;
		let port = container
			.get_host_port_ipv4(6379)
			.await
			.expect("Failed to get port");
		let redis_url = format!("redis://127.0.0.1:{}/", port);

		let backend = RedisBackend::new(&redis_url)
			.await
			.expect("Failed to connect to Redis");

		let result = backend.get_status(TaskId::new()).await;
		assert!(result.is_err());
		assert!(matches!(result, Err(TaskExecutionError::NotFound(_))));
	}

	#[tokio::test]
	#[serial(redis)]
	async fn test_redis_backend_dequeue() {
		let container = setup_redis().await;
		let port = container
			.get_host_port_ipv4(6379)
			.await
			.expect("Failed to get port");
		let redis_url = format!("redis://127.0.0.1:{}/", port);

		let backend = RedisBackend::new(&redis_url)
			.await
			.expect("Failed to connect to Redis");

		// Enqueue a task first
		let task = Box::new(TestTask {
			id: TaskId::new(),
			name: "dequeue_test".to_string(),
		});
		let task_id = task.id();
		backend.enqueue(task).await.expect("Failed to enqueue");

		// Dequeue the task
		let dequeued = backend.dequeue().await.expect("Failed to dequeue");
		assert_eq!(dequeued, Some(task_id));

		// Second dequeue should return None
		let empty = backend.dequeue().await.expect("Failed to dequeue");
		assert_eq!(empty, None);
	}

	#[tokio::test]
	#[serial(redis)]
	async fn test_redis_backend_update_status() {
		let container = setup_redis().await;
		let port = container
			.get_host_port_ipv4(6379)
			.await
			.expect("Failed to get port");
		let redis_url = format!("redis://127.0.0.1:{}/", port);

		let backend = RedisBackend::new(&redis_url)
			.await
			.expect("Failed to connect to Redis");

		// Enqueue a task
		let task = Box::new(TestTask {
			id: TaskId::new(),
			name: "status_test".to_string(),
		});
		let task_id = task.id();
		backend.enqueue(task).await.expect("Failed to enqueue");

		// Update status
		backend
			.update_status(task_id, TaskStatus::Success)
			.await
			.expect("Failed to update status");

		// Verify status
		let status = backend
			.get_status(task_id)
			.await
			.expect("Failed to get status");
		assert_eq!(status, TaskStatus::Success);
	}

	#[tokio::test]
	#[serial(redis)]
	async fn test_redis_result_backend_store_and_retrieve() {
		let container = setup_redis().await;
		let port = container
			.get_host_port_ipv4(6379)
			.await
			.expect("Failed to get port");
		let redis_url = format!("redis://127.0.0.1:{}/", port);

		let backend = RedisResultBackend::new(&redis_url)
			.await
			.expect("Failed to connect to Redis");

		let task_id = TaskId::new();
		let metadata = crate::result::TaskResultMetadata::new(
			task_id,
			TaskStatus::Success,
			Some("Test result".to_string()),
		);

		// Store result
		backend
			.store_result(metadata.clone())
			.await
			.expect("Failed to store result");

		// Retrieve result
		let retrieved = backend
			.get_result(task_id)
			.await
			.expect("Failed to get result");
		assert!(retrieved.is_some());
		assert_eq!(retrieved.unwrap().result(), Some("Test result"));
	}

	#[tokio::test]
	#[serial(redis)]
	async fn test_redis_result_backend_delete() {
		let container = setup_redis().await;
		let port = container
			.get_host_port_ipv4(6379)
			.await
			.expect("Failed to get port");
		let redis_url = format!("redis://127.0.0.1:{}/", port);

		let backend = RedisResultBackend::new(&redis_url)
			.await
			.expect("Failed to connect to Redis");

		let task_id = TaskId::new();
		let metadata = crate::result::TaskResultMetadata::new(task_id, TaskStatus::Success, None);

		// Store and then delete
		backend
			.store_result(metadata)
			.await
			.expect("Failed to store result");
		backend
			.delete_result(task_id)
			.await
			.expect("Failed to delete result");

		// Verify deleted
		let retrieved = backend
			.get_result(task_id)
			.await
			.expect("Failed to get result");
		assert!(retrieved.is_none());
	}
}
