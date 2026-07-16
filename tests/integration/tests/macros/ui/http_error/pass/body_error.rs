use reinhardt::{HttpError, Response, StatusCode};

#[derive(Debug, HttpError)]
#[http_error(response, body = "error")]
enum ApiError {
	#[http_error(status = BAD_REQUEST, message = "Invalid request")]
	Invalid,
}

fn main() {
	let response = Response::from(ApiError::Invalid);
	assert_eq!(response.status, StatusCode::BAD_REQUEST);
}
