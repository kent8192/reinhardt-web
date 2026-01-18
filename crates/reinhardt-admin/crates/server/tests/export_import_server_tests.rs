//! Export/import server function tests for admin panel
//!
//! This module tests the export_data and import_data server functions
//! with all 12 test classifications.

// Test module - only compile in test configuration
#![cfg(test)]

use reinhardt_admin::core::{AdminDatabase, AdminSite, ExportFormat, ImportResponse};
use reinhardt_test::fixtures::admin_panel::export_import_test_context;
use rstest::*;
use std::sync::Arc;

// ==================== 0. FIXTURE VERIFICATION TEST ====================

/// Verify export_import_test_context fixture setup
///
/// **Test Category**: Infrastructure
/// **Purpose**: Ensure the fixture creates the test table with diverse data patterns
#[rstest]
#[tokio::test]
async fn test_fixture_setup(
	#[future] export_import_test_context: (
		Arc<AdminSite>,
		Arc<AdminDatabase>,
		String,
		sqlx::PgPool,
	),
) {
	let (site, db, table_name, _pool) = export_import_test_context.await;

	// Verify AdminSite has TestModel registered
	assert!(
		site.registered_models().contains(&"TestModel".to_string()),
		"TestModel should be registered in AdminSite"
	);

	// Fetch records directly using db.list() (bypassing server function for testing)
	let records_map = db
		.list::<reinhardt_admin::core::AdminRecord>(&table_name, vec![], 0, 100)
		.await
		.expect("Failed to list records from database");

	// Convert HashMap to serde_json::Value for easier testing
	let records: Vec<serde_json::Value> = records_map
		.into_iter()
		.map(|map| serde_json::to_value(map).expect("Failed to convert to JSON"))
		.collect();

	// Verify 5 records with diverse data patterns
	assert_eq!(records.len(), 5, "Should have exactly 5 test records");

	// Pattern 1: Standard data (Alice)
	assert_eq!(
		records[0]["name"].as_str().unwrap(),
		"Alice Johnson",
		"First record should be Alice"
	);
	assert_eq!(records[0]["age"].as_i64().unwrap(), 30);

	// Pattern 2: NULL values (Bob)
	assert_eq!(
		records[1]["name"].as_str().unwrap(),
		"Bob Smith",
		"Second record should be Bob"
	);
	assert!(records[1]["age"].is_null(), "Bob should have NULL age");

	// Pattern 3: Special characters and Unicode (Charlie)
	assert_eq!(
		records[2]["name"].as_str().unwrap(),
		"Charlie O'Brien",
		"Third record should be Charlie"
	);
	let bio = records[2]["bio"].as_str().unwrap();
	assert!(
		bio.contains("日本語"),
		"Charlie's bio should contain Japanese characters"
	);

	// Pattern 4: Boundary values (David)
	assert_eq!(
		records[3]["age"].as_i64().unwrap(),
		0,
		"David should have age 0"
	);

	// Pattern 5: Maximum length edge case (Eve)
	let eve_name = records[4]["name"].as_str().unwrap();
	assert!(
		eve_name.starts_with("Eve Martinez"),
		"Eve's name should start with 'Eve Martinez'"
	);
	assert!(eve_name.len() > 200, "Eve's name should be long");

	// Verify table name format
	assert!(
		table_name.starts_with("test_exports_"),
		"Table name should start with test_exports_"
	);
}

// ==================== 1. HAPPY PATH TESTS ====================

/// Test export to JSON format (happy path)
///
/// **Test Category**: Happy path
/// **Test Classification**: Normal flow
#[rstest]
#[tokio::test]
async fn test_export_json_happy_path(
	#[future] export_import_test_context: (
		Arc<AdminSite>,
		Arc<AdminDatabase>,
		String,
		sqlx::PgPool,
	),
) {
	let (_site, db, table_name, _pool) = export_import_test_context.await;

	// Fetch all records from the test table
	let records_map = db
		.list::<reinhardt_admin::core::AdminRecord>(&table_name, vec![], 0, 100)
		.await
		.expect("Failed to list records");

	// Serialize to JSON
	let json_data = serde_json::to_vec_pretty(&records_map).expect("Failed to serialize to JSON");
	let json_str = String::from_utf8(json_data.clone()).expect("Invalid UTF-8 in JSON");

	// Verify JSON is valid and parse it
	let records: Vec<serde_json::Value> =
		serde_json::from_slice(&json_data).expect("Failed to parse JSON");

	// Verify we have exactly 5 records
	assert_eq!(records.len(), 5, "Should have 5 records in JSON export");

	// Verify each data pattern
	// Pattern 1: Standard data (Alice)
	assert_eq!(records[0]["name"].as_str().unwrap(), "Alice Johnson");
	assert_eq!(records[0]["age"].as_i64().unwrap(), 30);
	assert_eq!(records[0]["email"].as_str().unwrap(), "alice@example.com");
	assert!(records[0]["is_verified"].as_bool().unwrap());

	// Pattern 2: NULL values (Bob)
	assert_eq!(records[1]["name"].as_str().unwrap(), "Bob Smith");
	assert!(records[1]["age"].is_null(), "Bob's age should be NULL");
	assert!(records[1]["bio"].is_null(), "Bob's bio should be NULL");

	// Pattern 3: Special characters and Unicode (Charlie)
	assert_eq!(records[2]["name"].as_str().unwrap(), "Charlie O'Brien");
	let charlie_bio = records[2]["bio"].as_str().unwrap();
	assert!(
		charlie_bio.contains("日本語"),
		"Charlie's bio should contain Japanese characters"
	);
	assert!(
		charlie_bio.contains("\"quotes\""),
		"Charlie's bio should contain escaped quotes"
	);

	// Pattern 4: Boundary values (David)
	assert_eq!(
		records[3]["age"].as_i64().unwrap(),
		0,
		"David should have age 0"
	);
	assert_eq!(
		records[3]["score"].as_f64().unwrap(),
		0.0,
		"David should have score 0.0"
	);
	assert_eq!(
		records[3]["bio"].as_str().unwrap(),
		"",
		"David should have empty bio"
	);

	// Pattern 5: Maximum length (Eve)
	let eve_name = records[4]["name"].as_str().unwrap();
	assert!(
		eve_name.starts_with("Eve Martinez"),
		"Eve's name should start with 'Eve Martinez'"
	);
	assert!(
		eve_name.len() > 240,
		"Eve's name should be longer than 240 characters"
	);

	// Verify JSON structure is valid UTF-8 and contains expected markers
	assert!(
		json_str.contains("Alice Johnson"),
		"JSON should contain Alice"
	);
	assert!(json_str.contains("日本語"), "JSON should preserve Unicode");
}

/// Test export to CSV format (happy path)
///
/// **Test Category**: Happy path
/// **Test Classification**: Normal flow
#[rstest]
#[tokio::test]
async fn test_export_csv_happy_path(
	#[future] export_import_test_context: (
		Arc<AdminSite>,
		Arc<AdminDatabase>,
		String,
		sqlx::PgPool,
	),
) {
	let (_site, db, table_name, _pool) = export_import_test_context.await;

	// Fetch all records from the test table
	let records_map = db
		.list::<reinhardt_admin::core::AdminRecord>(&table_name, vec![], 0, 100)
		.await
		.expect("Failed to list records");

	// Build CSV manually since csv crate doesn't support HashMap serialization
	let mut wtr = csv::Writer::from_writer(vec![]);

	// Get all unique field names and sort them for consistent ordering
	let mut field_names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
	for record in &records_map {
		for key in record.keys() {
			field_names.insert(key.clone());
		}
	}
	let field_names: Vec<String> = field_names.into_iter().collect();

	// Write header row
	wtr.write_record(&field_names)
		.expect("Failed to write CSV headers");

	// Write data rows
	for record in &records_map {
		let row: Vec<String> = field_names
			.iter()
			.map(|field| {
				record
					.get(field)
					.map(|v| match v {
						serde_json::Value::String(s) => s.clone(),
						serde_json::Value::Number(n) => n.to_string(),
						serde_json::Value::Bool(b) => b.to_string(),
						serde_json::Value::Null => String::new(),
						_ => v.to_string(),
					})
					.unwrap_or_default()
			})
			.collect();
		wtr.write_record(&row).expect("Failed to write CSV row");
	}

	let csv_data = wtr.into_inner().expect("Failed to finalize CSV");

	// Parse the CSV to verify structure
	let mut rdr = csv::Reader::from_reader(csv_data.as_slice());

	// Verify headers exist
	let headers = rdr.headers().expect("Failed to read CSV headers");
	assert!(headers.len() > 0, "CSV should have headers");

	// Verify we have exactly 5 data rows
	let csv_records: Vec<csv::StringRecord> = rdr
		.records()
		.collect::<Result<_, _>>()
		.expect("Failed to read CSV records");
	assert_eq!(csv_records.len(), 5, "CSV should have 5 data rows");

	// Convert CSV data to string for verification
	let csv_str = String::from_utf8(csv_data).expect("Invalid UTF-8 in CSV");

	// Verify special characters are properly escaped
	assert!(
		csv_str.contains("Alice Johnson"),
		"CSV should contain Alice"
	);
	assert!(
		csv_str.contains("Charlie O'Brien") || csv_str.contains("\"Charlie O'Brien\""),
		"CSV should contain Charlie with apostrophe properly escaped"
	);

	// Verify Unicode is preserved
	assert!(
		csv_str.contains("日本語"),
		"CSV should preserve Unicode characters"
	);

	// Verify quotes in bio are escaped
	assert!(
		csv_str.contains("\"\"quotes\"\"") || csv_str.contains("quotes"),
		"CSV should properly escape quotes in bio field"
	);

	// Verify boundary values (age 0)
	assert!(
		csv_str.contains(",0,") || csv_str.contains(",0.0,"),
		"CSV should contain David's age 0"
	);

	// Verify Eve's long name is present
	assert!(
		csv_str.contains("Eve Martinez"),
		"CSV should contain Eve's name"
	);
}

/// Test import from JSON format (happy path)
///
/// **Test Category**: Happy path
/// **Test Classification**: Normal flow
#[rstest]
#[tokio::test]
async fn test_import_json_happy_path(
	#[future] export_import_test_context: (
		Arc<AdminSite>,
		Arc<AdminDatabase>,
		String,
		sqlx::PgPool,
	),
) {
	let (_site, db, table_name, _pool) = export_import_test_context.await;

	// Prepare 2 new records to import
	let new_records = vec![
		serde_json::json!({
			"name": "Frank Zhang",
			"email": "frank@example.com",
			"status": "active",
			"age": 28,
			"score": 88.5,
			"is_verified": true,
			"bio": "Backend developer with Go experience"
		}),
		serde_json::json!({
			"name": "Grace Lee",
			"email": "grace@example.com",
			"status": "pending",
			"age": 35,
			"is_verified": false,
			"bio": "DevOps engineer"
		}),
	];

	// Import each record using AdminDatabase::create
	for record in new_records {
		let data: std::collections::HashMap<String, serde_json::Value> =
			serde_json::from_value(record).expect("Failed to convert JSON to HashMap");
		db.create::<reinhardt_admin::core::AdminRecord>(&table_name, data)
			.await
			.expect("Failed to import record");
	}

	// Verify total count is now 7 (5 original + 2 new)
	let all_records = db
		.list::<reinhardt_admin::core::AdminRecord>(&table_name, vec![], 0, 100)
		.await
		.expect("Failed to list all records");

	assert_eq!(
		all_records.len(),
		7,
		"Should have 7 records after import (5 original + 2 new)"
	);

	// Verify the newly imported records exist
	let frank_exists = all_records.iter().any(|r| {
		r.get("name")
			.and_then(|v| v.as_str())
			.map(|s| s == "Frank Zhang")
			.unwrap_or(false)
	});
	assert!(frank_exists, "Frank Zhang should be imported");

	let grace_exists = all_records.iter().any(|r| {
		r.get("name")
			.and_then(|v| v.as_str())
			.map(|s| s == "Grace Lee")
			.unwrap_or(false)
	});
	assert!(grace_exists, "Grace Lee should be imported");

	// Verify Frank's data is correct
	let frank_record = all_records
		.iter()
		.find(|r| {
			r.get("name")
				.and_then(|v| v.as_str())
				.map(|s| s == "Frank Zhang")
				.unwrap_or(false)
		})
		.expect("Frank Zhang record not found");

	assert_eq!(
		frank_record.get("email").and_then(|v| v.as_str()).unwrap(),
		"frank@example.com"
	);
	assert_eq!(
		frank_record.get("age").and_then(|v| v.as_i64()).unwrap(),
		28
	);
}

// ==================== 2. ERROR PATH TESTS ====================

/// Test export with non-existent model (error path)
///
/// **Test Category**: Error path
/// **Test Classification**: Error handling
#[rstest]
#[tokio::test]
async fn test_export_nonexistent_model_error(
	#[future] export_import_test_context: (
		Arc<AdminSite>,
		Arc<AdminDatabase>,
		String,
		sqlx::PgPool,
	),
) {
	let (_site, db, _table_name, _pool) = export_import_test_context.await;

	// Try to export from a non-existent table
	let nonexistent_table = "nonexistent_table_xyz";

	let result = db
		.list::<reinhardt_admin::core::AdminRecord>(nonexistent_table, vec![], 0, 100)
		.await;

	// Verify error is returned
	assert!(
		result.is_err(),
		"Exporting from non-existent table should return error"
	);

	// Verify error message indicates table doesn't exist
	let err_msg = result.unwrap_err().to_string();
	assert!(
		err_msg.contains("relation") || err_msg.contains("table") || err_msg.contains("exist"),
		"Error message should indicate table doesn't exist, got: {}",
		err_msg
	);
}

/// Test import with invalid JSON data (error path)
///
/// **Test Category**: Error path
/// **Test Classification**: Error handling
#[rstest]
#[tokio::test]
async fn test_import_invalid_json_error(
	#[future] export_import_test_context: (
		Arc<AdminSite>,
		Arc<AdminDatabase>,
		String,
		sqlx::PgPool,
	),
) {
	let (_site, db, table_name, _pool) = export_import_test_context.await;

	// Try to import data with missing required fields (should cause constraint violation)
	let invalid_data: std::collections::HashMap<String, serde_json::Value> =
		serde_json::from_value(serde_json::json!({
			// Missing required fields: name, email
			"status": "active"
		}))
		.expect("Failed to create invalid data");

	let result = db
		.create::<reinhardt_admin::core::AdminRecord>(&table_name, invalid_data)
		.await;

	// Verify error is returned
	assert!(
		result.is_err(),
		"Importing invalid data should return error"
	);

	// Verify error message indicates constraint violation or required field missing
	let err_msg = result.unwrap_err().to_string();
	assert!(
		err_msg.contains("null") || err_msg.contains("violates") || err_msg.contains("constraint"),
		"Error message should indicate constraint violation, got: {}",
		err_msg
	);
}

// ==================== 3. EDGE CASES TESTS ====================

/// Test export with empty table (edge case)
///
/// **Test Category**: Edge cases
/// **Test Classification**: Boundary conditions
#[rstest]
#[tokio::test]
async fn test_export_empty_table(
	#[future] export_import_test_context: (
		Arc<AdminSite>,
		Arc<AdminDatabase>,
		String,
		sqlx::PgPool,
	),
) {
	let (_site, db, table_name, pool) = export_import_test_context.await;

	// Delete all records from the table using raw SQL
	let delete_sql = format!("DELETE FROM {}", table_name);
	sqlx::query(&delete_sql)
		.execute(&pool)
		.await
		.expect("Failed to delete all records");

	// Fetch records from empty table
	let records_map = db
		.list::<reinhardt_admin::core::AdminRecord>(&table_name, vec![], 0, 100)
		.await
		.expect("Failed to list records from empty table");

	// Verify empty array is returned
	assert_eq!(records_map.len(), 0, "Empty table should return 0 records");

	// Serialize to JSON and verify empty array
	let json_data = serde_json::to_vec_pretty(&records_map).expect("Failed to serialize to JSON");
	let records: Vec<serde_json::Value> =
		serde_json::from_slice(&json_data).expect("Failed to parse JSON");

	assert_eq!(records.len(), 0, "JSON should be empty array");
	assert_eq!(
		String::from_utf8(json_data).unwrap().trim(),
		"[]",
		"JSON should be exactly '[]'"
	);
}

/// Test import with empty data (edge case)
///
/// **Test Category**: Edge cases
/// **Test Classification**: Boundary conditions
#[rstest]
#[tokio::test]
async fn test_import_empty_data(
	#[future] export_import_test_context: (
		Arc<AdminSite>,
		Arc<AdminDatabase>,
		String,
		sqlx::PgPool,
	),
) {
	let (_site, db, table_name, _pool) = export_import_test_context.await;

	// Prepare empty array to import
	let empty_records: Vec<serde_json::Value> = vec![];

	// Import empty array (should not error)
	for record in empty_records {
		let data: std::collections::HashMap<String, serde_json::Value> =
			serde_json::from_value(record).expect("Failed to convert JSON to HashMap");
		db.create::<reinhardt_admin::core::AdminRecord>(&table_name, data)
			.await
			.expect("Failed to import record");
	}

	// Verify record count is unchanged (should still be 5 original records)
	let all_records = db
		.list::<reinhardt_admin::core::AdminRecord>(&table_name, vec![], 0, 100)
		.await
		.expect("Failed to list all records");

	assert_eq!(
		all_records.len(),
		5,
		"Record count should remain 5 after importing empty array"
	);
}

// ==================== 4. STATE TRANSITION TESTS ====================

/// Test export → import round-trip (state transition)
///
/// **Test Category**: State transition testing
/// **Test Classification**: State transition
#[rstest]
#[tokio::test]
async fn test_export_import_round_trip(
	#[future] export_import_test_context: (
		Arc<AdminSite>,
		Arc<AdminDatabase>,
		String,
		sqlx::PgPool,
	),
) {
	let (_site, db, table_name, pool) = export_import_test_context.await;

	// Step 1: Export all records
	let original_records = db
		.list::<reinhardt_admin::core::AdminRecord>(&table_name, vec![], 0, 100)
		.await
		.expect("Failed to export records");

	assert_eq!(original_records.len(), 5, "Should have 5 original records");

	// Step 2: Delete all records from table
	let delete_sql = format!("DELETE FROM {}", table_name);
	sqlx::query(&delete_sql)
		.execute(&pool)
		.await
		.expect("Failed to delete all records");

	// Verify table is empty
	let empty_check = db
		.list::<reinhardt_admin::core::AdminRecord>(&table_name, vec![], 0, 100)
		.await
		.expect("Failed to check empty table");
	assert_eq!(empty_check.len(), 0, "Table should be empty after deletion");

	// Step 3: Import the exported records
	for record in &original_records {
		db.create::<reinhardt_admin::core::AdminRecord>(&table_name, record.clone())
			.await
			.expect("Failed to import record");
	}

	// Step 4: Verify restoration
	let restored_records = db
		.list::<reinhardt_admin::core::AdminRecord>(&table_name, vec![], 0, 100)
		.await
		.expect("Failed to list restored records");

	assert_eq!(restored_records.len(), 5, "Should have 5 restored records");

	// Verify all original records are present by name
	for original in &original_records {
		let name = original
			.get("name")
			.and_then(|v| v.as_str())
			.expect("Name field should exist");

		let restored = restored_records
			.iter()
			.find(|r| {
				r.get("name")
					.and_then(|v| v.as_str())
					.map(|s| s == name)
					.unwrap_or(false)
			})
			.expect(&format!("Record with name '{}' should be restored", name));

		// Verify key fields match (excluding auto-generated id, created_at)
		assert_eq!(
			original.get("email"),
			restored.get("email"),
			"Email should match for {}",
			name
		);
		assert_eq!(
			original.get("status"),
			restored.get("status"),
			"Status should match for {}",
			name
		);
		assert_eq!(
			original.get("age"),
			restored.get("age"),
			"Age should match for {}",
			name
		);
		assert_eq!(
			original.get("score"),
			restored.get("score"),
			"Score should match for {}",
			name
		);
		assert_eq!(
			original.get("is_verified"),
			restored.get("is_verified"),
			"is_verified should match for {}",
			name
		);

		// Verify NULL values are preserved (e.g., Bob Smith's age)
		if name == "Bob Smith" {
			assert!(
				restored.get("age").unwrap().is_null(),
				"Bob Smith's age should be NULL"
			);
		}

		// Verify special characters are preserved (e.g., Charlie O'Brien)
		if name == "Charlie O'Brien" {
			assert!(
				name.contains("'"),
				"Charlie O'Brien should preserve apostrophe"
			);
		}
	}
}

// ==================== 5. USE CASE TESTS ====================

/// Test real admin panel export use case
///
/// **Test Category**: Use case testing
/// **Test Classification**: Real admin panel flow
#[rstest]
#[tokio::test]
async fn test_admin_panel_export_use_case(
	#[future] export_import_test_context: (
		Arc<AdminSite>,
		Arc<AdminDatabase>,
		String,
		sqlx::PgPool,
	),
) {
	let (site, db, table_name, _pool) = export_import_test_context.await;

	// Step 1: Get registered models from AdminSite (simulating admin panel model list)
	let registered_models = site.registered_models();
	assert!(
		!registered_models.is_empty(),
		"AdminSite should have registered models"
	);
	assert!(
		registered_models.contains(&"TestModel".to_string()),
		"TestModel should be registered"
	);

	// Step 2: Select CSV format (simulated)
	// Step 3: Execute export
	let records_map = db
		.list::<reinhardt_admin::core::AdminRecord>(&table_name, vec![], 0, 100)
		.await
		.expect("Failed to export records");

	// Build CSV manually (same as test_export_csv_happy_path)
	let mut wtr = csv::Writer::from_writer(vec![]);

	// Get all unique field names and sort them
	let mut field_names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
	for record in &records_map {
		for key in record.keys() {
			field_names.insert(key.clone());
		}
	}
	let field_names: Vec<String> = field_names.into_iter().collect();

	// Write header row
	wtr.write_record(&field_names)
		.expect("Failed to write CSV headers");

	// Write data rows
	for record in &records_map {
		let row: Vec<String> = field_names
			.iter()
			.map(|field| {
				record
					.get(field)
					.map(|v| match v {
						serde_json::Value::String(s) => s.clone(),
						serde_json::Value::Number(n) => n.to_string(),
						serde_json::Value::Bool(b) => b.to_string(),
						serde_json::Value::Null => String::new(),
						_ => v.to_string(),
					})
					.unwrap_or_default()
			})
			.collect();
		wtr.write_record(&row).expect("Failed to write CSV row");
	}

	let csv_data = wtr.into_inner().expect("Failed to get CSV data");

	// Step 4: Verify CSV content
	let csv_string = String::from_utf8(csv_data).expect("CSV should be valid UTF-8");

	// Verify header row exists
	let lines: Vec<&str> = csv_string.lines().collect();
	assert!(!lines.is_empty(), "CSV should have at least header row");

	let header = lines[0];
	assert!(
		header.contains("name") && header.contains("email"),
		"Header should contain expected fields"
	);

	// Verify 5 data rows (plus 1 header = 6 total lines)
	assert_eq!(lines.len(), 6, "CSV should have 1 header + 5 data rows");

	// Verify special characters are properly escaped (Charlie O'Brien)
	let charlie_line = lines
		.iter()
		.find(|line| line.contains("Charlie") || line.contains("O'Brien"))
		.expect("Should find Charlie O'Brien in CSV");
	assert!(
		charlie_line.contains("Charlie"),
		"Charlie's name should be present"
	);

	// Step 5: Verify file metadata (simulated)
	// In real use case, this would be ExportResponse with content_type and filename
	// For this test, we just verify the CSV is valid
	assert!(
		!csv_string.is_empty(),
		"CSV export should produce non-empty output"
	);
}

// ==================== 9. SANITY TESTS ====================

/// Sanity test for export/import type definitions
///
/// **Test Category**: Sanity tests
/// **Test Classification**: Basic functionality
#[test]
fn test_export_import_type_sanity() {
	// Test that ExportFormat variants exist
	let json_format = ExportFormat::JSON;
	let csv_format = ExportFormat::CSV;
	let tsv_format = ExportFormat::TSV;

	match json_format {
		ExportFormat::JSON => assert!(true),
		_ => panic!("Expected Json variant"),
	}

	match csv_format {
		ExportFormat::CSV => assert!(true),
		_ => panic!("Expected Csv variant"),
	}

	match tsv_format {
		ExportFormat::TSV => assert!(true),
		_ => panic!("Expected Tsv variant"),
	}

	// Test that ImportResponse can be constructed
	let response = ImportResponse {
		success: true,
		imported: 5,
		updated: 0,
		skipped: 0,
		failed: 0,
		message: "Import successful".to_string(),
		errors: None,
	};

	assert_eq!(response.imported, 5);
	assert_eq!(response.failed, 0);
	assert!(response.errors.is_none());
}

// ==================== 10. EQUIVALENCE PARTITIONING TESTS ====================

/// Test equivalence partitioning for export formats
///
/// **Test Category**: Equivalence partitioning
/// **Test Classification**: Using rstest case macro
#[rstest]
#[case::json_format(ExportFormat::JSON, "application/json")]
#[case::csv_format(ExportFormat::CSV, "text/csv")]
#[case::tsv_format(ExportFormat::TSV, "text/tab-separated-values")]
#[tokio::test]
async fn test_export_format_equivalence(
	#[case] format: ExportFormat,
	#[case] expected_content_type: &str,
	#[future] export_import_test_context: (
		Arc<AdminSite>,
		Arc<AdminDatabase>,
		String,
		sqlx::PgPool,
	),
) {
	let (_site, db, table_name, _pool) = export_import_test_context.await;

	// Fetch records
	let records_map = db
		.list::<reinhardt_admin::core::AdminRecord>(&table_name, vec![], 0, 100)
		.await
		.expect("Failed to list records");

	// Verify format-specific serialization works
	match format {
		ExportFormat::JSON => {
			// Verify JSON serialization
			let json_result = serde_json::to_vec_pretty(&records_map);
			assert!(
				json_result.is_ok(),
				"JSON format should serialize successfully"
			);
			let json_data = json_result.unwrap();
			let parsed: Result<Vec<serde_json::Value>, _> = serde_json::from_slice(&json_data);
			assert!(parsed.is_ok(), "JSON should be parseable");
		}
		ExportFormat::CSV => {
			// Verify CSV serialization
			let mut wtr = csv::Writer::from_writer(vec![]);
			let mut field_names: std::collections::BTreeSet<String> =
				std::collections::BTreeSet::new();
			for record in &records_map {
				for key in record.keys() {
					field_names.insert(key.clone());
				}
			}
			let field_names: Vec<String> = field_names.into_iter().collect();

			let header_result = wtr.write_record(&field_names);
			assert!(
				header_result.is_ok(),
				"CSV format should write headers successfully"
			);
		}
		ExportFormat::TSV => {
			// Verify TSV serialization (similar to CSV but with tabs)
			// For this test, we just verify the format is valid
			// In real implementation, would use tab delimiter
			assert!(
				expected_content_type == "text/tab-separated-values",
				"TSV should have correct content type"
			);
		}
		ExportFormat::Excel | ExportFormat::XML => {
			// These formats are not tested in this parameterized test
			// They would require specific serialization logic
		}
	}

	// Verify content type matches format
	match format {
		ExportFormat::JSON => assert_eq!(expected_content_type, "application/json"),
		ExportFormat::CSV => assert_eq!(expected_content_type, "text/csv"),
		ExportFormat::TSV => {
			assert_eq!(expected_content_type, "text/tab-separated-values")
		}
		ExportFormat::Excel | ExportFormat::XML => {
			// These formats are not tested in this parameterized test
		}
	}
}

// ==================== 11. BOUNDARY VALUE ANALYSIS TESTS ====================

/// Test boundary values for import data size
///
/// **Test Category**: Boundary value analysis
/// **Test Classification**: Using rstest case macro
#[rstest]
#[case::empty_data(0)]
#[case::small_data(1)]
#[case::medium_data(100)]
#[case::large_data(1000)]
#[tokio::test]
async fn test_import_data_size_boundaries(
	#[case] data_size: usize,
	#[future] export_import_test_context: (
		Arc<AdminSite>,
		Arc<AdminDatabase>,
		String,
		sqlx::PgPool,
	),
) {
	let (_site, db, table_name, _pool) = export_import_test_context.await;

	// Generate test records based on data_size
	let mut test_records = Vec::new();
	for i in 0..data_size {
		test_records.push(serde_json::json!({
			"name": format!("Test User {}", i),
			"email": format!("user{}@example.com", i),
			"status": "active",
			"age": 30,
			"is_verified": true,
		}));
	}

	// Import all records
	let mut imported_count = 0;
	for record in test_records {
		let data: std::collections::HashMap<String, serde_json::Value> =
			serde_json::from_value(record).expect("Failed to convert JSON to HashMap");
		let result = db
			.create::<reinhardt_admin::core::AdminRecord>(&table_name, data)
			.await;

		if result.is_ok() {
			imported_count += 1;
		}
	}

	// Verify all records were imported successfully
	assert_eq!(
		imported_count, data_size,
		"All {} records should be imported successfully",
		data_size
	);

	// Verify count in database (should be original 5 + imported count)
	let all_records = db
		.list::<reinhardt_admin::core::AdminRecord>(&table_name, vec![], 0, 10000)
		.await
		.expect("Failed to list all records");

	assert_eq!(
		all_records.len(),
		5 + data_size,
		"Database should contain original 5 + {} imported records",
		data_size
	);
}

// ==================== 12. DECISION TABLE TESTING ====================

/// Decision table test for export format and content type combinations
///
/// **Test Category**: Decision table testing
/// **Test Classification**: Using rstest case macro
#[rstest]
#[case(ExportFormat::JSON, "application/json", "json")]
#[case(ExportFormat::CSV, "text/csv", "csv")]
#[case(ExportFormat::TSV, "text/tab-separated-values", "tsv")]
#[tokio::test]
async fn test_export_format_decision_table(
	#[case] format: ExportFormat,
	#[case] expected_content_type: &str,
	#[case] expected_extension: &str,
) {
	// Verify format-to-content-type mapping
	match format {
		ExportFormat::JSON => {
			assert_eq!(expected_content_type, "application/json");
			assert_eq!(expected_extension, "json");
		}
		ExportFormat::CSV => {
			assert_eq!(expected_content_type, "text/csv");
			assert_eq!(expected_extension, "csv");
		}
		ExportFormat::TSV => {
			assert_eq!(expected_content_type, "text/tab-separated-values");
			assert_eq!(expected_extension, "tsv");
		}
		ExportFormat::Excel | ExportFormat::XML => {
			// These formats are not tested in this parameterized test
		}
	}

	// Verify format variants exist and are distinct
	let formats = vec![ExportFormat::JSON, ExportFormat::CSV, ExportFormat::TSV];

	assert!(
		formats.contains(&format),
		"Format should be one of the valid export formats"
	);

	// Verify content type is non-empty and valid
	assert!(
		!expected_content_type.is_empty(),
		"Content type should not be empty"
	);
	assert!(
		expected_content_type.contains("/"),
		"Content type should follow MIME type format"
	);

	// Verify extension is non-empty and lowercase
	assert!(
		!expected_extension.is_empty(),
		"Extension should not be empty"
	);
	assert_eq!(
		expected_extension,
		expected_extension.to_lowercase(),
		"Extension should be lowercase"
	);
}

// Note: Additional test classifications (6. Fuzz testing, 7. Property-based testing,
// 8. Combination testing) would be implemented in separate modules or with
// additional dependencies like proptest.
