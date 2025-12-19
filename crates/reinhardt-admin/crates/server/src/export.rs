//! Export operation Server Function
//!
//! Provides export operations for admin models.

use reinhardt_admin_core::{AdminDatabase, AdminRecord, AdminSite};
use reinhardt_admin_types::{ExportFormat, ExportResponse};
use reinhardt_pages::server_fn::{ServerFnError, server_fn};
use std::sync::Arc;

use super::error::MapServerFnError;

/// Export model data
///
/// Exports all records for the specified model in the requested format.
///
/// # Server Function
///
/// This function is automatically exposed as an HTTP endpoint by the `#[server_fn]` macro.
/// The AdminSite and AdminDatabase dependencies are automatically injected via the DI system.
/// Uses URL codec for simple parameters.
///
/// # Example
///
/// ```ignore
/// use reinhardt_admin_server::export_models;
/// use reinhardt_admin_types::ExportFormat;
///
/// // Client-side usage (automatically generates HTTP request)
/// let response = export_models("users".to_string(), ExportFormat::Csv).await?;
/// // Use response.data for download
/// println!("Exported to: {}", response.filename);
/// ```
#[server_fn(use_inject = true, codec = "url")]
pub async fn export_models(
	model_name: String,
	format: ExportFormat,
	#[inject] site: Arc<AdminSite>,
	#[inject] db: Arc<AdminDatabase>,
) -> Result<ExportResponse, ServerFnError> {
	// Get model configuration
	let model_admin = site.get_model_admin(&model_name).map_server_fn_error()?;
	let table_name = model_admin.table_name();

	// Get all items (no pagination for export)
	// Note: list() returns Vec<HashMap<String, serde_json::Value>> directly
	let data_items = db
		.list::<AdminRecord>(table_name, vec![], 0, u64::MAX)
		.await
		.map_server_fn_error()?;

	match format {
		ExportFormat::Json => {
			let json = serde_json::to_string_pretty(&data_items).map_err(|e| {
				ServerFnError::application(format!("JSON serialization failed: {}", e))
			})?;

			Ok(ExportResponse {
				data: json.into_bytes(),
				filename: format!("{}.json", model_name.to_lowercase()),
				content_type: "application/json".to_string(),
			})
		}
		ExportFormat::Csv | ExportFormat::Tsv => {
			let delimiter = if format == ExportFormat::Csv {
				b','
			} else {
				b'\t'
			};
			let extension = if format == ExportFormat::Csv {
				"csv"
			} else {
				"tsv"
			};

			let mut wtr = csv::WriterBuilder::new()
				.delimiter(delimiter)
				.from_writer(vec![]);

			// Write headers from first item
			if let Some(first) = data_items.first() {
				let headers: Vec<&str> = first.keys().map(|s| s.as_str()).collect();
				wtr.write_record(&headers).map_err(|e| {
					ServerFnError::application(format!("CSV header write failed: {}", e))
				})?;
			}

			// Write data rows
			for item in &data_items {
				let values: Vec<String> = item
					.values()
					.map(|v| match v {
						serde_json::Value::String(s) => s.clone(),
						_ => v.to_string(),
					})
					.collect();
				wtr.write_record(&values).map_err(|e| {
					ServerFnError::application(format!("CSV row write failed: {}", e))
				})?;
			}

			let data = wtr.into_inner().map_err(|e| {
				ServerFnError::application(format!("CSV finalization failed: {}", e))
			})?;

			let content_type = if format == ExportFormat::Csv {
				"text/csv"
			} else {
				"text/tab-separated-values"
			};

			Ok(ExportResponse {
				data,
				filename: format!("{}.{}", model_name.to_lowercase(), extension),
				content_type: content_type.to_string(),
			})
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;
	use std::collections::HashMap;

	#[test]
	fn test_export_response_structure() {
		let response = ExportResponse {
			data: b"test data".to_vec(),
			filename: "users.csv".to_string(),
			content_type: "text/csv".to_string(),
		};

		assert_eq!(response.data, b"test data");
		assert_eq!(response.filename, "users.csv");
		assert_eq!(response.content_type, "text/csv");
	}

	#[test]
	fn test_json_export_format() {
		let mut item1 = HashMap::new();
		item1.insert("id".to_string(), json!(1));
		item1.insert("name".to_string(), json!("Alice"));

		let mut item2 = HashMap::new();
		item2.insert("id".to_string(), json!(2));
		item2.insert("name".to_string(), json!("Bob"));

		let items = vec![item1, item2];
		let json = serde_json::to_string_pretty(&items).unwrap();

		assert!(json.contains("Alice"));
		assert!(json.contains("Bob"));
	}

	#[test]
	fn test_csv_export_format() {
		let mut wtr = csv::WriterBuilder::new()
			.delimiter(b',')
			.from_writer(vec![]);

		wtr.write_record(&["id", "name"]).unwrap();
		wtr.write_record(&["1", "Alice"]).unwrap();
		wtr.write_record(&["2", "Bob"]).unwrap();

		let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();

		assert!(data.contains("id,name"));
		assert!(data.contains("1,Alice"));
		assert!(data.contains("2,Bob"));
	}
}
