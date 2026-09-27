use bytes::Bytes;
use futures_util::{StreamExt, stream};
use hyper::{StatusCode, header};
use reinhardt_http::{Response, StreamBody, StreamingResponse};
use rstest::rstest;
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

struct DropProbe(Arc<AtomicUsize>);

impl Drop for DropProbe {
	fn drop(&mut self) {
		self.0.fetch_add(1, Ordering::SeqCst);
	}
}

fn pending_stream(drops: Arc<AtomicUsize>) -> StreamBody {
	let probe = DropProbe(drops);
	Box::pin(stream::unfold(probe, |probe| async move {
		std::future::pending::<()>().await;
		Some((Ok(Bytes::new()), probe))
	}))
}

#[rstest]
#[tokio::test]
async fn conversion_preserves_metadata_without_polling_or_buffering() {
	// Arrange
	let polls = Arc::new(AtomicUsize::new(0));
	let observed = polls.clone();
	let stream = stream::poll_fn(move |_| {
		observed.fetch_add(1, Ordering::SeqCst);
		std::task::Poll::Ready(Some(Ok(Bytes::from_static(b"data: hello\n\n"))))
	});
	let streaming = StreamingResponse::with_status(stream, StatusCode::ACCEPTED)
		.media_type("text/event-stream")
		.header(header::CONTENT_LENGTH, "0".parse().unwrap())
		.header(header::TRANSFER_ENCODING, "chunked".parse().unwrap());
	// Act
	let mut response = Response::from(streaming);
	// Assert
	assert_eq!(polls.load(Ordering::SeqCst), 0);
	assert_eq!(response.status, StatusCode::ACCEPTED);
	assert_eq!(response.headers[header::CONTENT_TYPE], "text/event-stream");
	assert!(!response.headers.contains_key(header::CONTENT_LENGTH));
	assert!(!response.headers.contains_key(header::TRANSFER_ENCODING));
	assert!(response.body.is_empty());
	let mut body = response.take_stream_body().unwrap();
	assert_eq!(body.next().await.unwrap().unwrap(), "data: hello\n\n");
	assert_eq!(polls.load(Ordering::SeqCst), 1);
}

#[rstest]
#[tokio::test]
async fn clones_transfer_one_producer_and_cannot_replay_it() {
	// Arrange
	let drops = Arc::new(AtomicUsize::new(0));
	let mut response = Response::ok().with_stream(pending_stream(drops.clone()));
	let mut clone = response.clone();
	assert_eq!(response, clone);
	// Act
	let stream = response.take_stream_body().unwrap();
	let error = clone
		.take_stream_body()
		.unwrap()
		.next()
		.await
		.unwrap()
		.unwrap_err();
	drop(stream);
	// Assert: response metadata remaining alive cannot retain the producer.
	assert_eq!(error.to_string(), "response stream has already been taken");
	assert_eq!(drops.load(Ordering::SeqCst), 1);
	assert!(response.take_stream_body().is_none());
}

#[rstest]
#[case("bytes")]
#[case("static")]
#[case("json")]
#[case("file")]
fn replacing_streaming_bodies_releases_the_producer(#[case] replacement: &str) {
	// Arrange
	let drops = Arc::new(AtomicUsize::new(0));
	let response = Response::ok()
		.with_stream(pending_stream(drops.clone()))
		.with_header("content-length", "999")
		.with_header("transfer-encoding", "chunked");
	// Act
	let mut response = match replacement {
		"bytes" => response.with_body("replacement"),
		"static" => response.with_static_body(b"replacement"),
		"json" => response.with_json(&"replacement").unwrap(),
		"file" => response
			.with_file_body(tempfile::tempfile().unwrap(), 0, 0)
			.unwrap(),
		_ => unreachable!(),
	};
	// Assert
	assert_eq!(drops.load(Ordering::SeqCst), 1);
	assert!(response.take_stream_body().is_none());
	assert!(!response.headers.contains_key(header::TRANSFER_ENCODING));
	assert_eq!(response.is_streaming(), replacement == "file");
	assert_eq!(
		response
			.headers
			.get(header::CONTENT_LENGTH)
			.map(|v| v.to_str().unwrap()),
		(replacement == "file").then_some("0")
	);
}

#[rstest]
fn streams_replace_file_ranges_and_keep_response_send_sync() {
	// Arrange
	fn assert_send_sync<T: Send + Sync>() {}
	assert_send_sync::<Response>();
	let response = Response::ok()
		.with_file_body(tempfile::tempfile().unwrap(), 0, 0)
		.unwrap();
	// Act
	let response = response.with_stream(stream::pending());
	// Assert
	assert!(response.is_streaming());
	assert!(response.file_body().is_none());
	assert!(!response.headers.contains_key(header::CONTENT_LENGTH));
}
