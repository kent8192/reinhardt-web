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

// TODO: TaskQueue.config field was removed as it was unused
// QueueConfig (including max_retries) is not currently utilized in enqueue()
// If queue configuration (e.g., max_retries, timeout) is needed in the future,
// the config should be properly integrated into the TaskQueue behavior
pub struct TaskQueue;

impl TaskQueue {
	pub fn new() -> Self {
		Self
	}

	pub fn with_config(_config: QueueConfig) -> Self {
		Self
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
