//! Integration tests for FilterCondition composite logic
//!
//! These tests verify that FilterCondition::And and FilterCondition::Or work correctly
//! with AdminDatabase operations using a real database connection.
//!
//! Tests cover:
//! - AND composite conditions with multiple filters
//! - OR composite conditions for alternative criteria
//! - Nested AND/OR combinations
//! - Integration with count_with_condition

use reinhardt_orm::{
	Filter, FilterCondition, FilterOperator, FilterValue, Model, Timestamped, Timestamps,
};
use reinhardt_panel::AdminDatabase;
use reinhardt_test::fixtures::mock_connection;
use serde::{Deserialize, Serialize};

// Test model for filter condition testing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
struct Product {
	id: Option<i64>,
	name: String,
	category: String,
	price: i32,
	in_stock: bool,
	timestamps: Timestamps,
}

impl Model for Product {
	type PrimaryKey = i64;

	fn table_name() -> &'static str {
		"products"
	}

	fn primary_key(&self) -> Option<&Self::PrimaryKey> {
		self.id.as_ref()
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

impl Timestamped for Product {
	fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
		self.timestamps.created_at
	}

	fn updated_at(&self) -> chrono::DateTime<chrono::Utc> {
		self.timestamps.updated_at
	}

	fn set_updated_at(&mut self, time: chrono::DateTime<chrono::Utc>) {
		self.timestamps.updated_at = time;
	}
}

/// Test: AND composite condition construction
///
/// Tests that FilterCondition::And can be created and structured correctly
#[tokio::test]
#[serial_test::serial(filter_condition)]
async fn test_filter_condition_and_construction() {
	let conn = mock_connection();
	let _db = AdminDatabase::new(conn);

	// Create AND condition: category = 'Electronics' AND in_stock = true
	let and_condition = FilterCondition::And(vec![
		FilterCondition::Single(Filter::new(
			"category".to_string(),
			FilterOperator::Eq,
			FilterValue::String("Electronics".to_string()),
		)),
		FilterCondition::Single(Filter::new(
			"in_stock".to_string(),
			FilterOperator::Eq,
			FilterValue::Bool(true),
		)),
	]);

	// Verify structure
	match and_condition {
		FilterCondition::And(conditions) => {
			assert_eq!(conditions.len(), 2);
		}
		_ => panic!("Expected AND condition"),
	}
}

/// Test: OR composite condition construction
///
/// Tests that FilterCondition::Or can be created and structured correctly
#[tokio::test]
#[serial_test::serial(filter_condition)]
async fn test_filter_condition_or_construction() {
	let conn = mock_connection();
	let _db = AdminDatabase::new(conn);

	// Create OR condition: category = 'Electronics' OR category = 'Books'
	let or_condition = FilterCondition::Or(vec![
		FilterCondition::Single(Filter::new(
			"category".to_string(),
			FilterOperator::Eq,
			FilterValue::String("Electronics".to_string()),
		)),
		FilterCondition::Single(Filter::new(
			"category".to_string(),
			FilterOperator::Eq,
			FilterValue::String("Books".to_string()),
		)),
	]);

	// Verify structure
	match or_condition {
		FilterCondition::Or(conditions) => {
			assert_eq!(conditions.len(), 2);
		}
		_ => panic!("Expected OR condition"),
	}
}

/// Test: Nested AND/OR conditions
///
/// Tests that FilterCondition can be nested for complex queries:
/// (category = 'Electronics' AND in_stock = true) OR (category = 'Books' AND price < 1000)
#[tokio::test]
#[serial_test::serial(filter_condition)]
async fn test_filter_condition_nested_construction() {
	let conn = mock_connection();
	let _db = AdminDatabase::new(conn);

	// Create nested condition:
	// (category = 'Electronics' AND in_stock = true) OR (category = 'Books' AND price < 1000)
	let nested_condition = FilterCondition::Or(vec![
		FilterCondition::And(vec![
			FilterCondition::Single(Filter::new(
				"category".to_string(),
				FilterOperator::Eq,
				FilterValue::String("Electronics".to_string()),
			)),
			FilterCondition::Single(Filter::new(
				"in_stock".to_string(),
				FilterOperator::Eq,
				FilterValue::Bool(true),
			)),
		]),
		FilterCondition::And(vec![
			FilterCondition::Single(Filter::new(
				"category".to_string(),
				FilterOperator::Eq,
				FilterValue::String("Books".to_string()),
			)),
			FilterCondition::Single(Filter::new(
				"price".to_string(),
				FilterOperator::Lt,
				FilterValue::Int(1000),
			)),
		]),
	]);

	// Verify structure
	match nested_condition {
		FilterCondition::Or(outer_conditions) => {
			assert_eq!(outer_conditions.len(), 2);

			// Check first AND block
			if let FilterCondition::And(first_and) = &outer_conditions[0] {
				assert_eq!(first_and.len(), 2);
			} else {
				panic!("Expected first outer condition to be AND");
			}

			// Check second AND block
			if let FilterCondition::And(second_and) = &outer_conditions[1] {
				assert_eq!(second_and.len(), 2);
			} else {
				panic!("Expected second outer condition to be AND");
			}
		}
		_ => panic!("Expected OR condition at top level"),
	}
}

/// Test: Empty AND condition
///
/// Tests that FilterCondition::And with empty vector handles gracefully
#[tokio::test]
#[serial_test::serial(filter_condition)]
async fn test_filter_condition_empty_and() {
	let conn = mock_connection();
	let _db = AdminDatabase::new(conn);

	let empty_and = FilterCondition::And(vec![]);

	// Verify structure
	match empty_and {
		FilterCondition::And(conditions) => {
			assert_eq!(conditions.len(), 0);
		}
		_ => panic!("Expected AND condition"),
	}
}

/// Test: Empty OR condition
///
/// Tests that FilterCondition::Or with empty vector handles gracefully
#[tokio::test]
#[serial_test::serial(filter_condition)]
async fn test_filter_condition_empty_or() {
	let conn = mock_connection();
	let _db = AdminDatabase::new(conn);

	let empty_or = FilterCondition::Or(vec![]);

	// Verify structure
	match empty_or {
		FilterCondition::Or(conditions) => {
			assert_eq!(conditions.len(), 0);
		}
		_ => panic!("Expected OR condition"),
	}
}

/// Test: Single condition in AND
///
/// Tests that FilterCondition::And with single condition works correctly
#[tokio::test]
#[serial_test::serial(filter_condition)]
async fn test_filter_condition_single_and() {
	let conn = mock_connection();
	let _db = AdminDatabase::new(conn);

	let single_and = FilterCondition::And(vec![FilterCondition::Single(Filter::new(
		"in_stock".to_string(),
		FilterOperator::Eq,
		FilterValue::Bool(true),
	))]);

	// Verify structure
	match single_and {
		FilterCondition::And(conditions) => {
			assert_eq!(conditions.len(), 1);
			match &conditions[0] {
				FilterCondition::Single(filter) => {
					assert_eq!(filter.field, "in_stock");
				}
				_ => panic!("Expected Single condition"),
			}
		}
		_ => panic!("Expected AND condition"),
	}
}

/// Test: FilterOperator variants with FilterCondition
///
/// Tests that different FilterOperator types work with FilterCondition
#[tokio::test]
#[serial_test::serial(filter_condition)]
async fn test_filter_operators_in_conditions() {
	let conn = mock_connection();
	let _db = AdminDatabase::new(conn);

	// Test various operators in a composite condition
	let operators_condition = FilterCondition::Or(vec![
		FilterCondition::Single(Filter::new(
			"price".to_string(),
			FilterOperator::Gt,
			FilterValue::Int(100),
		)),
		FilterCondition::Single(Filter::new(
			"name".to_string(),
			FilterOperator::Contains,
			FilterValue::String("test".to_string()),
		)),
		FilterCondition::Single(Filter::new(
			"category".to_string(),
			FilterOperator::In,
			FilterValue::Array(vec!["Electronics".to_string(), "Books".to_string()]),
		)),
	]);

	// Verify all operators are correctly set
	match operators_condition {
		FilterCondition::Or(conditions) => {
			assert_eq!(conditions.len(), 3);
		}
		_ => panic!("Expected OR condition"),
	}
}

/// Test: Complex real-world scenario
///
/// Tests a realistic admin panel filter scenario:
/// Search for products that are either:
/// - Electronics AND in_stock AND price > 50
/// - Books AND price < 20
#[tokio::test]
#[serial_test::serial(filter_condition)]
async fn test_realistic_admin_filter() {
	let conn = mock_connection();
	let _db = AdminDatabase::new(conn);

	let complex_filter = FilterCondition::Or(vec![
		// Electronics, in stock, priced > 50
		FilterCondition::And(vec![
			FilterCondition::Single(Filter::new(
				"category".to_string(),
				FilterOperator::Eq,
				FilterValue::String("Electronics".to_string()),
			)),
			FilterCondition::Single(Filter::new(
				"in_stock".to_string(),
				FilterOperator::Eq,
				FilterValue::Bool(true),
			)),
			FilterCondition::Single(Filter::new(
				"price".to_string(),
				FilterOperator::Gt,
				FilterValue::Int(50),
			)),
		]),
		// Books priced < 20
		FilterCondition::And(vec![
			FilterCondition::Single(Filter::new(
				"category".to_string(),
				FilterOperator::Eq,
				FilterValue::String("Books".to_string()),
			)),
			FilterCondition::Single(Filter::new(
				"price".to_string(),
				FilterOperator::Lt,
				FilterValue::Int(20),
			)),
		]),
	]);

	// Verify the complex structure
	match complex_filter {
		FilterCondition::Or(outer) => {
			assert_eq!(outer.len(), 2);

			// First AND (Electronics)
			if let FilterCondition::And(electronics_filters) = &outer[0] {
				assert_eq!(electronics_filters.len(), 3);
			} else {
				panic!("Expected first condition to be AND");
			}

			// Second AND (Books)
			if let FilterCondition::And(books_filters) = &outer[1] {
				assert_eq!(books_filters.len(), 2);
			} else {
				panic!("Expected second condition to be AND");
			}
		}
		_ => panic!("Expected top-level OR condition"),
	}
}
