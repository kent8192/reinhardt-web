use reinhardt::{HttpError, StatusCode};

#[derive(Debug, HttpError)]
enum ApiError {
	#[http_error(status = BAD_REQUEST, message = "Invalid request")]
	Invalid,
	#[http_error(status = SERVICE_UNAVAILABLE, message_fn = unavailable_message)]
	Unavailable(String),
}

impl ApiError {
	fn unavailable_message(&self) -> String {
		match self {
			Self::Unavailable(provider) => format!("{provider} is unavailable"),
			Self::Invalid => unreachable!(),
		}
	}
}

fn main() {
	let invalid = ApiError::Invalid;
	assert_eq!(invalid.status_code(), StatusCode::BAD_REQUEST);
	assert_eq!(invalid.client_message(), "Invalid request");

	let unavailable = ApiError::Unavailable("database".to_string());
	assert_eq!(unavailable.status_code(), StatusCode::SERVICE_UNAVAILABLE);
	assert_eq!(unavailable.client_message(), "database is unavailable");
}
