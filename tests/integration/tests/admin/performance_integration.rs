//! Performance tests for optimized admin features
//!
//! Tests verify performance improvements from:
//! - MemoryAuditLogger indexed queries
//! - Parallel CSV/JSON import processing
//! - FilterManager cached lookups
//! - Concurrent dashboard widget loading

use async_trait::async_trait;
use reinhardt_panel::{
	audit::{AuditAction, AuditLog, AuditLogQuery, AuditLogger, MemoryAuditLogger},
	dashboard::{DashboardWidget, WidgetConfig, WidgetContext, WidgetPosition, WidgetRegistry},
	import::{CsvImporter, JsonImporter},
	AdminResult, BooleanFilter, FilterManager,
};
use reinhardt_test::fixtures::admin::generate_test_audit_entries;
use std::sync::Arc;
use std::time::Instant;

/// Test: MemoryAuditLogger indexed query performance
#[tokio::test]
async fn test_audit_logger_indexed_query_performance() {
	let logger = MemoryAuditLogger::new();

	// Generate and insert 10,000 audit logs using fixture helper
	let test_entries = generate_test_audit_entries(10_000, 100, 10);

	let start = Instant::now();
	for entry in test_entries {
		let log = AuditLog::builder()
			.user_id(entry.user_id)
			.model_name(entry.model_name)
			.object_id(entry.object_id)
			.action(if entry.action == "Create" {
				AuditAction::Create
			} else if entry.action == "Update" {
				AuditAction::Update
			} else {
				AuditAction::Delete
			})
			.timestamp(entry.timestamp)
			.build();

		logger.log(log).await.expect("Failed to log");
	}
	let insert_duration = start.elapsed();
	println!("Inserted 10,000 logs in {:?}", insert_duration);

	// Query by user_id (should use index)
	let start = Instant::now();
	let query = AuditLogQuery::builder()
		.user_id("user_50".to_string())
		.build();
	let results = logger.query(&query).await.expect("Query failed");
	let query_duration = start.elapsed();

	println!(
		"Indexed query for user_50: {} results in {:?}",
		results.len(),
		query_duration
	);

	// Should be fast (< 10ms for indexed lookup)
	assert!(
		query_duration.as_millis() < 10,
		"Indexed query too slow: {:?}",
		query_duration
	);
	assert_eq!(results.len(), 100); // 10,000 / 100 users = 100 per user

	// Query by model_name (should use index)
	let start = Instant::now();
	let query = AuditLogQuery::builder()
		.model_name("Model_5".to_string())
		.limit(10_000) // Set high limit to get all matching entries
		.build();
	let results = logger.query(&query).await.expect("Query failed");
	let query_duration = start.elapsed();

	println!(
		"Indexed query for Model_5: {} results in {:?}",
		results.len(),
		query_duration
	);

	assert!(
		query_duration.as_millis() < 10,
		"Indexed query too slow: {:?}",
		query_duration
	);
	assert_eq!(results.len(), 1000); // 10,000 / 10 models = 1000 per model

	// Query by action (should use index)
	let start = Instant::now();
	let query = AuditLogQuery::builder().action(AuditAction::Create).build();
	let results = logger.query(&query).await.expect("Query failed");
	let query_duration = start.elapsed();

	println!(
		"Indexed query for Create action: {} results in {:?}",
		results.len(),
		query_duration
	);

	assert!(
		query_duration.as_millis() < 10,
		"Indexed query too slow: {:?}",
		query_duration
	);
}

/// Test: CSV import parallel processing performance
#[tokio::test]
async fn test_csv_import_parallel_processing() {
	// Generate large CSV (5000 rows to trigger parallel processing)
	let mut csv = "id,name,email,age\n".to_string();
	for i in 0..5000 {
		csv.push_str(&format!(
			"{},User{},user{}@example.com,{}\n",
			i,
			i,
			i,
			20 + (i % 50)
		));
	}

	let start = Instant::now();
	let result = CsvImporter::import(csv.as_bytes(), true);
	let duration = start.elapsed();

	assert!(result.is_ok());
	let records = result.unwrap();
	assert_eq!(records.len(), 5000);

	println!("Imported 5000 CSV rows in {:?}", duration);

	// Should complete in reasonable time (< 500ms with parallel processing)
	assert!(
		duration.as_millis() < 500,
		"CSV import too slow: {:?}",
		duration
	);
}

/// Test: JSON import parallel processing performance
#[tokio::test]
async fn test_json_import_parallel_processing() {
	// Generate large JSON array (3000 items to trigger parallel processing)
	let mut json_items = Vec::new();
	for i in 0..3000 {
		json_items.push(format!(
			r#"{{"id":"{}","name":"User{}","email":"user{}@example.com","age":{}}}"#,
			i,
			i,
			i,
			20 + (i % 50)
		));
	}
	let json = format!("[{}]", json_items.join(","));

	let start = Instant::now();
	let result = JsonImporter::import(json.as_bytes());
	let duration = start.elapsed();

	assert!(result.is_ok());
	let records = result.unwrap();
	assert_eq!(records.len(), 3000);

	println!("Imported 3000 JSON items in {:?}", duration);

	// Should complete in reasonable time (< 300ms with parallel processing)
	assert!(
		duration.as_millis() < 300,
		"JSON import too slow: {:?}",
		duration
	);
}

/// Test: FilterManager cached lookup performance
#[tokio::test]
async fn test_filter_manager_cache_performance() {
	// Create manager with 100 filters
	let mut manager = FilterManager::new();
	for i in 0..100 {
		manager = manager.add_filter(BooleanFilter::new(
			format!("field_{}", i),
			format!("Field {}", i),
		));
	}

	// First lookup (cache miss)
	let start = Instant::now();
	let filter = manager.get_filter("field_50");
	let first_duration = start.elapsed();
	assert!(filter.is_some());
	println!("First lookup (cache miss): {:?}", first_duration);

	// Second lookup (cache hit)
	let start = Instant::now();
	let filter = manager.get_filter("field_50");
	let second_duration = start.elapsed();
	assert!(filter.is_some());
	println!("Second lookup (cache hit): {:?}", second_duration);

	// Cache hit should be significantly faster
	assert!(
		second_duration < first_duration,
		"Cache hit not faster than miss"
	);

	// Multiple cached lookups
	let start = Instant::now();
	for _ in 0..1000 {
		let _ = manager.get_filter("field_50");
	}
	let batch_duration = start.elapsed();
	println!("1000 cached lookups: {:?}", batch_duration);

	// Should be very fast (< 5ms for 1000 cached lookups)
	assert!(
		batch_duration.as_millis() < 5,
		"Cached lookups too slow: {:?}",
		batch_duration
	);
}

/// Simple test widget for concurrent loading
struct TestWidget {
	id: String,
	delay_ms: u64,
}

impl TestWidget {
	fn new(id: impl Into<String>, delay_ms: u64) -> Self {
		Self {
			id: id.into(),
			delay_ms,
		}
	}
}

#[async_trait]
impl DashboardWidget for TestWidget {
	fn title(&self) -> &str {
		&self.id
	}

	fn icon(&self) -> Option<&str> {
		None
	}

	fn position(&self) -> WidgetPosition {
		WidgetPosition::TopLeft
	}

	fn size(&self) -> (u32, u32) {
		(4, 3)
	}

	fn refresh_interval(&self) -> Option<u32> {
		None
	}

	async fn is_visible(&self, _permissions: &[String]) -> bool {
		true
	}

	async fn render(&self, _context: &WidgetContext) -> AdminResult<String> {
		Ok(format!("<div>{}</div>", self.id))
	}

	async fn load_data(&self) -> AdminResult<serde_json::Value> {
		// Simulate async data loading with delay
		tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
		Ok(serde_json::json!({
			"id": self.id,
			"value": 42,
		}))
	}
}

/// Test: Dashboard widget concurrent loading performance
#[tokio::test]
async fn test_dashboard_widget_concurrent_loading() {
	let registry = WidgetRegistry::new();

	// Register 10 widgets, each with 100ms load delay
	for i in 0..10 {
		let widget = Arc::new(TestWidget::new(format!("widget_{}", i), 100));
		let config = WidgetConfig {
			id: format!("widget_{}", i),
			position: WidgetPosition::TopLeft,
			size: (1, 1),
			order: i,
			css_classes: vec![],
			style: std::collections::HashMap::new(),
			options: std::collections::HashMap::new(),
		};
		registry
			.register(widget, config)
			.expect("Failed to register");
	}

	let permissions = vec![];

	// Load all widgets concurrently
	let start = Instant::now();
	let results = registry.load_all_data(&permissions).await;
	let concurrent_duration = start.elapsed();

	assert_eq!(results.len(), 10);
	println!(
		"Loaded 10 widgets (100ms each) concurrently in {:?}",
		concurrent_duration
	);

	// Concurrent loading should take ~100ms (not 1000ms sequentially)
	// Allow some overhead, but should be < 300ms
	assert!(
		concurrent_duration.as_millis() < 300,
		"Concurrent loading not fast enough: {:?}",
		concurrent_duration
	);

	// Verify all widgets loaded successfully
	for (id, result) in results {
		assert!(
			result.is_ok(),
			"Widget {} failed to load: {:?}",
			id,
			result.err()
		);
	}
}

/// Test: Dashboard position-specific concurrent loading
#[tokio::test]
async fn test_dashboard_position_concurrent_loading() {
	let registry = WidgetRegistry::new();

	// Register widgets at different positions
	for i in 0..5 {
		let widget = Arc::new(TestWidget::new(format!("top_left_{}", i), 50));
		let config = WidgetConfig {
			id: format!("top_left_{}", i),
			position: WidgetPosition::TopLeft,
			size: (1, 1),
			order: i,
			css_classes: vec![],
			style: std::collections::HashMap::new(),
			options: std::collections::HashMap::new(),
		};
		registry
			.register(widget, config)
			.expect("Failed to register");
	}

	for i in 0..5 {
		let widget = Arc::new(TestWidget::new(format!("top_right_{}", i), 50));
		let config = WidgetConfig {
			id: format!("top_right_{}", i),
			position: WidgetPosition::TopRight,
			size: (1, 1),
			order: i,
			css_classes: vec![],
			style: std::collections::HashMap::new(),
			options: std::collections::HashMap::new(),
		};
		registry
			.register(widget, config)
			.expect("Failed to register");
	}

	let permissions = vec![];

	// Load only TopLeft widgets concurrently
	let start = Instant::now();
	let results = registry
		.load_position_data(WidgetPosition::TopLeft, &permissions)
		.await;
	let duration = start.elapsed();

	assert_eq!(results.len(), 5);
	println!(
		"Loaded 5 TopLeft widgets (50ms each) concurrently in {:?}",
		duration
	);

	// Should take ~50ms (not 250ms sequentially)
	assert!(
		duration.as_millis() < 150,
		"Position-specific concurrent loading too slow: {:?}",
		duration
	);

	// Verify correct widgets loaded
	for (id, _) in &results {
		assert!(id.starts_with("top_left_"));
	}
}

/// Test: Compare sequential vs concurrent widget loading
#[tokio::test]
async fn test_sequential_vs_concurrent_widget_loading() {
	let registry_sequential = WidgetRegistry::new();
	let registry_concurrent = WidgetRegistry::new();

	// Register same widgets in both registries
	for i in 0..8 {
		let widget_seq = Arc::new(TestWidget::new(format!("widget_{}", i), 50));
		let widget_conc = Arc::new(TestWidget::new(format!("widget_{}", i), 50));

		let config_seq = WidgetConfig {
			id: format!("widget_{}", i),
			position: WidgetPosition::Center,
			size: (1, 1),
			order: i,
			css_classes: vec![],
			style: std::collections::HashMap::new(),
			options: std::collections::HashMap::new(),
		};
		let config_conc = config_seq.clone();

		registry_sequential
			.register(widget_seq, config_seq)
			.expect("Failed to register");
		registry_concurrent
			.register(widget_conc, config_conc)
			.expect("Failed to register");
	}

	let permissions = vec![];

	// Sequential loading (simulated)
	let widgets = registry_sequential.get_visible(&permissions).await;
	let start = Instant::now();
	for (widget, _config) in widgets {
		let _ = widget.load_data().await;
	}
	let sequential_duration = start.elapsed();

	println!(
		"Sequential loading of 8 widgets (50ms each): {:?}",
		sequential_duration
	);

	// Concurrent loading
	let start = Instant::now();
	let _results = registry_concurrent.load_all_data(&permissions).await;
	let concurrent_duration = start.elapsed();

	println!(
		"Concurrent loading of 8 widgets (50ms each): {:?}",
		concurrent_duration
	);

	// Concurrent should be significantly faster
	let speedup = sequential_duration.as_millis() as f64 / concurrent_duration.as_millis() as f64;
	println!("Speedup: {:.2}x", speedup);

	assert!(
		concurrent_duration < sequential_duration,
		"Concurrent not faster than sequential"
	);
	assert!(
		speedup > 2.0,
		"Expected at least 2x speedup, got {:.2}x",
		speedup
	);
}
