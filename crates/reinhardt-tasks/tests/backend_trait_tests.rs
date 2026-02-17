//! TaskBackend trait implementation tests
//!
//! Tests the TaskBackend trait API and DummyBackend implementation.

use async_trait::async_trait;
use reinhardt_tasks::{
	Task, TaskId, TaskStatus,
	backend::{DummyBackend, TaskBackend, TaskExecutionError},
};
use rstest::rstest;
use std::sync::{Arc, Mutex};

/// Simple test task for testing TaskBackend implementations
struct TestTask {
	task_id: TaskId,
	task_name: String,
}

impl TestTask {
	fn new(name: impl Into<String>) -> Self {
		Self {
			task_id: TaskId::new(),
			task_name: name.into(),
		}
	}
}

impl Task for TestTask {
	fn id(&self) -> TaskId {
		self.task_id
	}

	fn name(&self) -> &str {
		&self.task_name
	}
}

/// Test: DummyBackend enqueue returns valid TaskId
#[rstest]
#[tokio::test]
async fn test_dummy_backend_enqueue() {
	let backend = DummyBackend::new();
	let task = Box::new(TestTask::new("test_task_1"));

	let result = backend.enqueue(task).await;
	assert!(result.is_ok());

	let task_id = result.unwrap();
	assert!(!task_id.to_string().is_empty());
}

/// Test: DummyBackend dequeue always returns None
#[rstest]
#[tokio::test]
async fn test_dummy_backend_dequeue() {
	let backend = DummyBackend::new();

	let result = backend.dequeue().await;
	assert!(result.is_ok());
	assert!(result.unwrap().is_none());
}

/// Test: DummyBackend get_status always returns Success
#[rstest]
#[tokio::test]
async fn test_dummy_backend_get_status() {
	let backend = DummyBackend::new();
	let task_id = TaskId::new();

	let result = backend.get_status(task_id).await;
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), TaskStatus::Success);
}

/// Test: DummyBackend update_status always succeeds
#[rstest]
#[tokio::test]
async fn test_dummy_backend_update_status() {
	let backend = DummyBackend::new();
	let task_id = TaskId::new();

	let result = backend.update_status(task_id, TaskStatus::Running).await;
	assert!(result.is_ok());

	// Status update doesn't affect get_status (DummyBackend doesn't store state)
	let status = backend.get_status(task_id).await.unwrap();
	assert_eq!(status, TaskStatus::Success);
}

/// Test: DummyBackend get_task_data always returns None
#[rstest]
#[tokio::test]
async fn test_dummy_backend_get_task_data() {
	let backend = DummyBackend::new();
	let task_id = TaskId::new();

	let result = backend.get_task_data(task_id).await;
	assert!(result.is_ok());
	assert!(result.unwrap().is_none());
}

/// Test: DummyBackend backend_name
#[rstest]
#[tokio::test]
async fn test_dummy_backend_name() {
	let backend = DummyBackend::new();
	assert_eq!(backend.backend_name(), "dummy");
}

/// Test: DummyBackend Default trait
#[rstest]
#[tokio::test]
async fn test_dummy_backend_default() {
	let backend = DummyBackend;
	let task = Box::new(TestTask::new("test_default"));

	let result = backend.enqueue(task).await;
	assert!(result.is_ok());
}

/// In-memory backend implementation for testing TaskBackend trait
struct InMemoryBackend {
	queue: Arc<Mutex<Vec<TaskId>>>,
	statuses: Arc<Mutex<std::collections::HashMap<TaskId, TaskStatus>>>,
}

impl InMemoryBackend {
	fn new() -> Self {
		Self {
			queue: Arc::new(Mutex::new(Vec::new())),
			statuses: Arc::new(Mutex::new(std::collections::HashMap::new())),
		}
	}
}

#[async_trait]
impl TaskBackend for InMemoryBackend {
	async fn enqueue(&self, _task: Box<dyn Task>) -> Result<TaskId, TaskExecutionError> {
		let task_id = TaskId::new();
		self.queue.lock().unwrap().push(task_id);
		self.statuses
			.lock()
			.unwrap()
			.insert(task_id, TaskStatus::Pending);
		Ok(task_id)
	}

	async fn dequeue(&self) -> Result<Option<TaskId>, TaskExecutionError> {
		Ok(self.queue.lock().unwrap().pop())
	}

	async fn get_status(&self, task_id: TaskId) -> Result<TaskStatus, TaskExecutionError> {
		self.statuses
			.lock()
			.unwrap()
			.get(&task_id)
			.copied()
			.ok_or(TaskExecutionError::NotFound(task_id))
	}

	async fn update_status(
		&self,
		task_id: TaskId,
		status: TaskStatus,
	) -> Result<(), TaskExecutionError> {
		if let std::collections::hash_map::Entry::Occupied(mut e) =
			self.statuses.lock().unwrap().entry(task_id)
		{
			e.insert(status);
			Ok(())
		} else {
			Err(TaskExecutionError::NotFound(task_id))
		}
	}

	async fn get_task_data(
		&self,
		_task_id: TaskId,
	) -> Result<Option<reinhardt_tasks::registry::SerializedTask>, TaskExecutionError> {
		// Simplified implementation for testing
		Ok(None)
	}

	fn backend_name(&self) -> &str {
		"in_memory"
	}
}

/// Test: InMemoryBackend enqueue and dequeue
#[rstest]
#[tokio::test]
async fn test_in_memory_backend_enqueue_dequeue() {
	let backend = InMemoryBackend::new();
	let task1 = Box::new(TestTask::new("task1"));
	let task2 = Box::new(TestTask::new("task2"));

	let id1 = backend.enqueue(task1).await.unwrap();
	let id2 = backend.enqueue(task2).await.unwrap();

	// LIFO order (pop from end)
	let dequeued1 = backend.dequeue().await.unwrap();
	assert_eq!(dequeued1, Some(id2));

	let dequeued2 = backend.dequeue().await.unwrap();
	assert_eq!(dequeued2, Some(id1));

	let dequeued3 = backend.dequeue().await.unwrap();
	assert_eq!(dequeued3, None);
}

/// Test: InMemoryBackend status management
#[rstest]
#[tokio::test]
async fn test_in_memory_backend_status_management() {
	let backend = InMemoryBackend::new();
	let task = Box::new(TestTask::new("status_test"));

	let task_id = backend.enqueue(task).await.unwrap();

	// Initial status should be Pending
	let status = backend.get_status(task_id).await.unwrap();
	assert_eq!(status, TaskStatus::Pending);

	// Update to Running
	backend
		.update_status(task_id, TaskStatus::Running)
		.await
		.unwrap();
	let status = backend.get_status(task_id).await.unwrap();
	assert_eq!(status, TaskStatus::Running);

	// Update to Success
	backend
		.update_status(task_id, TaskStatus::Success)
		.await
		.unwrap();
	let status = backend.get_status(task_id).await.unwrap();
	assert_eq!(status, TaskStatus::Success);
}

/// Test: InMemoryBackend NotFound error
#[rstest]
#[tokio::test]
async fn test_in_memory_backend_not_found() {
	let backend = InMemoryBackend::new();
	let non_existent_id = TaskId::new();

	// get_status on non-existent task
	let result = backend.get_status(non_existent_id).await;
	assert!(result.is_err());
	assert!(matches!(
		result.unwrap_err(),
		TaskExecutionError::NotFound(_)
	));

	// update_status on non-existent task
	let result = backend
		.update_status(non_existent_id, TaskStatus::Running)
		.await;
	assert!(result.is_err());
	assert!(matches!(
		result.unwrap_err(),
		TaskExecutionError::NotFound(_)
	));
}

/// Test: InMemoryBackend backend_name
#[rstest]
#[tokio::test]
async fn test_in_memory_backend_name() {
	let backend = InMemoryBackend::new();
	assert_eq!(backend.backend_name(), "in_memory");
}
