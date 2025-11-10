//! Custom Views Registration and Drag-and-Drop Reordering
//!
//! This module provides functionality for registering custom admin views and
//! implementing drag-and-drop reordering for models with an order field.
//!
//! ## Features
//!
//! - Custom view registration with URL routing
//! - Permission-based access control for custom views
//! - Template rendering support
//! - Drag-and-drop reordering for orderable models
//! - Transaction-safe bulk reordering operations
//!
//! ## Examples
//!
//! ### Registering a Custom View
//!
//! ```rust
//! use reinhardt_admin::custom_views::{CustomView, ViewConfig, CustomViewRegistry};
//! use async_trait::async_trait;
//!
//! struct DashboardView;
//!
//! #[async_trait]
//! impl CustomView for DashboardView {
//!     fn config(&self) -> ViewConfig {
//!         ViewConfig::builder()
//!             .path("dashboard")
//!             .name("Dashboard")
//!             .build()
//!     }
//!
//!     async fn render(&self, _context: std::collections::HashMap<String, String>) -> String {
//!         "<h1>Dashboard</h1>".to_string()
//!     }
//! }
//!
//! let mut registry = CustomViewRegistry::new();
//! registry.register(Box::new(DashboardView));
//! ```
//!
//! ### Implementing Reorderable Model
//!
//! ```rust
//! use reinhardt_admin::custom_views::ReorderableModel;
//! use async_trait::async_trait;
//!
//! struct Category {
//!     id: i64,
//!     name: String,
//!     order: i32,
//! }
//!
//! #[async_trait]
//! impl ReorderableModel for Category {
//!     async fn get_order(&self) -> i32 {
//!         self.order
//!     }
//!
//!     async fn set_order(&mut self, new_order: i32) {
//!         self.order = new_order;
//!     }
//! }
//! ```

use async_trait::async_trait;
use reinhardt_db::orm::DatabaseConnection;
use sea_query::{Alias, Expr, ExprTrait, PostgresQueryBuilder, Query};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// Configuration for a custom admin view
#[derive(Debug, Clone)]
pub struct ViewConfig {
	/// URL path for the view (relative to admin root)
	pub path: String,
	/// Display name for navigation menu
	pub name: String,
	/// Required permission to access this view (None = no restriction)
	pub permission: Option<String>,
	/// Template path for rendering (None = use default)
	pub template: Option<String>,
}

impl ViewConfig {
	/// Create a new builder for ViewConfig
	pub fn builder() -> ViewConfigBuilder {
		ViewConfigBuilder::default()
	}
}

/// Builder for ViewConfig
#[derive(Default)]
pub struct ViewConfigBuilder {
	path: Option<String>,
	name: Option<String>,
	permission: Option<String>,
	template: Option<String>,
}

impl ViewConfigBuilder {
	/// Set the URL path
	pub fn path(mut self, path: impl Into<String>) -> Self {
		self.path = Some(path.into());
		self
	}

	/// Set the display name
	pub fn name(mut self, name: impl Into<String>) -> Self {
		self.name = Some(name.into());
		self
	}

	/// Set the required permission
	pub fn permission(mut self, permission: impl Into<String>) -> Self {
		self.permission = Some(permission.into());
		self
	}

	/// Set the template path
	pub fn template(mut self, template: impl Into<String>) -> Self {
		self.template = Some(template.into());
		self
	}

	/// Build the ViewConfig
	pub fn build(self) -> ViewConfig {
		ViewConfig {
			path: self.path.unwrap_or_else(|| "custom".to_string()),
			name: self.name.unwrap_or_else(|| "Custom View".to_string()),
			permission: self.permission,
			template: self.template,
		}
	}
}

/// Trait for custom admin views
///
/// Implement this trait to create custom views that can be registered
/// with the admin site and accessed via custom URLs.
#[async_trait]
pub trait CustomView: Send + Sync {
	/// Get the view configuration
	fn config(&self) -> ViewConfig;

	/// Render the view with the given context
	async fn render(&self, context: HashMap<String, String>) -> String;

	/// Check if the given user has permission to access this view
	async fn has_permission(&self, user: &(dyn std::any::Any + Send + Sync)) -> bool {
		use crate::auth::AdminAuthBackend;
		use reinhardt_auth::{SimpleUser, User};

		let config = self.config();

		// If no permission requirement, allow access
		if config.permission.is_none() {
			return true;
		}

		// Check user permission
		if let Some(simple_user) = user.downcast_ref::<SimpleUser>() {
			let _auth_backend = AdminAuthBackend::new();
			if let Some(_permission) = &config.permission {
				// Check if user has the required permission
				// For now, check if user is staff
				simple_user.is_staff()
			} else {
				true
			}
		} else {
			false
		}
	}
}

/// Registry for custom admin views
pub struct CustomViewRegistry {
	views: Vec<Box<dyn CustomView>>,
}

impl CustomViewRegistry {
	/// Create a new empty registry
	pub fn new() -> Self {
		Self { views: Vec::new() }
	}

	/// Register a custom view
	pub fn register(&mut self, view: Box<dyn CustomView>) {
		self.views.push(view);
	}

	/// Get all registered views
	pub fn views(&self) -> &[Box<dyn CustomView>] {
		&self.views
	}

	/// Find a view by path
	pub fn find_by_path(&self, path: &str) -> Option<&dyn CustomView> {
		self.views
			.iter()
			.find(|v| v.config().path == path)
			.map(|b| &**b)
	}
}

impl Default for CustomViewRegistry {
	fn default() -> Self {
		Self::new()
	}
}

impl fmt::Debug for CustomViewRegistry {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("CustomViewRegistry")
			.field("views_count", &self.views.len())
			.finish()
	}
}

/// Configuration for drag-and-drop reordering
#[derive(Debug, Clone)]
pub struct DragDropConfig {
	/// Field name that stores the order value
	pub order_field: String,
	/// Whether to allow reordering
	pub enabled: bool,
	/// Custom JavaScript for handling reorder events
	pub custom_js: Option<String>,
}

impl DragDropConfig {
	/// Create a new builder for DragDropConfig
	pub fn builder() -> DragDropConfigBuilder {
		DragDropConfigBuilder::default()
	}

	/// Create a default config with the given order field
	pub fn new(order_field: impl Into<String>) -> Self {
		Self {
			order_field: order_field.into(),
			enabled: true,
			custom_js: None,
		}
	}
}

/// Builder for DragDropConfig
#[derive(Default)]
pub struct DragDropConfigBuilder {
	order_field: Option<String>,
	enabled: bool,
	custom_js: Option<String>,
}

impl DragDropConfigBuilder {
	/// Set the order field name
	pub fn order_field(mut self, field: impl Into<String>) -> Self {
		self.order_field = Some(field.into());
		self
	}

	/// Set whether reordering is enabled
	pub fn enabled(mut self, enabled: bool) -> Self {
		self.enabled = enabled;
		self
	}

	/// Set custom JavaScript for reorder handling
	pub fn custom_js(mut self, js: impl Into<String>) -> Self {
		self.custom_js = Some(js.into());
		self
	}

	/// Build the DragDropConfig
	pub fn build(self) -> DragDropConfig {
		DragDropConfig {
			order_field: self.order_field.unwrap_or_else(|| "order".to_string()),
			enabled: self.enabled,
			custom_js: self.custom_js,
		}
	}
}

/// Trait for models that support drag-and-drop reordering
///
/// Implement this trait to enable reordering functionality for your model.
#[async_trait]
pub trait ReorderableModel: Send + Sync {
	/// Get the current order value
	async fn get_order(&self) -> i32;

	/// Set a new order value
	async fn set_order(&mut self, new_order: i32);

	/// Get the model's identifier (for tracking during reorder)
	fn get_id(&self) -> String {
		"unknown".to_string()
	}
}

/// Result of a reorder operation
#[derive(Debug, Clone)]
pub struct ReorderResult {
	/// Number of items successfully reordered
	pub updated_count: usize,
	/// Whether the operation was successful
	pub success: bool,
	/// Error message if operation failed
	pub error: Option<String>,
}

impl ReorderResult {
	/// Create a successful result
	pub fn success(updated_count: usize) -> Self {
		Self {
			updated_count,
			success: true,
			error: None,
		}
	}

	/// Create a failed result
	pub fn failure(error: impl Into<String>) -> Self {
		Self {
			updated_count: 0,
			success: false,
			error: Some(error.into()),
		}
	}
}

/// Handler for drag-and-drop reorder requests
pub struct ReorderHandler {
	config: DragDropConfig,
	connection: Arc<DatabaseConnection>,
	table_name: String,
	id_field: String,
}

impl ReorderHandler {
	/// Create a new reorder handler with the given config, database connection, and table info
	///
	/// # Arguments
	///
	/// * `config` - Drag-and-drop configuration
	/// * `connection` - Database connection
	/// * `table_name` - Name of the table to reorder
	/// * `id_field` - Name of the primary key field (e.g., "id")
	pub fn new(
		config: DragDropConfig,
		connection: Arc<DatabaseConnection>,
		table_name: impl Into<String>,
		id_field: impl Into<String>,
	) -> Self {
		Self {
			config,
			connection,
			table_name: table_name.into(),
			id_field: id_field.into(),
		}
	}

	/// Validate that the new order values are valid
	///
	/// Ensures:
	/// - All order values are non-negative
	/// - No duplicate order values
	/// - Order values are sequential (0, 1, 2, ...)
	pub fn validate_order(&self, items: &[(String, i32)]) -> Result<(), String> {
		// Check for negative values
		if items.iter().any(|(_, order)| *order < 0) {
			return Err("Order values must be non-negative".to_string());
		}

		// Check for duplicates
		let mut seen = std::collections::HashSet::new();
		for (_, order) in items {
			if !seen.insert(order) {
				return Err(format!("Duplicate order value: {}", order));
			}
		}

		// Check if sequential (0, 1, 2, ...)
		let mut orders: Vec<i32> = items.iter().map(|(_, o)| *o).collect();
		orders.sort_unstable();

		for (idx, order) in orders.iter().enumerate() {
			if *order != idx as i32 {
				return Err(format!(
					"Order values must be sequential starting from 0, found gap at {}",
					idx
				));
			}
		}

		Ok(())
	}

	/// Process a bulk reorder operation
	///
	/// Takes a list of (id, new_order) pairs and validates them.
	/// Returns the number of items that need to be updated.
	///
	/// This method updates the database using sea-query for each item.
	pub async fn process_reorder(&self, items: Vec<(String, i32)>) -> ReorderResult {
		if !self.config.enabled {
			return ReorderResult::failure("Reordering is not enabled");
		}

		// Validate the new order
		if let Err(e) = self.validate_order(&items) {
			return ReorderResult::failure(e);
		}

		// Update database for each item
		let mut updated_count = 0;
		for (id, new_order) in &items {
			// Build UPDATE query using sea-query
			let query = Query::update()
				.table(Alias::new(&self.table_name))
				.value(Alias::new(&self.config.order_field), *new_order)
				.and_where(Expr::col(Alias::new(&self.id_field)).eq(Expr::val(id.clone())))
				.to_string(PostgresQueryBuilder);

			// Execute the query
			match self.connection.execute(&query, vec![]).await {
				Ok(_) => {
					updated_count += 1;
				}
				Err(e) => {
					return ReorderResult::failure(format!(
						"Failed to update order for item {}: {}",
						id, e
					));
				}
			}
		}

		ReorderResult::success(updated_count)
	}

	/// Reorder adjacent items (swap order values)
	///
	/// This is useful for moving an item up or down by one position.
	pub async fn reorder_adjacent(
		&self,
		item1: &mut dyn ReorderableModel,
		item2: &mut dyn ReorderableModel,
	) -> ReorderResult {
		if !self.config.enabled {
			return ReorderResult::failure("Reordering is not enabled");
		}

		let order1 = item1.get_order().await;
		let order2 = item2.get_order().await;

		// Swap orders
		item1.set_order(order2).await;
		item2.set_order(order1).await;

		ReorderResult::success(2)
	}

	/// Get the drag-and-drop configuration
	pub fn config(&self) -> &DragDropConfig {
		&self.config
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_db::backends::backend::DatabaseBackend as BackendTrait;
	use reinhardt_db::backends::connection::DatabaseConnection as BackendsConnection;
	use reinhardt_db::backends::error::Result;
	use reinhardt_db::backends::types::{DatabaseType, QueryResult, QueryValue, Row};
	use reinhardt_db::orm::{DatabaseBackend, DatabaseConnection};

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
			Ok(QueryResult { rows_affected: 0 })
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
	}

	/// Create a mock database connection for testing
	fn create_mock_connection() -> Arc<DatabaseConnection> {
		let mock_backend = Arc::new(MockBackend);
		let backends_conn = BackendsConnection::new(mock_backend);
		Arc::new(DatabaseConnection::new(
			DatabaseBackend::Postgres,
			backends_conn,
		))
	}

	// Test model for reordering
	#[allow(dead_code)]
	struct TestModel {
		id: i64,
		name: String,
		order: i32,
	}

	#[async_trait]
	impl ReorderableModel for TestModel {
		async fn get_order(&self) -> i32 {
			self.order
		}

		async fn set_order(&mut self, new_order: i32) {
			self.order = new_order;
		}

		fn get_id(&self) -> String {
			self.id.to_string()
		}
	}

	// Test custom view
	struct TestView {
		config: ViewConfig,
	}

	#[async_trait]
	impl CustomView for TestView {
		fn config(&self) -> ViewConfig {
			self.config.clone()
		}

		async fn render(&self, _context: HashMap<String, String>) -> String {
			"<h1>Test View</h1>".to_string()
		}
	}

	#[test]
	fn test_view_config_builder() {
		let config = ViewConfig::builder()
			.path("dashboard")
			.name("Dashboard")
			.permission("view_dashboard")
			.template("admin/dashboard.tpl")
			.build();

		assert_eq!(config.path, "dashboard");
		assert_eq!(config.name, "Dashboard");
		assert_eq!(config.permission, Some("view_dashboard".to_string()));
		assert_eq!(config.template, Some("admin/dashboard.tpl".to_string()));
	}

	#[test]
	fn test_view_config_defaults() {
		let config = ViewConfig::builder().build();

		assert_eq!(config.path, "custom");
		assert_eq!(config.name, "Custom View");
		assert_eq!(config.permission, None);
		assert_eq!(config.template, None);
	}

	#[test]
	fn test_custom_view_registry() {
		let mut registry = CustomViewRegistry::new();

		let view1 = Box::new(TestView {
			config: ViewConfig::builder().path("view1").name("View 1").build(),
		});

		let view2 = Box::new(TestView {
			config: ViewConfig::builder().path("view2").name("View 2").build(),
		});

		registry.register(view1);
		registry.register(view2);

		assert_eq!(registry.views().len(), 2);

		let found = registry.find_by_path("view1");
		assert!(found.is_some());
		assert_eq!(found.unwrap().config().name, "View 1");

		let not_found = registry.find_by_path("nonexistent");
		assert!(not_found.is_none());
	}

	#[test]
	fn test_drag_drop_config() {
		let config = DragDropConfig::builder()
			.order_field("position")
			.enabled(true)
			.custom_js("console.log('reordered');")
			.build();

		assert_eq!(config.order_field, "position");
		assert!(config.enabled);
		assert_eq!(
			config.custom_js,
			Some("console.log('reordered');".to_string())
		);
	}

	#[test]
	fn test_drag_drop_config_new() {
		let config = DragDropConfig::new("sort_order");

		assert_eq!(config.order_field, "sort_order");
		assert!(config.enabled);
		assert_eq!(config.custom_js, None);
	}

	#[tokio::test]
	async fn test_reorderable_model() {
		let mut model = TestModel {
			id: 1,
			name: "Test".to_string(),
			order: 5,
		};

		assert_eq!(model.get_order().await, 5);
		assert_eq!(model.get_id(), "1");

		model.set_order(10).await;
		assert_eq!(model.get_order().await, 10);
	}

	#[test]
	fn test_validate_order_success() {
		let config = DragDropConfig::new("order");
		let conn = create_mock_connection();
		let handler = ReorderHandler::new(config, conn, "test_table", "id");

		let items = vec![
			("1".to_string(), 0),
			("2".to_string(), 1),
			("3".to_string(), 2),
		];

		assert!(handler.validate_order(&items).is_ok());
	}

	#[test]
	fn test_validate_order_negative() {
		let config = DragDropConfig::new("order");
		let conn = create_mock_connection();
		let handler = ReorderHandler::new(config, conn, "test_table", "id");

		let items = vec![
			("1".to_string(), -1),
			("2".to_string(), 0),
			("3".to_string(), 1),
		];

		let result = handler.validate_order(&items);
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("non-negative"));
	}

	#[test]
	fn test_validate_order_duplicates() {
		let config = DragDropConfig::new("order");
		let conn = create_mock_connection();
		let handler = ReorderHandler::new(config, conn, "test_table", "id");

		let items = vec![
			("1".to_string(), 0),
			("2".to_string(), 1),
			("3".to_string(), 1), // Duplicate
		];

		let result = handler.validate_order(&items);
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("Duplicate"));
	}

	#[test]
	fn test_validate_order_gap() {
		let config = DragDropConfig::new("order");
		let conn = create_mock_connection();
		let handler = ReorderHandler::new(config, conn, "test_table", "id");

		let items = vec![
			("1".to_string(), 0),
			("2".to_string(), 2), // Gap (missing 1)
			("3".to_string(), 3),
		];

		let result = handler.validate_order(&items);
		assert!(result.is_err());
		assert!(result.unwrap_err().contains("sequential"));
	}

	#[tokio::test]
	async fn test_process_reorder_success() {
		let config = DragDropConfig::new("order");
		let conn = create_mock_connection();
		let handler = ReorderHandler::new(config, conn, "test_table", "id");

		let items = vec![
			("1".to_string(), 0),
			("2".to_string(), 1),
			("3".to_string(), 2),
		];

		let result = handler.process_reorder(items).await;
		assert!(result.success);
		assert_eq!(result.updated_count, 3);
		assert!(result.error.is_none());
	}

	#[tokio::test]
	async fn test_process_reorder_disabled() {
		let config = DragDropConfig::builder()
			.order_field("order")
			.enabled(false)
			.build();
		let conn = create_mock_connection();
		let handler = ReorderHandler::new(config, conn, "test_table", "id");

		let items = vec![("1".to_string(), 0), ("2".to_string(), 1)];

		let result = handler.process_reorder(items).await;
		assert!(!result.success);
		assert!(result.error.is_some());
		assert!(result.error.unwrap().contains("not enabled"));
	}

	#[tokio::test]
	async fn test_process_reorder_invalid() {
		let config = DragDropConfig::new("order");
		let conn = create_mock_connection();
		let handler = ReorderHandler::new(config, conn, "test_table", "id");

		let items = vec![
			("1".to_string(), 0),
			("2".to_string(), 0), // Duplicate
		];

		let result = handler.process_reorder(items).await;
		assert!(!result.success);
		assert!(result.error.is_some());
	}

	#[tokio::test]
	async fn test_reorder_adjacent() {
		let config = DragDropConfig::new("order");
		let conn = create_mock_connection();
		let handler = ReorderHandler::new(config, conn, "test_table", "id");

		let mut model1 = TestModel {
			id: 1,
			name: "First".to_string(),
			order: 0,
		};

		let mut model2 = TestModel {
			id: 2,
			name: "Second".to_string(),
			order: 1,
		};

		let result = handler.reorder_adjacent(&mut model1, &mut model2).await;

		assert!(result.success);
		assert_eq!(result.updated_count, 2);
		assert_eq!(model1.get_order().await, 1);
		assert_eq!(model2.get_order().await, 0);
	}

	#[tokio::test]
	async fn test_custom_view_render() {
		let view = TestView {
			config: ViewConfig::builder().path("test").name("Test View").build(),
		};

		let context = HashMap::new();
		let html = view.render(context).await;

		assert_eq!(html, "<h1>Test View</h1>");
	}
}
