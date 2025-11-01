//! Task chaining
//!
//! Allows multiple tasks to be executed in sequence, with each task receiving
//! the result of the previous task.

use crate::{TaskBackend, TaskExecutionError, TaskId, TaskStatus};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Task chain configuration
///
/// # Examples
///
/// ```rust
/// use reinhardt_tasks::TaskChain;
///
/// let chain = TaskChain::new("email-workflow");
/// assert_eq!(chain.name(), "email-workflow");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskChain {
	/// Chain identifier
	id: TaskId,
	/// Chain name
	name: String,
	/// Task IDs in execution order
	task_ids: Vec<TaskId>,
	/// Current task index
	current_index: usize,
	/// Chain status
	status: ChainStatus,
}

/// Status of a task chain
///
/// # Examples
///
/// ```rust
/// use reinhardt_tasks::ChainStatus;
///
/// let status = ChainStatus::Pending;
/// assert_eq!(status, ChainStatus::Pending);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChainStatus {
	/// Chain is waiting to start
	Pending,
	/// Chain is currently executing
	Running,
	/// Chain completed successfully
	Completed,
	/// Chain failed
	Failed,
}

impl TaskChain {
	/// Create a new task chain
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::TaskChain;
	///
	/// let chain = TaskChain::new("payment-processing");
	/// assert_eq!(chain.name(), "payment-processing");
	/// ```
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			id: TaskId::new(),
			name: name.into(),
			task_ids: Vec::new(),
			current_index: 0,
			status: ChainStatus::Pending,
		}
	}

	/// Get the chain ID
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::TaskChain;
	///
	/// let chain = TaskChain::new("test-chain");
	/// let id = chain.id();
	/// ```
	pub fn id(&self) -> TaskId {
		self.id
	}

	/// Get the chain name
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::TaskChain;
	///
	/// let chain = TaskChain::new("my-chain");
	/// assert_eq!(chain.name(), "my-chain");
	/// ```
	pub fn name(&self) -> &str {
		&self.name
	}

	/// Add a task to the chain
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::{TaskChain, TaskId};
	///
	/// let mut chain = TaskChain::new("workflow");
	/// let task_id = TaskId::new();
	/// chain.add_task(task_id);
	/// assert_eq!(chain.task_count(), 1);
	/// ```
	pub fn add_task(&mut self, task_id: TaskId) {
		self.task_ids.push(task_id);
	}

	/// Get the number of tasks in the chain
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::{TaskChain, TaskId};
	///
	/// let mut chain = TaskChain::new("workflow");
	/// chain.add_task(TaskId::new());
	/// chain.add_task(TaskId::new());
	/// assert_eq!(chain.task_count(), 2);
	/// ```
	pub fn task_count(&self) -> usize {
		self.task_ids.len()
	}

	/// Get the current task ID
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::{TaskChain, TaskId};
	///
	/// let mut chain = TaskChain::new("workflow");
	/// let task_id = TaskId::new();
	/// chain.add_task(task_id);
	///
	/// assert_eq!(chain.current_task(), Some(task_id));
	/// ```
	pub fn current_task(&self) -> Option<TaskId> {
		self.task_ids.get(self.current_index).copied()
	}

	/// Move to the next task in the chain
	///
	/// Returns `true` if there are more tasks, `false` if the chain is complete.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::{TaskChain, TaskId};
	///
	/// let mut chain = TaskChain::new("workflow");
	/// chain.add_task(TaskId::new());
	/// chain.add_task(TaskId::new());
	///
	/// assert!(chain.advance());
	/// assert!(!chain.advance()); // No more tasks
	/// ```
	pub fn advance(&mut self) -> bool {
		self.current_index += 1;
		self.current_index < self.task_ids.len()
	}

	/// Get the chain status
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::{TaskChain, ChainStatus};
	///
	/// let chain = TaskChain::new("workflow");
	/// assert_eq!(chain.status(), ChainStatus::Pending);
	/// ```
	pub fn status(&self) -> ChainStatus {
		self.status
	}

	/// Set the chain status
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::{TaskChain, ChainStatus};
	///
	/// let mut chain = TaskChain::new("workflow");
	/// chain.set_status(ChainStatus::Running);
	/// assert_eq!(chain.status(), ChainStatus::Running);
	/// ```
	pub fn set_status(&mut self, status: ChainStatus) {
		self.status = status;
	}

	/// Check if the chain is complete
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::{TaskChain, ChainStatus};
	///
	/// let mut chain = TaskChain::new("workflow");
	/// chain.set_status(ChainStatus::Completed);
	/// assert!(chain.is_complete());
	/// ```
	pub fn is_complete(&self) -> bool {
		matches!(self.status, ChainStatus::Completed | ChainStatus::Failed)
	}

	/// Execute the chain
	///
	/// This method will execute all tasks in the chain sequentially.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_tasks::{TaskChain, DummyBackend};
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let mut chain = TaskChain::new("workflow");
	/// let backend = Arc::new(DummyBackend::new());
	///
	/// chain.execute(backend).await?;
	/// assert!(chain.is_complete());
	/// # Ok(())
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example()).unwrap();
	/// ```
	pub async fn execute(
		&mut self,
		backend: Arc<dyn TaskBackend>,
	) -> Result<(), TaskExecutionError> {
		self.set_status(ChainStatus::Running);

		while let Some(task_id) = self.current_task() {
			// Check task status
			let status = backend.get_status(task_id).await?;

			match status {
				TaskStatus::Success => {
					// Task completed successfully, move to next
					if !self.advance() {
						// All tasks completed
						self.set_status(ChainStatus::Completed);
						return Ok(());
					}
				}
				TaskStatus::Failure => {
					// Task failed, mark chain as failed
					self.set_status(ChainStatus::Failed);
					return Err(TaskExecutionError::ExecutionFailed(format!(
						"Task {} in chain {} failed",
						task_id, self.name
					)));
				}
				TaskStatus::Pending | TaskStatus::Running | TaskStatus::Retry => {
					// Task still in progress, return and wait for next check
					return Ok(());
				}
			}
		}

		self.set_status(ChainStatus::Completed);
		Ok(())
	}
}

/// Task chain builder
///
/// Provides a fluent interface for building task chains.
///
/// # Examples
///
/// ```rust
/// use reinhardt_tasks::{TaskChainBuilder, TaskId};
///
/// let chain = TaskChainBuilder::new("payment-flow")
///     .add_task(TaskId::new())
///     .add_task(TaskId::new())
///     .build();
///
/// assert_eq!(chain.task_count(), 2);
/// ```
pub struct TaskChainBuilder {
	chain: TaskChain,
}

impl TaskChainBuilder {
	/// Create a new task chain builder
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::TaskChainBuilder;
	///
	/// let builder = TaskChainBuilder::new("my-workflow");
	/// ```
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			chain: TaskChain::new(name),
		}
	}

	/// Add a task to the chain
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::{TaskChainBuilder, TaskId};
	///
	/// let builder = TaskChainBuilder::new("workflow")
	///     .add_task(TaskId::new());
	/// ```
	pub fn add_task(mut self, task_id: TaskId) -> Self {
		self.chain.add_task(task_id);
		self
	}

	/// Add multiple tasks to the chain
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::{TaskChainBuilder, TaskId};
	///
	/// let tasks = vec![TaskId::new(), TaskId::new(), TaskId::new()];
	/// let chain = TaskChainBuilder::new("batch")
	///     .add_tasks(tasks)
	///     .build();
	///
	/// assert_eq!(chain.task_count(), 3);
	/// ```
	pub fn add_tasks(mut self, task_ids: Vec<TaskId>) -> Self {
		for task_id in task_ids {
			self.chain.add_task(task_id);
		}
		self
	}

	/// Build the task chain
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_tasks::{TaskChainBuilder, TaskId};
	///
	/// let chain = TaskChainBuilder::new("workflow")
	///     .add_task(TaskId::new())
	///     .build();
	/// ```
	pub fn build(self) -> TaskChain {
		self.chain
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{DummyBackend, Task, TaskPriority};

	struct TestTask {
		id: TaskId,
	}

	impl Task for TestTask {
		fn id(&self) -> TaskId {
			self.id
		}

		fn name(&self) -> &str {
			"test"
		}

		fn priority(&self) -> TaskPriority {
			TaskPriority::default()
		}
	}

	#[test]
	fn test_chain_creation() {
		let chain = TaskChain::new("test-chain");
		assert_eq!(chain.name(), "test-chain");
		assert_eq!(chain.task_count(), 0);
		assert_eq!(chain.status(), ChainStatus::Pending);
	}

	#[test]
	fn test_chain_add_task() {
		let mut chain = TaskChain::new("test");
		let task_id = TaskId::new();
		chain.add_task(task_id);
		assert_eq!(chain.task_count(), 1);
		assert_eq!(chain.current_task(), Some(task_id));
	}

	#[test]
	fn test_chain_advance() {
		let mut chain = TaskChain::new("test");
		chain.add_task(TaskId::new());
		chain.add_task(TaskId::new());

		assert!(chain.advance());
		assert!(!chain.advance());
	}

	#[test]
	fn test_chain_builder() {
		let task1 = TaskId::new();
		let task2 = TaskId::new();

		let chain = TaskChainBuilder::new("builder-test")
			.add_task(task1)
			.add_task(task2)
			.build();

		assert_eq!(chain.task_count(), 2);
		assert_eq!(chain.name(), "builder-test");
	}

	#[test]
	fn test_chain_builder_multiple() {
		let tasks = vec![TaskId::new(), TaskId::new(), TaskId::new()];
		let chain = TaskChainBuilder::new("batch").add_tasks(tasks).build();

		assert_eq!(chain.task_count(), 3);
	}

	#[test]
	fn test_chain_status() {
		let mut chain = TaskChain::new("test");
		assert_eq!(chain.status(), ChainStatus::Pending);

		chain.set_status(ChainStatus::Running);
		assert_eq!(chain.status(), ChainStatus::Running);

		chain.set_status(ChainStatus::Completed);
		assert!(chain.is_complete());
	}

	#[tokio::test]
	async fn test_chain_execution() {
		let backend = Arc::new(DummyBackend::new());
		let mut chain = TaskChain::new("test-execution");

		let task1 = Box::new(TestTask { id: TaskId::new() });
		let task2 = Box::new(TestTask { id: TaskId::new() });

		let id1 = backend.enqueue(task1).await.unwrap();
		let id2 = backend.enqueue(task2).await.unwrap();

		chain.add_task(id1);
		chain.add_task(id2);

		// DummyBackend always returns Success, so chain should complete
		chain.execute(backend).await.unwrap();
		assert_eq!(chain.status(), ChainStatus::Completed);
	}
}
