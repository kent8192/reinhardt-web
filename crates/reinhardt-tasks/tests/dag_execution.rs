//! DAG (Directed Acyclic Graph) task execution tests
//!
//! Tests DAG task dependencies, topological sorting, parallel execution,
//! failure handling, and cycle detection.

use reinhardt_tasks::{TaskDAG, TaskId, TaskNodeStatus};
use rstest::rstest;

/// Test: Basic DAG creation and task addition
#[rstest]
fn test_dag_creation() {
	let mut dag = TaskDAG::new();
	assert_eq!(dag.task_count(), 0);

	let task_a = TaskId::new();
	let task_b = TaskId::new();

	dag.add_task(task_a).unwrap();
	dag.add_task(task_b).unwrap();

	assert_eq!(dag.task_count(), 2);
	assert!(dag.get_task(task_a).is_some());
	assert!(dag.get_task(task_b).is_some());
}

/// Test: Simple dependency chain (A → B → C)
#[rstest]
fn test_simple_dependency_chain() {
	let mut dag = TaskDAG::new();

	let task_a = TaskId::new();
	let task_b = TaskId::new();
	let task_c = TaskId::new();

	dag.add_task(task_a).unwrap();
	dag.add_task(task_b).unwrap();
	dag.add_task(task_c).unwrap();

	// B depends on A, C depends on B
	dag.add_dependency(task_b, task_a).unwrap();
	dag.add_dependency(task_c, task_b).unwrap();

	// Topological sort should respect dependencies
	let order = dag.topological_sort().unwrap();
	assert_eq!(order.len(), 3);

	let a_pos = order.iter().position(|&id| id == task_a).unwrap();
	let b_pos = order.iter().position(|&id| id == task_b).unwrap();
	let c_pos = order.iter().position(|&id| id == task_c).unwrap();

	assert!(a_pos < b_pos, "A should come before B");
	assert!(b_pos < c_pos, "B should come before C");
}

/// Test: Diamond dependency pattern (A → B, A → C, B → D, C → D)
#[rstest]
fn test_diamond_dependency() {
	let mut dag = TaskDAG::new();

	let task_a = TaskId::new();
	let task_b = TaskId::new();
	let task_c = TaskId::new();
	let task_d = TaskId::new();

	dag.add_task(task_a).unwrap();
	dag.add_task(task_b).unwrap();
	dag.add_task(task_c).unwrap();
	dag.add_task(task_d).unwrap();

	// Diamond: A → B → D, A → C → D
	dag.add_dependency(task_b, task_a).unwrap();
	dag.add_dependency(task_c, task_a).unwrap();
	dag.add_dependency(task_d, task_b).unwrap();
	dag.add_dependency(task_d, task_c).unwrap();

	let order = dag.topological_sort().unwrap();
	assert_eq!(order.len(), 4);

	let a_pos = order.iter().position(|&id| id == task_a).unwrap();
	let b_pos = order.iter().position(|&id| id == task_b).unwrap();
	let c_pos = order.iter().position(|&id| id == task_c).unwrap();
	let d_pos = order.iter().position(|&id| id == task_d).unwrap();

	// A must come before B and C
	assert!(a_pos < b_pos);
	assert!(a_pos < c_pos);
	// B and C must both come before D
	assert!(b_pos < d_pos);
	assert!(c_pos < d_pos);
}

/// Test: Get ready tasks (tasks with satisfied dependencies)
#[rstest]
fn test_get_ready_tasks() {
	let mut dag = TaskDAG::new();

	let task_a = TaskId::new();
	let task_b = TaskId::new();
	let task_c = TaskId::new();

	dag.add_task(task_a).unwrap();
	dag.add_task(task_b).unwrap();
	dag.add_task(task_c).unwrap();

	// B depends on A, C depends on B
	dag.add_dependency(task_b, task_a).unwrap();
	dag.add_dependency(task_c, task_b).unwrap();

	// Initially, only task_a is ready (no dependencies)
	let ready = dag.get_ready_tasks();
	assert_eq!(ready.len(), 1);
	assert!(ready.contains(&task_a));

	// After completing task_a, task_b becomes ready
	dag.mark_completed(task_a).unwrap();
	let ready = dag.get_ready_tasks();
	assert_eq!(ready.len(), 1);
	assert!(ready.contains(&task_b));

	// After completing task_b, task_c becomes ready
	dag.mark_completed(task_b).unwrap();
	let ready = dag.get_ready_tasks();
	assert_eq!(ready.len(), 1);
	assert!(ready.contains(&task_c));

	// After completing task_c, no tasks are ready
	dag.mark_completed(task_c).unwrap();
	let ready = dag.get_ready_tasks();
	assert_eq!(ready.len(), 0);
}

/// Test: Cycle detection (A → B → C → A)
#[rstest]
fn test_cycle_detection() {
	let mut dag = TaskDAG::new();

	let task_a = TaskId::new();
	let task_b = TaskId::new();
	let task_c = TaskId::new();

	dag.add_task(task_a).unwrap();
	dag.add_task(task_b).unwrap();
	dag.add_task(task_c).unwrap();

	// A → B → C
	dag.add_dependency(task_b, task_a).unwrap();
	dag.add_dependency(task_c, task_b).unwrap();

	// Try to create cycle: C → A
	let result = dag.add_dependency(task_a, task_c);
	assert!(result.is_err(), "Should detect cycle");
	assert!(
		result.unwrap_err().to_string().contains("Cycle"),
		"Error should mention cycle"
	);
}

/// Test: Task status transitions
#[rstest]
fn test_task_status_transitions() {
	let mut dag = TaskDAG::new();
	let task_id = TaskId::new();

	dag.add_task(task_id).unwrap();

	// Initial status should be Pending
	let task = dag.get_task(task_id).unwrap();
	assert_eq!(task.status(), TaskNodeStatus::Pending);

	// Mark as running
	dag.mark_running(task_id).unwrap();
	let task = dag.get_task(task_id).unwrap();
	assert_eq!(task.status(), TaskNodeStatus::Running);

	// Mark as completed
	dag.mark_completed(task_id).unwrap();
	let task = dag.get_task(task_id).unwrap();
	assert_eq!(task.status(), TaskNodeStatus::Completed);
}

/// Test: Failure propagation (when a task fails, dependent tasks should not run)
#[rstest]
fn test_failure_handling() {
	let mut dag = TaskDAG::new();

	let task_a = TaskId::new();
	let task_b = TaskId::new();
	let task_c = TaskId::new();

	dag.add_task(task_a).unwrap();
	dag.add_task(task_b).unwrap();
	dag.add_task(task_c).unwrap();

	// B depends on A, C depends on B
	dag.add_dependency(task_b, task_a).unwrap();
	dag.add_dependency(task_c, task_b).unwrap();

	// Only task_a is ready initially
	let ready = dag.get_ready_tasks();
	assert_eq!(ready.len(), 1);
	assert!(ready.contains(&task_a));

	// Mark task_a as failed
	dag.mark_failed(task_a).unwrap();
	let task = dag.get_task(task_a).unwrap();
	assert_eq!(task.status(), TaskNodeStatus::Failed);

	// task_b should still not be ready (dependency failed)
	// Note: This implementation doesn't automatically propagate failure,
	// it's the executor's responsibility to check dependency status
	let ready = dag.get_ready_tasks();
	assert_eq!(
		ready.len(),
		0,
		"No tasks should be ready when dependency failed"
	);
}

/// Test: Parallel execution readiness (multiple independent tasks)
#[rstest]
fn test_parallel_execution_readiness() {
	let mut dag = TaskDAG::new();

	let task_a = TaskId::new();
	let task_b = TaskId::new();
	let task_c = TaskId::new();
	let task_d = TaskId::new();

	dag.add_task(task_a).unwrap();
	dag.add_task(task_b).unwrap();
	dag.add_task(task_c).unwrap();
	dag.add_task(task_d).unwrap();

	// A and B are independent, both depend on nothing
	// C and D both depend on A and B
	dag.add_dependency(task_c, task_a).unwrap();
	dag.add_dependency(task_c, task_b).unwrap();
	dag.add_dependency(task_d, task_a).unwrap();
	dag.add_dependency(task_d, task_b).unwrap();

	// Initially, A and B are ready (can run in parallel)
	let ready = dag.get_ready_tasks();
	assert_eq!(ready.len(), 2);
	assert!(ready.contains(&task_a));
	assert!(ready.contains(&task_b));

	// Complete both A and B
	dag.mark_completed(task_a).unwrap();
	dag.mark_completed(task_b).unwrap();

	// Now C and D are ready (can run in parallel)
	let ready = dag.get_ready_tasks();
	assert_eq!(ready.len(), 2);
	assert!(ready.contains(&task_c));
	assert!(ready.contains(&task_d));
}
