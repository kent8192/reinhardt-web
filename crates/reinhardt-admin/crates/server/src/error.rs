//! Error conversion for Server Functions
//!
//! This module provides error conversion from AdminError to ServerFnError.

use reinhardt_admin_types::AdminError;
use reinhardt_pages::server_fn::ServerFnError;

/// Extension trait for converting AdminError to ServerFnError
pub trait IntoServerFnError {
	/// Convert AdminError to ServerFnError
	fn into_server_fn_error(self) -> ServerFnError;
}

impl IntoServerFnError for AdminError {
	fn into_server_fn_error(self) -> ServerFnError {
		match self {
			AdminError::ModelNotRegistered(msg) => ServerFnError::server(404, msg),
			AdminError::PermissionDenied(msg) => ServerFnError::server(403, msg),
			AdminError::InvalidAction(msg) | AdminError::ValidationError(msg) => {
				ServerFnError::application(msg)
			}
			AdminError::DatabaseError(_) => {
				// Hide internal database error details from clients
				ServerFnError::server(500, "Database operation failed")
			}
			AdminError::TemplateError(_) => {
				// Hide internal template error details from clients
				ServerFnError::server(500, "Template rendering failed")
			}
		}
	}
}

/// Convert `Result<T, AdminError>` to `Result<T, ServerFnError>`
pub trait MapServerFnError<T> {
	/// Map AdminError to ServerFnError
	fn map_server_fn_error(self) -> Result<T, ServerFnError>;
}

impl<T> MapServerFnError<T> for Result<T, AdminError> {
	fn map_server_fn_error(self) -> Result<T, ServerFnError> {
		self.map_err(|e| e.into_server_fn_error())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_model_not_registered_converts_to_404() {
		let admin_err = AdminError::ModelNotRegistered("User".into());
		let server_err = admin_err.into_server_fn_error();

		match server_err {
			ServerFnError::Server { status, message } => {
				assert_eq!(status, 404);
				assert_eq!(message, "User");
			}
			_ => panic!("Expected Server error"),
		}
	}

	#[test]
	fn test_permission_denied_converts_to_403() {
		let admin_err = AdminError::PermissionDenied("Access denied".into());
		let server_err = admin_err.into_server_fn_error();

		match server_err {
			ServerFnError::Server { status, message } => {
				assert_eq!(status, 403);
				assert_eq!(message, "Access denied");
			}
			_ => panic!("Expected Server error"),
		}
	}

	#[test]
	fn test_validation_error_converts_to_application() {
		let admin_err = AdminError::ValidationError("Invalid input".into());
		let server_err = admin_err.into_server_fn_error();

		match server_err {
			ServerFnError::Application(msg) => {
				assert_eq!(msg, "Invalid input");
			}
			_ => panic!("Expected Application error"),
		}
	}

	#[test]
	fn test_database_error_hides_details() {
		let admin_err = AdminError::DatabaseError("SQL syntax error at line 42".into());
		let server_err = admin_err.into_server_fn_error();

		match server_err {
			ServerFnError::Server { status, message } => {
				assert_eq!(status, 500);
				assert_eq!(message, "Database operation failed");
				// Verify that the original error details are hidden
				assert!(!message.contains("SQL"));
				assert!(!message.contains("42"));
			}
			_ => panic!("Expected Server error"),
		}
	}

	#[test]
	fn test_result_conversion() {
		let result: Result<String, AdminError> = Err(AdminError::ModelNotRegistered("Post".into()));
		let server_result = result.map_server_fn_error();

		assert!(server_result.is_err());
		match server_result.unwrap_err() {
			ServerFnError::Server { status, .. } => assert_eq!(status, 404),
			_ => panic!("Expected Server error"),
		}
	}
}
