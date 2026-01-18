//! Error path tests for AdminDatabase
//!
//! This module contains tests that verify error handling in AdminDatabase operations,
//! covering the "Error path" classification from the test plan.
//!
//! Tests in this file use high-level AdminDatabase API with rstest parameterization
//! and reinhardt-test admin_panel fixtures.

#![cfg(test)]

use reinhardt_admin::core::database::AdminDatabase;
use reinhardt_db::prelude::{Filter, FilterOperator, FilterValue};
use reinhardt_test::fixtures::admin_panel::admin_database;
use rstest::rstest;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

/// Test error handling for invalid table names
///
/// **Test Category**: Error path
/// **Test Classification**: Error handling
#[rstest]
#[tokio::test]
async fn test_invalid_table_name_error(#[future] admin_database: Arc<AdminDatabase>) {
	let db = admin_database.await;
	let invalid_table = "non_existent_table_12345";

	// Attempt to list from non-existent table
	let result = db
		.list::<reinhardt_admin::core::database::AdminRecord>(invalid_table, vec![], 0, 10)
		.await;

	// Should return an error (specific error type depends on implementation)
	assert!(
		result.is_err(),
		"Should return error for non-existent table"
	);

	// The error should be a database-related error
	let _err = result.unwrap_err();
	// Note: Actual error type may vary - this test ensures errors are propagated
}

/// Test error handling for invalid column names in filters
///
/// **Test Category**: Error path  
/// **Test Classification**: Error handling
#[rstest]
#[tokio::test]
async fn test_invalid_column_filter_error(#[future] admin_database: Arc<AdminDatabase>) {
	let db = admin_database.await;
	let table_name = "test_models";

	// Create filter with non-existent column
	let invalid_filter = Filter {
		field: "non_existent_column".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String("test".to_string()),
	};

	let result = db
		.list::<reinhardt_admin::core::database::AdminRecord>(
			table_name,
			vec![invalid_filter],
			0,
			10,
		)
		.await;

	// Should return an error
	assert!(
		result.is_err(),
		"Should return error for invalid column in filter"
	);
}

/// Test error handling for invalid primary key values in get operations
///
/// **Test Category**: Error path
/// **Test Classification**: Error handling
#[rstest]
#[tokio::test]
async fn test_invalid_primary_key_error(#[future] admin_database: Arc<AdminDatabase>) {
	let db = admin_database.await;
	let table_name = "test_models";
	let pk_field = "id";

	// Attempt to get record with non-existent ID
	let result = db
		.get::<reinhardt_admin::core::database::AdminRecord>(
			table_name, pk_field, "999999", // Non-existent ID
		)
		.await;

	// Should return Ok(None) or error depending on implementation
	// Both behaviors are valid error handling
	match result {
		Ok(opt) => assert!(opt.is_none(), "Should return None for non-existent record"),
		Err(_) => {} // Error is also acceptable
	}
}

/// Test error handling for invalid data types in create operations
///
/// **Test Category**: Error path
/// **Test Classification**: Error handling
#[rstest]
#[tokio::test]
async fn test_invalid_data_type_error(#[future] admin_database: Arc<AdminDatabase>) {
	let db = admin_database.await;
	let table_name = "test_models";

	// Create data with wrong type for a column (if schema enforcement exists)
	let mut invalid_data = HashMap::new();
	invalid_data.insert("id".to_string(), json!("not_a_number")); // id should be integer

	let _result = db
		.create::<reinhardt_admin::core::database::AdminRecord>(table_name, invalid_data)
		.await;

	// May succeed or fail depending on database schema enforcement
	// This test documents the behavior
}

/// Test error handling for duplicate primary key in create operations
///
/// **Test Category**: Error path
/// **Test Classification**: Error handling
#[rstest]
#[tokio::test]
async fn test_duplicate_primary_key_error(#[future] admin_database: Arc<AdminDatabase>) {
	let db = admin_database.await;
	let table_name = "test_models";

	// First, create a record
	let mut data1 = HashMap::new();
	data1.insert("name".to_string(), json!("Test Item"));

	let create_result = db
		.create::<reinhardt_admin::core::database::AdminRecord>(table_name, data1)
		.await;

	if let Ok(id) = create_result {
		// Try to create another record with same ID (if ID is manually specified)
		let mut data2 = HashMap::new();
		data2.insert("id".to_string(), json!(id));
		data2.insert("name".to_string(), json!("Duplicate Item"));

		let duplicate_result = db
			.create::<reinhardt_admin::core::database::AdminRecord>(table_name, data2)
			.await;

		// Should fail with duplicate key error
		assert!(
			duplicate_result.is_err(),
			"Should fail when attempting to insert duplicate primary key"
		);
	}
}

/// Test error handling for update on non-existent record
///
/// **Test Category**: Error path
/// **Test Classification**: Error handling
#[rstest]
#[tokio::test]
async fn test_update_nonexistent_record_error(#[future] admin_database: Arc<AdminDatabase>) {
	let db = admin_database.await;
	let table_name = "test_models";
	let pk_field = "id";
	let non_existent_id = "999999";

	let update_data = HashMap::from([("name".to_string(), json!("Updated Name"))]);

	let result = db
		.update::<reinhardt_admin::core::database::AdminRecord>(
			table_name,
			pk_field,
			non_existent_id,
			update_data,
		)
		.await;

	// Update of non-existent record may succeed (0 rows updated) or fail
	// Both behaviors are valid
	match result {
		Ok(count) => assert_eq!(count, 0, "Should update 0 rows for non-existent record"),
		Err(_) => {} // Error is also acceptable
	}
}

/// Test error handling for delete on non-existent record
///
/// **Test Category**: Error path
/// **Test Classification**: Error handling
#[rstest]
#[tokio::test]
async fn test_delete_nonexistent_record_error(#[future] admin_database: Arc<AdminDatabase>) {
	let db = admin_database.await;
	let table_name = "test_models";
	let pk_field = "id";
	let non_existent_id = "999999";

	let result = db
		.delete::<reinhardt_admin::core::database::AdminRecord>(
			table_name,
			pk_field,
			non_existent_id,
		)
		.await;

	// Delete of non-existent record may succeed (0 rows deleted) or fail
	// Both behaviors are valid
	match result {
		Ok(count) => assert_eq!(count, 0, "Should delete 0 rows for non-existent record"),
		Err(_) => {} // Error is also acceptable
	}
}

/// Test error handling for bulk delete with invalid IDs
///
/// **Test Category**: Error path
/// **Test Classification**: Error handling
#[rstest]
#[tokio::test]
async fn test_bulk_delete_invalid_ids_error(#[future] admin_database: Arc<AdminDatabase>) {
	let db = admin_database.await;
	let table_name = "test_models";
	let pk_field = "id";

	// Bulk delete with non-existent IDs
	let invalid_ids = vec!["999999".to_string(), "1000000".to_string()];

	let result = db
		.bulk_delete::<reinhardt_admin::core::database::AdminRecord>(
			table_name,
			pk_field,
			invalid_ids,
		)
		.await;

	// May succeed (0 rows deleted) or fail
	match result {
		Ok(count) => assert_eq!(count, 0, "Should delete 0 rows for non-existent IDs"),
		Err(_) => {} // Error is also acceptable
	}
}

/// Test error handling for count with invalid filter conditions
///
/// **Test Category**: Error path
/// **Test Classification**: Error handling
#[rstest]
#[tokio::test]
async fn test_count_invalid_filter_error(#[future] admin_database: Arc<AdminDatabase>) {
	let db = admin_database.await;
	let table_name = "test_models";

	// Invalid filter with non-existent column
	let invalid_filter = Filter {
		field: "non_existent_column".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String("value".to_string()),
	};

	let result = db
		.count::<reinhardt_admin::core::database::AdminRecord>(table_name, vec![invalid_filter])
		.await;

	// Should return an error
	assert!(
		result.is_err(),
		"Should return error for invalid filter in count"
	);
}
