//! Tests for hybrid property comparators
//! Based on PropertyComparatorTest from SQLAlchemy

use reinhardt_db::hybrid::prelude::*;

/// Test model representing a database entity
#[derive(Debug)]
struct TestModel {
	id: i32,
	value: String,
}

impl TestModel {
	fn new(id: i32, value: String) -> Self {
		Self { id, value }
	}
}

#[test]
fn test_hybrid_property_comparator_set_get() {
	// Test basic getter and setter functionality
	let model = TestModel::new(1, "test_value".to_string());

	let property = HybridProperty::new(|m: &TestModel| m.value.clone());

	assert_eq!(property.get(&model), "test_value");
}

#[test]
fn test_hybrid_comparator_value_expression() {
	// Test that hybrid property can have both instance and expression getter
	let model = TestModel::new(1, "test".to_string());

	let property = HybridProperty::new(|m: &TestModel| m.value.to_uppercase())
		.with_expression(|| "UPPER(value)".to_string());

	assert_eq!(property.get(&model), "TEST");
	assert_eq!(property.expression(), Some("UPPER(value)".to_string()));
}

#[test]
fn test_comparator_uppercase() {
	// Test custom comparator that converts to uppercase
	let comparator = UpperCaseComparator::new("a.value".to_string());

	// Test equality comparison
	let sql = comparator.eq("'test'");
	assert_eq!(sql, "UPPER(a.value) = UPPER('test')");
}

#[test]
fn test_comparator_inequality() {
	// Test inequality comparison
	let comparator = UpperCaseComparator::new("a.value".to_string());

	let sql = comparator.ne("'test'");
	assert!(sql.contains("NOT"));
	assert!(sql.contains("UPPER(a.value)"));
	assert!(sql.contains("UPPER('test')"));
}

#[test]
fn test_comparator_less_than() {
	// Test less than comparison
	let comparator = UpperCaseComparator::new("a.value".to_string());

	let sql = comparator.lt("'test'");
	assert_eq!(sql, "UPPER(a.value) < UPPER('test')");
}

#[test]
fn test_comparator_greater_than() {
	// Test greater than comparison
	let comparator = UpperCaseComparator::new("a.value".to_string());

	let sql = comparator.gt("'test'");
	assert_eq!(sql, "UPPER(a.value) > UPPER('test')");
}

#[test]
fn test_property_with_transformation() {
	// Test property that transforms the value (like SQLAlchemy's value - 5)
	let model = TestModel::new(1, "10".to_string());

	let property = HybridProperty::new(|m: &TestModel| m.value.parse::<i32>().unwrap_or(0) - 5);

	assert_eq!(property.get(&model), 5);
}

#[test]
fn test_property_with_sql_expression() {
	// Test property with SQL expression for database queries
	let property = HybridProperty::new(|m: &TestModel| m.value.clone())
		.with_expression(|| "table.value".to_string());

	assert_eq!(property.expression(), Some("table.value".to_string()));
}

#[test]
fn test_multiple_properties() {
	// Test multiple properties on the same model
	let model = TestModel::new(1, "test".to_string());

	let prop1 = HybridProperty::new(|m: &TestModel| m.id);
	let prop2 = HybridProperty::new(|m: &TestModel| m.value.clone());

	assert_eq!(prop1.get(&model), 1);
	assert_eq!(prop2.get(&model), "test");
}

#[test]
fn test_expression_returns_none_when_not_set() {
	// Test that expression returns None when not configured
	let property = HybridProperty::new(|m: &TestModel| m.value.clone());

	assert_eq!(property.expression(), None);
}

#[test]
fn test_comparator_with_null() {
	// Test comparator handling of NULL values
	let comparator = UpperCaseComparator::new("a.value".to_string());

	let sql = comparator.eq("NULL");
	assert_eq!(sql, "UPPER(a.value) = UPPER(NULL)");
}

#[test]
fn test_property_chaining() {
	// Test that property can be used in a chain of operations
	let model = TestModel::new(1, "hello".to_string());

	let property = HybridProperty::new(|m: &TestModel| m.value.to_uppercase());

	let result = property.get(&model);
	assert_eq!(result, "HELLO");
	assert_eq!(result.len(), 5);
}
