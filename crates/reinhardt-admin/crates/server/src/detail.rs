//! Detail view Server Function
//!
//! Provides detail view operations for admin models.

use reinhardt_admin_core::{AdminDatabase, AdminRecord, AdminSite};
use reinhardt_admin_types::DetailResponse;
use reinhardt_pages::server_fn::{ServerFnError, server_fn};
use std::sync::Arc;

use super::error::MapServerFnError;

/// Get model detail by ID
///
/// Returns detailed information about a single model record.
///
/// # Server Function
///
/// This function is automatically exposed as an HTTP endpoint by the `#[server_fn]` macro.
/// The AdminSite and AdminDatabase dependencies are automatically injected via the DI system.
///
/// # Example
///
/// ```ignore
/// use reinhardt_admin_server::get_model_detail;
///
/// // Client-side usage (automatically generates HTTP request)
/// let detail = get_model_detail("users".to_string(), "1".to_string()).await?;
/// println!("Model: {}", detail.model_name);
/// ```
#[server_fn(use_inject = true)]
pub async fn get_model_detail(
	model_name: String,
	id: String,
	#[inject] site: Arc<AdminSite>,
	#[inject] db: Arc<AdminDatabase>,
) -> Result<DetailResponse, ServerFnError> {
	// Get model configuration
	let model_admin = site.get_model_admin(&model_name).map_server_fn_error()?;
	let table_name = model_admin.table_name();
	let pk_field = model_admin.pk_field();

	// Fetch record from database
	let data = db
		.get::<AdminRecord>(table_name, pk_field, &id)
		.await
		.map_server_fn_error()?
		.ok_or_else(|| {
			ServerFnError::application(format!("{} with id {} not found", model_name, id))
		})?;

	Ok(DetailResponse { model_name, data })
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_detail_response_structure() {
		use serde_json::json;
		use std::collections::HashMap;

		let mut data = HashMap::new();
		data.insert("id".to_string(), json!(1));
		data.insert("username".to_string(), json!("testuser"));
		data.insert("email".to_string(), json!("test@example.com"));

		let response = DetailResponse {
			model_name: "User".to_string(),
			data: data.clone(),
		};

		assert_eq!(response.model_name, "User");
		assert_eq!(response.data.get("id"), Some(&json!(1)));
		assert_eq!(response.data.get("username"), Some(&json!("testuser")));
	}
}
