//! # Reinhardt Admin
//!
//! Django-style auto-generated admin interface for Reinhardt framework.
//!
//! ## Overview
//!
//! Reinhardt Admin provides a powerful, customizable administration panel for managing
//! database models through a web interface. It's inspired by Django's admin panel but
//! built with Rust's type safety and performance in mind.
//!
//! ## Features
//!
//! - ✅ Auto-generated CRUD interfaces from models
//! - ✅ List views with filtering, searching, and sorting
//! - ✅ Detail/Edit forms with validation
//! - ✅ Batch actions (delete, custom actions)
//! - ✅ Permission-based access control
//! - ✅ Customizable templates
//! - ✅ Inline editing for related models
//!
//! ## Quick Start
//!
//! ### 1. Define Your Model
//!
//! ```rust,ignore
//! use reinhardt_orm::Model;
//!
//! #[derive(Model)]
//! #[reinhardt(table_name = "users")]
//! pub struct User {
//!     #[reinhardt(primary_key)]
//!     pub id: i64,
//!     pub username: String,
//!     pub email: String,
//!     pub is_active: bool,
//! }
//! ```
//!
//! ### 2. Register with Admin
//!
//! ```rust,ignore
//! use reinhardt_admin::{AdminSite, ModelAdmin};
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut admin = AdminSite::new("My Admin");
//!
//!     // Simple registration
//!     admin.register::<User>(UserAdmin::default()).await;
//!
//!     // Start admin server
//!     admin.serve("127.0.0.1:8001").await.unwrap();
//! }
//! ```
//!
//! ### 3. Customize the Admin
//!
//! ```rust,ignore
//! use reinhardt_admin::ModelAdmin;
//!
//! struct UserAdmin {
//!     list_display: Vec<String>,
//!     list_filter: Vec<String>,
//!     search_fields: Vec<String>,
//! }
//!
//! impl Default for UserAdmin {
//!     fn default() -> Self {
//!         Self {
//!             list_display: vec!["username".to_string(), "email".to_string(), "is_active".to_string()],
//!             list_filter: vec!["is_active".to_string()],
//!             search_fields: vec!["username".to_string(), "email".to_string()],
//!         }
//!     }
//! }
//! ```
//!
//! ## Comparison with Django Admin
//!
//! | Feature | Django Admin | Reinhardt Admin |
//! |---------|--------------|-----------------|
//! | Auto CRUD | ✅ | ✅ |
//! | List Filters | ✅ | ✅ |
//! | Search | ✅ | ✅ |
//! | Inline Editing | ✅ | ✅ |
//! | Custom Actions | ✅ | ✅ |
//! | Type Safety | ⚠️ Runtime | ✅ Compile-time |
//! | Performance | ⚠️ Python | ✅ Native Rust |
//! | Async Support | ⚠️ Limited | ✅ Full async |
//!
//! ## Security
//!
//! Admin panel should **only** be used for trusted internal staff, not for general users.
//! Always ensure proper authentication and authorization.

// Core modules
pub mod actions;
pub mod filters;
pub mod forms;
pub mod model_admin;
pub mod site;
pub mod views;

// Phase 2 modules - Advanced features
pub mod bulk_edit;
pub mod export;
pub mod import;
pub mod inline;
pub mod widgets;

// Phase 3 modules - Integration features
pub mod audit;
pub mod auth;
pub mod custom_views;
pub mod dashboard;
pub mod database;
pub mod templates;

// Re-exports
pub use actions::{ActionRegistry, ActionResult, AdminAction, DeleteSelectedAction};
pub use audit::{
	AuditAction, AuditLog, AuditLogBuilder, AuditLogQuery, AuditLogQueryBuilder, AuditLogger,
	DatabaseAuditLogger, MemoryAuditLogger,
};
pub use auth::{AdminAuthBackend, AdminAuthMiddleware, AdminPermissionChecker, PermissionAction};
pub use bulk_edit::{BulkEdit, BulkEditConfig, BulkEditField, BulkEditForm, BulkEditResult};
pub use custom_views::{
	CustomView, CustomViewRegistry, DragDropConfig, DragDropConfigBuilder, ReorderHandler,
	ReorderResult, ReorderableModel, ViewConfig, ViewConfigBuilder,
};
pub use dashboard::{
	Activity, ChartData, ChartDataset, ChartType, ChartWidget, DashboardWidget, QuickLink,
	QuickLinksWidget, RecentActivityWidget, StatWidget, TableWidget, UserInfo as DashboardUserInfo,
	WidgetConfig, WidgetContext, WidgetPosition, WidgetRegistry,
};
pub use database::AdminDatabase;
pub use export::{
	CsvExporter, ExportBuilder, ExportConfig, ExportFormat, ExportResult, JsonExporter, TsvExporter,
};
pub use filters::{
	BooleanFilter, ChoiceFilter, DateRangeFilter, FilterManager, FilterSpec, ListFilter,
	NumberRangeFilter,
};
pub use forms::{AdminForm, FieldType, FormBuilder, FormField};
pub use import::{
	CsvImporter, ImportBuilder, ImportConfig, ImportError, ImportFormat, ImportResult,
	JsonImporter, TsvImporter,
};
pub use inline::{InlineForm, InlineFormset, InlineModelAdmin, InlineType};
pub use model_admin::{ModelAdmin, ModelAdminConfig};
pub use site::AdminSite;
pub use templates::{
	AdminContext, AdminTemplateRenderer, DashboardContext, DeleteConfirmationContext,
	FormViewContext, ListViewContext, PaginationContext, UserContext,
};
pub use views::{CreateView, DeleteView, DetailView, ListView, UpdateView};
pub use widgets::{
	EditorType, ImageFormat, ImageUploadConfig, RichTextEditorConfig, Widget, WidgetFactory,
	WidgetType,
};

/// Admin panel error types
#[derive(Debug, thiserror::Error)]
pub enum AdminError {
	/// Model not registered with admin
	#[error("Model '{0}' is not registered with admin")]
	ModelNotRegistered(String),

	/// Permission denied
	#[error("Permission denied: {0}")]
	PermissionDenied(String),

	/// Invalid action
	#[error("Invalid action: {0}")]
	InvalidAction(String),

	/// Database error
	#[error("Database error: {0}")]
	DatabaseError(#[from] anyhow::Error),

	/// Validation error
	#[error("Validation error: {0}")]
	ValidationError(String),

	/// Template rendering error
	#[error("Template rendering error: {0}")]
	TemplateError(String),
}

pub type AdminResult<T> = Result<T, AdminError>;

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_admin_error_display() {
		let err = AdminError::ModelNotRegistered("User".to_string());
		assert_eq!(err.to_string(), "Model 'User' is not registered with admin");

		let err = AdminError::PermissionDenied("Not an admin user".to_string());
		assert_eq!(err.to_string(), "Permission denied: Not an admin user");
	}
}
