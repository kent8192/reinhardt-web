use reinhardt::HttpError;

#[derive(Debug, HttpError)]
enum ApiError {
	#[http_error(
		status = BAD_REQUEST,
		message = "Invalid request",
		message_fn = invalid_message
	)]
	Invalid,
}

impl ApiError {
	fn invalid_message(&self) -> &'static str {
		"Invalid request"
	}
}

fn main() {}
