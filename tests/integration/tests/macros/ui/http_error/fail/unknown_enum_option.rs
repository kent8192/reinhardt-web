use reinhardt::HttpError;

#[derive(Debug, HttpError)]
#[http_error(response, format = "json")]
enum ApiError {
	#[http_error(status = BAD_REQUEST, message = "Invalid request")]
	Invalid,
}

fn main() {}
