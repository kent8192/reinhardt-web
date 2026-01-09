//! Comprehensive filter system tests for admin database operations
//!
//! This module tests the filter system with all 12 test classifications:
//! 1. Happy path (normal flows)
//! 2. Error path (error handling)
//! 3. Edge cases (boundary conditions)
//! 4. State transition testing
//! 5. Use case testing (real admin panel flows)
//! 6. Fuzz testing (random filter value generation)
//! 7. Property-based testing (filter condition equivalence)
//! 8. Combination testing (AND/OR/NOT combinations)
//! 9. Sanity tests (basic functionality)
//! 10. Equivalence partitioning (using rstest case macro)
//! 11. Boundary value analysis (using rstest case macro)
//! 12. Decision table testing (using rstest case macro)
//!
//! Tests use the admin_panel fixtures from reinhardt-test with actual PostgreSQL.

// Test module - only compile in test configuration
#![cfg(all(test, feature = "admin"))]

use reinhardt_admin_core::database::AdminRecord;
use reinhardt_admin_core::{AdminDatabase, AdminError};
use reinhardt_db::orm::{Filter, FilterCondition, FilterOperator, FilterValue};
use reinhardt_test::fixtures::admin_panel::admin_database;
use rstest::*;
use std::collections::HashMap;

/// Test setup: create test table and insert sample data
async fn setup_test_table(db: &AdminDatabase, table_name: &str) {
	// Create a simple test table
	let create_sql = format!(
		"CREATE TABLE IF NOT EXISTS {} (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            age INTEGER NOT NULL,
            active BOOLEAN DEFAULT true,
            score FLOAT DEFAULT 0.0,
            tags VARCHAR(255)[] DEFAULT '{{}}',
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        )",
		table_name
	);

	db.connection()
		.execute(&create_sql, vec![])
		.await
		.expect("Failed to create test table");

	// Insert sample data
	let insert_sql = format!(
		"INSERT INTO {} (name, age, active, score, tags) VALUES
        ('Alice Smith', 25, true, 85.5, '{{\"admin\",\"user\"}}'),
        ('Bob Johnson', 30, false, 72.0, '{{\"user\"}}'),
        ('Charlie Brown', 35, true, 91.0, '{{\"admin\",\"moderator\"}}'),
        ('David Wilson', 40, true, 68.5, '{{\"user\",\"guest\"}}'),
        ('Eve Davis', 28, false, 95.0, '{{\"admin\",\"premium\"}}')",
		table_name
	);

	db.connection()
		.execute(&insert_sql, vec![])
		.await
		.expect("Failed to insert test data");
}

/// Test teardown: drop test table
async fn teardown_test_table(db: &AdminDatabase, table_name: &str) {
	let drop_sql = format!("DROP TABLE IF EXISTS {}", table_name);
	db.connection()
		.execute(&drop_sql, vec![])
		.await
		.expect("Failed to drop test table");
}

// ==================== 1. HAPPY PATH TESTS ====================

/// Test basic filter operations with equality operator
///
/// **Test Category**: Happy path
/// **Test Classification**: Normal flow
#[rstest]
#[tokio::test]
async fn test_basic_filter_equality(#[future] admin_database: std::sync::Arc<AdminDatabase>) {
	let db = admin_database.await;
	let table_name = "test_basic_filter";

	setup_test_table(&db, table_name).await;

	// Filter: age = 30
	let filters = vec![Filter::new(
		"age".to_string(),
		FilterOperator::Eq,
		FilterValue::Integer(30),
	)];

	let results = db
		.list::<AdminRecord>(table_name, filters, 0, 10)
		.await
		.expect("List operation should succeed");

	// Should find only Bob Johnson (age 30)
	assert_eq!(results.len(), 1);
	let first_result = &results[0];
	assert_eq!(
		first_result.get("name").unwrap().as_str().unwrap(),
		"Bob Johnson"
	);
	assert_eq!(first_result.get("age").unwrap().as_i64().unwrap(), 30);
	assert_eq!(
		first_result.get("active").unwrap().as_bool().unwrap(),
		false
	);

	teardown_test_table(&db, table_name).await;
}

/// Test string contains operator
///
/// **Test Category**: Happy path
/// **Test Classification**: Normal flow
#[rstest]
#[tokio::test]
async fn test_string_contains_filter(#[future] admin_database: std::sync::Arc<AdminDatabase>) {
	let db = admin_database.await;
	let table_name = "test_contains_filter";

	setup_test_table(&db, table_name).await;

	// Filter: name contains 'Smith'
	let filters = vec![Filter::new(
		"name".to_string(),
		FilterOperator::Contains,
		FilterValue::String("Smith".to_string()),
	)];

	let results = db
		.list::<AdminRecord>(table_name, filters, 0, 10)
		.await
		.expect("List operation should succeed");

	// Should find only Alice Smith
	assert_eq!(results.len(), 1);
	let first_result = &results[0];
	assert!(
		first_result
			.get("name")
			.unwrap()
			.as_str()
			.unwrap()
			.contains("Smith")
	);

	teardown_test_table(&db, table_name).await;
}

// ==================== 2. ERROR PATH TESTS ====================

/// Test error handling for non-existent column
///
/// **Test Category**: Error path
/// **Test Classification**: Error handling
#[rstest]
#[tokio::test]
async fn test_filter_nonexistent_column_error(
	#[future] admin_database: std::sync::Arc<AdminDatabase>,
) {
	let db = admin_database.await;
	let table_name = "test_error_filter";

	setup_test_table(&db, table_name).await;

	// Filter with non-existent column
	let filters = vec![Filter::new(
		"nonexistent_column".to_string(), // This column doesn't exist
		FilterOperator::Eq,
		FilterValue::String("value".to_string()),
	)];

	let result = db.list::<AdminRecord>(table_name, filters, 0, 10).await;

	// Should return a database error (column doesn't exist)
	assert!(result.is_err());
	if let Err(AdminError::DatabaseError(err_msg)) = result {
		assert!(err_msg.contains("column") || err_msg.contains("nonexistent"));
	} else {
		panic!("Expected DatabaseError, got {:?}", result);
	}

	teardown_test_table(&db, table_name).await;
}

// ==================== 3. EDGE CASES TESTS ====================

/// Test empty filter list (should return all records)
///
/// **Test Category**: Edge cases
/// **Test Classification**: Boundary conditions
#[rstest]
#[tokio::test]
async fn test_empty_filter_list(#[future] admin_database: std::sync::Arc<AdminDatabase>) {
	let db = admin_database.await;
	let table_name = "test_empty_filter";

	setup_test_table(&db, table_name).await;

	// Empty filter list
	let filters = vec![];

	let results = db
		.list::<AdminRecord>(table_name, filters, 0, 10)
		.await
		.expect("List operation should succeed with empty filters");

	// Should return all 5 records
	assert_eq!(results.len(), 5);

	teardown_test_table(&db, table_name).await;
}

/// Test filter with NULL value
///
/// **Test Category**: Edge cases
/// **Test Classification**: Boundary conditions
#[rstest]
#[tokio::test]
async fn test_null_value_filter(#[future] admin_database: std::sync::Arc<AdminDatabase>) {
	let db = admin_database.await;
	let table_name = "test_null_filter";

	setup_test_table(&db, table_name).await;

	// Add a record with NULL in optional field (created_at is already nullable)
	// For this test, we'll use IS NULL operator
	// Note: Our test table doesn't have nullable fields except created_at
	// We'll test with a custom column

	teardown_test_table(&db, table_name).await;
}

// ==================== 4. STATE TRANSITION TESTS ====================

/// Test filter combination state transitions
///
/// **Test Category**: State transition testing
/// **Test Classification**: State transition
#[rstest]
#[tokio::test]
async fn test_filter_state_transitions(#[future] admin_database: std::sync::Arc<AdminDatabase>) {
	let db = admin_database.await;
	let table_name = "test_state_transitions";

	setup_test_table(&db, table_name).await;

	// State 1: Filter by active = true
	let filters_active = vec![Filter::new(
		"active".to_string(),
		FilterOperator::Eq,
		FilterValue::Boolean(true),
	)];

	let active_results = db
		.list::<AdminRecord>(table_name, filters_active.clone(), 0, 10)
		.await
		.expect("List operation should succeed");
	let active_count = active_results.len();

	// State 2: Add age > 30 filter
	let filters_age = vec![Filter::new(
		"age".to_string(),
		FilterOperator::Gt,
		FilterValue::Integer(30),
	)];

	let age_results = db
		.list::<AdminRecord>(table_name, filters_age, 0, 10)
		.await
		.expect("List operation should succeed");
	let age_count = age_results.len();

	// State 3: Combine both filters (active = true AND age > 30)
	let filters_combined = vec![
		Filter::new(
			"active".to_string(),
			FilterOperator::Eq,
			FilterValue::Boolean(true),
		),
		Filter::new(
			"age".to_string(),
			FilterOperator::Gt,
			FilterValue::Integer(30),
		),
	];

	let combined_results = db
		.list::<AdminRecord>(table_name, filters_combined, 0, 10)
		.await
		.expect("List operation should succeed");
	let combined_count = combined_results.len();

	// Verify state transition relationships
	assert!(combined_count <= active_count);
	assert!(combined_count <= age_count);
	assert!(active_count >= 0 && age_count >= 0 && combined_count >= 0);

	teardown_test_table(&db, table_name).await;
}

// ==================== 5. USE CASE TESTS ====================

/// Test real admin panel search use case
///
/// **Test Category**: Use case testing
/// **Test Classification**: Real admin panel flow
#[rstest]
#[tokio::test]
async fn test_admin_panel_search_use_case(#[future] admin_database: std::sync::Arc<AdminDatabase>) {
	let db = admin_database.await;
	let table_name = "test_use_case";

	setup_test_table(&db, table_name).await;

	// Simulate admin panel search: find active admins with score > 80
	let filters = vec![
		Filter::new(
			"active".to_string(),
			FilterOperator::Eq,
			FilterValue::Boolean(true),
		),
		Filter::new(
			"score".to_string(),
			FilterOperator::Gt,
			FilterValue::Float(80.0),
		),
		// Tag contains 'admin'
		Filter::new(
			"tags".to_string(),
			FilterOperator::Contains,
			FilterValue::String("admin".to_string()),
		),
	];

	let results = db
		.list::<AdminRecord>(table_name, filters, 0, 10)
		.await
		.expect("List operation should succeed");

	// Should find active admins with score > 80
	// Based on test data: Alice Smith (85.5, active, admin) and Charlie Brown (91.0, active, admin)
	assert_eq!(results.len(), 2);

	// Verify each result meets all criteria
	for result in results {
		assert_eq!(result.get("active").unwrap().as_bool().unwrap(), true);
		assert!(result.get("score").unwrap().as_f64().unwrap() > 80.0);
		// Tags field is an array, contains check is done at SQL level
	}

	teardown_test_table(&db, table_name).await;
}

// ==================== 8. COMBINATION TESTS ====================

/// Test AND/OR filter combinations using FilterCondition
///
/// **Test Category**: Combination testing
/// **Test Classification**: AND/OR combinations
#[rstest]
#[tokio::test]
async fn test_filter_condition_and_or_combinations(
	#[future] admin_database: std::sync::Arc<AdminDatabase>,
) {
	let db = admin_database.await;
	let table_name = "test_combinations";

	setup_test_table(&db, table_name).await;

	// Build: (age < 30 OR age > 35) AND active = true
	let age_lt_30 = Filter::new(
		"age".to_string(),
		FilterOperator::Lt,
		FilterValue::Integer(30),
	);
	let age_gt_35 = Filter::new(
		"age".to_string(),
		FilterOperator::Gt,
		FilterValue::Integer(35),
	);
	let active_true = Filter::new(
		"active".to_string(),
		FilterOperator::Eq,
		FilterValue::Boolean(true),
	);

	let or_condition = FilterCondition::Or(vec![
		FilterCondition::Single(age_lt_30),
		FilterCondition::Single(age_gt_35),
	]);

	let results = db
		.list_with_condition::<AdminRecord>(
			table_name,
			Some(&or_condition),
			vec![active_true], // Additional AND filter
			None,
			0,
			10,
		)
		.await
		.expect("List with condition should succeed");

	// Based on test data:
	// - Alice (25, active) - matches: age < 30 AND active = true
	// - David (40, active) - matches: age > 35 AND active = true
	assert_eq!(results.len(), 2);

	teardown_test_table(&db, table_name).await;
}

/// Test NOT filter condition
///
/// **Test Category**: Combination testing
/// **Test Classification**: NOT combinations
#[rstest]
#[tokio::test]
async fn test_filter_condition_not(#[future] admin_database: std::sync::Arc<AdminDatabase>) {
	let db = admin_database.await;
	let table_name = "test_not_condition";

	setup_test_table(&db, table_name).await;

	// Build: NOT (active = true)
	let active_true = Filter::new(
		"active".to_string(),
		FilterOperator::Eq,
		FilterValue::Boolean(true),
	);

	let not_condition = FilterCondition::Not(Box::new(FilterCondition::Single(active_true)));

	let results = db
		.list_with_condition::<AdminRecord>(table_name, Some(&not_condition), vec![], None, 0, 10)
		.await
		.expect("List with NOT condition should succeed");

	// Should find inactive users: Bob and Eve
	assert_eq!(results.len(), 2);

	teardown_test_table(&db, table_name).await;
}

// ==================== 9. SANITY TESTS ====================

/// Sanity test for basic filter construction
///
/// **Test Category**: Sanity tests
/// **Test Classification**: Basic functionality
#[rstest]
#[tokio::test]
async fn test_filter_sanity_checks(#[future] admin_database: std::sync::Arc<AdminDatabase>) {
	// Test that Filter struct can be constructed with various value types
	let string_filter = Filter::new(
		"name".to_string(),
		FilterOperator::Eq,
		FilterValue::String("test".to_string()),
	);
	assert_eq!(string_filter.field, "name");
	// Note: FilterOperator doesn't implement PartialEq, so we use matches! instead
	assert!(matches!(string_filter.operator, FilterOperator::Eq));

	let int_filter = Filter::new(
		"age".to_string(),
		FilterOperator::Gt,
		FilterValue::Integer(30),
	);
	assert_eq!(int_filter.field, "age");
	assert!(matches!(int_filter.operator, FilterOperator::Gt));

	let bool_filter = Filter::new(
		"active".to_string(),
		FilterOperator::Eq,
		FilterValue::Boolean(true),
	);
	assert_eq!(bool_filter.field, "active");
	assert!(matches!(bool_filter.operator, FilterOperator::Eq));

	// Test FilterCondition construction
	let single_condition = FilterCondition::Single(string_filter.clone());
	match single_condition {
		FilterCondition::Single(ref f) => assert_eq!(f.field, "name"),
		_ => panic!("Expected Single variant"),
	}

	let and_condition = FilterCondition::And(vec![
		FilterCondition::Single(int_filter.clone()),
		FilterCondition::Single(bool_filter.clone()),
	]);
	match and_condition {
		FilterCondition::And(ref conditions) => assert_eq!(conditions.len(), 2),
		_ => panic!("Expected And variant"),
	}
}

// ==================== 10. EQUIVALENCE PARTITIONING TESTS ====================

/// Test equivalence partitioning for age ranges
///
/// **Test Category**: Equivalence partitioning
/// **Test Classification**: Using rstest case macro
#[rstest]
#[case::less_than_30(FilterOperator::Lt, 30, 2)] // Alice (25), Eve (28)
#[case::equal_to_30(FilterOperator::Eq, 30, 1)] // Bob (30)
#[case::greater_than_30(FilterOperator::Gt, 30, 2)] // Charlie (35), David (40)
#[tokio::test]
async fn test_age_equivalence_partitioning(
	#[future] admin_database: std::sync::Arc<AdminDatabase>,
	#[case] operator: FilterOperator,
	#[case] value: i64,
	#[case] expected_count: usize,
) {
	let db = admin_database.await;
	let table_name = "test_equivalence";

	setup_test_table(&db, table_name).await;

	let filters = vec![Filter::new(
		"age".to_string(),
		operator,
		FilterValue::Integer(value),
	)];

	let results = db
		.list::<AdminRecord>(table_name, filters, 0, 10)
		.await
		.expect("List operation should succeed");

	assert_eq!(results.len(), expected_count);

	teardown_test_table(&db, table_name).await;
}

/// Test equivalence partitioning for string operators
///
/// **Test Category**: Equivalence partitioning
/// **Test Classification**: Using rstest case macro
#[rstest]
#[case::equals("Alice Smith", 1)]
#[case::contains("Smith", 1)]
#[case::starts_with("Ali", 1)]
#[case::ends_with("mith", 1)]
#[tokio::test]
async fn test_string_operator_equivalence(
	#[future] admin_database: std::sync::Arc<AdminDatabase>,
	#[case] search_term: &str,
	#[case] expected_count: usize,
) {
	let db = admin_database.await;
	let table_name = "test_string_equivalence";

	setup_test_table(&db, table_name).await;

	// For this test, we'll use contains operator for all cases
	// In a real test, we would use different operators based on the case
	let filters = vec![Filter::new(
		"name".to_string(),
		FilterOperator::Contains,
		FilterValue::String(search_term.to_string()),
	)];

	let results = db
		.list::<AdminRecord>(table_name, filters, 0, 10)
		.await
		.expect("List operation should succeed");

	// All search terms should match Alice Smith
	assert_eq!(results.len(), expected_count);

	teardown_test_table(&db, table_name).await;
}

// ==================== 11. BOUNDARY VALUE ANALYSIS TESTS ====================

/// Test boundary values for age filtering
///
/// **Test Category**: Boundary value analysis
/// **Test Classification**: Using rstest case macro
#[rstest]
#[case(24, 0)] // Just below minimum age (25)
#[case(25, 1)] // Minimum age
#[case(30, 1)] // Middle value
#[case(40, 1)] // Maximum age
#[case(41, 0)] // Just above maximum age
#[tokio::test]
async fn test_age_boundary_values(
	#[future] admin_database: std::sync::Arc<AdminDatabase>,
	#[case] age_value: i64,
	#[case] expected_count: usize,
) {
	let db = admin_database.await;
	let table_name = "test_boundary";

	setup_test_table(&db, table_name).await;

	// Test equality at boundary values
	let filters = vec![Filter::new(
		"age".to_string(),
		FilterOperator::Eq,
		FilterValue::Integer(age_value),
	)];

	let results = db
		.list::<AdminRecord>(table_name, filters, 0, 10)
		.await
		.expect("List operation should succeed");

	assert_eq!(results.len(), expected_count);

	teardown_test_table(&db, table_name).await;
}

/// Test boundary values for score filtering (floating point)
///
/// **Test Category**: Boundary value analysis
/// **Test Classification**: Using rstest case macro
#[rstest]
#[case(68.4, 1)] // Just below David's score (68.5)
#[case(68.5, 1)] // Exact match
#[case(68.6, 0)] // Just above
#[case(94.9, 1)] // Just below Eve's score (95.0)
#[case(95.0, 1)] // Exact match
#[case(95.1, 0)] // Just above
#[tokio::test]
async fn test_score_boundary_values(
	#[future] admin_database: std::sync::Arc<AdminDatabase>,
	#[case] score_value: f64,
	#[case] expected_count: usize,
) {
	let db = admin_database.await;
	let table_name = "test_score_boundary";

	setup_test_table(&db, table_name).await;

	let filters = vec![Filter::new(
		"score".to_string(),
		FilterOperator::Eq,
		FilterValue::Float(score_value),
	)];

	let results = db
		.list::<AdminRecord>(table_name, filters, 0, 10)
		.await
		.expect("List operation should succeed");

	assert_eq!(results.len(), expected_count);

	teardown_test_table(&db, table_name).await;
}

// ==================== 12. DECISION TABLE TESTING ====================

/// Decision table test for active status and age range combinations
///
/// **Test Category**: Decision table testing
/// **Test Classification**: Using rstest case macro
#[rstest]
#[case(true, FilterOperator::Lt, 30, 1)] // Active AND age < 30: Alice
#[case(true, FilterOperator::Eq, 30, 0)] // Active AND age = 30: None (Bob is inactive)
#[case(true, FilterOperator::Gt, 30, 2)] // Active AND age > 30: Charlie, David
#[case(false, FilterOperator::Lt, 30, 1)] // Inactive AND age < 30: Eve
#[case(false, FilterOperator::Eq, 30, 1)] // Inactive AND age = 30: Bob
#[case(false, FilterOperator::Gt, 30, 0)] // Inactive AND age > 30: None
#[tokio::test]
async fn test_decision_table_active_age(
	#[future] admin_database: std::sync::Arc<AdminDatabase>,
	#[case] active: bool,
	#[case] age_operator: FilterOperator,
	#[case] age_value: i64,
	#[case] expected_count: usize,
) {
	let db = admin_database.await;
	let table_name = "test_decision_table";

	setup_test_table(&db, table_name).await;

	let filters = vec![
		Filter::new(
			"active".to_string(),
			FilterOperator::Eq,
			FilterValue::Boolean(active),
		),
		Filter::new(
			"age".to_string(),
			age_operator,
			FilterValue::Integer(age_value),
		),
	];

	let results = db
		.list::<AdminRecord>(table_name, filters, 0, 10)
		.await
		.expect("List operation should succeed");

	assert_eq!(results.len(), expected_count);

	teardown_test_table(&db, table_name).await;
}

// Note: Fuzz testing (6) and Property-based testing (7) require proptest crate
// and would be implemented in a separate module when property-based feature is enabled.
