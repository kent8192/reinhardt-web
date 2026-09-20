use async_trait::async_trait;
use hyper::{StatusCode, header};
use reinhardt_http::{Handler, Middleware, Request, Response};
use reinhardt_middleware::{
	ConditionalGetMiddleware, cache::CacheMiddleware, etag::ETagMiddleware,
};
use rstest::rstest;
use std::{
	io::Write,
	sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	},
};

struct FileHandler {
	response: Response,
	calls: AtomicUsize,
}

impl FileHandler {
	fn new(etag: Option<&str>) -> Arc<Self> {
		let mut file = tempfile::tempfile().unwrap();
		file.write_all(b"actual file bytes").unwrap();
		let mut response = Response::ok()
			.with_file_body(file, 0, 17)
			.unwrap()
			.with_header("content-type", "text/plain");
		if let Some(etag) = etag {
			response = response.with_header("etag", etag);
		}
		Arc::new(Self {
			response,
			calls: AtomicUsize::new(0),
		})
	}
}

#[async_trait]
impl Handler for FileHandler {
	async fn handle(&self, _: Request) -> reinhardt_core::exception::Result<Response> {
		self.calls.fetch_add(1, Ordering::SeqCst);
		Ok(self.response.clone())
	}
}

#[rstest]
#[tokio::test]
async fn shared_cache_does_not_store_an_empty_buffer_in_place_of_a_file() {
	// Arrange
	let handler = FileHandler::new(None);
	let middleware = CacheMiddleware::with_defaults();
	// Act / Assert
	for _ in 0..2 {
		let response = middleware
			.process(
				Request::builder().uri("/video").build().unwrap(),
				handler.clone(),
			)
			.await
			.unwrap();
		assert_eq!(
			response
				.file_body()
				.unwrap()
				.read_chunk(0, 17)
				.unwrap()
				.as_ref(),
			b"actual file bytes"
		);
	}
	assert_eq!(handler.calls.load(Ordering::SeqCst), 2);
	assert!(middleware.store().is_empty());
}

#[rstest]
#[case(false)]
#[case(true)]
#[tokio::test]
async fn file_responses_never_receive_the_etag_of_an_empty_buffer(#[case] conditional: bool) {
	// Arrange
	let handler = FileHandler::new(None);
	let middleware: Box<dyn Middleware> = if conditional {
		Box::new(ConditionalGetMiddleware::new())
	} else {
		Box::new(ETagMiddleware::with_defaults())
	};
	// Act
	let response = middleware
		.process(Request::builder().uri("/video").build().unwrap(), handler)
		.await
		.unwrap();
	// Assert
	assert_eq!(response.status, StatusCode::OK);
	assert!(!response.headers.contains_key(header::ETAG));
	assert_eq!(response.file_body().unwrap().len(), 17);
}

#[rstest]
#[tokio::test]
async fn etag_middleware_preserves_the_file_owners_validator() {
	// Arrange
	let handler = FileHandler::new(Some("\"file-digest\""));
	let middleware = ETagMiddleware::with_defaults();
	// Act
	let response = middleware
		.process(Request::builder().uri("/video").build().unwrap(), handler)
		.await
		.unwrap();
	// Assert
	assert_eq!(response.headers[header::ETAG], "\"file-digest\"");
	assert_eq!(response.file_body().unwrap().len(), 17);
}
