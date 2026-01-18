//! Import operation Server Function
//!
//! Provides import operations for admin models from various formats (JSON, CSV, TSV).

use reinhardt_admin::adapters::{
	AdminDatabase, AdminRecord, AdminSite, ImportFormat, ImportResponse,
};
use reinhardt_pages::server_fn::{ServerFnError, server_fn};
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use super::error::MapServerFnError;

/// Import model data from various formats
///
/// Imports records from uploaded data in the specified format (JSON, CSV, TSV).
/// Each record is inserted as a new entry. Returns statistics about the import operation.
///
/// # Server Function
///
/// This function is automatically exposed as an HTTP endpoint by the `#[server_fn]` macro.
/// AdminSite and AdminDatabase dependencies are automatically injected via the DI system.
///
/// # Example
///
/// ```ignore
/// use reinhardt_admin::server::import_data;
/// use reinhardt_admin::types::ImportFormat;
///
/// // Client-side usage (automatically generates HTTP request)
/// let file_data = vec![/* binary data */];
/// let response = import_data(
///     "User".to_string(),
///     ImportFormat::JSON,
///     file_data
/// ).await?;
/// println!("Imported {} records", response.imported);
/// ```
#[server_fn(use_inject = true)]
pub async fn import_data(
	model_name: String,
	format: ImportFormat,
	data: Vec<u8>,
	#[inject] site: Arc<AdminSite>,
	#[inject] db: Arc<AdminDatabase>,
) -> Result<ImportResponse, ServerFnError> {
	let model_admin = site.get_model_admin(&model_name).map_server_fn_error()?;
	let table_name = model_admin.table_name();

	// Parse data based on format
	let records: Vec<HashMap<String, serde_json::Value>> = match format {
		ImportFormat::JSON => serde_json::from_slice(&data)
			.map_err(|e| ServerFnError::deserialization(format!("JSON parse failed: {}", e)))?,
		ImportFormat::CSV => {
			let mut rdr = csv::Reader::from_reader(&data[..]);
			rdr.deserialize()
				.collect::<Result<Vec<_>, _>>()
				.map_err(|e| ServerFnError::deserialization(format!("CSV parse failed: {}", e)))?
		}
		ImportFormat::TSV => {
			let mut rdr = csv::ReaderBuilder::new()
				.delimiter(b'\t')
				.from_reader(&data[..]);
			rdr.deserialize()
				.collect::<Result<Vec<_>, _>>()
				.map_err(|e| ServerFnError::deserialization(format!("TSV parse failed: {}", e)))?
		}
	};

	// Import records
	let mut imported = 0;
	let mut failed = 0;
	let mut errors = Vec::new();

	for (index, record) in records.into_iter().enumerate() {
		match db.create::<AdminRecord>(table_name, record).await {
			Ok(_) => imported += 1,
			Err(e) => {
				failed += 1;
				errors.push(format!("Record {}: {}", index + 1, e));
			}
		}
	}

	Ok(ImportResponse {
		success: failed == 0,
		imported,
		updated: 0, // Not supporting updates in basic import
		skipped: 0,
		failed,
		message: if failed == 0 {
			format!("Successfully imported {} {} records", imported, model_name)
		} else {
			format!(
				"Imported {} {} records, {} failed",
				imported, model_name, failed
			)
		},
		errors: if errors.is_empty() {
			None
		} else {
			Some(errors)
		},
	})
}
