//! Request types for admin panel API

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Query parameters for list endpoint
#[derive(Debug, Deserialize, Default)]
pub struct ListQueryParams {
	/// Page number (1-indexed)
	pub page: Option<u64>,
	/// Items per page
	pub page_size: Option<u64>,
	/// Search query
	pub search: Option<String>,
	/// Sort field (prefix with "-" for descending, e.g., "created_at" or "-created_at")
	pub sort_by: Option<String>,
	/// Filter field=value pairs
	#[serde(flatten)]
	pub filters: HashMap<String, String>,
}

/// Request body for create/update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationRequest {
	/// Data to create/update
	#[serde(flatten)]
	pub data: HashMap<String, serde_json::Value>,
}

/// Request body for bulk delete
#[derive(Debug, Deserialize)]
pub struct BulkDeleteRequest {
	/// IDs to delete
	pub ids: Vec<String>,
}

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ExportFormat {
	#[default]
	Json,
	Csv,
	Tsv,
}
