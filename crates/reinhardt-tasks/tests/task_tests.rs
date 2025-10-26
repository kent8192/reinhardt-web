//! Task system tests based on Django's task test cases

use reinhardt_tasks::{
    DummyBackend, ImmediateBackend, Task, TaskBackend, TaskBackends, TaskExecutor, TaskId,
    TaskPriority, TaskResult, TaskStatus, TASK_MAX_PRIORITY, TASK_MIN_PRIORITY,
};
use std::sync::Arc;

/// Test task implementation
#[derive(Debug, Clone)]
struct NoopTask {
    id: TaskId,
    name: String,
    priority: TaskPriority,
}

impl NoopTask {
    fn new(name: &str) -> Self {
        Self {
            id: TaskId::new(),
            name: name.to_string(),
            priority: TaskPriority::default(),
        }
    }

    fn with_priority(mut self, priority: i32) -> Self {
        self.priority = TaskPriority::new(priority);
        self
    }
}

impl Task for NoopTask {
    fn id(&self) -> TaskId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> TaskPriority {
        self.priority
    }
}

#[async_trait::async_trait]
impl TaskExecutor for NoopTask {
    async fn execute(&self) -> TaskResult<()> {
        Ok(())
    }
}

/// Another test task implementation
#[derive(Debug, Clone)]
struct AsyncNoopTask {
    id: TaskId,
    name: String,
    priority: TaskPriority,
}

impl AsyncNoopTask {
    fn new(name: &str) -> Self {
        Self {
            id: TaskId::new(),
            name: name.to_string(),
            priority: TaskPriority::default(),
        }
    }

    fn with_priority(mut self, priority: i32) -> Self {
        self.priority = TaskPriority::new(priority);
        self
    }
}

impl Task for AsyncNoopTask {
    fn id(&self) -> TaskId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> TaskPriority {
        self.priority
    }
}

#[async_trait::async_trait]
impl TaskExecutor for AsyncNoopTask {
    async fn execute(&self) -> TaskResult<()> {
        Ok(())
    }
}

#[tokio::test]
async fn test_using_correct_backend() {
    let backend = Arc::new(DummyBackend::new());
    let backends = TaskBackends::new();

    // This test verifies the backend type matches
    assert_eq!(backend.backend_name(), "dummy");
}

#[tokio::test]
async fn test_task_creation() {
    let task = NoopTask::new("noop_task");
    assert_eq!(task.name(), "noop_task");
}

#[tokio::test]
async fn test_enqueue_task() {
    let backend = DummyBackend::new();
    let task = Box::new(NoopTask::new("test_task"));

    let task_id = backend.enqueue(task).await.unwrap();
    assert_ne!(task_id, TaskId::new());

    let status = backend.get_status(task_id).await.unwrap();
    assert_eq!(status, TaskStatus::Success);
}

#[tokio::test]
async fn test_using_priority() {
    let task = NoopTask::new("test");
    assert_eq!(task.priority().value(), 5); // Default priority

    let task_with_priority = task.clone().with_priority(1);
    assert_eq!(task_with_priority.priority().value(), 1);
    assert_eq!(task.priority().value(), 5); // Original unchanged
}

#[tokio::test]
async fn test_using_creates_new_instance() {
    let task1 = NoopTask::new("test");
    let task2 = task1.clone();

    // Verify they have the same values
    assert_eq!(task1.name(), task2.name());
    assert_eq!(task1.priority(), task2.priority());
}

#[tokio::test]
async fn test_chained_priority() {
    let task = NoopTask::new("test");

    let priority_task = task.with_priority(9);
    assert_eq!(priority_task.priority().value(), 9);
}

#[tokio::test]
async fn test_invalid_priority() {
    let task = NoopTask::new("test");

    // Priorities outside the valid range should be clamped
    let invalid_low = task.clone().with_priority(TASK_MIN_PRIORITY - 1);
    let invalid_high = task.clone().with_priority(TASK_MAX_PRIORITY + 1);

    assert_eq!(invalid_low.priority().value(), TASK_MIN_PRIORITY);
    assert_eq!(invalid_high.priority().value(), TASK_MAX_PRIORITY);

    // Valid priority values
    let valid_max = task.clone().with_priority(TASK_MAX_PRIORITY);
    let valid_min = task.clone().with_priority(TASK_MIN_PRIORITY);

    assert_eq!(valid_max.priority().value(), TASK_MAX_PRIORITY);
    assert_eq!(valid_min.priority().value(), TASK_MIN_PRIORITY);
}

#[tokio::test]
async fn test_get_backend_name() {
    let dummy_backend = DummyBackend::new();
    assert_eq!(dummy_backend.backend_name(), "dummy");

    let immediate_backend = ImmediateBackend::new();
    assert_eq!(immediate_backend.backend_name(), "immediate");
}

#[tokio::test]
async fn test_task_name() {
    let task = NoopTask::new("noop_task");
    assert_eq!(task.name(), "noop_task");

    let async_task = AsyncNoopTask::new("noop_task_async");
    assert_eq!(async_task.name(), "noop_task_async");
}

#[tokio::test]
async fn test_immediate_backend_executes_immediately() {
    let backend = ImmediateBackend::new();
    let task = Box::new(NoopTask::new("test_task"));

    let task_id = backend.enqueue(task).await.unwrap();
    let status = backend.get_status(task_id).await.unwrap();

    // Immediate backend should return Success status
    assert_eq!(status, TaskStatus::Success);
}

#[tokio::test]
async fn test_task_equality() {
    let task1 = NoopTask::new("test_task");
    let task2 = task1.clone();

    assert_eq!(task1.name(), task2.name());
    assert_eq!(task1.id(), task2.id());
}

#[tokio::test]
async fn test_task_priority_ordering() {
    let low_task = NoopTask::new("test").with_priority(0);
    let normal_task = NoopTask::new("test").with_priority(5);
    let high_task = NoopTask::new("test").with_priority(9);

    assert!(high_task.priority() > normal_task.priority());
    assert!(normal_task.priority() > low_task.priority());
}

#[tokio::test]
async fn test_task_id_uniqueness() {
    let id1 = TaskId::new();
    let id2 = TaskId::new();
    assert_ne!(id1, id2);
}

#[tokio::test]
async fn test_task_priority_clamping() {
    let p1 = TaskPriority::new(5);
    assert_eq!(p1.value(), 5);

    // Out of range values are clamped
    let p2 = TaskPriority::new(100);
    assert_eq!(p2.value(), TASK_MAX_PRIORITY);

    let p3 = TaskPriority::new(-10);
    assert_eq!(p3.value(), TASK_MIN_PRIORITY);
}

#[tokio::test]
async fn test_task_status_variants() {
    let statuses = vec![
        TaskStatus::Pending,
        TaskStatus::Running,
        TaskStatus::Success,
        TaskStatus::Failure,
        TaskStatus::Retry,
    ];

    for status in statuses {
        // Verify all status variants can be created
        assert!(matches!(
            status,
            TaskStatus::Pending
                | TaskStatus::Running
                | TaskStatus::Success
                | TaskStatus::Failure
                | TaskStatus::Retry
        ));
    }
}

#[tokio::test]
async fn test_task_backends_creation() {
    let backends = TaskBackends::new();
    // TaskBackends is a simple struct in current implementation
    // Just verify it can be created
    let _ = backends;
}

#[tokio::test]
async fn test_dummy_backend_multiple_tasks() {
    let backend = DummyBackend::new();

    let task1 = Box::new(NoopTask::new("task1"));
    let task2 = Box::new(NoopTask::new("task2"));

    let id1 = backend.enqueue(task1).await.unwrap();
    let id2 = backend.enqueue(task2).await.unwrap();

    assert_ne!(id1, id2);

    let status1 = backend.get_status(id1).await.unwrap();
    let status2 = backend.get_status(id2).await.unwrap();

    assert_eq!(status1, TaskStatus::Success);
    assert_eq!(status2, TaskStatus::Success);
}

#[tokio::test]
async fn test_task_executor_trait() {
    let task = NoopTask::new("test");

    // Execute the task
    let result = task.execute().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_task_priority_comparison() {
    let high = TaskPriority::new(9);
    let low = TaskPriority::new(0);

    assert!(high > low);
    assert!(low < high);
    assert_eq!(high, TaskPriority::new(9));
}
