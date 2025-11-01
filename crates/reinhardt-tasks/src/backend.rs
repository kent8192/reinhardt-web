//! Task backend implementations

use crate::{Task, TaskId, TaskStatus, registry::SerializedTask};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TaskExecutionError {
	#[error("Task execution failed: {0}")]
	ExecutionFailed(String),

	#[error("Task not found: {0}")]
	NotFound(TaskId),

	#[error("Backend error: {0}")]
	BackendError(String),
}

pub type ResultStatus = TaskStatus;
pub type TaskResultStatus = TaskStatus;

#[async_trait]
pub trait TaskBackend: Send + Sync {
	async fn enqueue(&self, task: Box<dyn Task>) -> Result<TaskId, TaskExecutionError>;
	async fn dequeue(&self) -> Result<Option<TaskId>, TaskExecutionError>;
	async fn get_status(&self, task_id: TaskId) -> Result<TaskStatus, TaskExecutionError>;
	async fn update_status(
		&self,
		task_id: TaskId,
		status: TaskStatus,
	) -> Result<(), TaskExecutionError>;

	/// Get serialized task data by task ID
	///
	/// Returns the task data if found, None otherwise.
	async fn get_task_data(
		&self,
		task_id: TaskId,
	) -> Result<Option<SerializedTask>, TaskExecutionError>;

	fn backend_name(&self) -> &str;
}

pub struct TaskBackends;

impl TaskBackends {
	pub fn new() -> Self {
		Self
	}
}

impl Default for TaskBackends {
	fn default() -> Self {
		Self::new()
	}
}

pub struct DummyBackend;

impl DummyBackend {
	pub fn new() -> Self {
		Self
	}
}

impl Default for DummyBackend {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl TaskBackend for DummyBackend {
	async fn enqueue(&self, _task: Box<dyn Task>) -> Result<TaskId, TaskExecutionError> {
		Ok(TaskId::new())
	}

	async fn dequeue(&self) -> Result<Option<TaskId>, TaskExecutionError> {
		Ok(None)
	}

	async fn get_status(&self, _task_id: TaskId) -> Result<TaskStatus, TaskExecutionError> {
		Ok(TaskStatus::Success)
	}

	async fn update_status(
		&self,
		_task_id: TaskId,
		_status: TaskStatus,
	) -> Result<(), TaskExecutionError> {
		Ok(())
	}

	async fn get_task_data(
		&self,
		_task_id: TaskId,
	) -> Result<Option<SerializedTask>, TaskExecutionError> {
		// DummyBackend doesn't store task data
		Ok(None)
	}

	fn backend_name(&self) -> &str {
		"dummy"
	}
}

pub struct ImmediateBackend;

impl ImmediateBackend {
	pub fn new() -> Self {
		Self
	}
}

impl Default for ImmediateBackend {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl TaskBackend for ImmediateBackend {
	async fn enqueue(&self, _task: Box<dyn Task>) -> Result<TaskId, TaskExecutionError> {
		Ok(TaskId::new())
	}

	async fn dequeue(&self) -> Result<Option<TaskId>, TaskExecutionError> {
		Ok(None)
	}

	async fn get_status(&self, _task_id: TaskId) -> Result<TaskStatus, TaskExecutionError> {
		Ok(TaskStatus::Success)
	}

	async fn update_status(
		&self,
		_task_id: TaskId,
		_status: TaskStatus,
	) -> Result<(), TaskExecutionError> {
		Ok(())
	}

	async fn get_task_data(
		&self,
		_task_id: TaskId,
	) -> Result<Option<SerializedTask>, TaskExecutionError> {
		// ImmediateBackend doesn't store task data
		Ok(None)
	}

	fn backend_name(&self) -> &str {
		"immediate"
	}
}
