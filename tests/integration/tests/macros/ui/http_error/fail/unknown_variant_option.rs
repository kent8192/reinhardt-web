use reinhardt::HttpError;

#[derive(Debug, HttpError)]
enum ApiError {
	#[http_error(status = BAD_REQUEST, message = "Invalid request", retryable)]
	Invalid,
}

fn main() {}
