use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::header::{CONTENT_LENGTH, TRANSFER_ENCODING};
use hyper::{HeaderMap, Method};
use std::fmt;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug)]
pub(super) enum CollectRequestBodyError {
	TooLarge,
	Read(BoxError),
}

impl CollectRequestBodyError {
	pub(super) fn is_too_large(&self) -> bool {
		matches!(self, Self::TooLarge)
	}

	pub(super) fn into_box_error(self) -> BoxError {
		match self {
			Self::TooLarge => Box::new(std::io::Error::new(
				std::io::ErrorKind::InvalidData,
				"Request body exceeds size limit",
			)),
			Self::Read(error) => error,
		}
	}
}

impl From<BoxError> for CollectRequestBodyError {
	fn from(error: BoxError) -> Self {
		if error
			.downcast_ref::<http_body_util::LengthLimitError>()
			.is_some()
		{
			Self::TooLarge
		} else {
			Self::Read(error)
		}
	}
}

impl fmt::Display for CollectRequestBodyError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::TooLarge => f.write_str("Request body exceeds size limit"),
			Self::Read(error) => fmt::Display::fmt(error, f),
		}
	}
}

impl std::error::Error for CollectRequestBodyError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::TooLarge => None,
			Self::Read(error) => Some(error.as_ref()),
		}
	}
}

pub(super) async fn collect_request_body(
	body: Incoming,
	max_body_size: u64,
) -> Result<Bytes, CollectRequestBodyError> {
	http_body_util::Limited::new(body, max_body_size as usize)
		.collect()
		.await
		.map_err(CollectRequestBodyError::from)
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
	request_body_plan_with_empty_fast_path(method, headers, max_body_size, true)
}

pub(super) fn request_body_plan_collecting_unsized(
	method: &Method,
	headers: &HeaderMap,
	max_body_size: u64,
) -> RequestBodyPlan {
	request_body_plan_with_empty_fast_path(method, headers, max_body_size, false)
}

fn request_body_plan_with_empty_fast_path(
	method: &Method,
	headers: &HeaderMap,
	max_body_size: u64,
	allow_empty_fast_path: bool,
) -> RequestBodyPlan {
	let has_transfer_encoding = headers.contains_key(TRANSFER_ENCODING);
	let content_length_header = headers.get(CONTENT_LENGTH);
	let content_length = content_length_header
		.and_then(|value| value.to_str().ok())
		.and_then(|value| value.parse::<u64>().ok());

	if content_length.is_some_and(|len| len > max_body_size) {
		return RequestBodyPlan::RejectTooLarge;
	}

	if allow_empty_fast_path
		&& is_empty_body_fast_path_method(method)
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

	#[tokio::test]
	async fn collect_error_identifies_length_limit_errors() {
		let error = http_body_util::Limited::new(
			http_body_util::Full::new(Bytes::from_static(b"too large")),
			1,
		)
		.collect()
		.await
		.expect_err("body should exceed the configured limit");
		let error = CollectRequestBodyError::from(error);

		assert!(error.is_too_large());
	}

	#[test]
	fn skips_get_without_declared_body() {
		assert_eq!(
			request_body_plan(&Method::GET, &HeaderMap::new(), 10),
			RequestBodyPlan::Empty
		);
	}

	#[test]
	fn collects_get_without_declared_body_when_empty_fast_path_disabled() {
		assert_eq!(
			request_body_plan_collecting_unsized(&Method::GET, &HeaderMap::new(), 10),
			RequestBodyPlan::Collect
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
	fn collects_head_with_zero_content_length_when_empty_fast_path_disabled() {
		let mut headers = HeaderMap::new();
		headers.insert(CONTENT_LENGTH, HeaderValue::from_static("0"));

		assert_eq!(
			request_body_plan_collecting_unsized(&Method::HEAD, &headers, 10),
			RequestBodyPlan::Collect
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
