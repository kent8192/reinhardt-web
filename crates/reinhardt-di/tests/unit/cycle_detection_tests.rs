//! Unit tests for cycle detection mechanism

use reinhardt_di::cycle_detection::{begin_resolution, register_type_name, CycleError};
use rstest::*;
use std::any::TypeId;

// Test type definitions
struct TypeA;
struct TypeB;
struct TypeC;

#[rstest]
fn resolution_guard_begins_tracking() {
	// Arrange
	let type_id = TypeId::of::<TypeA>();
	register_type_name::<TypeA>("TypeA");

	// Act
	let result = begin_resolution(type_id, "TypeA");

	// Assert
	assert!(result.is_ok());
}

#[rstest]
fn resolution_guard_drops_removes_tracking() {
	// Arrange
	let type_id = TypeId::of::<TypeA>();
	register_type_name::<TypeA>("TypeA");

	// Act - Create guard
	let guard = begin_resolution(type_id, "TypeA").unwrap();

	// Drop guard
	drop(guard);

	// Should be able to resolve the same type again
	let result = begin_resolution(type_id, "TypeA");

	// Assert
	assert!(result.is_ok());
}

#[rstest]
fn begin_resolution_detects_cycle() {
	// Arrange
	let type_id = TypeId::of::<TypeA>();
	register_type_name::<TypeA>("TypeA");

	// Act - Begin first resolution
	let _guard = begin_resolution(type_id, "TypeA").unwrap();

	// Attempting to resolve the same type again should cause a circular dependency error
	let result = begin_resolution(type_id, "TypeA");

	// Assert
	assert!(result.is_err());
	match result {
		Err(CycleError::CircularDependency { type_name, path }) => {
			assert_eq!(type_name, "TypeA");
			assert!(path.contains("TypeA"));
		}
		_ => panic!("Expected CircularDependency error"),
	}
}

#[rstest]
fn register_type_name_stores_name() {
	// Act
	register_type_name::<TypeA>("TypeA");
	register_type_name::<TypeB>("TypeB");
	register_type_name::<TypeC>("TypeC");

	// Assert - Verify type name is registered (can be resolved without error)
	let type_id_a = TypeId::of::<TypeA>();
	let result_a = begin_resolution(type_id_a, "TypeA");
	assert!(result_a.is_ok());
}

#[rstest]
fn cycle_error_contains_path() {
	// Arrange
	let type_id_a = TypeId::of::<TypeA>();
	let type_id_b = TypeId::of::<TypeB>();
	register_type_name::<TypeA>("TypeA");
	register_type_name::<TypeB>("TypeB");

	// Act - Create A -> B -> A circular dependency
	let _guard_a = begin_resolution(type_id_a, "TypeA").unwrap();
	let _guard_b = begin_resolution(type_id_b, "TypeB").unwrap();

	// Attempting to resolve A again should cause circular dependency error
	let result = begin_resolution(type_id_a, "TypeA");

	// Assert
	assert!(result.is_err());
	match result {
		Err(CycleError::CircularDependency { type_name, path }) => {
			assert_eq!(type_name, "TypeA");
			assert!(path.contains("TypeA"));
			assert!(path.contains("TypeB"));
			assert!(path.contains("->"));
		}
		_ => panic!("Expected CircularDependency error with path"),
	}
}

#[rstest]
fn performance_overhead_minimal() {
	// Arrange
	let type_id = TypeId::of::<TypeA>();
	register_type_name::<TypeA>("TypeA");

	// Act - Simulate multiple resolutions
	let iterations = 1000;
	let start = std::time::Instant::now();

	for _ in 0..iterations {
		let _guard = begin_resolution(type_id, "TypeA").unwrap();
		drop(_guard);
	}

	let duration = start.elapsed();

	// Assert - Verify 1000 resolutions complete within 100ms
	assert!(
		duration.as_millis() < 100,
		"Performance overhead too high: {:?}",
		duration
	);
}
