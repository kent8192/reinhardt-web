//! Integration tests for Custom Views and Drag-and-Drop Reordering
//!
//! Tests custom view registration, rendering, and model reordering functionality

use async_trait::async_trait;
use reinhardt_panel::{
	CustomView, CustomViewRegistry, DragDropConfig, ReorderableModel, ViewConfig,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Simple test model for reordering
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TestItem {
	id: String,
	name: String,
	order: i32,
}

#[async_trait]
impl ReorderableModel for TestItem {
	async fn get_order(&self) -> i32 {
		self.order
	}

	async fn set_order(&mut self, new_order: i32) {
		self.order = new_order;
	}

	fn get_id(&self) -> String {
		self.id.clone()
	}
}

/// Simple custom view for testing
struct DashboardView;

#[async_trait]
impl CustomView for DashboardView {
	fn config(&self) -> ViewConfig {
		ViewConfig::builder()
			.path("/dashboard")
			.name("Dashboard")
			.build()
	}

	async fn render(&self, _context: HashMap<String, String>) -> String {
		"<h1>Custom Dashboard</h1>".to_string()
	}

	async fn has_permission(&self, _user: &(dyn std::any::Any + Send + Sync)) -> bool {
		true // Allow all for testing
	}
}

/// Test: Register and find custom view
#[tokio::test]
async fn test_custom_view_registration_and_lookup() {
	let mut registry = CustomViewRegistry::new();

	// Register view
	let view = Box::new(DashboardView);
	registry.register(view);

	// Find by path
	let found = registry.find_by_path("/dashboard");
	assert!(found.is_some());

	// Render the view
	let context = HashMap::new();
	let html = found.unwrap().render(context).await;
	assert!(html.contains("Custom Dashboard"));
}

/// Test: Multiple custom views with different paths
#[tokio::test]
async fn test_multiple_custom_views() {
	struct ReportsView;

	#[async_trait]
	impl CustomView for ReportsView {
		fn config(&self) -> ViewConfig {
			ViewConfig::builder()
				.path("/reports")
				.name("Reports")
				.template("/templates/reports.html")
				.build()
		}

		async fn render(&self, _context: HashMap<String, String>) -> String {
			"<h1>Reports</h1>".to_string()
		}

		async fn has_permission(&self, _user: &(dyn std::any::Any + Send + Sync)) -> bool {
			true
		}
	}

	let mut registry = CustomViewRegistry::new();

	// Register multiple views
	registry.register(Box::new(DashboardView));
	registry.register(Box::new(ReportsView));

	// Verify both are registered
	assert_eq!(registry.views().len(), 2);

	// Find each view
	assert!(registry.find_by_path("/dashboard").is_some());
	assert!(registry.find_by_path("/reports").is_some());
	assert!(registry.find_by_path("/nonexistent").is_none());
}

/// Helper function to create mock database connection for testing
fn create_mock_connection() -> Arc<reinhardt_db::orm::DatabaseConnection> {
	use reinhardt_db::backends::backend::DatabaseBackend as BackendTrait;
	use reinhardt_db::backends::connection::DatabaseConnection as BackendsConnection;
	use reinhardt_db::backends::error::Result;
	use reinhardt_db::backends::types::{
		DatabaseType, QueryResult, QueryValue, Row, TransactionExecutor,
	};
	use reinhardt_db::orm::{DatabaseBackend, DatabaseConnection};

	// Mock transaction executor for testing
	struct MockTransactionExecutor;

	#[async_trait::async_trait]
	impl TransactionExecutor for MockTransactionExecutor {
		async fn execute(&mut self, _sql: &str, _params: Vec<QueryValue>) -> Result<QueryResult> {
			Ok(QueryResult { rows_affected: 0 })
		}

		async fn fetch_one(&mut self, _sql: &str, _params: Vec<QueryValue>) -> Result<Row> {
			Ok(Row::new())
		}

		async fn fetch_all(&mut self, _sql: &str, _params: Vec<QueryValue>) -> Result<Vec<Row>> {
			Ok(Vec::new())
		}

		async fn fetch_optional(
			&mut self,
			_sql: &str,
			_params: Vec<QueryValue>,
		) -> Result<Option<Row>> {
			Ok(None)
		}

		async fn commit(self: Box<Self>) -> Result<()> {
			Ok(())
		}

		async fn rollback(self: Box<Self>) -> Result<()> {
			Ok(())
		}
	}

	// Mock backend for testing
	struct MockBackend;

	#[async_trait::async_trait]
	impl BackendTrait for MockBackend {
		fn database_type(&self) -> DatabaseType {
			DatabaseType::Postgres
		}

		fn placeholder(&self, index: usize) -> String {
			format!("${}", index)
		}

		fn supports_returning(&self) -> bool {
			true
		}

		fn supports_on_conflict(&self) -> bool {
			true
		}

		async fn execute(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<QueryResult> {
			Ok(QueryResult { rows_affected: 1 })
		}

		async fn fetch_one(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Row> {
			Ok(Row::new())
		}

		async fn fetch_all(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Vec<Row>> {
			Ok(Vec::new())
		}

		async fn fetch_optional(
			&self,
			_sql: &str,
			_params: Vec<QueryValue>,
		) -> Result<Option<Row>> {
			Ok(None)
		}

		fn as_any(&self) -> &dyn std::any::Any {
			self
		}

		async fn begin(&self) -> Result<Box<dyn TransactionExecutor>> {
			Ok(Box::new(MockTransactionExecutor))
		}
	}

	let mock_backend = Arc::new(MockBackend);
	let backends_conn = BackendsConnection::new(mock_backend);
	Arc::new(DatabaseConnection::new(
		DatabaseBackend::Postgres,
		backends_conn,
	))
}

/// Test: Basic item reordering
#[tokio::test]
async fn test_basic_reordering() {
	use reinhardt_panel::ReorderHandler;

	let config = DragDropConfig::new("order");
	let conn = create_mock_connection();
	let handler = ReorderHandler::new(config, conn, "test_items", "id");

	// Reorder: item1(order=0), item2(order=1), item3(order=2) → item3(0), item1(1), item2(2)
	let items = vec![
		("3".to_string(), 0),
		("1".to_string(), 1),
		("2".to_string(), 2),
	];

	let result = handler.process_reorder(items).await;
	assert!(result.success);
	assert_eq!(result.updated_count, 3);
	assert!(result.error.is_none());
}

/// Test: Reorder validation catches invalid orders
#[tokio::test]
async fn test_reorder_validation_errors() {
	use reinhardt_panel::ReorderHandler;

	let config = DragDropConfig::new("order");
	let conn = create_mock_connection();
	let handler = ReorderHandler::new(config.clone(), conn.clone(), "test_items", "id");

	// Test 1: Negative order
	let items_negative = vec![("1".to_string(), -1), ("2".to_string(), 1)];
	let result = handler.process_reorder(items_negative).await;
	assert!(!result.success);
	assert!(result
		.error
		.as_ref()
		.unwrap()
		.contains("Order values must be non-negative"));

	// Test 2: Duplicate orders
	let handler2 = ReorderHandler::new(config.clone(), conn.clone(), "test_items", "id");
	let items_duplicate = vec![("1".to_string(), 0), ("2".to_string(), 0)];
	let result = handler2.process_reorder(items_duplicate).await;
	assert!(!result.success);
	assert!(result
		.error
		.as_ref()
		.unwrap()
		.contains("Duplicate order value"));

	// Test 3: Gap in sequence
	let handler3 = ReorderHandler::new(config, conn, "test_items", "id");
	let items_gap = vec![("1".to_string(), 0), ("2".to_string(), 2)]; // Missing 1
	let result = handler3.process_reorder(items_gap).await;
	assert!(!result.success);
	assert!(result
		.error
		.as_ref()
		.unwrap()
		.contains("Order values must be sequential"));
}

/// Test: Adjacent item reordering
#[tokio::test]
async fn test_adjacent_reordering() {
	use reinhardt_panel::ReorderHandler;

	let config = DragDropConfig::new("order");
	let conn = create_mock_connection();
	let handler = ReorderHandler::new(config, conn, "test_items", "id");

	// Swap first (id="1", order=0) and second (id="2", order=1) items
	// New order: Second(0), First(1), Third(2)
	let items = vec![
		("2".to_string(), 0), // Second becomes first
		("1".to_string(), 1), // First becomes second
		("3".to_string(), 2), // Third unchanged
	];

	let result = handler.process_reorder(items).await;
	assert!(result.success);
	assert_eq!(result.updated_count, 3); // All three items in the list are updated
	assert!(result.error.is_none());
}

/// Test: Reordering with custom view integration
#[tokio::test]
async fn test_reordering_with_custom_view() {
	use reinhardt_panel::ReorderHandler;

	// Simulate custom view managing items: Alpha(0), Beta(1), Gamma(2)
	let config = DragDropConfig::new("order");
	let conn = create_mock_connection();
	let handler = ReorderHandler::new(config.clone(), conn.clone(), "custom_items", "id");

	// Initial reorder: [a=0, b=1, c=2] → [c=0, a=1, b=2]
	// This simulates user dragging Gamma to the top
	let items = vec![
		("c".to_string(), 0), // Gamma → position 0
		("a".to_string(), 1), // Alpha → position 1
		("b".to_string(), 2), // Beta → position 2
	];

	let result = handler.process_reorder(items).await;
	assert!(result.success);
	assert_eq!(result.updated_count, 3);
	assert!(result.error.is_none());

	// Second reorder: Reverse the order [c=0, a=1, b=2] → [b=0, a=1, c=2]
	let handler2 = ReorderHandler::new(config.clone(), conn.clone(), "custom_items", "id");
	let items2 = vec![
		("b".to_string(), 0), // Beta → position 0
		("a".to_string(), 1), // Alpha stays at 1
		("c".to_string(), 2), // Gamma → position 2
	];

	let result2 = handler2.process_reorder(items2).await;
	assert!(result2.success);
	assert_eq!(result2.updated_count, 3); // All three items in the list are updated
	assert!(result2.error.is_none());
}

/// Test: Disabled drag-drop configuration
#[tokio::test]
async fn test_drag_drop_disabled() {
	use reinhardt_panel::ReorderHandler;

	let config = DragDropConfig::builder()
		.order_field("order")
		.enabled(false)
		.build();

	let conn = create_mock_connection();
	let handler = ReorderHandler::new(config, conn, "test_items", "id");

	let items = vec![("1".to_string(), 0)];

	let result = handler.process_reorder(items).await;
	assert!(!result.success);
	assert!(result
		.error
		.as_ref()
		.unwrap()
		.contains("Reordering is not enabled"));
}

/// Test: Custom JavaScript integration
#[tokio::test]
async fn test_custom_js_configuration() {
	let custom_js = r#"
        document.addEventListener('dragend', function(e) {
            updateOrder(e.target.dataset.id);
        });
    "#;

	let config = DragDropConfig::builder()
		.order_field("position")
		.custom_js(custom_js.to_string())
		.build();

	assert_eq!(config.order_field, "position");
	assert!(config.custom_js.is_some());
	assert!(config.custom_js.as_ref().unwrap().contains("dragend"));
}

/// Test: Permission-based custom view access
#[tokio::test]
async fn test_permission_based_view_access() {
	struct AdminOnlyView;

	#[async_trait]
	impl CustomView for AdminOnlyView {
		fn config(&self) -> ViewConfig {
			ViewConfig::builder()
				.path("/admin/settings")
				.name("Admin Settings")
				.permission("admin.settings.view")
				.build()
		}

		async fn render(&self, _context: HashMap<String, String>) -> String {
			"<h1>Admin Settings</h1>".to_string()
		}

		async fn has_permission(&self, user: &(dyn std::any::Any + Send + Sync)) -> bool {
			use reinhardt_auth::{SimpleUser, User};

			// For testing: check if user has admin.settings.view permission
			if let Some(simple_user) = user.downcast_ref::<SimpleUser>() {
				// In a real implementation, check user permissions
				// For testing, just check if user is staff
				simple_user.is_staff()
			} else {
				false
			}
		}
	}

	let view = AdminOnlyView;

	// Test with admin user (staff)
	let admin_user = reinhardt_auth::SimpleUser {
		id: uuid::Uuid::new_v4(),
		username: "admin".to_string(),
		email: "admin@example.com".to_string(),
		is_active: true,
		is_admin: true,
		is_staff: true, // Required for has_permission to return true
		is_superuser: true,
	};
	assert!(
		view.has_permission(&admin_user as &(dyn std::any::Any + Send + Sync))
			.await
	);

	// Test with regular user (not staff)
	let regular_user = reinhardt_auth::SimpleUser {
		id: uuid::Uuid::new_v4(),
		username: "user".to_string(),
		email: "user@example.com".to_string(),
		is_active: true,
		is_admin: false,
		is_staff: false, // Not staff, should fail permission check
		is_superuser: false,
	};
	assert!(
		!view
			.has_permission(&regular_user as &(dyn std::any::Any + Send + Sync))
			.await
	);
}

/// Test: Large-scale reordering performance
#[tokio::test]
async fn test_large_scale_reordering() {
	use reinhardt_panel::ReorderHandler;

	let config = DragDropConfig::new("order");
	let conn = create_mock_connection();
	let handler = ReorderHandler::new(config, conn, "test_items", "id");

	// Create 100 items in reversed order: [99, 98, ..., 1, 0]
	// This simulates moving item 99 to position 0, item 98 to position 1, etc.
	let items: Vec<(String, i32)> = (0..100)
		.rev()
		.enumerate()
		.map(|(new_order, old_id)| (old_id.to_string(), new_order as i32))
		.collect();

	let start = std::time::Instant::now();
	let result = handler.process_reorder(items).await;
	let duration = start.elapsed();

	assert!(result.success);
	assert_eq!(result.updated_count, 100);
	assert!(result.error.is_none());

	// Performance check: should complete in under 100ms
	assert!(
		duration.as_millis() < 100,
		"Reordering 100 items took too long: {:?}",
		duration
	);
}

/// Test: Reordering with context data in custom view
#[tokio::test]
async fn test_custom_view_with_reorder_context() {
	struct CategoryView;

	#[async_trait]
	impl CustomView for CategoryView {
		fn config(&self) -> ViewConfig {
			ViewConfig::builder()
				.path("/categories")
				.name("Categories")
				.build()
		}

		async fn render(&self, context: HashMap<String, String>) -> String {
			let reorder_enabled = context
				.get("reorder_enabled")
				.map(|v| v == "true")
				.unwrap_or(false);

			if reorder_enabled {
				"<div class='reorderable'>Categories with drag-drop</div>".to_string()
			} else {
				"<div>Categories (read-only)</div>".to_string()
			}
		}

		async fn has_permission(&self, _user: &(dyn std::any::Any + Send + Sync)) -> bool {
			true
		}
	}

	let view = CategoryView;

	// Context with reordering enabled
	let mut context_enabled = HashMap::new();
	context_enabled.insert("reorder_enabled".to_string(), "true".to_string());

	let html_enabled = view.render(context_enabled).await;
	assert!(html_enabled.contains("reorderable"));
	assert!(html_enabled.contains("drag-drop"));

	// Context with reordering disabled
	let mut context_disabled = HashMap::new();
	context_disabled.insert("reorder_enabled".to_string(), "false".to_string());

	let html_disabled = view.render(context_disabled).await;
	assert!(html_disabled.contains("read-only"));
	assert!(!html_disabled.contains("drag-drop"));
}
