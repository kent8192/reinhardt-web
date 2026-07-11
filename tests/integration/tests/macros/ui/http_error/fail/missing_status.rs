use reinhardt::HttpError;

#[derive(Debug, HttpError)]
enum ApiError {
	#[http_error(message = "Invalid request")]
	Invalid,
}

fn main() {}
