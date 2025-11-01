//! Error Module Tests
//!
//! Tests inspired by Django and Django Rest Framework error handling tests

use reinhardt_apps::Error;

#[test]
fn test_error_status_code_http() {
	let error = Error::Http("Bad request".to_string());
	assert_eq!(error.status_code(), 400);
}

#[test]
fn test_error_status_code_database() {
	let error = Error::Database("Connection failed".to_string());
	assert_eq!(error.status_code(), 500);
}

#[test]
fn test_error_status_code_serialization() {
	let error = Error::Serialization("Invalid JSON".to_string());
	assert_eq!(error.status_code(), 400);
}

#[test]
fn test_error_status_code_validation() {
	let error = Error::Validation("Field is required".to_string());
	assert_eq!(error.status_code(), 400);
}

#[test]
fn test_error_status_code_authentication() {
	let error = Error::Authentication("Invalid credentials".to_string());
	assert_eq!(error.status_code(), 401);
}

#[test]
fn test_error_status_code_authorization() {
	let error = Error::Authorization("Permission denied".to_string());
	assert_eq!(error.status_code(), 403);
}

#[test]
fn test_error_status_code_not_found() {
	let error = Error::NotFound("Resource not found".to_string());
	assert_eq!(error.status_code(), 404);
}

#[test]
fn test_error_status_code_internal() {
	let error = Error::Internal("Server error".to_string());
	assert_eq!(error.status_code(), 500);
}

#[test]
fn test_error_status_code_other() {
	let error = Error::Other(anyhow::anyhow!("Unknown error"));
	assert_eq!(error.status_code(), 500);
}

#[test]
fn test_error_display_http() {
	let error = Error::Http("Bad request".to_string());
	assert_eq!(error.to_string(), "HTTP error: Bad request");
}

#[test]
fn test_error_display_database() {
	let error = Error::Database("Connection failed".to_string());
	assert_eq!(error.to_string(), "Database error: Connection failed");
}

#[test]
fn test_error_display_serialization() {
	let error = Error::Serialization("Invalid JSON".to_string());
	assert_eq!(error.to_string(), "Serialization error: Invalid JSON");
}

#[test]
fn test_error_display_validation() {
	let error = Error::Validation("Field is required".to_string());
	assert_eq!(error.to_string(), "Validation error: Field is required");
}

#[test]
fn test_error_display_authentication() {
	let error = Error::Authentication("Invalid credentials".to_string());
	assert_eq!(
		error.to_string(),
		"Authentication error: Invalid credentials"
	);
}

#[test]
fn test_error_display_authorization() {
	let error = Error::Authorization("Permission denied".to_string());
	assert_eq!(error.to_string(), "Authorization error: Permission denied");
}

#[test]
fn test_error_display_not_found() {
	let error = Error::NotFound("Resource not found".to_string());
	assert_eq!(error.to_string(), "Not found: Resource not found");
}

#[test]
fn test_error_display_internal() {
	let error = Error::Internal("Server error".to_string());
	assert_eq!(error.to_string(), "Internal server error: Server error");
}

#[test]
fn test_error_from_anyhow() {
	let anyhow_error = anyhow::anyhow!("Some error");
	let error: Error = anyhow_error.into();

	match error {
		Error::Other(_) => {} // Expected
		_ => panic!("Expected Error::Other variant"),
	}
}

#[test]
fn test_result_type_ok() {
	let result: reinhardt_apps::Result<i32> = Ok(42);
	assert_eq!(result.unwrap(), 42);
}

#[test]
fn test_result_type_err() {
	let result: reinhardt_apps::Result<i32> = Err(Error::NotFound("Not found".to_string()));
	assert!(result.is_err());
}

#[test]
fn test_error_categories_client_errors() {
	// Client errors (4xx)
	assert_eq!(Error::Http("test".to_string()).status_code(), 400);
	assert_eq!(Error::Serialization("test".to_string()).status_code(), 400);
	assert_eq!(Error::Validation("test".to_string()).status_code(), 400);
	assert_eq!(Error::Authentication("test".to_string()).status_code(), 401);
	assert_eq!(Error::Authorization("test".to_string()).status_code(), 403);
	assert_eq!(Error::NotFound("test".to_string()).status_code(), 404);
}

#[test]
fn test_error_categories_server_errors() {
	// Server errors (5xx)
	assert_eq!(Error::Database("test".to_string()).status_code(), 500);
	assert_eq!(Error::Internal("test".to_string()).status_code(), 500);
	assert_eq!(Error::Other(anyhow::anyhow!("test")).status_code(), 500);
}

#[test]
fn test_error_messages_contain_context() {
	let error = Error::NotFound("User with id 123".to_string());
	let message = error.to_string();

	assert!(message.contains("User with id 123"));
	assert!(message.contains("Not found"));
}

#[test]
fn test_error_debug_format() {
	let error = Error::Validation("Invalid email format".to_string());
	let debug_str = format!("{:?}", error);

	assert!(debug_str.contains("Validation"));
	assert!(debug_str.contains("Invalid email format"));
}

#[test]
fn test_multiple_errors_different_codes() {
	let errors = vec![
		Error::Authentication("auth error".to_string()),
		Error::NotFound("not found".to_string()),
		Error::Internal("internal error".to_string()),
	];

	assert_eq!(errors[0].status_code(), 401);
	assert_eq!(errors[1].status_code(), 404);
	assert_eq!(errors[2].status_code(), 500);
}

#[test]
fn test_error_response_conversion() {
	use reinhardt_apps::Response;

	let error = Error::NotFound("Resource not found".to_string());
	let response: Response = error.into();

	assert_eq!(response.status.as_u16(), 404);

	// Check that the response body contains error information
	let body_str = String::from_utf8(response.body.to_vec()).unwrap();
	assert!(body_str.contains("error"));
}

#[test]
fn test_error_response_conversion_authentication() {
	use reinhardt_apps::Response;

	let error = Error::Authentication("Invalid token".to_string());
	let response: Response = error.into();

	assert_eq!(response.status.as_u16(), 401);
}

#[test]
fn test_error_response_conversion_validation() {
	use reinhardt_apps::Response;

	let error = Error::Validation("Invalid input".to_string());
	let response: Response = error.into();

	assert_eq!(response.status.as_u16(), 400);
}

#[test]
fn test_error_response_has_json_body() {
	use reinhardt_apps::Response;

	let error = Error::NotFound("Item not found".to_string());
	let response: Response = error.into();

	// Response should have JSON content type
	let content_type = response.headers.get(hyper::header::CONTENT_TYPE);
	assert!(content_type.is_some());

	let body_str = String::from_utf8(response.body.to_vec()).unwrap();
	let json: serde_json::Value = serde_json::from_str(&body_str).unwrap();

	assert!(json.get("error").is_some());
}
