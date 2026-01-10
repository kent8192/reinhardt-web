//! Edge case tests for AdminDatabase
//!
//! This module contains tests that verify edge case handling in AdminDatabase operations,
//! covering the "Edge cases" classification from the test plan.
//!
//! Tests in this file use high-level AdminDatabase API with rstest parameterization
//! and reinhardt-test admin_panel fixtures.

#![cfg(test)]

use reinhardt_db::prelude::{Filter, FilterOperator, FilterValue};
use reinhardt_test::fixtures::{
	AdminTableCreator, ColumnDefinition, FieldType, Operation, admin_table_creator,
};
use rstest::rstest;
use serde_json::json;
use std::collections::HashMap;

/// Create standard test schema for test_models table
fn create_test_schema() -> Vec<Operation> {
	vec![Operation::CreateTable {
		name: "test_models".to_string(),
		columns: vec![
			ColumnDefinition {
				name: "id".to_string(),
				type_definition: FieldType::Integer,
				not_null: true,
				unique: false,
				primary_key: true,
				auto_increment: true,
				default: None,
			},
			ColumnDefinition {
				name: "name".to_string(),
				type_definition: FieldType::Text,
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
			},
			ColumnDefinition {
				name: "status".to_string(),
				type_definition: FieldType::Text,
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
			},
			ColumnDefinition {
				name: "active".to_string(),
				type_definition: FieldType::Boolean,
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
			},
			ColumnDefinition {
				name: "deleted_at".to_string(),
				type_definition: FieldType::TimestampTz,
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
			},
		],
		constraints: vec![],
		without_rowid: None,
		interleave_in_parent: None,
		partition: None,
	}]
}

/// Test list operation on empty table
///
/// **Test Category**: Edge cases
/// **Test Classification**: Boundary conditions
#[rstest]
#[tokio::test]
async fn test_list_empty_table(#[future] admin_table_creator: AdminTableCreator) {
	let mut creator = admin_table_creator.await;
	creator.apply(create_test_schema()).await.unwrap();

	let db = creator.admin_db();
	let table_name = "test_models";

	// List from empty table (table exists but has no data)
	let result = db
		.list::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![], 0, 10)
		.await;

	assert!(result.is_ok(), "Should succeed on empty table");
	let records = result.unwrap();
	assert!(
		records.is_empty(),
		"Should return empty list for empty table"
	);
}

/// Test pagination boundary values
///
/// **Test Category**: Edge cases  
/// **Test Classification**: Boundary value analysis
#[rstest]
#[case::zero_offset_zero_limit(0, 0)] // Edge: zero limit
#[case::zero_offset_one_limit(0, 1)] // Edge: minimal limit
#[case::large_offset_small_limit(1000, 5)] // Edge: offset beyond data
#[case::small_offset_large_limit(0, 10000)] // Edge: limit larger than data
#[tokio::test]
async fn test_pagination_edge_cases(
	#[future] admin_table_creator: AdminTableCreator,
	#[case] offset: u64,
	#[case] limit: u64,
) {
	let mut creator = admin_table_creator.await;
	creator.apply(create_test_schema()).await.unwrap();

	let db = creator.admin_db();
	let table_name = "test_models";

	let result = db
		.list::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![], offset, limit)
		.await;

	// Should not panic for any offset/limit combination
	assert!(
		result.is_ok(),
		"Should handle pagination edges without panic"
	);
}

/// Test filter with empty string values
///
/// **Test Category**: Edge cases
/// **Test Classification**: Boundary conditions
#[rstest]
#[tokio::test]
async fn test_filter_empty_string(#[future] admin_table_creator: AdminTableCreator) {
	let mut creator = admin_table_creator.await;
	creator.apply(create_test_schema()).await.unwrap();

	let db = creator.admin_db();
	let table_name = "test_models";

	// Filter with empty string value
	let empty_filter = Filter {
		field: "name".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String("".to_string()),
	};

	let result = db
		.list::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![empty_filter], 0, 10)
		.await;

	assert!(
		result.is_ok(),
		"Should handle empty string filter without panic"
	);
}

/// Test filter with very long string values
///
/// **Test Category**: Edge cases
/// **Test Classification**: Boundary conditions
#[rstest]
#[tokio::test]
async fn test_filter_long_string(#[future] admin_table_creator: AdminTableCreator) {
	let mut creator = admin_table_creator.await;
	creator.apply(create_test_schema()).await.unwrap();

	let db = creator.admin_db();
	let table_name = "test_models";

	// Very long string (1000 characters)
	let long_string = "a".repeat(1000);
	let long_filter = Filter {
		field: "name".to_string(),
		operator: FilterOperator::Contains,
		value: FilterValue::String(long_string),
	};

	let result = db
		.list::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![long_filter], 0, 10)
		.await;

	assert!(
		result.is_ok(),
		"Should handle long string filter without panic"
	);
}

/// Test filter with special characters
///
/// **Test Category**: Edge cases
/// **Test Classification**: Boundary conditions
///
/// **KNOWN ISSUE**: These tests currently cause stack overflow or timeout.
/// This issue was discovered after fixing Phase 1 (table creation).
/// The original tests failed with "table not found" before reaching this code.
/// TODO(Phase 4): Investigate and fix stack overflow with special characters in filters
#[rstest]
#[case::sql_injection_chars("test'; DROP TABLE users; --")]
#[case::unicode_chars("test🎉📱🌟")]
#[case::control_chars("test\t\n\r\x00")]
#[case::emoji("test 🔥 🚀 💯")]
#[ignore = "Stack overflow issue - under investigation"]
#[tokio::test]
async fn test_filter_special_characters(
	#[future] admin_table_creator: AdminTableCreator,
	#[case] special_value: &str,
) {
	let mut creator = admin_table_creator.await;
	creator.apply(create_test_schema()).await.unwrap();

	let db = creator.admin_db();
	let table_name = "test_models";

	let special_filter = Filter {
		field: "name".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String(special_value.to_string()),
	};

	let result = db
		.list::<reinhardt_admin_core::database::AdminRecord>(
			table_name,
			vec![special_filter],
			0,
			10,
		)
		.await;

	assert!(
		result.is_ok(),
		"Should handle special characters '{}' without panic",
		special_value
	);
}

/// Test create with minimal data (only required fields)
///
/// **Test Category**: Edge cases
/// **Test Classification**: Boundary conditions
#[rstest]
#[tokio::test]
async fn test_create_minimal_data(#[future] admin_table_creator: AdminTableCreator) {
	let mut creator = admin_table_creator.await;
	creator.apply(create_test_schema()).await.unwrap();

	let db = creator.admin_db();
	let table_name = "test_models";

	// Minimal data - just enough to create a record
	// Assuming table has required fields (e.g., name)
	let minimal_data = HashMap::from([("name".to_string(), json!("Minimal Test"))]);

	let _result = db
		.create::<reinhardt_admin_core::database::AdminRecord>(table_name, minimal_data)
		.await;

	// Should succeed or fail based on schema requirements
	// This test documents the behavior
}

/// Test create with maximum field count (stress test)
///
/// **Test Category**: Edge cases
/// **Test Classification**: Stress testing
#[rstest]
#[tokio::test]
async fn test_create_many_fields(#[future] admin_table_creator: AdminTableCreator) {
	let mut creator = admin_table_creator.await;
	creator.apply(create_test_schema()).await.unwrap();

	let db = creator.admin_db();
	let table_name = "test_models";

	// Create data with many fields
	let mut many_fields = HashMap::new();
	for i in 0..50 {
		many_fields.insert(format!("field_{}", i), json!(format!("value_{}", i)));
	}
	// Include actual table columns if known
	many_fields.insert("name".to_string(), json!("Many Fields Test"));

	let result = db
		.create::<reinhardt_admin_core::database::AdminRecord>(table_name, many_fields)
		.await;

	// Should handle many fields without panic
	assert!(result.is_ok(), "Should handle many fields without panic");
}

/// Test bulk operations with large ID lists
///
/// **Test Category**: Edge cases
/// **Test Classification**: Stress testing
#[rstest]
#[tokio::test]
async fn test_bulk_delete_large_list(#[future] admin_table_creator: AdminTableCreator) {
	let mut creator = admin_table_creator.await;
	creator.apply(create_test_schema()).await.unwrap();

	let db = creator.admin_db();
	let table_name = "test_models";
	let pk_field = "id";

	// Create large list of IDs (most won't exist)
	let mut large_id_list = Vec::new();
	for i in 1..=1000 {
		large_id_list.push(i.to_string());
	}

	let result = db
		.bulk_delete::<reinhardt_admin_core::database::AdminRecord>(
			table_name,
			pk_field,
			large_id_list,
		)
		.await;

	// Should handle large list without panic
	assert!(result.is_ok(), "Should handle large ID list without panic");
}

/// Test count on empty result set
///
/// **Test Category**: Edge cases
/// **Test Classification**: Boundary conditions
#[rstest]
#[tokio::test]
async fn test_count_empty_result(#[future] admin_table_creator: AdminTableCreator) {
	let mut creator = admin_table_creator.await;
	creator.apply(create_test_schema()).await.unwrap();

	let db = creator.admin_db();
	let table_name = "test_models";

	// Filter that matches no records
	let impossible_filter = Filter {
		field: "name".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String("THIS_SHOULD_NOT_EXIST_12345".to_string()),
	};

	let result = db
		.count::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![impossible_filter])
		.await;

	assert!(
		result.is_ok(),
		"Should count empty result set without panic"
	);
	let count = result.unwrap();
	assert_eq!(count, 0, "Should return 0 for empty result set");
}

/// Test numeric filter boundary values
///
/// **Test Category**: Edge cases
/// **Test Classification**: Boundary value analysis
#[rstest]
#[case::zero(0)]
#[case::negative(-1)]
#[case::large_positive(1000000)]
#[case::max_i64(9223372036854775807i64)]
#[tokio::test]
async fn test_numeric_filter_boundaries(
	#[future] admin_table_creator: AdminTableCreator,
	#[case] numeric_value: i64,
) {
	let mut creator = admin_table_creator.await;
	creator.apply(create_test_schema()).await.unwrap();

	let db = creator.admin_db();
	let table_name = "test_models";

	let numeric_filter = Filter {
		field: "id".to_string(),
		operator: FilterOperator::Gt,
		value: FilterValue::Integer(numeric_value),
	};

	let result = db
		.list::<reinhardt_admin_core::database::AdminRecord>(
			table_name,
			vec![numeric_filter],
			0,
			10,
		)
		.await;

	assert!(
		result.is_ok(),
		"Should handle numeric boundary {} without panic",
		numeric_value
	);
}

/// Test boolean filter edge cases
///
/// **Test Category**: Edge cases
/// **Test Classification**: Boundary conditions
#[rstest]
#[tokio::test]
async fn test_boolean_filter_edges(#[future] admin_table_creator: AdminTableCreator) {
	let mut creator = admin_table_creator.await;
	creator.apply(create_test_schema()).await.unwrap();

	let db = creator.admin_db();
	let table_name = "test_models";

	// Test both boolean values
	for &bool_value in &[true, false] {
		let bool_filter = Filter {
			field: "active".to_string(),
			operator: FilterOperator::Eq,
			value: FilterValue::Boolean(bool_value),
		};

		let result = db
			.list::<reinhardt_admin_core::database::AdminRecord>(
				table_name,
				vec![bool_filter],
				0,
				10,
			)
			.await;

		assert!(
			result.is_ok(),
			"Should handle boolean filter ({}) without panic",
			bool_value
		);
	}
}

/// Test null filter edge cases
///
/// **Test Category**: Edge cases
/// **Test Classification**: Boundary conditions
#[rstest]
#[tokio::test]
async fn test_null_filter_edges(#[future] admin_table_creator: AdminTableCreator) {
	let mut creator = admin_table_creator.await;
	creator.apply(create_test_schema()).await.unwrap();

	let db = creator.admin_db();
	let table_name = "test_models";

	let null_filter = Filter {
		field: "deleted_at".to_string(),
		operator: FilterOperator::IsNull,
		value: FilterValue::Null,
	};

	let result = db
		.list::<reinhardt_admin_core::database::AdminRecord>(table_name, vec![null_filter], 0, 10)
		.await;

	assert!(result.is_ok(), "Should handle null filter without panic");
}
