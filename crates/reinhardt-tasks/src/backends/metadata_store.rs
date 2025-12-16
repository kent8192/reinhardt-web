//! Pluggable metadata store for task backends
//!
//! This module provides a trait for external task metadata storage,
//! enabling backends like RabbitMQ to track task status and data
//! independently from the message queue.
//!
//! # Design Rationale
//!
//! Message queues like RabbitMQ and SQS are designed for message delivery,
//! not data storage. To support features like status tracking and task
//! data retrieval, we need a separate storage layer.
//!
//! # Available Implementations
//!
//! - [`InMemoryMetadataStore`]: Default, non-persistent implementation
//!
//! # Examples
//!
//! ```
//! use reinhardt_tasks::backends::metadata_store::{InMemoryMetadataStore, MetadataStore, TaskMetadata};
//! use reinhardt_tasks::{TaskId, TaskStatus};
//!
//! # async fn example() {
//! let store = InMemoryMetadataStore::new();
//!
//! let task_id = TaskId::new();
//! let metadata = TaskMetadata::new(task_id, "my_task".to_string());
//!
//! // Store metadata
//! store.store(metadata.clone()).await.unwrap();
//!
//! // Retrieve metadata
//! let retrieved = store.get(task_id).await.unwrap();
//! assert!(retrieved.is_some());
//!
//! // Update status
//! store.update_status(task_id, TaskStatus::Running).await.unwrap();
//! # }
//! ```

use crate::{TaskId, TaskStatus, registry::SerializedTask};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Error type for metadata store operations
#[derive(Debug, Clone)]
pub enum MetadataStoreError {
	/// Task not found in store
	NotFound(TaskId),
	/// Storage operation failed
	StorageError(String),
	/// Serialization/deserialization error
	SerializationError(String),
}

impl std::fmt::Display for MetadataStoreError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			MetadataStoreError::NotFound(id) => {
				write!(f, "Task {} not found in metadata store", id)
			}
			MetadataStoreError::StorageError(msg) => write!(f, "Metadata storage error: {}", msg),
			MetadataStoreError::SerializationError(msg) => {
				write!(f, "Metadata serialization error: {}", msg)
			}
		}
	}
}

impl std::error::Error for MetadataStoreError {}

/// Task metadata for external storage
///
/// Contains all information needed to track task status and retrieve task data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
	/// Unique task identifier
	pub id: TaskId,
	/// Task name (e.g., "send_email", "process_image")
	pub name: String,
	/// Current task status
	pub status: TaskStatus,
	/// Unix timestamp when task was created
	pub created_at: i64,
	/// Unix timestamp when task was last updated
	pub updated_at: i64,
	/// Optional serialized task data
	pub task_data: Option<SerializedTask>,
}

impl TaskMetadata {
	/// Create new task metadata with Pending status
	pub fn new(id: TaskId, name: String) -> Self {
		let now = chrono::Utc::now().timestamp();
		Self {
			id,
			name,
			status: TaskStatus::Pending,
			created_at: now,
			updated_at: now,
			task_data: None,
		}
	}

	/// Create new task metadata with serialized task data
	pub fn with_task_data(id: TaskId, name: String, task_data: SerializedTask) -> Self {
		let now = chrono::Utc::now().timestamp();
		Self {
			id,
			name,
			status: TaskStatus::Pending,
			created_at: now,
			updated_at: now,
			task_data: Some(task_data),
		}
	}

	/// Update the status and timestamp
	pub fn set_status(&mut self, status: TaskStatus) {
		self.status = status;
		self.updated_at = chrono::Utc::now().timestamp();
	}
}

/// Trait for pluggable metadata storage backends
///
/// Implement this trait to provide custom storage for task metadata.
/// Common implementations include Redis, PostgreSQL, and DynamoDB.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to support concurrent access
/// from multiple async tasks.
///
/// # Examples
///
/// ```
/// use reinhardt_tasks::backends::metadata_store::{MetadataStore, MetadataStoreError, TaskMetadata};
/// use reinhardt_tasks::{TaskId, TaskStatus};
/// use async_trait::async_trait;
///
/// struct MyCustomStore { /* ... */ }
///
/// #[async_trait]
/// impl MetadataStore for MyCustomStore {
///     async fn store(&self, metadata: TaskMetadata) -> Result<(), MetadataStoreError> {
///         // Store in database
///         Ok(())
///     }
///
///     async fn get(&self, task_id: TaskId) -> Result<Option<TaskMetadata>, MetadataStoreError> {
///         // Retrieve from database
///         Ok(None)
///     }
///
///     async fn update_status(&self, task_id: TaskId, status: TaskStatus) -> Result<(), MetadataStoreError> {
///         // Update status in database
///         Ok(())
///     }
///
///     async fn delete(&self, task_id: TaskId) -> Result<(), MetadataStoreError> {
///         // Delete from database
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait MetadataStore: Send + Sync {
	/// Store task metadata
	async fn store(&self, metadata: TaskMetadata) -> Result<(), MetadataStoreError>;

	/// Retrieve task metadata by ID
	async fn get(&self, task_id: TaskId) -> Result<Option<TaskMetadata>, MetadataStoreError>;

	/// Update task status
	async fn update_status(
		&self,
		task_id: TaskId,
		status: TaskStatus,
	) -> Result<(), MetadataStoreError>;

	/// Delete task metadata
	async fn delete(&self, task_id: TaskId) -> Result<(), MetadataStoreError>;
}

/// In-memory implementation of MetadataStore
///
/// This is the default implementation for development and testing.
/// Data is stored in memory and will be lost when the process exits.
///
/// # Thread Safety
///
/// Uses `RwLock` for concurrent read/write access.
///
/// # Examples
///
/// ```
/// use reinhardt_tasks::backends::metadata_store::{InMemoryMetadataStore, MetadataStore, TaskMetadata};
/// use reinhardt_tasks::TaskId;
///
/// # async fn example() {
/// let store = InMemoryMetadataStore::new();
///
/// let task_id = TaskId::new();
/// let metadata = TaskMetadata::new(task_id, "test_task".to_string());
///
/// store.store(metadata).await.unwrap();
/// # }
/// ```
pub struct InMemoryMetadataStore {
	store: Arc<RwLock<HashMap<TaskId, TaskMetadata>>>,
}

impl InMemoryMetadataStore {
	/// Create a new in-memory metadata store
	pub fn new() -> Self {
		Self {
			store: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	/// Get the number of entries in the store
	pub async fn len(&self) -> usize {
		self.store.read().await.len()
	}

	/// Check if the store is empty
	pub async fn is_empty(&self) -> bool {
		self.store.read().await.is_empty()
	}

	/// Clear all entries from the store
	pub async fn clear(&self) {
		self.store.write().await.clear();
	}
}

impl Default for InMemoryMetadataStore {
	fn default() -> Self {
		Self::new()
	}
}

impl Clone for InMemoryMetadataStore {
	fn clone(&self) -> Self {
		Self {
			store: Arc::clone(&self.store),
		}
	}
}

#[async_trait]
impl MetadataStore for InMemoryMetadataStore {
	async fn store(&self, metadata: TaskMetadata) -> Result<(), MetadataStoreError> {
		let mut store = self.store.write().await;
		store.insert(metadata.id, metadata);
		Ok(())
	}

	async fn get(&self, task_id: TaskId) -> Result<Option<TaskMetadata>, MetadataStoreError> {
		let store = self.store.read().await;
		Ok(store.get(&task_id).cloned())
	}

	async fn update_status(
		&self,
		task_id: TaskId,
		status: TaskStatus,
	) -> Result<(), MetadataStoreError> {
		let mut store = self.store.write().await;

		if let Some(metadata) = store.get_mut(&task_id) {
			metadata.set_status(status);
			Ok(())
		} else {
			Err(MetadataStoreError::NotFound(task_id))
		}
	}

	async fn delete(&self, task_id: TaskId) -> Result<(), MetadataStoreError> {
		let mut store = self.store.write().await;
		store.remove(&task_id);
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_in_memory_store_basic_operations() {
		let store = InMemoryMetadataStore::new();

		let task_id = TaskId::new();
		let metadata = TaskMetadata::new(task_id, "test_task".to_string());

		// Store
		store.store(metadata.clone()).await.unwrap();
		assert_eq!(store.len().await, 1);

		// Get
		let retrieved = store.get(task_id).await.unwrap().unwrap();
		assert_eq!(retrieved.id, task_id);
		assert_eq!(retrieved.name, "test_task");
		assert_eq!(retrieved.status, TaskStatus::Pending);

		// Update status
		store
			.update_status(task_id, TaskStatus::Running)
			.await
			.unwrap();
		let updated = store.get(task_id).await.unwrap().unwrap();
		assert_eq!(updated.status, TaskStatus::Running);

		// Delete
		store.delete(task_id).await.unwrap();
		assert!(store.get(task_id).await.unwrap().is_none());
	}

	#[tokio::test]
	async fn test_in_memory_store_update_nonexistent() {
		let store = InMemoryMetadataStore::new();
		let task_id = TaskId::new();

		let result = store.update_status(task_id, TaskStatus::Running).await;
		assert!(matches!(result, Err(MetadataStoreError::NotFound(_))));
	}

	#[tokio::test]
	async fn test_in_memory_store_clear() {
		let store = InMemoryMetadataStore::new();

		for _ in 0..10 {
			let metadata = TaskMetadata::new(TaskId::new(), "test".to_string());
			store.store(metadata).await.unwrap();
		}

		assert_eq!(store.len().await, 10);
		store.clear().await;
		assert!(store.is_empty().await);
	}

	#[tokio::test]
	async fn test_task_metadata_with_task_data() {
		let task_id = TaskId::new();
		let task_data =
			SerializedTask::new("test_task".to_string(), r#"{"key": "value"}"#.to_string());
		let metadata = TaskMetadata::with_task_data(task_id, "test_task".to_string(), task_data);

		assert!(metadata.task_data.is_some());
		assert_eq!(metadata.task_data.as_ref().unwrap().name(), "test_task");
	}

	#[test]
	fn test_metadata_store_error_display() {
		let task_id = TaskId::new();
		let not_found = MetadataStoreError::NotFound(task_id);
		assert!(not_found.to_string().contains("not found"));

		let storage_error = MetadataStoreError::StorageError("connection failed".to_string());
		assert!(storage_error.to_string().contains("connection failed"));
	}
}
