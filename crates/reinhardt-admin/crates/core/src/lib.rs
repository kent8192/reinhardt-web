//! Core logic for Reinhardt admin panel
//!
//! This crate provides the core business logic for the admin panel,
//! including:
//! - ModelAdmin trait and configuration
//! - AdminSite registry
//! - Database operations
//! - Import/Export functionality

pub mod database;
pub mod export;
pub mod import;
pub mod model_admin;
pub mod router;
pub mod site;

// Re-exports
pub use database::{AdminDatabase, AdminRecord};
pub use export::ExportFormat;
pub use import::{ImportBuilder, ImportError, ImportFormat, ImportResult};
pub use model_admin::{ModelAdmin, ModelAdminConfig, ModelAdminConfigBuilder};
pub use reinhardt_admin_types::{
	AdminError, AdminResult, BulkDeleteRequest, BulkDeleteResponse, ColumnInfo, DashboardResponse,
	DetailResponse, ExportFormat as TypesExportFormat, FieldInfo, FieldType, FilterChoice,
	FilterInfo, FilterType, ImportResponse, ListQueryParams, ListResponse, ModelInfo,
	MutationRequest, MutationResponse,
};
pub use router::AdminRouter;
pub use site::{AdminSite, AdminSiteConfig};
