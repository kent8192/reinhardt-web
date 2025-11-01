//! Task registry for dynamic task dispatch
//!
//! This module provides a registry system to store and retrieve task executors
//! by name, enabling dynamic task dispatch in distributed task systems.

use crate::{TaskError, TaskExecutor, TaskResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Serialized task data for storage and transmission
///
/// # Examples
///
/// ```rust
/// use reinhardt_tasks::SerializedTask;
///
/// let task = SerializedTask::new(
///     "send_email".to_string(),
///     r#"{"to":"user@example.com","subject":"Hello"}"#.to_string(),
/// );
///
/// assert_eq!(task.name(), "send_email");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedTask {
	name: String,
	data: String,
}

impl SerializedTask {
	/// Create a new serialized task
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::SerializedTask;
	///
	/// let task = SerializedTask::new("process_data".to_string(), "{}".to_string());
	/// ```
	pub fn new(name: String, data: String) -> Self {
		Self { name, data }
	}

	/// Get the task name
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::SerializedTask;
	///
	/// let task = SerializedTask::new("task_name".to_string(), "{}".to_string());
	/// assert_eq!(task.name(), "task_name");
	/// ```
	pub fn name(&self) -> &str {
		&self.name
	}

	/// Get the task data
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::SerializedTask;
	///
	/// let task = SerializedTask::new("task".to_string(), r#"{"key":"value"}"#.to_string());
	/// assert_eq!(task.data(), r#"{"key":"value"}"#);
	/// ```
	pub fn data(&self) -> &str {
		&self.data
	}

	/// Convert to JSON string
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::SerializedTask;
	///
	/// let task = SerializedTask::new("test".to_string(), "{}".to_string());
	/// let json = task.to_json().unwrap();
	/// assert!(json.contains("\"name\":\"test\""));
	/// ```
	pub fn to_json(&self) -> Result<String, serde_json::Error> {
		serde_json::to_string(self)
	}

	/// Create from JSON string
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::SerializedTask;
	///
	/// let json = r#"{"name":"test","data":"{}"}"#;
	/// let task = SerializedTask::from_json(json).unwrap();
	/// assert_eq!(task.name(), "test");
	/// ```
	pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
		serde_json::from_str(json)
	}
}

/// Task factory trait for creating task executors from serialized data
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_tasks::{TaskFactory, TaskResult, TaskExecutor, Task, TaskError, TaskId};
/// use async_trait::async_trait;
///
/// struct EmailTask { to: String }
///
/// impl Task for EmailTask {
///     fn name(&self) -> &str { "EmailTask" }
///     fn id(&self) -> TaskId { TaskId::new() }
/// }
///
/// #[async_trait]
/// impl TaskExecutor for EmailTask {
///     async fn execute(&self) -> TaskResult<()> { Ok(()) }
/// }
///
/// struct EmailTaskFactory;
///
/// #[async_trait]
/// impl TaskFactory for EmailTaskFactory {
///     async fn create(&self, data: &str) -> TaskResult<Box<dyn TaskExecutor>> {
///         // Deserialize data and create task executor
///         let email_data: serde_json::Value = serde_json::from_str(data)
///             .map_err(|e| TaskError::SerializationError(e.to_string()))?;
///         Ok(Box::new(EmailTask { to: email_data["to"].as_str().unwrap().to_string() }))
///     }
/// }
/// ```
#[async_trait]
pub trait TaskFactory: Send + Sync {
	/// Create a task executor from serialized data
	async fn create(&self, data: &str) -> TaskResult<Box<dyn TaskExecutor>>;
}

/// Global task registry for dynamic task dispatch
///
/// This registry maintains a mapping of task names to their factory functions,
/// allowing workers to dynamically create and execute tasks based on serialized data.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_tasks::{TaskRegistry, TaskFactory, TaskResult, TaskExecutor, Task, TaskError, TaskId};
/// use async_trait::async_trait;
/// use std::sync::Arc;
///
/// # struct EmailTaskFactory;
/// # struct EmailTask { to: String }
/// # impl Task for EmailTask {
/// #     fn name(&self) -> &str { "EmailTask" }
/// #     fn id(&self) -> TaskId { TaskId::new() }
/// # }
/// # #[async_trait]
/// # impl TaskExecutor for EmailTask {
/// #     async fn execute(&self) -> TaskResult<()> { Ok(()) }
/// # }
/// # #[async_trait]
/// # impl TaskFactory for EmailTaskFactory {
/// #     async fn create(&self, data: &str) -> TaskResult<Box<dyn TaskExecutor>> {
/// #         let email_data: serde_json::Value = serde_json::from_str(data)
/// #             .map_err(|e| TaskError::SerializationError(e.to_string()))?;
/// #         Ok(Box::new(EmailTask { to: email_data["to"].as_str().unwrap().to_string() }))
/// #     }
/// # }
///
/// # async fn example() -> TaskResult<()> {
/// let registry = TaskRegistry::new();
///
/// // Register a task factory
/// registry.register("send_email".to_string(), Arc::new(EmailTaskFactory)).await;
///
/// // Create task from serialized data
/// let task_data = r#"{"to":"user@example.com"}"#;
/// let executor = registry.create("send_email", task_data).await?;
/// # Ok(())
/// # }
/// ```
pub struct TaskRegistry {
	factories: Arc<RwLock<HashMap<String, Arc<dyn TaskFactory>>>>,
}

impl TaskRegistry {
	/// Create a new task registry
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::TaskRegistry;
	///
	/// let registry = TaskRegistry::new();
	/// ```
	pub fn new() -> Self {
		Self {
			factories: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	/// Register a task factory
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_tasks::{TaskRegistry, TaskFactory, Task, TaskId};
	/// use std::sync::Arc;
	///
	/// # struct MyTaskFactory;
	/// # struct MyTask;
	/// # impl Task for MyTask {
	/// #     fn name(&self) -> &str { "MyTask" }
	/// #     fn id(&self) -> TaskId { TaskId::new() }
	/// # }
	/// # #[async_trait::async_trait]
	/// # impl reinhardt_tasks::TaskExecutor for MyTask {
	/// #     async fn execute(&self) -> reinhardt_tasks::TaskResult<()> { Ok(()) }
	/// # }
	/// # #[async_trait::async_trait]
	/// # impl TaskFactory for MyTaskFactory {
	/// #     async fn create(&self, _data: &str) -> reinhardt_tasks::TaskResult<Box<dyn reinhardt_tasks::TaskExecutor>> {
	/// #         Ok(Box::new(MyTask))
	/// #     }
	/// # }
	///
	/// # async fn example() {
	/// let registry = TaskRegistry::new();
	/// let factory = Arc::new(MyTaskFactory);
	/// registry.register("my_task".to_string(), factory).await;
	/// # }
	/// ```
	pub async fn register(&self, name: String, factory: Arc<dyn TaskFactory>) {
		let mut factories = self.factories.write().await;
		factories.insert(name, factory);
	}

	/// Unregister a task factory
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_tasks::TaskRegistry;
	///
	/// # async fn example() {
	/// let registry = TaskRegistry::new();
	/// registry.unregister("task_name").await;
	/// # }
	/// ```
	pub async fn unregister(&self, name: &str) {
		let mut factories = self.factories.write().await;
		factories.remove(name);
	}

	/// Check if a task is registered
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_tasks::TaskRegistry;
	///
	/// # async fn example() {
	/// let registry = TaskRegistry::new();
	/// let exists = registry.has("task_name").await;
	/// assert!(!exists);
	/// # }
	/// ```
	pub async fn has(&self, name: &str) -> bool {
		let factories = self.factories.read().await;
		factories.contains_key(name)
	}

	/// Create a task executor from serialized data
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_tasks::TaskRegistry;
	///
	/// # async fn example() -> reinhardt_tasks::TaskResult<()> {
	/// let registry = TaskRegistry::new();
	/// let executor = registry.create("task_name", r#"{"key":"value"}"#).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn create(&self, name: &str, data: &str) -> TaskResult<Box<dyn TaskExecutor>> {
		let factories = self.factories.read().await;

		let factory = factories
			.get(name)
			.ok_or_else(|| TaskError::ExecutionFailed(format!("Task not registered: {}", name)))?;

		factory.create(data).await
	}

	/// Get all registered task names
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_tasks::TaskRegistry;
	///
	/// # async fn example() {
	/// let registry = TaskRegistry::new();
	/// let task_names = registry.list().await;
	/// println!("Registered tasks: {:?}", task_names);
	/// # }
	/// ```
	pub async fn list(&self) -> Vec<String> {
		let factories = self.factories.read().await;
		factories.keys().cloned().collect()
	}

	/// Clear all registered task factories
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::TaskRegistry;
	///
	/// # async fn example() {
	/// let registry = TaskRegistry::new();
	/// registry.clear().await;
	/// # }
	/// ```
	pub async fn clear(&self) {
		let mut factories = self.factories.write().await;
		factories.clear();
	}
}

impl Default for TaskRegistry {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{Task, TaskId, TaskPriority};

	struct TestTask {
		id: TaskId,
	}

	impl Task for TestTask {
		fn id(&self) -> TaskId {
			self.id
		}

		fn name(&self) -> &str {
			"test_task"
		}

		fn priority(&self) -> TaskPriority {
			TaskPriority::default()
		}
	}

	#[async_trait]
	impl TaskExecutor for TestTask {
		async fn execute(&self) -> TaskResult<()> {
			Ok(())
		}
	}

	struct TestTaskFactory;

	#[async_trait]
	impl TaskFactory for TestTaskFactory {
		async fn create(&self, _data: &str) -> TaskResult<Box<dyn TaskExecutor>> {
			Ok(Box::new(TestTask { id: TaskId::new() }))
		}
	}

	#[test]
	fn test_serialized_task() {
		let task = SerializedTask::new("test".to_string(), r#"{"key":"value"}"#.to_string());
		assert_eq!(task.name(), "test");
		assert_eq!(task.data(), r#"{"key":"value"}"#);
	}

	#[test]
	fn test_serialized_task_json() {
		let task = SerializedTask::new("test".to_string(), "{}".to_string());
		let json = task.to_json().unwrap();
		let restored = SerializedTask::from_json(&json).unwrap();
		assert_eq!(restored.name(), "test");
	}

	#[tokio::test]
	async fn test_registry_register_and_has() {
		let registry = TaskRegistry::new();
		let factory = Arc::new(TestTaskFactory);

		assert!(!registry.has("test_task").await);

		registry.register("test_task".to_string(), factory).await;

		assert!(registry.has("test_task").await);
	}

	#[tokio::test]
	async fn test_registry_unregister() {
		let registry = TaskRegistry::new();
		let factory = Arc::new(TestTaskFactory);

		registry.register("test_task".to_string(), factory).await;
		assert!(registry.has("test_task").await);

		registry.unregister("test_task").await;
		assert!(!registry.has("test_task").await);
	}

	#[tokio::test]
	async fn test_registry_create() {
		let registry = TaskRegistry::new();
		let factory = Arc::new(TestTaskFactory);

		registry.register("test_task".to_string(), factory).await;

		let executor = registry.create("test_task", "{}").await;
		assert!(executor.is_ok());
	}

	#[tokio::test]
	async fn test_registry_create_not_found() {
		let registry = TaskRegistry::new();

		let result = registry.create("nonexistent", "{}").await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_registry_list() {
		let registry = TaskRegistry::new();
		let factory = Arc::new(TestTaskFactory);

		registry
			.register("task1".to_string(), factory.clone())
			.await;
		registry.register("task2".to_string(), factory).await;

		let names = registry.list().await;
		assert_eq!(names.len(), 2);
		assert!(names.contains(&"task1".to_string()));
		assert!(names.contains(&"task2".to_string()));
	}

	#[tokio::test]
	async fn test_registry_clear() {
		let registry = TaskRegistry::new();
		let factory = Arc::new(TestTaskFactory);

		registry.register("task1".to_string(), factory).await;
		assert!(registry.has("task1").await);

		registry.clear().await;
		assert!(!registry.has("task1").await);
	}
}
