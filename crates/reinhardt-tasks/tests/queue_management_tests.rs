//! Task queue management tests
//!
//! Tests QueueConfig and TaskQueue implementations.

use reinhardt_tasks::{
	Task, TaskId,
	backend::DummyBackend,
	queue::{QueueConfig, TaskQueue},
};
use rstest::rstest;

/// Simple test task
struct SimpleTask {
	task_id: TaskId,
	task_name: String,
}

impl SimpleTask {
	fn new(name: impl Into<String>) -> Self {
		Self {
			task_id: TaskId::new(),
			task_name: name.into(),
		}
	}
}

impl Task for SimpleTask {
	fn id(&self) -> TaskId {
		self.task_id
	}

	fn name(&self) -> &str {
		&self.task_name
	}
}

/// Test: QueueConfig creation with name
#[rstest]
fn test_queue_config_new() {
	let config = QueueConfig::new("test_queue".to_string());
	assert_eq!(config.name, "test_queue");
	assert_eq!(config.max_retries, 3); // Default max_retries
}

/// Test: QueueConfig default values
#[rstest]
fn test_queue_config_default() {
	let config = QueueConfig::default();
	assert_eq!(config.name, "default");
	assert_eq!(config.max_retries, 3);
}

/// Test: QueueConfig clone
#[rstest]
fn test_queue_config_clone() {
	let config1 = QueueConfig::new("original".to_string());
	let config2 = config1.clone();

	assert_eq!(config1.name, config2.name);
	assert_eq!(config1.max_retries, config2.max_retries);
}

/// Test: TaskQueue creation
#[rstest]
fn test_task_queue_new() {
	let queue = TaskQueue::new();
	// Queue creation should succeed (no panic)
	assert_eq!(std::mem::size_of_val(&queue), 0); // TaskQueue is a unit struct
}

/// Test: TaskQueue with_config
#[rstest]
fn test_task_queue_with_config() {
	let config = QueueConfig::new("custom_queue".to_string());
	let queue = TaskQueue::with_config(config);
	// Queue creation with config should succeed
	assert_eq!(std::mem::size_of_val(&queue), 0);
}

/// Test: TaskQueue default
#[rstest]
fn test_task_queue_default() {
	let queue = TaskQueue;
	assert_eq!(std::mem::size_of_val(&queue), 0);
}

/// Test: TaskQueue enqueue returns valid TaskId
#[rstest]
#[tokio::test]
async fn test_task_queue_enqueue() {
	let queue = TaskQueue::new();
	let backend = DummyBackend::new();
	let task = Box::new(SimpleTask::new("test_task"));

	let result = queue.enqueue(task, &backend).await;
	assert!(result.is_ok());

	let task_id = result.unwrap();
	assert!(!task_id.to_string().is_empty());
}

/// Test: TaskQueue multiple enqueues
#[rstest]
#[tokio::test]
async fn test_task_queue_multiple_enqueues() {
	let queue = TaskQueue::new();
	let backend = DummyBackend::new();

	let task1 = Box::new(SimpleTask::new("task1"));
	let task2 = Box::new(SimpleTask::new("task2"));
	let task3 = Box::new(SimpleTask::new("task3"));

	let id1 = queue.enqueue(task1, &backend).await.unwrap();
	let id2 = queue.enqueue(task2, &backend).await.unwrap();
	let id3 = queue.enqueue(task3, &backend).await.unwrap();

	// All task IDs should be unique
	assert_ne!(id1, id2);
	assert_ne!(id2, id3);
	assert_ne!(id1, id3);
}

/// Test: TaskQueue with different backends
#[rstest]
#[tokio::test]
async fn test_task_queue_with_different_backends() {
	let queue = TaskQueue::new();
	let backend1 = DummyBackend::new();
	let backend2 = DummyBackend::new();

	let task1 = Box::new(SimpleTask::new("backend1_task"));
	let task2 = Box::new(SimpleTask::new("backend2_task"));

	let result1 = queue.enqueue(task1, &backend1).await;
	let result2 = queue.enqueue(task2, &backend2).await;

	assert!(result1.is_ok());
	assert!(result2.is_ok());
}
