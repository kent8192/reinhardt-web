//! Unit tests for OverrideRegistry

use reinhardt_di::OverrideRegistry;
use rstest::*;

// Test factory functions
fn factory_a() -> String {
	"production_a".to_string()
}

fn factory_b() -> i32 {
	42
}

#[rstest]
fn override_registry_new_empty() {
	// Act
	let registry = OverrideRegistry::new();

	// Assert
	assert!(registry.is_empty());
	assert_eq!(registry.len(), 0);
}

#[rstest]
fn set_override_stores_value() {
	// Arrange
	let registry = OverrideRegistry::new();
	let func_ptr = factory_a as usize;

	// Act
	registry.set(func_ptr, "override_value".to_string());

	// Assert
	assert!(!registry.is_empty());
	assert_eq!(registry.len(), 1);
	assert!(registry.has(func_ptr));
}

#[rstest]
fn get_override_retrieves_value() {
	// Arrange
	let registry = OverrideRegistry::new();
	let func_ptr = factory_a as usize;

	registry.set(func_ptr, "test_value".to_string());

	// Act
	let value: Option<String> = registry.get(func_ptr);

	// Assert
	assert!(value.is_some());
	assert_eq!(value.unwrap(), "test_value");
}

#[rstest]
fn get_override_returns_none_for_missing() {
	// Arrange
	let registry = OverrideRegistry::new();
	let func_ptr = factory_a as usize;

	// Act
	let value: Option<String> = registry.get(func_ptr);

	// Assert
	assert!(value.is_none());
}

#[rstest]
fn clear_removes_all_overrides() {
	// Arrange
	let registry = OverrideRegistry::new();
	let func_ptr_a = factory_a as usize;
	let func_ptr_b = factory_b as usize;

	registry.set(func_ptr_a, "value_a".to_string());
	registry.set(func_ptr_b, 100);

	assert_eq!(registry.len(), 2);

	// Act
	registry.clear();

	// Assert
	assert!(registry.is_empty());
	assert_eq!(registry.len(), 0);
	assert!(!registry.has(func_ptr_a));
	assert!(!registry.has(func_ptr_b));
}

#[rstest]
fn override_priority_over_normal_resolution() {
	// Arrange
	let registry = OverrideRegistry::new();
	let func_ptr = factory_a as usize;

	// Act - Set override
	registry.set(func_ptr, "override".to_string());

	// Assert - Override takes priority
	let value: Option<String> = registry.get(func_ptr);
	assert_eq!(value, Some("override".to_string()));

	// Different from normal function execution result
	assert_ne!(value.unwrap(), factory_a());
}
