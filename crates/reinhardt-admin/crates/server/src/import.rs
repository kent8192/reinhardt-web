//! Import operation Server Function
//!
//! Provides import operations for admin models from various formats (JSON, CSV, TSV).

use reinhardt_admin_core::{
	AdminDatabase, AdminRecord, AdminSite, ImportBuilder, ImportError, ImportFormat, ImportResult,
};
use reinhardt_admin_types::ImportResponse;
use reinhardt_pages::server_fn::{ServerFnError, server_fn};
use std::collections::HashMap;
use std::sync::Arc;

use super::error::MapServerFnError;

/// Import model data
///
/// Imports records for the specified model from the provided data in the requested format.
///
/// # Server Function
///
/// This function is automatically exposed as an HTTP endpoint by the `#[server_fn]` macro.
/// The AdminSite and AdminDatabase dependencies are automatically injected via the DI system.
/// Uses JSON codec to handle binary data.
///
/// # Example
///
/// ```ignore
/// use reinhardt_admin_server::import_models;
/// use reinhardt_admin_core::ImportFormat;
///
/// // Client-side usage (automatically generates HTTP request)
/// let csv_data = b"id,name\n1,Alice\n2,Bob".to_vec();
/// let response = import_models("users".to_string(), ImportFormat::CSV, csv_data).await?;
/// println!("Imported: {}", response.imported);
/// ```
#[server_fn(use_inject = true, codec = "json")]
pub async fn import_models(
	model_name: String,
	format: ImportFormat,
	data: Vec<u8>,
	#[inject] site: Arc<AdminSite>,
	#[inject] db: Arc<AdminDatabase>,
) -> Result<ImportResponse, ServerFnError> {
	// Get model configuration
	let model_admin = site.get_model_admin(&model_name).map_server_fn_error()?;
	let table_name = model_admin.table_name();

	// Parse data based on format
	let builder = ImportBuilder::new(&model_name, format).data(data);
	let records = builder
		.parse()
		.map_err(|e| ServerFnError::application(format!("Failed to parse import data: {}", e)))?;

	// Import records into database
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
	Ok(ImportResponse {
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
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_admin_core::ImportFormat;

	#[test]
	fn test_import_response_structure() {
		let response = ImportResponse {
			success: true,
			imported: 10,
			updated: 5,
			skipped: 2,
			failed: 0,
			message: "Import completed".to_string(),
			errors: None,
		};

		assert!(response.success);
		assert_eq!(response.imported, 10);
		assert_eq!(response.updated, 5);
		assert_eq!(response.skipped, 2);
		assert_eq!(response.failed, 0);
	}

	#[test]
	fn test_import_format_detection() {
		assert_eq!(
			ImportFormat::from_content_type("application/json"),
			Some(ImportFormat::JSON)
		);
		assert_eq!(
			ImportFormat::from_content_type("text/csv"),
			Some(ImportFormat::CSV)
		);
		assert_eq!(
			ImportFormat::from_content_type("text/tab-separated-values"),
			Some(ImportFormat::TSV)
		);
	}

	#[test]
	fn test_csv_data_parsing() {
		let csv_data = b"id,name\n1,Alice\n2,Bob";
		let builder = ImportBuilder::new("users", ImportFormat::CSV).data(csv_data.to_vec());
		let result = builder.parse();

		assert!(result.is_ok());
		let records = result.unwrap();
		assert_eq!(records.len(), 2);
	}

	#[test]
	fn test_json_data_parsing() {
		let json_data = br#"[{"id":"1","name":"Alice"},{"id":"2","name":"Bob"}]"#;
		let builder = ImportBuilder::new("users", ImportFormat::JSON).data(json_data.to_vec());
		let result = builder.parse();

		assert!(result.is_ok());
		let records = result.unwrap();
		assert_eq!(records.len(), 2);
	}
}
