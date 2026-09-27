//! HTTP fixture shutdown must also cancel accepted connections and their resources.

use std::io::ErrorKind;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use futures::{StreamExt, stream};
use reinhardt_http::{Handler, Request, Response, StreamingResponse};
use reinhardt_test::fixtures::api_client_from_url;
use reinhardt_test::fixtures::server::{TestServer, TestServerGuard, test_server_guard};
use reinhardt_test::{ClientError, TestResponse};
use reinhardt_urls::routers::ServerRouter;
use rstest::{fixture, rstest};
use tokio::net::TcpStream;
use tokio::sync::Notify;
use tokio::time::timeout;

const TEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Copy)]
enum ServerKind {
	Guard,
	Builder,
}

enum RunningServer {
	Guard(TestServerGuard),
	Builder(TestServer),
}

impl RunningServer {
	fn url(&self) -> &str {
		match self {
			Self::Guard(server) => &server.url,
			Self::Builder(server) => &server.url,
		}
	}

	fn shutdown(&self) {
		match self {
			Self::Guard(server) => server.coordinator.shutdown(),
			Self::Builder(server) => server.coordinator.shutdown(),
		}
	}
}

impl ServerKind {
	async fn start(self, handler: impl Handler + 'static) -> RunningServer {
		let router = ServerRouter::new().handler("/probe", handler);
		match self {
			Self::Guard => RunningServer::Guard(test_server_guard(router).await),
			Self::Builder => RunningServer::Builder(
				TestServer::builder()
					.handler(Arc::new(router))
					.build()
					.await
					.expect("HTTP fixture should start"),
			),
		}
	}
}

struct CountingHandler(Arc<AtomicUsize>);

#[async_trait]
impl Handler for CountingHandler {
	async fn handle(&self, request: Request) -> reinhardt_http::Result<Response> {
		self.0.fetch_add(1, Ordering::SeqCst);
		// The peer address proves that successive requests use the same connection.
		Ok(Response::ok().with_body(request.remote_addr.unwrap().to_string()))
	}
}

#[fixture]
fn request_count() -> Arc<AtomicUsize> {
	Arc::new(AtomicUsize::new(0))
}

async fn wait_for_listener_closed(url: &str) {
	let address = url.strip_prefix("http://").unwrap();
	let error = timeout(TEST_TIMEOUT, async {
		loop {
			match TcpStream::connect(address).await {
				// Each successful probe is closed immediately; only listener liveness matters.
				Ok(stream) => drop(stream),
				// Shutdown can reset a probe that raced with the final accept.
				Err(error) if error.kind() == ErrorKind::ConnectionReset => {}
				Err(error) => break error,
			}
			tokio::task::yield_now().await;
		}
	})
	.await
	.expect("fixture should close its listener");
	assert_eq!(error.kind(), ErrorKind::ConnectionRefused);
}

fn assert_connection_closed(result: Result<TestResponse, ClientError>) {
	match result {
		Err(error) => assert!(!error.is_timeout(), "shutdown must close the connection"),
		Ok(response) => panic!(
			"request must fail after fixture shutdown, received status {}",
			response.status_code()
		),
	}
}

#[rstest]
#[case::guard(ServerKind::Guard)]
#[case::builder(ServerKind::Builder)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropping_fixture_closes_pooled_connections(
	#[case] kind: ServerKind,
	request_count: Arc<AtomicUsize>,
) {
	// Arrange
	let server = kind
		.start(CountingHandler(Arc::clone(&request_count)))
		.await;
	let url = server.url().to_owned();
	let clients = [api_client_from_url(&url), api_client_from_url(&url)];
	let mut peers = Vec::new();
	for client in &clients {
		let first = client.get("/probe").await.unwrap();
		let second = client.get("/probe").await.unwrap();
		assert_eq!(first.status_code(), 200);
		assert_eq!(second.status_code(), 200);
		assert_eq!(first.text(), second.text());
		peers.push(first.text().to_owned());
	}
	assert_ne!(peers[0], peers[1]);
	assert_eq!(request_count.load(Ordering::SeqCst), 4);

	// Act
	drop(server);
	wait_for_listener_closed(&url).await;

	// Assert
	for client in &clients {
		let response = timeout(TEST_TIMEOUT, client.get("/probe"))
			.await
			.expect("request should fail promptly after fixture shutdown");
		assert_connection_closed(response);
	}
	assert_eq!(request_count.load(Ordering::SeqCst), 4);

	let successor_count = Arc::new(AtomicUsize::new(0));
	let successor = kind
		.start(CountingHandler(Arc::clone(&successor_count)))
		.await;
	let response = api_client_from_url(successor.url())
		.get("/probe")
		.await
		.unwrap();
	assert_eq!(response.status_code(), 200);
	assert_eq!(successor_count.load(Ordering::SeqCst), 1);
	assert_eq!(request_count.load(Ordering::SeqCst), 4);
}

#[derive(Default)]
struct ProducerLifecycle {
	started: Notify,
	dropped: Notify,
}

struct ProducerGuard(Arc<ProducerLifecycle>);

impl Drop for ProducerGuard {
	fn drop(&mut self) {
		self.0.dropped.notify_one();
	}
}

struct PendingProducerHandler(Arc<ProducerLifecycle>);

#[async_trait]
impl Handler for PendingProducerHandler {
	async fn handle(&self, _request: Request) -> reinhardt_http::Result<Response> {
		let producer = ProducerGuard(Arc::clone(&self.0));
		let stream = stream::unfold(producer, |producer| async move {
			producer.0.started.notify_one();
			std::future::pending::<()>().await;
			Some((
				Ok::<_, Box<dyn std::error::Error + Send + Sync>>(Bytes::new()),
				producer,
			))
		});
		let response = StreamingResponse::new(stream);
		// Buffered HTTP handlers await their producer before returning a response.
		// The connection must own this pending stream through the handler future.
		let stream = response.into_stream();
		tokio::pin!(stream);
		let chunk = stream.next().await.unwrap().unwrap();
		Ok(Response::ok().with_body(chunk))
	}
}

#[fixture]
fn producer_lifecycle() -> Arc<ProducerLifecycle> {
	Arc::new(ProducerLifecycle::default())
}

#[rstest]
#[case::guard_drop(ServerKind::Guard, true)]
#[case::guard_signal(ServerKind::Guard, false)]
#[case::builder_drop(ServerKind::Builder, true)]
#[case::builder_signal(ServerKind::Builder, false)]
#[tokio::test]
async fn fixture_shutdown_drops_pending_producers(
	#[case] kind: ServerKind,
	#[case] drop_guard: bool,
	producer_lifecycle: Arc<ProducerLifecycle>,
) {
	// Arrange
	let server = kind
		.start(PendingProducerHandler(Arc::clone(&producer_lifecycle)))
		.await;
	let url = server.url().to_owned();
	let client = api_client_from_url(&url);
	let request = client.get("/probe");
	tokio::pin!(request);
	timeout(TEST_TIMEOUT, async {
		tokio::select! {
			_ = producer_lifecycle.started.notified() => {}
			_ = &mut request => panic!("producer must remain pending until fixture shutdown"),
		}
	})
	.await
	.expect("request should start polling the streaming producer");

	// Act
	// Keep the client and request alive: only fixture shutdown may release the producer.
	let _remaining_server = if drop_guard {
		drop(server);
		None
	} else {
		server.shutdown();
		Some(server)
	};

	// Assert
	timeout(TEST_TIMEOUT, producer_lifecycle.dropped.notified())
		.await
		.expect("fixture shutdown must drop the pending producer owned by the handler");
	wait_for_listener_closed(&url).await;
	let response = timeout(TEST_TIMEOUT, request)
		.await
		.expect("cancelled request should finish promptly");
	assert_connection_closed(response);
}
