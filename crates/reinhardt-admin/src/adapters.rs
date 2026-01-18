//! Adapter layer for unified type imports
//!
//! This crate provides a single import path for types that have different
//! implementations on server vs WASM targets:
//!
//! - **Server-side**: Re-exports actual implementations from `reinhardt-admin-core`
//! - **WASM**: Re-exports stub types from `reinhardt-admin-types`
//!
//! This eliminates the need for conditional imports in Server Function files.

// Server-side: Use actual implementations
#[cfg(not(target_arch = "wasm32"))]
pub use crate::core::{
	AdminDatabase, AdminRecord, AdminSite, ExportFormat, ImportBuilder, ImportError, ImportFormat,
	ImportResult, ModelAdmin, ModelAdminConfig, ModelAdminConfigBuilder,
};

// WASM: Use stub types
#[cfg(target_arch = "wasm32")]
pub use crate::types::{
	AdminDatabase, AdminRecord, AdminSite, ExportFormat, ImportBuilder, ImportError, ImportFormat,
	ImportResult, ModelAdmin,
};

// Re-export shared types (DTOs) that are always from reinhardt-admin-types
pub use crate::types::{
	AdminError, BulkDeleteRequest, BulkDeleteResponse, ColumnInfo, DashboardResponse,
	DetailResponse, ExportFormat as ExportFormatRequest, ExportResponse, FieldInfo, FieldType,
	FieldsResponse, FilterChoice, FilterInfo, FilterType, ImportResponse, ListQueryParams,
	ListResponse, ModelInfo, MutationRequest, MutationResponse,
};
