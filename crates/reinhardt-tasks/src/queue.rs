//! Task queue management

use crate::backend::TaskExecutionError;
use crate::{Task, TaskBackend, TaskId};

#[derive(Debug, Clone)]
pub struct QueueConfig {
	pub name: String,
	pub max_retries: u32,
}

impl QueueConfig {
	pub fn new(name: String) -> Self {
		Self {
			name,
			max_retries: 3,
		}
	}
}

impl Default for QueueConfig {
	fn default() -> Self {
		Self::new("default".to_string())
	}
}

pub struct TaskQueue {
	#[allow(dead_code)]
	config: QueueConfig,
}

impl TaskQueue {
	pub fn new() -> Self {
		Self {
			config: QueueConfig::default(),
		}
	}

	pub fn with_config(config: QueueConfig) -> Self {
		Self { config }
	}

	pub async fn enqueue(
		&self,
		_task: Box<dyn Task>,
		_backend: &dyn TaskBackend,
	) -> Result<TaskId, TaskExecutionError> {
		Ok(TaskId::new())
	}
}

impl Default for TaskQueue {
	fn default() -> Self {
		Self::new()
	}
}
