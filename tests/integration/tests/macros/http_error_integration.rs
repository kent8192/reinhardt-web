use reinhardt::{HttpError, Response, StatusCode};
use rstest::rstest;

#[derive(Debug, HttpError)]
#[http_error(response)]
enum SafeApiError {
	#[http_error(status = BAD_REQUEST, message = "Invalid request")]
	Invalid,
	#[http_error(status = INTERNAL_SERVER_ERROR, message = "Database password leaked")]
	Internal,
}

#[derive(Debug, HttpError)]
#[http_error(response, body = "error")]
enum EnvelopeApiError {
	#[http_error(status = SERVICE_UNAVAILABLE, message_fn = client_message)]
	Unavailable(String),
}

impl EnvelopeApiError {
	fn client_message(&self) -> String {
		match self {
			Self::Unavailable(provider) => format!("{provider} is unavailable"),
		}
	}
}

#[rstest]
fn derive_maps_status_and_fixed_message() {
	// Arrange
	let error = SafeApiError::Invalid;

	// Act
	let status = error.status_code();
	let message = error.client_message();

	// Assert
	assert_eq!(status, StatusCode::BAD_REQUEST);
	assert_eq!(message, "Invalid request");
}

#[rstest]
fn safe_response_includes_4xx_detail() {
	// Arrange
	let error = SafeApiError::Invalid;

	// Act
	let response = Response::from(error);
	let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();

	// Assert
	assert_eq!(response.status, StatusCode::BAD_REQUEST);
	assert_eq!(body["error"], "Bad Request");
	assert_eq!(body["detail"], "Invalid request");
}

#[rstest]
fn safe_response_omits_5xx_detail() {
	// Arrange
	let error = SafeApiError::Internal;

	// Act
	let response = Response::from(error);
	let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();

	// Assert
	assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
	assert_eq!(body["error"], "Internal Server Error");
	assert_eq!(body.get("detail"), None);
}

#[rstest]
fn error_body_response_uses_client_message() {
	// Arrange
	let error = EnvelopeApiError::Unavailable("OpenAI".to_string());

	// Act
	let response = Response::from(error);
	let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();

	// Assert
	assert_eq!(response.status, StatusCode::SERVICE_UNAVAILABLE);
	assert_eq!(body, serde_json::json!({"error": "OpenAI is unavailable"}));
}
