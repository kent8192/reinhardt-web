//! State transition tests for AdminDatabase
//!
//! This module contains tests that verify state transitions in AdminDatabase operations,
//! covering the "State transition testing" classification from the test plan.
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
			ColumnDefinition {
				name: "batch".to_string(),
				type_definition: FieldType::Text,
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
			},
			ColumnDefinition {
				name: "priority".to_string(),
				type_definition: FieldType::Text,
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
			},
			ColumnDefinition {
				name: "processed_by".to_string(),
				type_definition: FieldType::Text,
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

/// Test complete CRUD lifecycle: Create → Read → Update → Delete
///
/// **Test Category**: State transition testing
/// **Test Classification**: Normal path with state changes
#[rstest]
#[tokio::test]
async fn test_crud_lifecycle_state_transitions(#[future] admin_table_creator: AdminTableCreator) {
	let mut creator = admin_table_creator.await;
	creator.apply(create_test_schema()).await.unwrap();

	let db = creator.admin_db();
	let table_name = "test_models";
	let pk_field = "id";

	// 1. CREATE: Create a new record
	let create_data = HashMap::from([
		("name".to_string(), json!("CRUD Lifecycle Test")),
		("status".to_string(), json!("active")),
	]);

	let create_result = db
		.create::<reinhardt_admin::core::database::AdminRecord>(table_name, create_data.clone())
		.await;

	assert!(create_result.is_ok(), "Create should succeed");
	let record_id = create_result.unwrap();

	// 2. READ: Verify the record was created
	let read_result = db
		.get::<reinhardt_admin::core::database::AdminRecord>(
			table_name,
			pk_field,
			&record_id.to_string(),
		)
		.await;

	assert!(read_result.is_ok(), "Read after create should succeed");
	let record = read_result.unwrap();
	assert!(record.is_some(), "Record should exist after create");

	if let Some(record) = record {
		assert_eq!(
			record.get("name"),
			Some(&json!("CRUD Lifecycle Test")),
			"Record data should match created data"
		);
	}

	// 3. UPDATE: Modify the record
	let update_data = HashMap::from([("status".to_string(), json!("inactive"))]);

	let update_result = db
		.update::<reinhardt_admin::core::database::AdminRecord>(
			table_name,
			pk_field,
			&record_id.to_string(),
			update_data,
		)
		.await;

	assert!(update_result.is_ok(), "Update should succeed");
	let updated_count = update_result.unwrap();
	assert_eq!(updated_count, 1, "Should update exactly one record");

	// 4. READ after UPDATE: Verify the update
	let read_after_update = db
		.get::<reinhardt_admin::core::database::AdminRecord>(
			table_name,
			pk_field,
			&record_id.to_string(),
		)
		.await;

	assert!(
		read_after_update.is_ok(),
		"Read after update should succeed"
	);
	let updated_record = read_after_update.unwrap();
	assert!(
		updated_record.is_some(),
		"Record should still exist after update"
	);

	if let Some(record) = updated_record {
		assert_eq!(
			record.get("status"),
			Some(&json!("inactive")),
			"Record should reflect update"
		);
	}

	// 5. DELETE: Remove the record
	let delete_result = db
		.delete::<reinhardt_admin::core::database::AdminRecord>(
			table_name,
			pk_field,
			&record_id.to_string(),
		)
		.await;

	assert!(delete_result.is_ok(), "Delete should succeed");
	let deleted_count = delete_result.unwrap();
	assert_eq!(deleted_count, 1, "Should delete exactly one record");

	// 6. READ after DELETE: Verify deletion
	let read_after_delete = db
		.get::<reinhardt_admin::core::database::AdminRecord>(
			table_name,
			pk_field,
			&record_id.to_string(),
		)
		.await;

	assert!(
		read_after_delete.is_ok(),
		"Read after delete should succeed"
	);
	let deleted_record = read_after_delete.unwrap();
	assert!(
		deleted_record.is_none(),
		"Record should not exist after delete"
	);
}

/// Test state transitions with multiple sequential updates
///
/// **Test Category**: State transition testing
/// **Test Classification**: Sequential state changes
#[rstest]
#[tokio::test]
async fn test_multiple_sequential_updates(#[future] admin_table_creator: AdminTableCreator) {
	let mut creator = admin_table_creator.await;
	creator.apply(create_test_schema()).await.unwrap();

	let db = creator.admin_db();
	let table_name = "test_models";
	let pk_field = "id";

	// Create initial record
	let create_data = HashMap::from([("name".to_string(), json!("Sequential Updates Test"))]);

	let record_id = db
		.create::<reinhardt_admin::core::database::AdminRecord>(table_name, create_data)
		.await
		.expect("Create should succeed");

	// Perform multiple sequential updates
	let updates = vec![
		HashMap::from([("status".to_string(), json!("pending"))]),
		HashMap::from([("status".to_string(), json!("processing"))]),
		HashMap::from([("status".to_string(), json!("completed"))]),
		HashMap::from([("status".to_string(), json!("archived"))]),
	];

	for (i, update_data) in updates.into_iter().enumerate() {
		let update_result = db
			.update::<reinhardt_admin::core::database::AdminRecord>(
				table_name,
				pk_field,
				&record_id.to_string(),
				update_data.clone(),
			)
			.await;

		assert!(update_result.is_ok(), "Update {} should succeed", i + 1);

		// Verify the update took effect
		let read_result = db
			.get::<reinhardt_admin::core::database::AdminRecord>(
				table_name,
				pk_field,
				&record_id.to_string(),
			)
			.await;

		assert!(
			read_result.is_ok(),
			"Read after update {} should succeed",
			i + 1
		);

		if let Some(record) = read_result.unwrap() {
			// Check that the last update is reflected
			for (key, value) in update_data {
				assert_eq!(
					record.get(&key),
					Some(&value),
					"Field {} should be updated to {:?}",
					key,
					value
				);
			}
		}
	}

	// Clean up
	db.delete::<reinhardt_admin::core::database::AdminRecord>(
		table_name,
		pk_field,
		&record_id.to_string(),
	)
	.await
	.expect("Delete should succeed");
}

/// Test state transitions with create → update → create (same table)
///
/// **Test Category**: State transition testing
/// **Test Classification**: Interleaved operations
#[rstest]
#[tokio::test]
async fn test_interleaved_create_update_operations(
	#[future] admin_table_creator: AdminTableCreator,
) {
	let mut creator = admin_table_creator.await;
	creator.apply(create_test_schema()).await.unwrap();

	let db = creator.admin_db();
	let table_name = "test_models";

	// Create first record
	let data1 = HashMap::from([("name".to_string(), json!("Record 1"))]);
	let id1 = db
		.create::<reinhardt_admin::core::database::AdminRecord>(table_name, data1)
		.await
		.expect("First create should succeed");

	// Create second record
	let data2 = HashMap::from([("name".to_string(), json!("Record 2"))]);
	let id2 = db
		.create::<reinhardt_admin::core::database::AdminRecord>(table_name, data2)
		.await
		.expect("Second create should succeed");

	// Update first record
	let update_data = HashMap::from([("status".to_string(), json!("updated"))]);
	db.update::<reinhardt_admin::core::database::AdminRecord>(
		table_name,
		"id",
		&id1.to_string(),
		update_data,
	)
	.await
	.expect("Update should succeed");

	// Verify both records exist with correct states
	let list_result = db
		.list::<reinhardt_admin::core::database::AdminRecord>(table_name, vec![], 0, 10)
		.await
		.expect("List should succeed");

	assert!(list_result.len() >= 2, "Should have at least 2 records");

	// Clean up
	for id in &[id1, id2] {
		db.delete::<reinhardt_admin::core::database::AdminRecord>(table_name, "id", &id.to_string())
			.await
			.expect("Delete should succeed");
	}
}

/// Test state transitions with filter-based operations
///
/// **Test Category**: State transition testing
/// **Test Classification**: Filter-based state changes
#[rstest]
#[tokio::test]
async fn test_filter_based_state_transitions(#[future] admin_table_creator: AdminTableCreator) {
	let mut creator = admin_table_creator.await;
	creator.apply(create_test_schema()).await.unwrap();

	let db = creator.admin_db();
	let table_name = "test_models";

	// Create test records with different statuses
	let records = vec![
		HashMap::from([
			("name".to_string(), json!("Active Record")),
			("status".to_string(), json!("active")),
		]),
		HashMap::from([
			("name".to_string(), json!("Inactive Record")),
			("status".to_string(), json!("inactive")),
		]),
		HashMap::from([
			("name".to_string(), json!("Pending Record")),
			("status".to_string(), json!("pending")),
		]),
	];

	let mut created_ids = Vec::new();
	for record_data in records {
		let id = db
			.create::<reinhardt_admin::core::database::AdminRecord>(table_name, record_data)
			.await
			.expect("Create should succeed");
		created_ids.push(id);
	}

	// State transition: Update all "active" records to "archived"
	let active_filter = Filter {
		field: "status".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String("active".to_string()),
	};

	// First, count active records
	let active_count = db
		.count::<reinhardt_admin::core::database::AdminRecord>(
			table_name,
			vec![active_filter.clone()],
		)
		.await
		.expect("Count should succeed");

	// Update active records
	let update_data = HashMap::from([("status".to_string(), json!("archived"))]);

	// Note: AdminDatabase.update doesn't support filters directly, so we'll
	// list then update individually for this test
	let active_records = db
		.list::<reinhardt_admin::core::database::AdminRecord>(table_name, vec![active_filter], 0, 10)
		.await
		.expect("List should succeed");

	for record in active_records {
		if let Some(id_value) = record.get("id") {
			// ID can be either a number or a string, convert to string for update
			let id_str = match id_value {
				serde_json::Value::Number(n) => n.to_string(),
				serde_json::Value::String(s) => s.clone(),
				_ => continue,
			};
			db.update::<reinhardt_admin::core::database::AdminRecord>(
				table_name,
				"id",
				&id_str,
				update_data.clone(),
			)
			.await
			.expect("Update should succeed");
		}
	}

	// Verify state transition: No more "active" records
	let active_filter_after = Filter {
		field: "status".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String("active".to_string()),
	};

	let active_count_after = db
		.count::<reinhardt_admin::core::database::AdminRecord>(table_name, vec![active_filter_after])
		.await
		.expect("Count should succeed");

	assert_eq!(
		active_count_after, 0,
		"Should be no active records after state transition"
	);

	// Verify new state: Count "archived" records
	let archived_filter = Filter {
		field: "status".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String("archived".to_string()),
	};

	let archived_count = db
		.count::<reinhardt_admin::core::database::AdminRecord>(table_name, vec![archived_filter])
		.await
		.expect("Count should succeed");

	assert_eq!(
		archived_count, active_count,
		"All previously active records should now be archived"
	);

	// Clean up
	for id in created_ids {
		db.delete::<reinhardt_admin::core::database::AdminRecord>(table_name, "id", &id.to_string())
			.await
			.expect("Delete should succeed");
	}
}

/// Test state transitions with bulk operations
///
/// **Test Category**: State transition testing
/// **Test Classification**: Bulk state changes
#[rstest]
#[tokio::test]
async fn test_bulk_operations_state_transitions(#[future] admin_table_creator: AdminTableCreator) {
	let mut creator = admin_table_creator.await;
	creator.apply(create_test_schema()).await.unwrap();

	let db = creator.admin_db();
	let table_name = "test_models";

	// Create multiple records
	let mut created_ids = Vec::new();
	for i in 1..=5 {
		let data = HashMap::from([
			("name".to_string(), json!(format!("Bulk Test {}", i))),
			("batch".to_string(), json!("A")),
		]);

		let id = db
			.create::<reinhardt_admin::core::database::AdminRecord>(table_name, data)
			.await
			.expect("Create should succeed");
		created_ids.push(id);
	}

	// State transition: Update all records in batch A to batch B
	let batch_a_filter = Filter {
		field: "batch".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String("A".to_string()),
	};

	// Count before transition
	let count_before = db
		.count::<reinhardt_admin::core::database::AdminRecord>(
			table_name,
			vec![batch_a_filter.clone()],
		)
		.await
		.expect("Count should succeed");

	assert_eq!(count_before, 5, "Should have 5 records in batch A");

	// Update each record (simulating bulk update)
	let update_data = HashMap::from([("batch".to_string(), json!("B"))]);
	for id in &created_ids {
		db.update::<reinhardt_admin::core::database::AdminRecord>(
			table_name,
			"id",
			&id.to_string(),
			update_data.clone(),
		)
		.await
		.expect("Update should succeed");
	}

	// Verify state transition: No records in batch A
	let count_after_a = db
		.count::<reinhardt_admin::core::database::AdminRecord>(table_name, vec![batch_a_filter])
		.await
		.expect("Count should succeed");

	assert_eq!(
		count_after_a, 0,
		"Should be no records in batch A after transition"
	);

	// Verify new state: All records in batch B
	let batch_b_filter = Filter {
		field: "batch".to_string(),
		operator: FilterOperator::Eq,
		value: FilterValue::String("B".to_string()),
	};

	let count_after_b = db
		.count::<reinhardt_admin::core::database::AdminRecord>(table_name, vec![batch_b_filter])
		.await
		.expect("Count should succeed");

	assert_eq!(count_after_b, 5, "All 5 records should now be in batch B");

	// Final state transition: Bulk delete all records in batch B
	let delete_count = db
		.bulk_delete::<reinhardt_admin::core::database::AdminRecord>(
			table_name,
			"id",
			created_ids.iter().map(|id| id.to_string()).collect(),
		)
		.await
		.expect("Bulk delete should succeed");

	assert_eq!(delete_count, 5, "Should delete all 5 records");

	// Verify final state: No records
	let final_count = db
		.count::<reinhardt_admin::core::database::AdminRecord>(table_name, vec![])
		.await
		.expect("Count should succeed");

	assert_eq!(final_count, 0, "Should be no records after bulk delete");
}

/// Test rollback scenario simulation (partial failure)
///
/// **Test Category**: State transition testing
/// **Test Classification**: Error recovery state changes
#[rstest]
#[tokio::test]
async fn test_partial_failure_state_transitions(#[future] admin_table_creator: AdminTableCreator) {
	let mut creator = admin_table_creator.await;
	creator.apply(create_test_schema()).await.unwrap();

	let db = creator.admin_db();
	let table_name = "test_models";

	// Create initial record
	let initial_data = HashMap::from([("name".to_string(), json!("Original"))]);
	let record_id = db
		.create::<reinhardt_admin::core::database::AdminRecord>(table_name, initial_data)
		.await
		.expect("Create should succeed");

	// Attempt an update that would fail (e.g., invalid data type)
	// This simulates a partial operation scenario
	let invalid_update = HashMap::from([
		("name".to_string(), json!("Updated")),
		// Assuming "id" is integer, try to set it to string (might fail)
		("id".to_string(), json!("not_a_number")),
	]);

	let _update_result = db
		.update::<reinhardt_admin::core::database::AdminRecord>(
			table_name,
			"id",
			&record_id.to_string(),
			invalid_update,
		)
		.await;

	// The update might fail or succeed depending on database constraints
	// Either way, verify the original record is still accessible

	let current_record = db
		.get::<reinhardt_admin::core::database::AdminRecord>(
			table_name,
			"id",
			&record_id.to_string(),
		)
		.await
		.expect("Get should succeed");

	// Record should still exist regardless of update outcome
	assert!(
		current_record.is_some(),
		"Original record should still exist"
	);

	// Clean up
	db.delete::<reinhardt_admin::core::database::AdminRecord>(
		table_name,
		"id",
		&record_id.to_string(),
	)
	.await
	.expect("Delete should succeed");
}

/// Test concurrent operation state consistency
///
/// **Test Category**: State transition testing
/// **Test Classification**: Concurrent state changes
#[rstest]
#[tokio::test]
async fn test_concurrent_operations_state_consistency(
	#[future] admin_table_creator: AdminTableCreator,
) {
	let mut creator = admin_table_creator.await;
	creator.apply(create_test_schema()).await.unwrap();

	let db = creator.admin_db();
	let table_name = "test_models";

	// Create a record
	let record_id = db
		.create::<reinhardt_admin::core::database::AdminRecord>(
			table_name,
			HashMap::from([("name".to_string(), json!("Concurrent Test"))]),
		)
		.await
		.expect("Create should succeed");

	// Simulate concurrent operations by performing multiple
	// sequential operations that would conflict if truly concurrent
	let operations = vec![
		("status", json!("processing")),
		("priority", json!("high")),
		("processed_by", json!("worker1")),
	];

	for (field, value) in operations {
		let update_data = HashMap::from([(field.to_string(), value.clone())]);

		db.update::<reinhardt_admin::core::database::AdminRecord>(
			table_name,
			"id",
			&record_id.to_string(),
			update_data,
		)
		.await
		.expect("Update should succeed");

		// Verify state after each "concurrent" operation
		let current = db
			.get::<reinhardt_admin::core::database::AdminRecord>(
				table_name,
				"id",
				&record_id.to_string(),
			)
			.await
			.expect("Get should succeed");

		assert!(
			current.is_some(),
			"Record should exist after each operation"
		);

		if let Some(record) = current {
			// The field from this operation should be set
			assert_eq!(
				record.get(field),
				Some(&value),
				"Field {} should be {} after operation",
				field,
				value
			);
		}
	}

	// Final state verification
	let final_record = db
		.get::<reinhardt_admin::core::database::AdminRecord>(
			table_name,
			"id",
			&record_id.to_string(),
		)
		.await
		.expect("Get should succeed")
		.expect("Record should exist");

	// All fields should be set
	assert_eq!(final_record.get("status"), Some(&json!("processing")));
	assert_eq!(final_record.get("priority"), Some(&json!("high")));
	assert_eq!(final_record.get("processed_by"), Some(&json!("worker1")));

	// Clean up
	db.delete::<reinhardt_admin::core::database::AdminRecord>(
		table_name,
		"id",
		&record_id.to_string(),
	)
	.await
	.expect("Delete should succeed");
}
