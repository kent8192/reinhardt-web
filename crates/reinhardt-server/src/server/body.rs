use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::header::{CONTENT_LENGTH, TRANSFER_ENCODING};
use hyper::{HeaderMap, Method};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub(super) async fn collect_request_body(
	method: &Method,
	headers: &HeaderMap,
	body: Incoming,
	max_body_size: u64,
) -> Result<Bytes, BoxError> {
	if can_skip_body_collect(method, headers) {
		return Ok(Bytes::new());
	}

	http_body_util::Limited::new(body, max_body_size as usize)
		.collect()
		.await
		.map_err(|_| {
			Box::new(std::io::Error::new(
				std::io::ErrorKind::InvalidData,
				"Request body exceeds size limit",
			)) as BoxError
		})
		.map(|collected| collected.to_bytes())
}

fn can_skip_body_collect(method: &Method, headers: &HeaderMap) -> bool {
	is_empty_body_fast_path_method(method) && !has_declared_body(headers)
}

fn is_empty_body_fast_path_method(method: &Method) -> bool {
	method == Method::GET || method == Method::HEAD
}

fn has_declared_body(headers: &HeaderMap) -> bool {
	if headers.contains_key(TRANSFER_ENCODING) {
		return true;
	}

	let Some(content_length) = headers.get(CONTENT_LENGTH) else {
		return false;
	};

	match content_length
		.to_str()
		.ok()
		.and_then(|value| value.parse::<u64>().ok())
	{
		Some(0) => false,
		Some(_) | None => true,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use hyper::header::HeaderValue;

	#[test]
	fn skips_get_without_declared_body() {
		assert!(can_skip_body_collect(&Method::GET, &HeaderMap::new()));
	}

	#[test]
	fn skips_head_with_zero_content_length() {
		let mut headers = HeaderMap::new();
		headers.insert(CONTENT_LENGTH, HeaderValue::from_static("0"));

		assert!(can_skip_body_collect(&Method::HEAD, &headers));
	}

	#[test]
	fn collects_get_with_positive_content_length() {
		let mut headers = HeaderMap::new();
		headers.insert(CONTENT_LENGTH, HeaderValue::from_static("3"));

		assert!(!can_skip_body_collect(&Method::GET, &headers));
	}

	#[test]
	fn collects_get_with_transfer_encoding() {
		let mut headers = HeaderMap::new();
		headers.insert(TRANSFER_ENCODING, HeaderValue::from_static("chunked"));

		assert!(!can_skip_body_collect(&Method::GET, &headers));
	}

	#[test]
	fn collects_post_without_declared_body() {
		assert!(!can_skip_body_collect(&Method::POST, &HeaderMap::new()));
	}

	#[test]
	fn collects_get_with_invalid_content_length() {
		let mut headers = HeaderMap::new();
		headers.insert(CONTENT_LENGTH, HeaderValue::from_static("invalid"));

		assert!(!can_skip_body_collect(&Method::GET, &headers));
	}
}
