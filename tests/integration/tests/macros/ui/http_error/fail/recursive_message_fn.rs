use reinhardt::HttpError;

#[derive(Debug, HttpError)]
enum ApiError {
	// Without an inherent method, this name would otherwise resolve to the
	// generated trait method and recurse.
	#[http_error(status = BAD_REQUEST, message_fn = client_message)]
	Invalid,
}

fn main() {}
