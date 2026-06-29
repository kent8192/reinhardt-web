use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::header::{CONTENT_LENGTH, TRANSFER_ENCODING};
use hyper::{HeaderMap, Method};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub(super) async fn collect_request_body(
	body: Incoming,
	max_body_size: u64,
) -> Result<Bytes, BoxError> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RequestBodyPlan {
	Empty,
	Collect,
	RejectTooLarge,
}

pub(super) fn request_body_plan(
	method: &Method,
	headers: &HeaderMap,
	max_body_size: u64,
) -> RequestBodyPlan {
	let has_transfer_encoding = headers.contains_key(TRANSFER_ENCODING);
	let content_length_header = headers.get(CONTENT_LENGTH);
	let content_length = content_length_header
		.and_then(|value| value.to_str().ok())
		.and_then(|value| value.parse::<u64>().ok());

	if content_length.is_some_and(|len| len > max_body_size) {
		return RequestBodyPlan::RejectTooLarge;
	}

	if is_empty_body_fast_path_method(method)
		&& !has_transfer_encoding
		&& matches!(
			(content_length_header, content_length),
			(None, _) | (Some(_), Some(0))
		) {
		return RequestBodyPlan::Empty;
	}

	RequestBodyPlan::Collect
}

fn is_empty_body_fast_path_method(method: &Method) -> bool {
	method == Method::GET || method == Method::HEAD
}

#[cfg(test)]
mod tests {
	use super::*;
	use hyper::header::HeaderValue;

	#[test]
	fn skips_get_without_declared_body() {
		assert_eq!(
			request_body_plan(&Method::GET, &HeaderMap::new(), 10),
			RequestBodyPlan::Empty
		);
	}

	#[test]
	fn skips_head_with_zero_content_length() {
		let mut headers = HeaderMap::new();
		headers.insert(CONTENT_LENGTH, HeaderValue::from_static("0"));

		assert_eq!(
			request_body_plan(&Method::HEAD, &headers, 10),
			RequestBodyPlan::Empty
		);
	}

	#[test]
	fn collects_get_with_positive_content_length() {
		let mut headers = HeaderMap::new();
		headers.insert(CONTENT_LENGTH, HeaderValue::from_static("3"));

		assert_eq!(
			request_body_plan(&Method::GET, &headers, 10),
			RequestBodyPlan::Collect
		);
	}

	#[test]
	fn collects_get_with_transfer_encoding() {
		let mut headers = HeaderMap::new();
		headers.insert(TRANSFER_ENCODING, HeaderValue::from_static("chunked"));

		assert_eq!(
			request_body_plan(&Method::GET, &headers, 10),
			RequestBodyPlan::Collect
		);
	}

	#[test]
	fn collects_post_without_declared_body() {
		assert_eq!(
			request_body_plan(&Method::POST, &HeaderMap::new(), 10),
			RequestBodyPlan::Collect
		);
	}

	#[test]
	fn collects_get_with_invalid_content_length() {
		let mut headers = HeaderMap::new();
		headers.insert(CONTENT_LENGTH, HeaderValue::from_static("invalid"));

		assert_eq!(
			request_body_plan(&Method::GET, &headers, 10),
			RequestBodyPlan::Collect
		);
	}

	#[test]
	fn rejects_content_length_above_limit() {
		let mut headers = HeaderMap::new();
		headers.insert(CONTENT_LENGTH, HeaderValue::from_static("11"));

		assert_eq!(
			request_body_plan(&Method::POST, &headers, 10),
			RequestBodyPlan::RejectTooLarge
		);
	}
}
