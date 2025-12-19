//! Delete operation Server Functions
//!
//! Provides delete operations for admin models (single and bulk).

use reinhardt_admin_core::{AdminDatabase, AdminRecord, AdminSite};
use reinhardt_admin_types::{BulkDeleteRequest, BulkDeleteResponse, MutationResponse};
use reinhardt_pages::server_fn::{ServerFnError, server_fn};
use std::sync::Arc;

use super::error::MapServerFnError;

/// Delete a single model record
///
/// Deletes a record for the specified model by ID.
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
/// use reinhardt_admin_server::delete_model;
///
/// // Client-side usage (automatically generates HTTP DELETE request)
/// let response = delete_model("users".to_string(), "1".to_string()).await?;
/// println!("Deleted: {}", response.message);
/// ```
#[server_fn(use_inject = true, codec = "url")]
pub async fn delete_model(
	model_name: String,
	id: String,
	#[inject] site: Arc<AdminSite>,
	#[inject] db: Arc<AdminDatabase>,
) -> Result<MutationResponse, ServerFnError> {
	// Get model configuration
	let model_admin = site.get_model_admin(&model_name).map_server_fn_error()?;
	let table_name = model_admin.table_name();
	let pk_field = model_admin.pk_field();

	// Delete record from database
	let affected = db
		.delete::<AdminRecord>(table_name, pk_field, &id)
		.await
		.map_server_fn_error()?;

	Ok(MutationResponse {
		success: affected > 0,
		message: if affected > 0 {
			format!("{} deleted successfully", model_name)
		} else {
			format!("{} with id {} not found", model_name, id)
		},
		affected: Some(affected),
		data: None,
	})
}

/// Bulk delete model records
///
/// Deletes multiple records for the specified model by IDs.
///
/// # Server Function
///
/// This function is automatically exposed as an HTTP endpoint by the `#[server_fn]` macro.
/// The AdminSite and AdminDatabase dependencies are automatically injected via the DI system.
/// Uses JSON codec for the ID list.
///
/// # Example
///
/// ```ignore
/// use reinhardt_admin_server::bulk_delete_models;
/// use reinhardt_admin_types::BulkDeleteRequest;
///
/// let request = BulkDeleteRequest {
///     ids: vec!["1".to_string(), "2".to_string(), "3".to_string()],
/// };
///
/// // Client-side usage (automatically generates HTTP POST request)
/// let response = bulk_delete_models("users".to_string(), request).await?;
/// println!("Deleted {} records", response.deleted);
/// ```
#[server_fn(use_inject = true, codec = "json")]
pub async fn bulk_delete_models(
	model_name: String,
	request: BulkDeleteRequest,
	#[inject] site: Arc<AdminSite>,
	#[inject] db: Arc<AdminDatabase>,
) -> Result<BulkDeleteResponse, ServerFnError> {
	// Get model configuration
	let model_admin = site.get_model_admin(&model_name).map_server_fn_error()?;
	let table_name = model_admin.table_name();
	let pk_field = model_admin.pk_field();

	// Bulk delete records from database
	let deleted = db
		.bulk_delete_by_table(table_name, pk_field, request.ids)
		.await
		.map_server_fn_error()?;

	Ok(BulkDeleteResponse {
		success: true,
		deleted,
		message: format!("{} {} deleted successfully", deleted, model_name),
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_delete_response_success() {
		let response = MutationResponse {
			success: true,
			message: "User deleted successfully".to_string(),
			affected: Some(1),
			data: None,
		};

		assert!(response.success);
		assert_eq!(response.message, "User deleted successfully");
		assert_eq!(response.affected, Some(1));
	}

	#[test]
	fn test_delete_response_not_found() {
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

	#[test]
	fn test_bulk_delete_response() {
		let response = BulkDeleteResponse {
			success: true,
			deleted: 3,
			message: "3 User deleted successfully".to_string(),
		};

		assert!(response.success);
		assert_eq!(response.deleted, 3);
		assert!(response.message.contains("3"));
	}

	#[test]
	fn test_bulk_delete_request() {
		let request = BulkDeleteRequest {
			ids: vec!["1".to_string(), "2".to_string(), "3".to_string()],
		};

		assert_eq!(request.ids.len(), 3);
		assert_eq!(request.ids[0], "1");
		assert_eq!(request.ids[1], "2");
		assert_eq!(request.ids[2], "3");
	}
}
