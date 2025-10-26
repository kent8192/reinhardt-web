//! # Reinhardt Background Tasks
//!
//! Celery-inspired background task queue for Reinhardt framework.
//!
//! ## Features
//!
//! - Async task execution
//! - Task scheduling (cron-like)
//! - Task retries with exponential backoff
//! - Task priority
//! - Task chaining
//! - Result backend
//!
//! ## Planned Features
//! TODO: Implement Redis/Database result persistence (RedisResultBackend, DatabaseResultBackend)
//! TODO: Complete distributed task execution across multiple workers
//! TODO: Add worker load balancing
//! TODO: Implement task locking mechanism for distributed systems
//! TODO: Complete actual task execution with deserialization and dispatch
//! TODO: Implement task registry for dynamic task dispatch
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_tasks::{Task, TaskQueue};
//!
//! #[derive(Task)]
//! struct SendEmailTask {
//!     to: String,
//!     subject: String,
//!     body: String,
//! }
//!
//! #[async_trait]
//! impl TaskExecutor for SendEmailTask {
//!     async fn execute(&self) -> TaskResult<()> {
//!         // Send email
//!         Ok(())
//!     }
//! }
//!
//! // Queue the task
//! let queue = TaskQueue::new();
//! queue.enqueue(SendEmailTask { ... }).await?;
//! ```

pub mod backend;
pub mod backends;
pub mod chain;
pub mod queue;
pub mod result;
pub mod retry;
pub mod scheduler;
pub mod task;
pub mod worker;

pub use backend::{
    DummyBackend, ImmediateBackend, ResultStatus, TaskBackend, TaskBackends, TaskExecutionError,
    TaskResultStatus,
};

#[cfg(feature = "redis-backend")]
pub use backends::RedisBackend;

#[cfg(feature = "database-backend")]
pub use backends::SqliteBackend;
pub use chain::{ChainStatus, TaskChain, TaskChainBuilder};
pub use queue::{QueueConfig, TaskQueue};
pub use result::{
    MemoryResultBackend, ResultBackend, TaskOutput, TaskResult as TaskResultBackend,
    TaskResultMetadata,
};
pub use retry::{RetryState, RetryStrategy};
pub use scheduler::{CronSchedule, Schedule, Scheduler};
pub use task::{
    DEFAULT_TASK_QUEUE_NAME, TASK_MAX_PRIORITY, TASK_MIN_PRIORITY, Task, TaskExecutor, TaskId,
    TaskPriority, TaskStatus,
};
pub use worker::{Worker, WorkerConfig};

use thiserror::Error;

/// Result type for task operations
pub type TaskResult<T> = Result<T, TaskError>;

/// Task-related errors
///
/// # Example
///
/// ```rust
/// use reinhardt_tasks::TaskError;
///
/// let error = TaskError::ExecutionFailed("Database connection failed".to_string());
/// assert_eq!(error.to_string(), "Task execution failed: Database connection failed");
/// ```
#[derive(Debug, Error)]
pub enum TaskError {
    /// Task execution failed
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_tasks::TaskError;
    ///
    /// let error = TaskError::ExecutionFailed("Network error".to_string());
    /// ```
    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),

    /// Task not found
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_tasks::TaskError;
    ///
    /// let error = TaskError::TaskNotFound("task-123".to_string());
    /// assert_eq!(error.to_string(), "Task not found: task-123");
    /// ```
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    /// Queue error
    #[error("Queue error: {0}")]
    QueueError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Task timeout
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_tasks::TaskError;
    ///
    /// let error = TaskError::Timeout;
    /// assert_eq!(error.to_string(), "Task timeout");
    /// ```
    #[error("Task timeout")]
    Timeout,

    /// Max retries exceeded
    ///
    /// # Example
    ///
    /// ```rust
    /// use reinhardt_tasks::TaskError;
    ///
    /// let error = TaskError::MaxRetriesExceeded;
    /// assert_eq!(error.to_string(), "Max retries exceeded");
    /// ```
    #[error("Max retries exceeded")]
    MaxRetriesExceeded,
}
