//! HTTP handlers for admin panel
//!
//! This module provides HTTP handlers for admin panel CRUD operations.
//! All handlers return JSON responses and use dependency injection.

use crate::{
	AdminDatabase, AdminError, AdminRecord, AdminResult, AdminSite, ImportBuilder, ImportError,
	ImportFormat, ImportResult,
};
use hyper::StatusCode;
use reinhardt_admin_types::*;
use reinhardt_db::orm::{Filter, FilterCondition, FilterOperator, FilterValue};
use reinhardt_http::{Request, Response, ViewResult};
use reinhardt_macros::{delete, get, post, put};
use reinhardt_params::{Json, Path, Query};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Query parameters for export endpoint
#[derive(Debug, Deserialize, Default)]
pub struct ExportQueryParams {
	/// Export format
	#[serde(default)]
	pub format: ExportFormat,
}

/// Dashboard handler - returns list of registered models
#[get("/", name = "admin_dashboard")]
pub async fn dashboard(req: Request) -> ViewResult<Response> {
	// Manually resolve dependencies from DI context
	let di_ctx = req
		.extensions
		.get::<reinhardt_di::InjectionContext>()
		.ok_or_else(|| AdminError::ValidationError("DI context not found".into()))?;

	let site = di_ctx
		.resolve::<AdminSite>()
		.await
		.map_err(|e| AdminError::ValidationError(format!("Failed to resolve AdminSite: {}", e)))?;

	// Collect model information
	let models: Vec<ModelInfo> = site
		.registered_models()
		.into_iter()
		.map(|name| {
			let list_url = format!("{}/{}/", site.url_prefix(), name.to_lowercase());
			ModelInfo { name, list_url }
		})
		.collect();

	// Generate CSRF token from request session
	let csrf_token = generate_csrf_token_from_request(&req);

	// Build JSON response
	let response = DashboardResponse {
		site_name: site.name().to_string(),
		url_prefix: site.url_prefix().to_string(),
		models,
		csrf_token,
	};

	// Return JSON response
	Response::ok().with_json(&response)
}

/// Favicon handler - returns favicon image
#[get("/favicon.ico", name = "admin_favicon", use_inject = true)]
pub async fn favicon(#[inject] site: Arc<AdminSite>) -> ViewResult<Response> {
	match site.favicon_data() {
		Some(data) => {
			let content_type = detect_favicon_content_type(&data);
			Ok(Response::ok()
				.with_header("Content-Type", content_type)
				.with_header("Cache-Control", "public, max-age=86400")
				.with_body(data))
		}
		None => Err(AdminError::ValidationError("Favicon not configured".into()).into()),
	}
}

/// Detect favicon content type from magic bytes
fn detect_favicon_content_type(data: &[u8]) -> &'static str {
	// PNG: 89 50 4E 47
	if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
		return "image/png";
	}
	// ICO: 00 00 01 00
	if data.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
		return "image/x-icon";
	}
	// JPEG: FF D8 FF
	if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
		return "image/jpeg";
	}
	// GIF: 47 49 46 38
	if data.starts_with(&[0x47, 0x49, 0x46, 0x38]) {
		return "image/gif";
	}
	// SVG (text-based)
	if data.starts_with(b"<?xml") || data.starts_with(b"<svg") {
		return "image/svg+xml";
	}
	// Default to ICO
	"image/x-icon"
}

/// List handler - returns paginated list of model instances
#[get("/{<str:model>}/", name = "admin_list", use_inject = true)]
pub async fn list(
	Path(model_name): Path<String>,
	Query(params): Query<ListQueryParams>,
	#[inject] site: Arc<AdminSite>,
	#[inject] db: Arc<AdminDatabase>,
) -> ViewResult<Response> {
	let model_admin = site
		.get_model_admin(&model_name)
		.map_err(|e| AdminError::ModelNotRegistered(e.to_string()))?;

	let page = params.page.unwrap_or(1).max(1);
	let page_size = params
		.page_size
		.unwrap_or_else(|| model_admin.list_per_page().unwrap_or(100) as u64);
	let offset = (page - 1) * page_size;

	// Build search filter condition (OR across multiple fields)
	let search_condition = if let Some(search) = &params.search {
		let search_fields = model_admin.search_fields();
		if !search_fields.is_empty() {
			// Build OR condition across all search fields
			let search_filters: Vec<Filter> = search_fields
				.iter()
				.map(|field| {
					Filter::new(
						field.to_string(),
						FilterOperator::Contains,
						FilterValue::String(search.clone()),
					)
				})
				.collect();

			// Create OR condition for search (matches any field)
			Some(FilterCondition::or_filters(search_filters))
		} else {
			None
		}
	} else {
		None
	};

	// Build additional filters from query params (AND logic)
	let mut additional_filters = Vec::new();
	let filter_fields = model_admin.list_filter();
	for field in filter_fields {
		if let Some(value) = params.filters.get(field) {
			additional_filters.push(Filter::new(
				field.to_string(),
				FilterOperator::Eq,
				FilterValue::String(value.clone()),
			));
		}
	}

	let table_name = model_admin.table_name();

	// Get total count
	let count = db
		.count_with_condition::<AdminRecord>(
			table_name,
			search_condition.as_ref(),
			additional_filters.clone(),
		)
		.await
		.map_err(|e| AdminError::DatabaseError(e.to_string()))?;

	// Get items
	let results = db
		.list_with_condition::<AdminRecord>(
			table_name,
			search_condition.as_ref(),
			additional_filters,
			params.sort_by.as_deref(),
			offset,
			page_size,
		)
		.await
		.map_err(|e| AdminError::DatabaseError(e.to_string()))?;

	let total_pages = count.div_ceil(page_size);

	// Generate filter metadata
	let available_filters = generate_mock_filters(&model_name, &params.filters);

	// Generate column info from model admin configuration
	let columns = generate_columns(model_admin.as_ref());

	let response = ListResponse {
		model_name: model_name.to_string(),
		count,
		page,
		page_size,
		total_pages,
		results,
		available_filters: Some(available_filters),
		columns: Some(columns),
	};

	json_response(StatusCode::OK, &response).map_err(Into::into)
}

/// Convert field name to human-readable label
///
/// Transforms snake_case field names to Title Case labels.
/// Examples: "user_name" -> "User Name", "id" -> "Id"
fn field_to_label(field: &str) -> String {
	field
		.split('_')
		.map(|word| {
			let mut chars = word.chars();
			match chars.next() {
				None => String::new(),
				Some(first) => first.to_uppercase().chain(chars).collect(),
			}
		})
		.collect::<Vec<_>>()
		.join(" ")
}

/// Generate column info from model admin configuration
///
/// Creates column metadata for list view based on list_display() fields.
/// Sortable status is determined by checking if the field is in ordering().
fn generate_columns(model_admin: &dyn crate::ModelAdmin) -> Vec<ColumnInfo> {
	let display_fields = model_admin.list_display();
	let ordering = model_admin.ordering();

	// Extract sortable fields from ordering (strip "-" prefix for descending)
	let sortable_fields: Vec<&str> = ordering
		.iter()
		.map(|f| f.strip_prefix('-').unwrap_or(f))
		.collect();

	display_fields
		.iter()
		.map(|field| ColumnInfo {
			field: field.to_string(),
			label: field_to_label(field),
			sortable: sortable_fields.contains(field),
		})
		.collect()
}

/// Generate mock filter metadata for Phase 4
///
/// This is a temporary implementation that provides hardcoded filters.
/// In Phase 5+, this will be replaced with dynamic filter generation
/// based on model definitions.
fn generate_mock_filters(
	_model_name: &str,
	active_filters: &HashMap<String, String>,
) -> Vec<FilterInfo> {
	vec![
		FilterInfo {
			field: "status".to_string(),
			title: "Status".to_string(),
			filter_type: FilterType::Choice {
				choices: vec![
					FilterChoice {
						value: "active".to_string(),
						label: "Active".to_string(),
					},
					FilterChoice {
						value: "inactive".to_string(),
						label: "Inactive".to_string(),
					},
				],
			},
			current_value: active_filters.get("status").cloned(),
		},
		FilterInfo {
			field: "is_published".to_string(),
			title: "Published".to_string(),
			filter_type: FilterType::Boolean,
			current_value: active_filters.get("is_published").cloned(),
		},
	]
}

/// Detail handler - returns single model instance
#[get("/{<str:model>}/{<str:id>}/", name = "admin_detail", use_inject = true)]
pub async fn detail(
	Path((model_name, id)): Path<(String, String)>,
	#[inject] site: Arc<AdminSite>,
	#[inject] db: Arc<AdminDatabase>,
) -> ViewResult<Response> {
	let model_admin = site
		.get_model_admin(&model_name)
		.map_err(|e| AdminError::ModelNotRegistered(e.to_string()))?;
	let table_name = model_admin.table_name();
	let pk_field = model_admin.pk_field();

	let data = db
		.get::<AdminRecord>(table_name, pk_field, &id)
		.await
		.map_err(|e| AdminError::DatabaseError(e.to_string()))?
		.ok_or_else(|| {
			AdminError::ValidationError(format!("{} with id {} not found", model_name, id))
		})?;

	let response = DetailResponse {
		model_name: model_name.to_string(),
		data,
	};

	json_response(StatusCode::OK, &response).map_err(Into::into)
}

/// Create handler - creates new model instance
#[post("/{<str:model>}/", name = "admin_create", use_inject = true)]
pub async fn create(
	Path(model_name): Path<String>,
	Json(request): Json<MutationRequest>,
	#[inject] site: Arc<AdminSite>,
	#[inject] db: Arc<AdminDatabase>,
) -> ViewResult<Response> {
	let model_admin = site
		.get_model_admin(&model_name)
		.map_err(|e| AdminError::ModelNotRegistered(e.to_string()))?;
	let table_name = model_admin.table_name();

	let affected = db
		.create::<AdminRecord>(table_name, request.data.clone())
		.await
		.map_err(|e| AdminError::DatabaseError(e.to_string()))?;

	let response = MutationResponse {
		success: affected > 0,
		message: format!("{} created successfully", model_name),
		affected: Some(affected),
		data: Some(request.data),
	};

	json_response(StatusCode::CREATED, &response).map_err(Into::into)
}

/// Update handler - updates existing model instance
#[put("/{<str:model>}/{<str:id>}/", name = "admin_update", use_inject = true)]
pub async fn update(
	Path((model_name, id)): Path<(String, String)>,
	Json(request): Json<MutationRequest>,
	#[inject] site: Arc<AdminSite>,
	#[inject] db: Arc<AdminDatabase>,
) -> ViewResult<Response> {
	let model_admin = site
		.get_model_admin(&model_name)
		.map_err(|e| AdminError::ModelNotRegistered(e.to_string()))?;
	let table_name = model_admin.table_name();
	let pk_field = model_admin.pk_field();

	let affected = db
		.update::<AdminRecord>(table_name, pk_field, &id, request.data)
		.await
		.map_err(|e| AdminError::DatabaseError(e.to_string()))?;

	let response = MutationResponse {
		success: affected > 0,
		message: if affected > 0 {
			format!("{} updated successfully", model_name)
		} else {
			format!("{} with id {} not found", model_name, id)
		},
		affected: Some(affected),
		data: None,
	};

	let status = if affected > 0 {
		StatusCode::OK
	} else {
		StatusCode::NOT_FOUND
	};

	json_response(status, &response).map_err(Into::into)
}

/// Delete handler - deletes model instance
#[delete("/{<str:model>}/{<str:id>}/", name = "admin_delete", use_inject = true)]
pub async fn delete(
	Path((model_name, id)): Path<(String, String)>,
	#[inject] site: Arc<AdminSite>,
	#[inject] db: Arc<AdminDatabase>,
) -> ViewResult<Response> {
	let model_admin = site
		.get_model_admin(&model_name)
		.map_err(|e| AdminError::ModelNotRegistered(e.to_string()))?;
	let table_name = model_admin.table_name();
	let pk_field = model_admin.pk_field();

	let affected = db
		.delete::<AdminRecord>(table_name, pk_field, &id)
		.await
		.map_err(|e| AdminError::DatabaseError(e.to_string()))?;

	let response = MutationResponse {
		success: affected > 0,
		message: if affected > 0 {
			format!("{} deleted successfully", model_name)
		} else {
			format!("{} with id {} not found", model_name, id)
		},
		affected: Some(affected),
		data: None,
	};

	let status = if affected > 0 {
		StatusCode::OK
	} else {
		StatusCode::NOT_FOUND
	};

	json_response(status, &response).map_err(Into::into)
}

/// Bulk delete handler - deletes multiple model instances
#[post(
	"/{<str:model>}/bulk-delete/",
	name = "admin_bulk_delete",
	use_inject = true
)]
pub async fn bulk_delete(
	Path(model_name): Path<String>,
	Json(request): Json<BulkDeleteRequest>,
	#[inject] site: Arc<AdminSite>,
	#[inject] db: Arc<AdminDatabase>,
) -> ViewResult<Response> {
	let model_admin = site
		.get_model_admin(&model_name)
		.map_err(|e| AdminError::ModelNotRegistered(e.to_string()))?;
	let table_name = model_admin.table_name();
	let pk_field = model_admin.pk_field();

	let deleted = db
		.bulk_delete_by_table(table_name, pk_field, request.ids)
		.await
		.map_err(|e| AdminError::DatabaseError(e.to_string()))?;

	let response = BulkDeleteResponse {
		success: true,
		deleted,
		message: format!("{} {} deleted successfully", deleted, model_name),
	};

	json_response(StatusCode::OK, &response).map_err(Into::into)
}

/// Export handler - exports model data
#[get("/{<str:model>}/export/", name = "admin_export", use_inject = true)]
pub async fn export(
	Path(model_name): Path<String>,
	Query(params): Query<ExportQueryParams>,
	#[inject] site: Arc<AdminSite>,
	#[inject] db: Arc<AdminDatabase>,
) -> ViewResult<Response> {
	let model_admin = site
		.get_model_admin(&model_name)
		.map_err(|e| AdminError::ModelNotRegistered(e.to_string()))?;
	let table_name = model_admin.table_name();

	// Get all items (no pagination for export)
	let items = db
		.list::<AdminRecord>(table_name, vec![], 0, u64::MAX)
		.await
		.map_err(|e| AdminError::DatabaseError(e.to_string()))?;

	match params.format {
		ExportFormat::Json => {
			let json = serde_json::to_string_pretty(&items)
				.map_err(|e| AdminError::ValidationError(e.to_string()))?;

			let content_disposition = format!(
				"attachment; filename=\"{}.json\"",
				model_name.to_lowercase()
			);
			let response = Response::new(StatusCode::OK)
				.with_header("Content-Type", "application/json")
				.with_header("Content-Disposition", &content_disposition)
				.with_body(json);

			Ok(response)
		}
		ExportFormat::Csv | ExportFormat::Tsv => {
			let delimiter = if params.format == ExportFormat::Csv {
				b','
			} else {
				b'\t'
			};
			let extension = if params.format == ExportFormat::Csv {
				"csv"
			} else {
				"tsv"
			};

			let mut wtr = csv::WriterBuilder::new()
				.delimiter(delimiter)
				.from_writer(vec![]);

			// Write headers from first item
			if let Some(first) = items.first() {
				let headers: Vec<&str> = first.keys().map(|s| s.as_str()).collect();
				wtr.write_record(&headers)
					.map_err(|e| AdminError::ValidationError(e.to_string()))?;
			}

			// Write data rows
			for item in &items {
				let values: Vec<String> = item
					.values()
					.map(|v| match v {
						serde_json::Value::String(s) => s.clone(),
						_ => v.to_string(),
					})
					.collect();
				wtr.write_record(&values)
					.map_err(|e| AdminError::ValidationError(e.to_string()))?;
			}

			let data = wtr
				.into_inner()
				.map_err(|e| AdminError::ValidationError(e.to_string()))?;

			let content_type = if params.format == ExportFormat::Csv {
				"text/csv"
			} else {
				"text/tab-separated-values"
			};

			let content_disposition = format!(
				"attachment; filename=\"{}.{}\"",
				model_name.to_lowercase(),
				extension
			);
			let response = Response::new(StatusCode::OK)
				.with_header("Content-Type", content_type)
				.with_header("Content-Disposition", &content_disposition)
				.with_body(data);

			Ok(response)
		}
	}
}

/// Import handler - imports model data
///
/// Parses the request body based on content type and inserts records.
/// Supports JSON, CSV, and TSV formats.
///
/// # Arguments
///
/// * `model_name` - Name of the model to import data into
/// * `content_type` - Content-Type header value
/// * `body` - Raw request body bytes
///
/// # Returns
///
/// Returns an `ImportResponse` with counts of imported/skipped/failed records.
#[post("/{<str:model>}/import/", name = "admin_import", use_inject = true)]
pub async fn import(req: Request) -> ViewResult<Response> {
	// Extract model_name from path
	let model_name = req
		.path_params
		.get("model")
		.ok_or_else(|| AdminError::ValidationError("Missing model parameter".into()))?
		.clone();

	// Manually resolve dependencies from DI context
	let di_ctx = req
		.extensions
		.get::<reinhardt_di::InjectionContext>()
		.ok_or_else(|| AdminError::ValidationError("DI context not found".into()))?;

	let site = di_ctx
		.resolve::<AdminSite>()
		.await
		.map_err(|e| AdminError::ValidationError(format!("Failed to resolve AdminSite: {}", e)))?;

	let db = di_ctx.resolve::<AdminDatabase>().await.map_err(|e| {
		AdminError::ValidationError(format!("Failed to resolve AdminDatabase: {}", e))
	})?;
	// Extract content type from headers
	let content_type = req
		.headers
		.get("content-type")
		.and_then(|v| v.to_str().ok())
		.unwrap_or("application/json")
		.to_string();

	// Extract body from request
	let body = req.read_body()?.to_vec();

	// 1. Parse content type to determine format
	let format = ImportFormat::from_content_type(&content_type).ok_or_else(|| {
		AdminError::ValidationError(format!(
			"Unsupported content type: {}. Supported types: application/json, text/csv, text/tab-separated-values",
			content_type
		))
	})?;

	// 2. Get model admin for table information
	let model_admin = site
		.get_model_admin(&model_name)
		.map_err(|e| AdminError::ModelNotRegistered(e.to_string()))?;
	let table_name = model_admin.table_name();

	// 3. Parse body based on format
	let builder = ImportBuilder::new(&model_name, format).data(body);
	let records = builder
		.parse()
		.map_err(|e| AdminError::ValidationError(e.to_string()))?;

	// 4. Insert records
	let mut result = ImportResult::new();
	let mut row_number = 1usize;

	for record in records {
		// Convert HashMap<String, String> to HashMap<String, serde_json::Value>
		let data: HashMap<String, serde_json::Value> = record
			.into_iter()
			.map(|(k, v)| (k, serde_json::Value::String(v)))
			.collect();

		match db.create::<AdminRecord>(table_name, data).await {
			Ok(_) => {
				result.add_imported();
			}
			Err(e) => {
				let error = ImportError::new(row_number, format!("Failed to insert record: {}", e));
				result.add_failed(error);
			}
		}
		row_number += 1;
	}

	// Build response
	let response = ImportResponse {
		success: result.is_successful(),
		imported: result.imported_count as u64,
		updated: result.updated_count as u64,
		skipped: result.skipped_count as u64,
		failed: result.failed_count as u64,
		message: format!(
			"Import completed: {} imported, {} updated, {} skipped, {} failed",
			result.imported_count, result.updated_count, result.skipped_count, result.failed_count
		),
		errors: if result.errors.is_empty() {
			None
		} else {
			Some(result.errors.iter().map(|e| e.message.clone()).collect())
		},
	};

	// Return OK for full success or partial success (imported_count > 0)
	// Return BAD_REQUEST only when no records were imported at all
	let status = if result.is_successful() || result.imported_count > 0 {
		StatusCode::OK
	} else {
		StatusCode::BAD_REQUEST
	};

	json_response(status, &response).map_err(Into::into)
}

/// Helper function to create JSON response
fn json_response<T: Serialize>(status: StatusCode, body: &T) -> AdminResult<Response> {
	let json =
		serde_json::to_string(body).map_err(|e| AdminError::ValidationError(e.to_string()))?;

	let response = Response::new(status)
		.with_header("Content-Type", "application/json")
		.with_body(json);

	Ok(response)
}

/// Extract session ID from request cookies
///
/// Parses the Cookie header and extracts the session ID value.
/// Returns None if no session cookie is found.
fn extract_session_id_from_request(req: &Request) -> Option<String> {
	req.headers
		.get(hyper::header::COOKIE)
		.and_then(|v| v.to_str().ok())
		.and_then(|cookie_str| {
			for part in cookie_str.split(';') {
				let mut iter = part.trim().splitn(2, '=');
				if let (Some(name), Some(value)) = (iter.next(), iter.next())
					&& name == "sessionid"
				{
					return Some(value.to_string());
				}
			}
			None
		})
}

/// Generate CSRF token from request session
///
/// This function generates a UUID-based CSRF token for each request.
/// In a full implementation with session store access, this would:
/// 1. Extract session ID from cookies
/// 2. Look up session in session store
/// 3. Get or create CSRF token in session
///
/// For now, we generate a unique token per request as a placeholder.
/// The token can be validated on subsequent requests by embedding it
/// in both the response and a cookie.
fn generate_csrf_token_from_request(req: &Request) -> Option<String> {
	// Try to extract session ID from request
	let session_id = extract_session_id_from_request(req);

	// Generate a unique token
	// In production, this should be tied to the session and stored
	// For now, we generate a new UUID-based token
	if session_id.is_some() {
		Some(format!("csrf_{}", uuid::Uuid::new_v4()))
	} else {
		// Even without session, generate a token for demonstration purposes
		Some(format!("csrf_{}", uuid::Uuid::new_v4()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_export_format_default() {
		let format = ExportFormat::default();
		assert_eq!(format, ExportFormat::Json);
	}

	#[test]
	fn test_dashboard_response_serialize() {
		let response = DashboardResponse {
			site_name: "Test Admin".to_string(),
			url_prefix: "/admin".to_string(),
			models: vec![ModelInfo {
				name: "User".to_string(),
				list_url: "/admin/user/".to_string(),
			}],
			csrf_token: None,
		};

		let json = serde_json::to_string(&response).unwrap();
		assert!(json.contains("Test Admin"));
		assert!(json.contains("User"));
	}

	#[test]
	fn test_mutation_response_serialize() {
		let response = MutationResponse {
			success: true,
			message: "Created".to_string(),
			affected: Some(1),
			data: None,
		};

		let json = serde_json::to_string(&response).unwrap();
		assert!(json.contains("success"));
		assert!(json.contains("true"));
		assert!(!json.contains("data")); // skip_serializing_if = "Option::is_none"
	}

	// ==================== ImportResponse tests ====================

	#[test]
	fn test_import_response_serialize() {
		let response = ImportResponse {
			success: true,
			imported: 10,
			updated: 5,
			skipped: 2,
			failed: 1,
			message: "Import completed".to_string(),
			errors: Some(vec!["Row 5: Invalid email format".to_string()]),
		};

		let json = serde_json::to_string(&response).unwrap();
		assert!(json.contains("\"success\":true"));
		assert!(json.contains("\"imported\":10"));
		assert!(json.contains("\"updated\":5"));
		assert!(json.contains("\"skipped\":2"));
		assert!(json.contains("\"failed\":1"));
		assert!(json.contains("\"message\":\"Import completed\""));
		assert!(json.contains("\"errors\":[\"Row 5: Invalid email format\"]"));
	}

	#[test]
	fn test_import_response_serialize_no_errors() {
		let response = ImportResponse {
			success: true,
			imported: 10,
			updated: 0,
			skipped: 0,
			failed: 0,
			message: "All records imported successfully".to_string(),
			errors: None,
		};

		let json = serde_json::to_string(&response).unwrap();
		assert!(json.contains("\"success\":true"));
		assert!(json.contains("\"imported\":10"));
		// errors field should be skipped when None (skip_serializing_if = "Option::is_none")
		assert!(!json.contains("\"errors\""));
	}

	#[test]
	fn test_import_response_serialize_empty_errors() {
		let response = ImportResponse {
			success: true,
			imported: 5,
			updated: 3,
			skipped: 1,
			failed: 0,
			message: "Import completed".to_string(),
			errors: Some(vec![]),
		};

		let json = serde_json::to_string(&response).unwrap();
		// Empty vec is still serialized (not skipped)
		assert!(json.contains("\"errors\":[]"));
	}

	#[test]
	fn test_import_response_serialize_multiple_errors() {
		let response = ImportResponse {
			success: false,
			imported: 5,
			updated: 0,
			skipped: 0,
			failed: 3,
			message: "Import completed with errors".to_string(),
			errors: Some(vec![
				"Row 1: Missing required field 'name'".to_string(),
				"Row 3: Invalid date format".to_string(),
				"Row 7: Duplicate entry".to_string(),
			]),
		};

		let json = serde_json::to_string(&response).unwrap();
		assert!(json.contains("\"success\":false"));
		assert!(json.contains("\"failed\":3"));
		assert!(json.contains("Missing required field 'name'"));
		assert!(json.contains("Invalid date format"));
		assert!(json.contains("Duplicate entry"));
	}

	#[test]
	fn test_import_response_deserialize_roundtrip() {
		let original = ImportResponse {
			success: true,
			imported: 10,
			updated: 5,
			skipped: 2,
			failed: 1,
			message: "Import completed".to_string(),
			errors: Some(vec!["Error 1".to_string(), "Error 2".to_string()]),
		};

		let json = serde_json::to_string(&original).unwrap();
		let deserialized: serde_json::Value = serde_json::from_str(&json).unwrap();

		// Verify structure
		assert_eq!(deserialized["success"], true);
		assert_eq!(deserialized["imported"], 10);
		assert_eq!(deserialized["updated"], 5);
		assert_eq!(deserialized["skipped"], 2);
		assert_eq!(deserialized["failed"], 1);
		assert_eq!(deserialized["message"], "Import completed");
		assert!(deserialized["errors"].is_array());
		assert_eq!(deserialized["errors"].as_array().unwrap().len(), 2);
	}

	// ==================== Column generation tests ====================

	#[test]
	fn test_field_to_label_simple() {
		assert_eq!(field_to_label("id"), "Id");
		assert_eq!(field_to_label("name"), "Name");
	}

	#[test]
	fn test_field_to_label_snake_case() {
		assert_eq!(field_to_label("user_name"), "User Name");
		assert_eq!(field_to_label("created_at"), "Created At");
		assert_eq!(field_to_label("is_active"), "Is Active");
	}

	#[test]
	fn test_field_to_label_multiple_underscores() {
		assert_eq!(field_to_label("first_name_initial"), "First Name Initial");
	}

	#[test]
	fn test_field_to_label_empty() {
		assert_eq!(field_to_label(""), "");
	}

	#[test]
	fn test_generate_columns_basic() {
		use crate::ModelAdminConfig;

		let admin = ModelAdminConfig::new("User");
		let columns = generate_columns(&admin);

		// Default list_display is ["id"], ordering is ["-id"]
		assert_eq!(columns.len(), 1);
		assert_eq!(columns[0].field, "id");
		assert_eq!(columns[0].label, "Id");
		assert!(columns[0].sortable); // "id" is in ordering
	}

	#[test]
	fn test_generate_columns_custom_list_display() {
		use crate::ModelAdminConfig;

		let admin = ModelAdminConfig::builder()
			.model_name("User")
			.list_display(vec!["id", "user_name", "email", "is_active"])
			.ordering(vec!["-id", "user_name"])
			.build();

		let columns = generate_columns(&admin);

		assert_eq!(columns.len(), 4);

		// id - sortable (in ordering as "-id")
		assert_eq!(columns[0].field, "id");
		assert_eq!(columns[0].label, "Id");
		assert!(columns[0].sortable);

		// user_name - sortable (in ordering)
		assert_eq!(columns[1].field, "user_name");
		assert_eq!(columns[1].label, "User Name");
		assert!(columns[1].sortable);

		// email - not sortable (not in ordering)
		assert_eq!(columns[2].field, "email");
		assert_eq!(columns[2].label, "Email");
		assert!(!columns[2].sortable);

		// is_active - not sortable (not in ordering)
		assert_eq!(columns[3].field, "is_active");
		assert_eq!(columns[3].label, "Is Active");
		assert!(!columns[3].sortable);
	}
}
