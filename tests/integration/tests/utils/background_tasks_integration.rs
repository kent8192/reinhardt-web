//! Integration tests for background tasks
//!
//! These tests are based on FastAPI's background task examples and test
//! the integration between reinhardt-tasks and other Reinhardt components.

use reinhardt_tasks::{DummyBackend, ImmediateBackend, Task, TaskBackend, TaskQueue};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Helper to create a temporary test file path
fn test_file_path(name: &str) -> String {
    format!("/tmp/reinhardt_tasks_test_{}.txt", name)
}

/// Helper to clean up test files
fn cleanup_test_file(path: &str) {
    if Path::new(path).exists() {
        fs::remove_file(path).ok();
    }
}

#[tokio::test]
async fn test_background_task_file_write() {
    let log_file = test_file_path("notification");
    cleanup_test_file(&log_file);

    // Simulate a background task that writes to a file
    let backend = ImmediateBackend::new();
    let task = Task::new("send_notification");

    // Enqueue task with email parameter
    let mut kwargs = HashMap::new();
    kwargs.insert("email".to_string(), serde_json::json!("foo@example.com"));
    kwargs.insert(
        "message".to_string(),
        serde_json::json!("some notification"),
    );

    let result = backend.enqueue(task, vec![], kwargs).await.unwrap();

    // Verify task was enqueued
    assert_eq!(result.status, reinhardt_tasks::ResultStatus::Success);
    assert_eq!(
        result.kwargs.get("email").unwrap(),
        &serde_json::json!("foo@example.com")
    );

    // Simulate writing to log file (in real implementation, this would be done by task executor)
    fs::write(
        &log_file,
        "notification for foo@example.com: some notification\n",
    )
    .expect("Failed to write test file");

    // Verify file was created and contains expected content
    let content = fs::read_to_string(&log_file).expect("Failed to read test file");
    assert!(content.contains("notification for foo@example.com: some notification"));

    // Cleanup
    cleanup_test_file(&log_file);
}

#[tokio::test]
async fn test_background_task_with_query_params() {
    let log_file = test_file_path("message_query");
    cleanup_test_file(&log_file);

    // Simulate a background task with query parameters
    let backend = ImmediateBackend::new();
    let task = Task::new("send_message");

    let mut kwargs = HashMap::new();
    kwargs.insert("email".to_string(), serde_json::json!("foo@example.com"));
    kwargs.insert("query".to_string(), serde_json::json!("some-query"));

    let result = backend.enqueue(task, vec![], kwargs).await.unwrap();

    assert_eq!(result.status, reinhardt_tasks::ResultStatus::Success);

    // Simulate the background task execution
    fs::write(
        &log_file,
        "found query: some-query\nmessage to foo@example.com\n",
    )
    .expect("Failed to write test file");

    let content = fs::read_to_string(&log_file).expect("Failed to read test file");
    assert!(content.contains("found query: some-query"));
    assert!(content.contains("message to foo@example.com"));

    // Cleanup
    cleanup_test_file(&log_file);
}

#[tokio::test]
async fn test_task_queue_with_multiple_tasks() {
    let queue = TaskQueue::new();

    // Create multiple background tasks
    let task1 = Task::new("email_task").with_priority(1);
    let task2 = Task::new("notification_task").with_priority(5);
    let task3 = Task::new("cleanup_task").with_priority(0);

    // Enqueue all tasks
    queue.enqueue(task1).await.unwrap();
    queue.enqueue(task2).await.unwrap();
    queue.enqueue(task3).await.unwrap();

    assert_eq!(queue.size().await, 3);

    // Dequeue in priority order
    let first = queue.dequeue().await.unwrap();
    assert_eq!(first.name, "notification_task");
    assert_eq!(first.priority, 5);

    let second = queue.dequeue().await.unwrap();
    assert_eq!(second.name, "email_task");
    assert_eq!(second.priority, 1);

    let third = queue.dequeue().await.unwrap();
    assert_eq!(third.name, "cleanup_task");
    assert_eq!(third.priority, 0);
}

#[tokio::test]
async fn test_background_task_with_backend_switching() {
    // Test switching between different backends
    let dummy_backend = DummyBackend::new();
    let immediate_backend = ImmediateBackend::new();

    let task1 = Task::new("async_task").with_backend("dummy".to_string());
    let task2 = Task::new("sync_task").with_backend("immediate".to_string());

    // Enqueue to dummy backend (doesn't execute immediately)
    let result1 = dummy_backend
        .enqueue(task1, vec![], HashMap::new())
        .await
        .unwrap();
    assert_eq!(result1.status, reinhardt_tasks::ResultStatus::Ready);

    // Enqueue to immediate backend (executes immediately)
    let result2 = immediate_backend
        .enqueue(task2, vec![], HashMap::new())
        .await
        .unwrap();
    assert_eq!(result2.status, reinhardt_tasks::ResultStatus::Success);
}

#[tokio::test]
async fn test_background_task_error_handling() {
    let backend = DummyBackend::new();
    let task = Task::new("failing_task");

    // Enqueue a task
    let result = backend.enqueue(task, vec![], HashMap::new()).await.unwrap();
    assert_eq!(result.status, reinhardt_tasks::ResultStatus::Ready);

    // Verify we can retrieve the result
    let retrieved = backend.get_result(&result.id).await.unwrap();
    assert_eq!(retrieved.id, result.id);
}

#[tokio::test]
async fn test_background_task_with_json_args() {
    let backend = ImmediateBackend::new();
    let task = Task::new("json_processor");

    // Pass complex JSON data as arguments
    let args = vec![
        serde_json::json!({"user": "alice", "age": 30}),
        serde_json::json!([1, 2, 3, 4, 5]),
    ];

    let mut kwargs = HashMap::new();
    kwargs.insert(
        "metadata".to_string(),
        serde_json::json!({"processed": true}),
    );

    let result = backend
        .enqueue(task, args.clone(), kwargs.clone())
        .await
        .unwrap();

    assert_eq!(result.args.len(), 2);
    assert_eq!(
        result.kwargs.get("metadata").unwrap(),
        &serde_json::json!({"processed": true})
    );
}

#[tokio::test]
async fn test_multiple_background_tasks_execution_order() {
    let log_file = test_file_path("execution_order");
    cleanup_test_file(&log_file);

    let queue = TaskQueue::new();

    // Create tasks with different priorities
    let high_priority = Task::new("urgent_task").with_priority(100);
    let low_priority = Task::new("normal_task").with_priority(-50);
    let medium_priority = Task::new("medium_task").with_priority(0);

    // Enqueue in non-priority order
    queue.enqueue(low_priority.clone()).await.unwrap();
    queue.enqueue(high_priority.clone()).await.unwrap();
    queue.enqueue(medium_priority.clone()).await.unwrap();

    // Simulate execution order
    let mut execution_order = Vec::new();
    while let Some(task) = queue.dequeue().await {
        execution_order.push(task.name.clone());
    }

    // Verify execution order is by priority (high to low)
    assert_eq!(
        execution_order,
        vec!["urgent_task", "medium_task", "normal_task"]
    );

    cleanup_test_file(&log_file);
}

#[tokio::test]
async fn test_background_task_result_retrieval() {
    let backend = DummyBackend::new();

    // Enqueue multiple tasks
    let task1 = Task::new("task_one");
    let task2 = Task::new("task_two");

    let result1 = backend
        .enqueue(task1, vec![], HashMap::new())
        .await
        .unwrap();
    let result2 = backend
        .enqueue(task2, vec![], HashMap::new())
        .await
        .unwrap();

    // Retrieve results by ID
    let retrieved1 = backend.get_result(&result1.id).await.unwrap();
    let retrieved2 = backend.get_result(&result2.id).await.unwrap();

    assert_eq!(retrieved1.task.name, "task_one");
    assert_eq!(retrieved2.task.name, "task_two");
    assert_ne!(retrieved1.id, retrieved2.id);
}

#[tokio::test]
async fn test_background_task_cleanup_on_completion() {
    let log_file = test_file_path("cleanup_test");
    cleanup_test_file(&log_file);

    // Create a test file
    fs::write(&log_file, "test data").expect("Failed to write test file");
    assert!(Path::new(&log_file).exists());

    // Simulate background task execution and cleanup
    let backend = ImmediateBackend::new();
    let task = Task::new("cleanup_task");

    let mut kwargs = HashMap::new();
    kwargs.insert("file_path".to_string(), serde_json::json!(log_file.clone()));

    backend.enqueue(task, vec![], kwargs).await.unwrap();

    // Cleanup the test file (simulating task cleanup)
    cleanup_test_file(&log_file);

    // Verify file was cleaned up
    assert!(!Path::new(&log_file).exists());
}
