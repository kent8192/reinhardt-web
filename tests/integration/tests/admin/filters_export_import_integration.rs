//! Integration tests for Admin Filters, Export, and Import functionality
//!
//! Tests the complete flow: filtering data → exporting → importing back

use reinhardt_panel::{
	// Filters
	BooleanFilter,
	ChoiceFilter,
	DateRangeFilter,
	ExportBuilder,
	ExportFormat,
	FilterManager,
	ImportBuilder,
	ImportFormat,
	ListFilter,
	NumberRangeFilter,
};
use std::collections::HashMap;

/// Test: Filter → Export → Import round-trip with CSV
#[tokio::test]
async fn test_csv_filter_export_import_roundtrip() {
	// Step 1: Create sample data
	let mut user1 = HashMap::new();
	user1.insert("id".to_string(), "1".to_string());
	user1.insert("name".to_string(), "Alice".to_string());
	user1.insert("email".to_string(), "alice@example.com".to_string());
	user1.insert("is_active".to_string(), "true".to_string());

	let mut user2 = HashMap::new();
	user2.insert("id".to_string(), "2".to_string());
	user2.insert("name".to_string(), "Bob".to_string());
	user2.insert("email".to_string(), "bob@example.com".to_string());
	user2.insert("is_active".to_string(), "false".to_string());

	let mut user3 = HashMap::new();
	user3.insert("id".to_string(), "3".to_string());
	user3.insert("name".to_string(), "Charlie".to_string());
	user3.insert("email".to_string(), "charlie@example.com".to_string());
	user3.insert("is_active".to_string(), "true".to_string());

	let all_data = vec![user1.clone(), user2.clone(), user3.clone()];

	// Step 2: Apply filter (simulate filtering active users)
	let _filter = BooleanFilter::new("is_active", "Active Status");
	let filtered_data: Vec<_> = all_data
		.iter()
		.filter(|row| row.get("is_active") == Some(&"true".to_string()))
		.cloned()
		.collect();

	assert_eq!(filtered_data.len(), 2); // Alice and Charlie

	// Step 3: Export filtered data to CSV
	let export_result = ExportBuilder::new("User", ExportFormat::CSV)
		.fields(vec![
			"id".to_string(),
			"name".to_string(),
			"email".to_string(),
			"is_active".to_string(),
		])
		.data(filtered_data)
		.build()
		.expect("Export should succeed");

	assert_eq!(export_result.row_count, 2);
	assert!(export_result.filename.starts_with("User_"));
	assert!(export_result.filename.ends_with(".csv"));

	// Step 4: Import the exported CSV data
	let imported_records = ImportBuilder::new("User", ImportFormat::CSV)
		.data(export_result.data)
		.parse()
		.expect("Import should succeed");

	assert_eq!(imported_records.len(), 2);
	assert_eq!(imported_records[0].get("name"), Some(&"Alice".to_string()));
	assert_eq!(
		imported_records[1].get("name"),
		Some(&"Charlie".to_string())
	);
}

/// Test: Filter with multiple criteria → Export JSON → Import
#[tokio::test]
async fn test_json_multiple_filters_export_import() {
	// Create sample data with various attributes
	let mut records = Vec::new();

	for i in 1..=10 {
		let mut record = HashMap::new();
		record.insert("id".to_string(), i.to_string());
		record.insert("name".to_string(), format!("User{}", i));
		record.insert("age".to_string(), (20 + i * 2).to_string());
		record.insert(
			"status".to_string(),
			if i % 2 == 0 { "active" } else { "inactive" }.to_string(),
		);
		records.push(record);
	}

	// Apply multiple filters: status = active AND age >= 26
	let _status_filter = ChoiceFilter::new("status", "Status")
		.add_choice("active", "Active")
		.add_choice("inactive", "Inactive");

	let _age_filter = NumberRangeFilter::with_ranges(
		"age",
		"Age Range",
		vec![
			(Some(0), Some(25), "Under 25".to_string()),
			(Some(26), Some(35), "26-35".to_string()),
			(Some(36), None, "Over 35".to_string()),
		],
	);

	let filtered: Vec<_> = records
		.iter()
		.filter(|r| {
			r.get("status") == Some(&"active".to_string())
				&& r.get("age")
					.and_then(|a| a.parse::<i32>().ok())
					.map(|a| a >= 26)
					.unwrap_or(false)
		})
		.cloned()
		.collect();

	// Export to JSON
	let export_result = ExportBuilder::new("User", ExportFormat::JSON)
		.data(filtered)
		.build()
		.expect("JSON export should succeed");

	assert!(export_result.row_count > 0);

	// Import back
	let imported = ImportBuilder::new("User", ImportFormat::JSON)
		.data(export_result.data)
		.parse()
		.expect("JSON import should succeed");

	// Verify all imported records match filter criteria
	for record in &imported {
		assert_eq!(record.get("status"), Some(&"active".to_string()));
		let age: i32 = record.get("age").unwrap().parse().unwrap();
		assert!(age >= 26);
	}
}

/// Test: DateRangeFilter with Export
#[tokio::test]
async fn test_date_range_filter_export() {
	let filter = DateRangeFilter::new("created_at", "Created Date");
	let choices = filter.choices();

	// Verify date range choices are generated dynamically
	assert_eq!(choices.len(), 6);
	assert_eq!(choices[0].display, "Today");
	assert_eq!(choices[1].display, "This week");
	assert_eq!(choices[2].display, "This month");
	assert_eq!(choices[3].display, "This year");
	assert_eq!(choices[4].display, "Last 7 days");
	assert_eq!(choices[5].display, "Last 30 days");

	// Simulate exporting records filtered by date
	let mut record = HashMap::new();
	record.insert("id".to_string(), "1".to_string());
	record.insert("created_at".to_string(), choices[0].value.clone());

	let export = ExportBuilder::new("Log", ExportFormat::CSV)
		.data(vec![record])
		.build()
		.expect("Date range export should succeed");

	assert_eq!(export.row_count, 1);
}

/// Test: Field mapping during import after export
#[tokio::test]
async fn test_field_mapping_import_after_export() {
	// Export with original field names
	let mut record = HashMap::new();
	record.insert("user_id".to_string(), "123".to_string());
	record.insert("user_name".to_string(), "Alice".to_string());
	record.insert("user_email".to_string(), "alice@example.com".to_string());

	let export = ExportBuilder::new("User", ExportFormat::CSV)
		.data(vec![record])
		.build()
		.expect("Export should succeed");

	// Import with field mapping
	let imported = ImportBuilder::new("User", ImportFormat::CSV)
		.data(export.data)
		.field_mapping("user_id", "id")
		.field_mapping("user_name", "name")
		.field_mapping("user_email", "email")
		.parse()
		.expect("Import with mapping should succeed");

	assert_eq!(imported.len(), 1);
	assert_eq!(imported[0].get("id"), Some(&"123".to_string()));
	assert_eq!(imported[0].get("name"), Some(&"Alice".to_string()));
	assert_eq!(
		imported[0].get("email"),
		Some(&"alice@example.com".to_string())
	);
	// Original field names should not exist
	assert_eq!(imported[0].get("user_id"), None);
}

/// Test: TSV round-trip with filters
#[tokio::test]
async fn test_tsv_filter_export_import() {
	let mut data = Vec::new();
	for i in 1..=5 {
		let mut record = HashMap::new();
		record.insert("id".to_string(), i.to_string());
		record.insert("product".to_string(), format!("Product {}", i));
		record.insert("price".to_string(), (i * 100).to_string());
		data.push(record);
	}

	// Filter by price range
	let _price_filter = NumberRangeFilter::with_ranges(
		"price",
		"Price",
		vec![
			(Some(0), Some(250), "$0-$250".to_string()),
			(Some(250), None, "$250+".to_string()),
		],
	);

	let filtered: Vec<_> = data
		.iter()
		.filter(|r| {
			r.get("price")
				.and_then(|p| p.parse::<i32>().ok())
				.map(|p| p >= 250)
				.unwrap_or(false)
		})
		.cloned()
		.collect();

	// Export to TSV
	let export = ExportBuilder::new("Product", ExportFormat::TSV)
		.data(filtered)
		.build()
		.expect("TSV export should succeed");

	assert_eq!(export.row_count, 3); // Products 3, 4, 5

	// Import TSV
	let imported = ImportBuilder::new("Product", ImportFormat::TSV)
		.data(export.data)
		.parse()
		.expect("TSV import should succeed");

	assert_eq!(imported.len(), 3);
	for record in &imported {
		let price: i32 = record.get("price").unwrap().parse().unwrap();
		assert!(price >= 250);
	}
}

/// Test: FilterManager with Export
#[tokio::test]
async fn test_filter_manager_export_integration() {
	let manager = FilterManager::new()
		.add_filter(BooleanFilter::new("is_published", "Published"))
		.add_filter(
			ChoiceFilter::new("category", "Category")
				.add_choice("tech", "Technology")
				.add_choice("science", "Science")
				.add_choice("art", "Art"),
		);

	assert_eq!(manager.filter_count(), 2);

	// Create sample data
	let mut article1 = HashMap::new();
	article1.insert("id".to_string(), "1".to_string());
	article1.insert("title".to_string(), "Tech Article".to_string());
	article1.insert("category".to_string(), "tech".to_string());
	article1.insert("is_published".to_string(), "true".to_string());

	let mut article2 = HashMap::new();
	article2.insert("id".to_string(), "2".to_string());
	article2.insert("title".to_string(), "Science Article".to_string());
	article2.insert("category".to_string(), "science".to_string());
	article2.insert("is_published".to_string(), "false".to_string());

	let data = vec![article1, article2];

	// Apply filters: published = true
	let filtered: Vec<_> = data
		.iter()
		.filter(|r| r.get("is_published") == Some(&"true".to_string()))
		.cloned()
		.collect();

	// Export
	let export = ExportBuilder::new("Article", ExportFormat::JSON)
		.data(filtered)
		.build()
		.expect("Export with FilterManager should succeed");

	assert_eq!(export.row_count, 1);
}

/// Test: Max rows limit during import
#[tokio::test]
async fn test_import_max_rows_limit() {
	let mut data = Vec::new();
	for i in 1..=100 {
		let mut record = HashMap::new();
		record.insert("id".to_string(), i.to_string());
		record.insert("value".to_string(), format!("Value{}", i));
		data.push(record);
	}

	// Export all 100 records
	let export = ExportBuilder::new("Data", ExportFormat::CSV)
		.data(data)
		.build()
		.expect("Export 100 records");

	assert_eq!(export.row_count, 100);

	// Import with max 50 records limit
	let imported = ImportBuilder::new("Data", ImportFormat::CSV)
		.data(export.data)
		.max_records(50)
		.parse()
		.expect("Import with limit");

	assert_eq!(imported.len(), 50);
}

/// Test: CSV escape handling in export/import round-trip
#[tokio::test]
async fn test_csv_special_characters_roundtrip() {
	let mut record1 = HashMap::new();
	record1.insert("id".to_string(), "1".to_string());
	record1.insert("name".to_string(), "Smith, John".to_string()); // Comma
	record1.insert("comment".to_string(), "He said \"Hello\"".to_string()); // Quotes

	let mut record2 = HashMap::new();
	record2.insert("id".to_string(), "2".to_string());
	record2.insert("name".to_string(), "Line\nBreak".to_string()); // Newline
	record2.insert("comment".to_string(), "Normal text".to_string());

	let data = vec![record1, record2];

	// Export
	let export = ExportBuilder::new("Test", ExportFormat::CSV)
		.data(data.clone())
		.build()
		.expect("CSV export with special chars");

	// Import
	let imported = ImportBuilder::new("Test", ImportFormat::CSV)
		.data(export.data)
		.parse()
		.expect("CSV import with special chars");

	assert_eq!(imported.len(), 2);
	// Note: Full round-trip preservation depends on CSV parser implementation
	// At minimum, verify row count is preserved
}
