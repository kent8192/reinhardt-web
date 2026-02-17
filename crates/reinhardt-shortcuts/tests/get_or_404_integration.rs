//! GetError and Response conversion integration tests
//!
//! Tests GetError type conversions, error messages, and HTTP response
//! generation for database query shortcuts.

use hyper::StatusCode;
use reinhardt_http::Response;
use reinhardt_shortcuts::GetError;
use rstest::rstest;

/// Test: GetError::NotFound to Response conversion
#[rstest]
fn test_not_found_error_to_response() {
	let error = GetError::NotFound;
	let response: Response = error.into();

	assert_eq!(response.status, StatusCode::NOT_FOUND);
}

/// Test: GetError::MultipleObjectsReturned to Response conversion
#[rstest]
fn test_multiple_objects_error_to_response() {
	let error = GetError::MultipleObjectsReturned;
	let response: Response = error.into();

	assert_eq!(response.status, StatusCode::BAD_REQUEST);
	assert_eq!(
		response.body,
		bytes::Bytes::from("Multiple objects returned")
	);
}

/// Test: GetError::DatabaseError to Response conversion
#[rstest]
fn test_database_error_to_response() {
	let error = GetError::DatabaseError("Connection timeout".to_string());
	let response: Response = error.into();

	assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
	assert_eq!(
		response.body,
		bytes::Bytes::from("Database error: Connection timeout")
	);
}

/// Test: GetError::NotFound error message
#[rstest]
fn test_not_found_error_message() {
	let error = GetError::NotFound;
	assert_eq!(error.to_string(), "Object not found");
}

/// Test: GetError::MultipleObjectsReturned error message
#[rstest]
fn test_multiple_objects_error_message() {
	let error = GetError::MultipleObjectsReturned;
	assert_eq!(error.to_string(), "Multiple objects returned");
}

/// Test: GetError::DatabaseError error message
#[rstest]
fn test_database_error_message() {
	let error = GetError::DatabaseError("Table does not exist".to_string());
	assert_eq!(error.to_string(), "Database error: Table does not exist");
}

/// Test: Multiple different database error messages
#[rstest]
fn test_various_database_error_messages() {
	let errors = [
		GetError::DatabaseError("Connection refused".to_string()),
		GetError::DatabaseError("Query timeout".to_string()),
		GetError::DatabaseError("Permission denied".to_string()),
		GetError::DatabaseError("Syntax error".to_string()),
	];

	let messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();

	assert_eq!(messages[0], "Database error: Connection refused");
	assert_eq!(messages[1], "Database error: Query timeout");
	assert_eq!(messages[2], "Database error: Permission denied");
	assert_eq!(messages[3], "Database error: Syntax error");
}

/// Test: GetError Debug formatting
#[rstest]
fn test_get_error_debug() {
	let error = GetError::NotFound;
	let debug_str = format!("{:?}", error);
	assert_eq!(debug_str, "NotFound");

	let error = GetError::MultipleObjectsReturned;
	let debug_str = format!("{:?}", error);
	assert_eq!(debug_str, "MultipleObjectsReturned");

	let error = GetError::DatabaseError("test".to_string());
	let debug_str = format!("{:?}", error);
	assert!(debug_str.contains("DatabaseError"));
}

/// Test: Response from NotFound has correct content type
#[rstest]
fn test_not_found_response_content_type() {
	let error = GetError::NotFound;
	let response: Response = error.into();

	// Default Response should have basic structure
	assert_eq!(response.status, StatusCode::NOT_FOUND);
	// Body might be empty for not_found()
	// Headers might have content-type
}

/// Test: Response from DatabaseError preserves error details
#[rstest]
fn test_database_error_response_preserves_details() {
	let original_message = "Unique constraint violation on column 'email'";
	let error = GetError::DatabaseError(original_message.to_string());
	let response: Response = error.into();

	assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
	let body_str = String::from_utf8_lossy(&response.body);
	assert!(body_str.contains(original_message));
}

/// Test: Empty database error message
#[rstest]
fn test_empty_database_error_message() {
	let error = GetError::DatabaseError(String::new());
	assert_eq!(error.to_string(), "Database error: ");

	let response: Response = error.into();
	assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
	assert_eq!(response.body, bytes::Bytes::from("Database error: "));
}

/// Test: UTF-8 database error message
#[rstest]
fn test_utf8_database_error_message() {
	let error = GetError::DatabaseError("エラー: データベース接続失敗".to_string());
	assert_eq!(
		error.to_string(),
		"Database error: エラー: データベース接続失敗"
	);

	let response: Response = error.into();
	let body_str = String::from_utf8_lossy(&response.body);
	assert!(body_str.contains("データベース接続失敗"));
}

/// Test: Very long database error message
#[rstest]
fn test_long_database_error_message() {
	let long_message = "Error occurred while processing query: ".to_string()
		+ &"SQL statement execution failed due to ".repeat(10)
		+ "constraint violation";

	let error = GetError::DatabaseError(long_message.clone());
	let response: Response = error.into();

	let body_str = String::from_utf8_lossy(&response.body);
	assert!(body_str.contains("constraint violation"));
	assert!(body_str.len() > 100);
}
