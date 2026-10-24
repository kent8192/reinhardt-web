//! Task definitions and execution

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Default queue name used when no specific queue is specified.
pub const DEFAULT_TASK_QUEUE_NAME: &str = "default";
/// Minimum allowed task priority value.
pub const TASK_MIN_PRIORITY: i32 = 0;
/// Maximum allowed task priority value.
pub const TASK_MAX_PRIORITY: i32 = 9;

/// Unique identifier for a task
///
/// # Example
///
/// ```rust
/// use reinhardt_tasks::TaskId;
///
/// let id1 = TaskId::new();
/// let id2 = TaskId::new();
/// assert_ne!(id1, id2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub uuid::Uuid);

impl TaskId {
	/// Create a new unique task ID
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_tasks::TaskId;
	///
	/// let id = TaskId::new();
	/// println!("Task ID: {}", id);
	/// ```
	pub fn new() -> Self {
		Self(uuid::Uuid::now_v7())
	}
}

impl Default for TaskId {
	fn default() -> Self {
		Self::new()
	}
}

impl fmt::Display for TaskId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl FromStr for TaskId {
	type Err = uuid::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(uuid::Uuid::parse_str(s)?))
	}
}

/// Status of a task
///
/// # Example
///
/// ```rust
/// use reinhardt_tasks::TaskStatus;
///
/// let status = TaskStatus::Pending;
/// assert_eq!(status, TaskStatus::Pending);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
	/// The task is waiting to be executed.
	Pending,
	/// The task is currently being executed.
	Running,
	/// The task completed successfully.
	Success,
	/// The task failed during execution.
	Failure,
	/// The task is scheduled for retry after a failure.
	Retry,
}

impl TaskStatus {
	/// Returns the canonical lowercase name of this status.
	///
	/// Backends that persist the status as text use this name together with
	/// the [`FromStr`] implementation, so the mapping between variants and
	/// stored strings is defined in exactly one place. The returned names
	/// are part of the on-disk format and must stay stable.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_tasks::TaskStatus;
	///
	/// assert_eq!(TaskStatus::Pending.as_str(), "pending");
	/// assert_eq!(TaskStatus::Retry.as_str(), "retry");
	/// ```
	pub const fn as_str(&self) -> &'static str {
		match self {
			Self::Pending => "pending",
			Self::Running => "running",
			Self::Success => "success",
			Self::Failure => "failure",
			Self::Retry => "retry",
		}
	}
}

/// Error returned when a string is not a known [`TaskStatus`] name.
///
/// Produced by the [`FromStr`] implementation of [`TaskStatus`]. Persistent
/// backends surface it instead of substituting a default status, so a
/// corrupted or newer-format value is never reported as a pending task.
///
/// # Example
///
/// ```rust
/// use reinhardt_tasks::TaskStatus;
///
/// let error = "bogus".parse::<TaskStatus>().unwrap_err();
/// assert_eq!(error.value(), "bogus");
/// assert_eq!(error.to_string(), "unknown task status 'bogus'");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown task status '{value}'")]
pub struct ParseTaskStatusError {
	value: String,
}

impl ParseTaskStatusError {
	/// Returns the string that could not be parsed.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_tasks::TaskStatus;
	///
	/// let error = "Pending".parse::<TaskStatus>().unwrap_err();
	/// assert_eq!(error.value(), "Pending");
	/// ```
	pub fn value(&self) -> &str {
		&self.value
	}
}

impl FromStr for TaskStatus {
	type Err = ParseTaskStatusError;

	/// Parses a canonical status name (as returned by [`TaskStatus::as_str`]).
	///
	/// Matching is exact and case-sensitive. Any other input is rejected with
	/// a [`ParseTaskStatusError`] rather than falling back to a default.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_tasks::TaskStatus;
	///
	/// assert_eq!("success".parse::<TaskStatus>(), Ok(TaskStatus::Success));
	/// assert!("bogus".parse::<TaskStatus>().is_err());
	/// ```
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"pending" => Ok(Self::Pending),
			"running" => Ok(Self::Running),
			"success" => Ok(Self::Success),
			"failure" => Ok(Self::Failure),
			"retry" => Ok(Self::Retry),
			other => Err(ParseTaskStatusError {
				value: other.to_owned(),
			}),
		}
	}
}

/// Task priority (0-9, where 9 is highest)
///
/// # Example
///
/// ```rust
/// use reinhardt_tasks::TaskPriority;
///
/// let high = TaskPriority::new(9);
/// let low = TaskPriority::new(0);
/// assert!(high > low);
/// assert_eq!(high.value(), 9);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TaskPriority(i32);

impl TaskPriority {
	/// Create a new task priority, clamped to valid range (0-9)
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_tasks::TaskPriority;
	///
	/// let p1 = TaskPriority::new(5);
	/// assert_eq!(p1.value(), 5);
	///
	/// // Out of range values are clamped
	/// let p2 = TaskPriority::new(100);
	/// assert_eq!(p2.value(), 9);
	///
	/// let p3 = TaskPriority::new(-10);
	/// assert_eq!(p3.value(), 0);
	/// ```
	pub fn new(priority: i32) -> Self {
		Self(priority.clamp(TASK_MIN_PRIORITY, TASK_MAX_PRIORITY))
	}

	/// Get the priority value
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_tasks::TaskPriority;
	///
	/// let priority = TaskPriority::new(7);
	/// assert_eq!(priority.value(), 7);
	/// ```
	pub fn value(&self) -> i32 {
		self.0
	}
}

impl Default for TaskPriority {
	fn default() -> Self {
		Self(5)
	}
}

/// Core trait that all tasks must implement, providing identity and metadata.
pub trait Task: Send + Sync {
	/// Returns the unique identifier for this task instance.
	fn id(&self) -> TaskId;
	/// Returns the name of this task type.
	fn name(&self) -> &str;
	/// Returns the priority of this task. Defaults to medium priority (5).
	fn priority(&self) -> TaskPriority {
		TaskPriority::default()
	}
}

/// Trait for tasks that can be executed asynchronously.
#[async_trait]
pub trait TaskExecutor: Task {
	/// Executes the task and returns the result.
	async fn execute(&self) -> crate::TaskResult<()>;
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	#[case(TaskStatus::Pending, "pending")]
	#[case(TaskStatus::Running, "running")]
	#[case(TaskStatus::Success, "success")]
	#[case(TaskStatus::Failure, "failure")]
	#[case(TaskStatus::Retry, "retry")]
	fn test_task_status_round_trips_through_as_str_and_from_str(
		#[case] status: TaskStatus,
		#[case] expected: &str,
	) {
		// Arrange & Act
		let name = status.as_str();
		let parsed = name.parse::<TaskStatus>();

		// Assert
		assert_eq!(name, expected);
		assert_eq!(parsed, Ok(status));
	}

	#[rstest]
	#[case("bogus")]
	#[case("")]
	#[case("Pending")]
	#[case("SUCCESS")]
	#[case(" pending")]
	#[case("pending\n")]
	fn test_task_status_from_str_rejects_unknown_value(#[case] input: &str) {
		// Arrange & Act
		let error = input
			.parse::<TaskStatus>()
			.expect_err("unknown status must be rejected");

		// Assert
		assert_eq!(error.value(), input);
		assert_eq!(error.to_string(), format!("unknown task status '{input}'"));
	}
}
