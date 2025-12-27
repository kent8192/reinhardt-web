//! Unit tests for RequestScope and SingletonScope

use reinhardt_di::{RequestScope, SingletonScope};
use rstest::*;
use std::sync::Arc;

// Test type definitions
#[derive(Clone, Debug, PartialEq)]
struct TestData {
	value: String,
}

#[rstest]
fn request_scope_new_creates_empty_scope() {
	// Act
	let scope = RequestScope::new();

	// Assert
	let value: Option<Arc<TestData>> = scope.get();
	assert!(value.is_none());
}

#[rstest]
fn request_scope_set_stores_value() {
	// Arrange
	let scope = RequestScope::new();
	let data = TestData {
		value: "request_data".to_string(),
	};

	// Act
	scope.set(data.clone());

	// Assert
	let retrieved: Option<Arc<TestData>> = scope.get();
	assert!(retrieved.is_some());
	assert_eq!(retrieved.unwrap().value, "request_data");
}

#[rstest]
fn request_scope_get_retrieves_value() {
	// Arrange
	let scope = RequestScope::new();
	let data = TestData {
		value: "test_value".to_string(),
	};
	scope.set(data.clone());

	// Act
	let retrieved: Option<Arc<TestData>> = scope.get();

	// Assert
	assert!(retrieved.is_some());
	assert_eq!(retrieved.unwrap().value, "test_value");
}

#[rstest]
fn request_scope_get_returns_none_for_missing_type() {
	// Arrange
	let scope = RequestScope::new();

	// Act
	let retrieved: Option<Arc<TestData>> = scope.get();

	// Assert
	assert!(retrieved.is_none());
}

#[rstest]
fn singleton_scope_new_creates_empty_scope() {
	// Act
	let scope = SingletonScope::new();

	// Assert
	let value: Option<Arc<TestData>> = scope.get();
	assert!(value.is_none());
}

#[rstest]
fn singleton_scope_set_stores_value() {
	// Arrange
	let scope = SingletonScope::new();
	let data = TestData {
		value: "singleton_data".to_string(),
	};

	// Act
	scope.set(data.clone());

	// Assert
	let retrieved: Option<Arc<TestData>> = scope.get();
	assert!(retrieved.is_some());
	assert_eq!(retrieved.unwrap().value, "singleton_data");
}

#[rstest]
fn singleton_scope_get_retrieves_value() {
	// Arrange
	let scope = SingletonScope::new();
	let data = TestData {
		value: "test_singleton".to_string(),
	};
	scope.set(data.clone());

	// Act
	let retrieved: Option<Arc<TestData>> = scope.get();

	// Assert
	assert!(retrieved.is_some());
	assert_eq!(retrieved.unwrap().value, "test_singleton");
}

#[rstest]
fn singleton_scope_shared_across_requests() {
	// Arrange
	let singleton_scope = Arc::new(SingletonScope::new());
	let data = TestData {
		value: "shared".to_string(),
	};

	// Act
	singleton_scope.set(data.clone());

	// 複数のリクエストで同じsingleton scopeを共有
	let retrieved1: Option<Arc<TestData>> = singleton_scope.get();
	let retrieved2: Option<Arc<TestData>> = singleton_scope.get();

	// Assert
	assert!(retrieved1.is_some());
	assert!(retrieved2.is_some());
	assert_eq!(retrieved1.as_ref().unwrap().value, "shared");
	assert_eq!(retrieved2.as_ref().unwrap().value, "shared");

	// 同じArcを指していることを確認
	assert!(Arc::ptr_eq(
		retrieved1.as_ref().unwrap(),
		retrieved2.as_ref().unwrap()
	));
}

#[rstest]
fn request_scope_isolated_between_requests() {
	// Arrange
	let scope1 = RequestScope::new();
	let scope2 = RequestScope::new();

	let data1 = TestData {
		value: "request1".to_string(),
	};
	let data2 = TestData {
		value: "request2".to_string(),
	};

	// Act
	scope1.set(data1.clone());
	scope2.set(data2.clone());

	// Assert
	let retrieved1: Option<Arc<TestData>> = scope1.get();
	let retrieved2: Option<Arc<TestData>> = scope2.get();

	assert_eq!(retrieved1.unwrap().value, "request1");
	assert_eq!(retrieved2.unwrap().value, "request2");

	// 別々のArcを指していることを確認
	let retrieved1_again: Option<Arc<TestData>> = scope1.get();
	let retrieved2_again: Option<Arc<TestData>> = scope2.get();
	assert!(!Arc::ptr_eq(
		retrieved1_again.as_ref().unwrap(),
		retrieved2_again.as_ref().unwrap()
	));
}
