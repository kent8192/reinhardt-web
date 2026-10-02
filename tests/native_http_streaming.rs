//! Regression coverage for HTTP macro streams over the native transports (#6407).

// Proc-macro self-resolution uses the package name in the facade's own tests.
extern crate reinhardt as reinhardt_web;

use bytes::Bytes;
use futures_util::{Stream, StreamExt, stream};
use reinhardt::http::{
	Handler, Middleware, MiddlewareChain, Request, Response, StreamBody, StreamingResponse,
	ViewResult,
};
use reinhardt::server::{Http2Server, HttpServer};
use reinhardt::{ServerRouter, get};
use rstest::{fixture, rstest};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::{Notify, mpsc};
use tokio::task::JoinHandle;

type Chunk = Result<Bytes, Box<dyn std::error::Error + Send + Sync>>;
const DEADLINE: Duration = Duration::from_secs(5);

#[get("/finite", name = "finite-stream")]
async fn finite() -> ViewResult<StreamingResponse<StreamBody>> {
	let stream: StreamBody = Box::pin(stream::iter([Ok(Bytes::from_static(b"data: hello\n\n"))]));
	Ok(StreamingResponse::new(stream).media_type("text/event-stream"))
}

#[derive(Clone)]
struct StreamState {
	receiver: Arc<Mutex<Option<mpsc::Receiver<Chunk>>>>,
	dropped: Arc<Notify>,
}

struct EventStream {
	receiver: mpsc::Receiver<Chunk>,
	dropped: Arc<Notify>,
}

impl Stream for EventStream {
	type Item = Chunk;
	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Chunk>> {
		self.receiver.poll_recv(cx)
	}
}

impl Drop for EventStream {
	fn drop(&mut self) {
		self.dropped.notify_one();
	}
}

fn event_response(request: Request) -> StreamingResponse<StreamBody> {
	let state = request.extensions.get::<StreamState>().unwrap();
	let stream = EventStream {
		receiver: state.receiver.lock().unwrap().take().unwrap(),
		dropped: state.dropped.clone(),
	};
	StreamingResponse::new(Box::pin(stream) as StreamBody).media_type("text/event-stream")
}

#[get("/events", name = "native-events")]
async fn events(request: Request) -> ViewResult<StreamingResponse<StreamBody>> {
	Ok(event_response(request))
}

struct HeadEvents;

impl reinhardt::core::endpoint::EndpointInfo for HeadEvents {
	fn path() -> &'static str {
		"/events"
	}
	fn method() -> hyper::Method {
		hyper::Method::HEAD
	}
	fn name() -> &'static str {
		"native-events-head"
	}
}

#[async_trait::async_trait]
impl Handler for HeadEvents {
	async fn handle(&self, request: Request) -> ViewResult<Response> {
		Ok(event_response(request).into())
	}
}

#[get("/buffered", name = "buffered-response")]
async fn buffered() -> ViewResult<Response> {
	Ok(Response::ok().with_static_body(b"buffered"))
}

#[get("/extracted/{id}", name = "extracted-events")]
async fn extracted_events(
	request: Request,
	id: reinhardt::di::params::Path<u64>,
) -> ViewResult<StreamingResponse<StreamBody>> {
	assert_eq!(id.into_inner(), 42);
	Ok(event_response(request))
}

struct AuthenticatedStream(StreamState);

#[async_trait::async_trait]
impl Middleware for AuthenticatedStream {
	async fn process(&self, request: Request, next: Arc<dyn Handler>) -> ViewResult<Response> {
		if request
			.headers
			.get("authorization")
			.and_then(|value| value.to_str().ok())
			!= Some("Bearer test")
		{
			return Ok(Response::unauthorized());
		}
		request.extensions.insert(self.0.clone());
		let response = next.handle(request).await?;
		Ok(response.with_header("x-stream-middleware", "applied"))
	}
}

struct ServerTask(JoinHandle<()>);

impl Drop for ServerTask {
	fn drop(&mut self) {
		self.0.abort();
	}
}

struct StreamFixture {
	sender: mpsc::Sender<Chunk>,
	state: StreamState,
}

#[fixture]
fn stream_fixture() -> StreamFixture {
	let (sender, receiver) = mpsc::channel(1);
	StreamFixture {
		sender,
		state: StreamState {
			receiver: Arc::new(Mutex::new(Some(receiver))),
			dropped: Arc::new(Notify::new()),
		},
	}
}

async fn start_server(state: StreamState, http2: bool) -> (String, ServerTask, reqwest::Client) {
	let router = ServerRouter::new()
		.endpoint(finite)
		.endpoint(events)
		.endpoint(|| HeadEvents)
		.endpoint(buffered)
		.endpoint(extracted_events);
	let handler = Arc::new(
		MiddlewareChain::new(Arc::new(router))
			.with_middleware(Arc::new(AuthenticatedStream(state))),
	);
	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
	let address = listener.local_addr().unwrap();
	let task = ServerTask(tokio::spawn(async move {
		let (stream, peer) = listener.accept().await.unwrap();
		// Disconnect and producer-error cases intentionally terminate the connection.
		if http2 {
			let _result = Http2Server::handle_connection(stream, handler).await;
		} else {
			let _result = HttpServer::handle_connection(stream, peer, handler, None).await;
		}
	}));
	let mut client = reqwest::Client::builder().timeout(DEADLINE).no_proxy();
	if http2 {
		client = client.http2_prior_knowledge();
	}
	(format!("http://{address}"), task, client.build().unwrap())
}

#[rstest]
#[case(false, "/events")]
#[case(true, "/events")]
#[case(false, "/extracted/42")]
#[case(true, "/extracted/42")]
#[tokio::test]
async fn first_sse_frame_arrives_before_completion_and_disconnect_drops_producer(
	stream_fixture: StreamFixture,
	#[case] http2: bool,
	#[case] path: &str,
) {
	// Arrange: the sender remains open after one frame, so the producer cannot finish.
	let StreamFixture { sender, state } = stream_fixture;
	let dropped = state.dropped.clone();
	sender
		.send(Ok(Bytes::from_static(b"data: first\n\n")))
		.await
		.unwrap();
	let (url, _server, client) = start_server(state, http2).await;
	// Act
	let mut response = client
		.get(format!("{url}{path}"))
		.bearer_auth("test")
		.send()
		.await
		.unwrap();
	let first = tokio::time::timeout(DEADLINE, response.chunk())
		.await
		.unwrap()
		.unwrap()
		.unwrap();
	// Assert
	assert_eq!(response.status(), 200);
	assert_eq!(response.headers()["content-type"], "text/event-stream");
	assert_eq!(response.headers()["x-stream-middleware"], "applied");
	assert_eq!(response.content_length(), None);
	assert_eq!(first, "data: first\n\n");
	assert!(!sender.is_closed());
	drop(response);
	drop(client);
	tokio::time::timeout(DEADLINE, dropped.notified())
		.await
		.expect("disconnect must drop the producer");
	assert!(sender.is_closed());
}

#[rstest]
#[case(false)]
#[case(true)]
#[tokio::test]
async fn producer_errors_abort_the_http_body(stream_fixture: StreamFixture, #[case] http2: bool) {
	// Arrange
	let StreamFixture { sender, state } = stream_fixture;
	let dropped = state.dropped.clone();
	sender
		.send(Ok(Bytes::from_static(b"data: first\n\n")))
		.await
		.unwrap();
	let (url, _server, client) = start_server(state, http2).await;
	let mut response = client
		.get(format!("{url}/events"))
		.bearer_auth("test")
		.send()
		.await
		.unwrap();
	assert_eq!(response.chunk().await.unwrap().unwrap(), "data: first\n\n");
	// Act: fail only after the client has received a successful frame.
	sender
		.send(Err(std::io::Error::other("producer failed").into()))
		.await
		.unwrap();
	let result = tokio::time::timeout(DEADLINE, response.chunk())
		.await
		.unwrap();
	// Assert
	assert!(
		result.is_err(),
		"a producer failure must not become successful EOF"
	);
	tokio::time::timeout(DEADLINE, dropped.notified())
		.await
		.unwrap();
	assert!(sender.is_closed());
}

#[rstest]
#[case(false)]
#[case(true)]
#[tokio::test]
async fn zero_argument_macro_stream_reaches_the_native_server(
	stream_fixture: StreamFixture,
	#[case] http2: bool,
) {
	// Arrange
	let (url, _server, client) = start_server(stream_fixture.state, http2).await;
	// Act
	let response = client
		.get(format!("{url}/finite"))
		.bearer_auth("test")
		.send()
		.await
		.unwrap();
	// Assert
	assert_eq!(response.status(), 200);
	assert_eq!(response.bytes().await.unwrap(), "data: hello\n\n");
}

#[rstest]
#[tokio::test]
async fn authentication_can_reject_before_creating_the_stream(stream_fixture: StreamFixture) {
	// Arrange
	let state = stream_fixture.state.clone();
	let (url, _server, client) = start_server(state.clone(), false).await;
	// Act
	let response = client.get(format!("{url}/events")).send().await.unwrap();
	// Assert
	assert_eq!(response.status(), 401);
	assert!(state.receiver.lock().unwrap().is_some());
}

#[rstest]
#[case(false)]
#[case(true)]
#[tokio::test]
async fn head_responses_drop_streams_without_waiting_for_a_chunk(
	stream_fixture: StreamFixture,
	#[case] http2: bool,
) {
	// Arrange: no chunk is ever made available.
	let StreamFixture { sender, state } = stream_fixture;
	let dropped = state.dropped.clone();
	let (url, _server, client) = start_server(state, http2).await;
	// Act
	let response = client
		.head(format!("{url}/events"))
		.bearer_auth("test")
		.send()
		.await
		.unwrap();
	// Assert
	assert_eq!(response.status(), 200);
	assert!(response.bytes().await.unwrap().is_empty());
	tokio::time::timeout(DEADLINE, dropped.notified())
		.await
		.unwrap();
	assert!(sender.is_closed());
}

#[rstest]
#[case(false)]
#[case(true)]
#[tokio::test]
async fn buffered_macro_responses_keep_their_body(
	stream_fixture: StreamFixture,
	#[case] http2: bool,
) {
	// Arrange
	let (url, _server, client) = start_server(stream_fixture.state, http2).await;
	// Act
	let response = client
		.get(format!("{url}/buffered"))
		.bearer_auth("test")
		.send()
		.await
		.unwrap();
	// Assert
	assert_eq!(response.status(), 200);
	assert_eq!(response.bytes().await.unwrap(), "buffered");
}

#[rstest]
#[tokio::test]
async fn typed_streaming_response_can_be_converted_by_a_manual_handler() {
	// Arrange
	struct ManualHandler;
	#[async_trait::async_trait]
	impl Handler for ManualHandler {
		async fn handle(&self, _: Request) -> ViewResult<Response> {
			Ok(StreamingResponse::new(stream::iter([Ok(Bytes::from_static(b"manual"))])).into())
		}
	}
	// Act
	let mut response = ManualHandler
		.handle(Request::builder().uri("/").build().unwrap())
		.await
		.unwrap();
	// Assert
	assert_eq!(
		response
			.take_stream_body()
			.unwrap()
			.next()
			.await
			.unwrap()
			.unwrap(),
		"manual"
	);
}

#[cfg(feature = "reinhardt-middleware")]
mod body_middleware {
	use super::*;
	use reinhardt_middleware::{
		ConditionalGetMiddleware, cache::CacheMiddleware, etag::ETagMiddleware,
	};
	use std::sync::atomic::{AtomicUsize, Ordering};

	struct FreshStream(AtomicUsize);

	#[async_trait::async_trait]
	impl Handler for FreshStream {
		async fn handle(&self, _: Request) -> ViewResult<Response> {
			self.0.fetch_add(1, Ordering::SeqCst);
			Ok(Response::ok()
				.with_stream(stream::iter([Ok(Bytes::from_static(
					b"data: uncached\n\n",
				))]))
				.with_header("content-type", "text/event-stream"))
		}
	}

	#[rstest]
	#[case(false)]
	#[case(true)]
	#[tokio::test]
	async fn streaming_responses_are_not_hashed_as_empty_buffers(#[case] conditional: bool) {
		// Arrange
		let middleware: Box<dyn Middleware> = if conditional {
			Box::new(ConditionalGetMiddleware::new())
		} else {
			Box::new(ETagMiddleware::with_defaults())
		};
		// Act
		let mut response = middleware
			.process(
				Request::builder().uri("/events").build().unwrap(),
				Arc::new(FreshStream(AtomicUsize::new(0))),
			)
			.await
			.unwrap();
		// Assert
		assert!(!response.headers.contains_key("etag"));
		assert!(response.is_streaming());
		assert_eq!(
			response
				.take_stream_body()
				.unwrap()
				.next()
				.await
				.unwrap()
				.unwrap(),
			"data: uncached\n\n"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn streaming_responses_are_never_stored_in_the_buffered_cache() {
		// Arrange
		let middleware = CacheMiddleware::with_defaults();
		let handler = Arc::new(FreshStream(AtomicUsize::new(0)));
		// Act / Assert
		for _ in 0..2 {
			let mut response = middleware
				.process(
					Request::builder().uri("/events").build().unwrap(),
					handler.clone(),
				)
				.await
				.unwrap();
			assert_eq!(
				response
					.take_stream_body()
					.unwrap()
					.next()
					.await
					.unwrap()
					.unwrap(),
				"data: uncached\n\n"
			);
		}
		assert_eq!(handler.0.load(Ordering::SeqCst), 2);
		assert!(middleware.store().is_empty());
	}

	#[cfg(feature = "middleware-compression")]
	#[rstest]
	#[case(false)]
	#[case(true)]
	#[tokio::test]
	async fn compression_preserves_an_unknown_length_stream(#[case] brotli: bool) {
		// Arrange
		let middleware: Box<dyn Middleware> = if brotli {
			Box::new(reinhardt_middleware::BrotliMiddleware::new())
		} else {
			Box::new(reinhardt_middleware::GZipMiddleware::new())
		};
		let request = Request::builder()
			.uri("/events")
			.header("accept-encoding", "gzip, br")
			.build()
			.unwrap();
		// Act
		let mut response = middleware
			.process(request, Arc::new(FreshStream(AtomicUsize::new(0))))
			.await
			.unwrap();
		// Assert
		assert!(!response.headers.contains_key("content-encoding"));
		assert!(!response.headers.contains_key("content-length"));
		assert_eq!(
			response
				.take_stream_body()
				.unwrap()
				.next()
				.await
				.unwrap()
				.unwrap(),
			"data: uncached\n\n"
		);
	}
}
