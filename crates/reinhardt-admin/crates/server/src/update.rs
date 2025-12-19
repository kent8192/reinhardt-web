//! Update operation Server Function
//!
//! Provides update operations for admin models.

use reinhardt_admin_core::{AdminDatabase, AdminRecord, AdminSite};
use reinhardt_admin_types::{MutationRequest, MutationResponse};
use reinhardt_pages::server_fn::{ServerFnError, server_fn};
use std::sync::Arc;

use super::error::MapServerFnError;

/// Update an existing model record
///
/// Updates a record for the specified model by ID.
///
/// # Server Function
///
/// This function is automatically exposed as an HTTP endpoint by the `#[server_fn]` macro.
/// The AdminSite and AdminDatabase dependencies are automatically injected via the DI system.
/// Uses JSON codec for complex form data.
///
/// # Example
///
/// ```ignore
/// use reinhardt_admin_server::update_model;
/// use reinhardt_admin_types::MutationRequest;
/// use std::collections::HashMap;
///
/// let mut data = HashMap::new();
/// data.insert("name".to_string(), serde_json::json!("Alice Updated"));
///
/// // Client-side usage (automatically generates HTTP PUT request)
/// let request = MutationRequest { data };
/// let response = update_model("users".to_string(), "1".to_string(), request).await?;
/// println!("Updated: {}", response.message);
/// ```
#[server_fn(use_inject = true, codec = "json")]
pub async fn update_model(
	model_name: String,
	id: String,
	request: MutationRequest,
	#[inject] site: Arc<AdminSite>,
	#[inject] db: Arc<AdminDatabase>,
) -> Result<MutationResponse, ServerFnError> {
	// Get model configuration
	let model_admin = site.get_model_admin(&model_name).map_server_fn_error()?;
	let table_name = model_admin.table_name();
	let pk_field = model_admin.pk_field();

	// Update record in database
	let affected = db
		.update::<AdminRecord>(table_name, pk_field, &id, request.data)
		.await
		.map_server_fn_error()?;

	Ok(MutationResponse {
		success: affected > 0,
		message: if affected > 0 {
			format!("{} updated successfully", model_name)
		} else {
			format!("{} with id {} not found", model_name, id)
		},
		affected: Some(affected),
		data: None,
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_mutation_response_success() {
		let response = MutationResponse {
			success: true,
			message: "User updated successfully".to_string(),
			affected: Some(1),
			data: None,
		};

		assert!(response.success);
		assert_eq!(response.message, "User updated successfully");
		assert_eq!(response.affected, Some(1));
		assert!(response.data.is_none());
	}

	#[test]
	fn test_mutation_response_not_found() {
		let response = MutationResponse {
			success: false,
			message: "User with id 999 not found".to_string(),
			affected: Some(0),
			data: None,
		};

		assert!(!response.success);
		assert!(response.message.contains("not found"));
		assert_eq!(response.affected, Some(0));
	}
}
