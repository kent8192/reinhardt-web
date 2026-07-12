use reinhardt::HttpError;

#[derive(Debug, HttpError)]
enum ApiError {
	#[http_error(status = BAD_REQUEST)]
	Invalid,
}

fn main() {}
