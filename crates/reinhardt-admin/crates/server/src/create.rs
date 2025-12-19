//! Create operation Server Function
//!
//! Provides create operations for admin models.

use reinhardt_admin_core::{AdminDatabase, AdminRecord, AdminSite};
use reinhardt_admin_types::{MutationRequest, MutationResponse};
use reinhardt_pages::server_fn::{ServerFnError, server_fn};
use std::sync::Arc;

use super::error::MapServerFnError;

/// Create a new model record
///
/// Creates a new record for the specified model.
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
/// use reinhardt_admin_server::create_model;
/// use reinhardt_admin_types::MutationRequest;
/// use std::collections::HashMap;
///
/// let mut data = HashMap::new();
/// data.insert("name".to_string(), serde_json::json!("Alice"));
/// data.insert("email".to_string(), serde_json::json!("alice@example.com"));
///
/// // Client-side usage (automatically generates HTTP POST request)
/// let request = MutationRequest { data };
/// let response = create_model("users".to_string(), request).await?;
/// println!("Created: {}", response.message);
/// ```
#[server_fn(use_inject = true, codec = "json")]
pub async fn create_model(
	model_name: String,
	request: MutationRequest,
	#[inject] site: Arc<AdminSite>,
	#[inject] db: Arc<AdminDatabase>,
) -> Result<MutationResponse, ServerFnError> {
	// Get model configuration
	let model_admin = site.get_model_admin(&model_name).map_server_fn_error()?;
	let table_name = model_admin.table_name();

	// Create record in database
	let affected = db
		.create::<AdminRecord>(table_name, request.data.clone())
		.await
		.map_server_fn_error()?;

	Ok(MutationResponse {
		success: affected > 0,
		message: format!("{} created successfully", model_name),
		affected: Some(affected),
		data: Some(request.data),
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_mutation_response_structure() {
		use serde_json::json;
		use std::collections::HashMap;

		let mut data = HashMap::new();
		data.insert("username".to_string(), json!("newuser"));
		data.insert("email".to_string(), json!("newuser@example.com"));

		let response = MutationResponse {
			success: true,
			message: "User created successfully".to_string(),
			affected: Some(1),
			data: Some(data.clone()),
		};

		assert!(response.success);
		assert_eq!(response.message, "User created successfully");
		assert_eq!(response.affected, Some(1));
		assert!(response.data.is_some());
	}
}
