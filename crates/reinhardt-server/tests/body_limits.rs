//! Per-instance limits must reach the actual HTTP listeners, not just handlers.
use reinhardt_http::Handler;
use reinhardt_server::{Http2Server, HttpServer, Request, Response, ShutdownCoordinator};
use rstest::rstest;
use std::{
	sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	},
	time::Duration,
};
use tokio::task::JoinHandle;

struct BodyLength(Arc<AtomicUsize>);
#[async_trait::async_trait]
impl Handler for BodyLength {
	async fn handle(&self, request: Request) -> reinhardt_http::Result<Response> {
		self.0.fetch_add(1, Ordering::SeqCst);
		Ok(Response::ok().with_body(request.body().len().to_string()))
	}
}

#[derive(Clone, Copy, Debug)]
enum Protocol {
	Http1,
	Http2,
}
struct ServerTask {
	task: JoinHandle<()>,
	coordinator: ShutdownCoordinator,
}
impl Drop for ServerTask {
	fn drop(&mut self) {
		self.coordinator.shutdown();
		self.task.abort();
	}
}

async fn listener(
	protocol: Protocol,
	shutdown: bool,
	limit: Option<u64>,
	handled: Arc<AtomicUsize>,
) -> (String, reqwest::Client, ServerTask) {
	// The public address-based APIs own binding. Reserve an ephemeral address,
	// release it immediately before spawn, and wait for its actual listener.
	let reservation = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
	let address = reservation.local_addr().unwrap();
	let coordinator = ShutdownCoordinator::new(Duration::from_secs(1));
	let serving = coordinator.clone();
	drop(reservation);
	let task = tokio::spawn(async move {
		match protocol {
			Protocol::Http1 => {
				let mut server = HttpServer::new(BodyLength(handled));
				if let Some(limit) = limit {
					server = server.with_max_body_size(limit);
				}
				if shutdown {
					server.listen_with_shutdown(address, serving).await.unwrap();
				} else {
					server.listen(address).await.unwrap();
				}
			}
			Protocol::Http2 => {
				let mut server = Http2Server::new(BodyLength(handled));
				if let Some(limit) = limit {
					server = server.with_max_body_size(limit);
				}
				if shutdown {
					server.listen_with_shutdown(address, serving).await.unwrap();
				} else {
					server.listen(address).await.unwrap();
				}
			}
		}
	});
	let guard = ServerTask { task, coordinator };
	let mut client = reqwest::Client::builder().timeout(Duration::from_secs(10));
	client = match protocol {
		Protocol::Http1 => client.http1_only(),
		Protocol::Http2 => client.http2_prior_knowledge(),
	};
	let client = client.build().unwrap();
	let endpoint = format!("http://{address}/upload");
	for _ in 0..100 {
		if client.get(&endpoint).send().await.is_ok() {
			return (endpoint, client, guard);
		}
		assert!(
			!guard.task.is_finished(),
			"server terminated before binding"
		);
		tokio::time::sleep(Duration::from_millis(10)).await;
	}
	panic!("listener failed to start");
}

async fn oversized_headers_status(
	protocol: Protocol,
	endpoint: &str,
	client: &reqwest::Client,
	size: usize,
) -> u16 {
	match protocol {
		Protocol::Http1 => {
			use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
			let authority = endpoint
				.strip_prefix("http://")
				.unwrap()
				.split('/')
				.next()
				.unwrap();
			let mut stream = tokio::net::TcpStream::connect(authority).await.unwrap();
			// Read the early 413 before sending a large, already-rejected body.
			// Otherwise a racing writer can see a TCP reset instead of the response.
			stream.write_all(format!("POST /upload HTTP/1.1\r\nHost: {authority}\r\nContent-Length: {size}\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
			let mut status = String::new();
			tokio::time::timeout(
				Duration::from_secs(10),
				BufReader::new(stream).read_line(&mut status),
			)
			.await
			.unwrap()
			.unwrap();
			status.split_whitespace().nth(1).unwrap().parse().unwrap()
		}
		Protocol::Http2 => client
			.post(endpoint)
			.body(vec![b'x'; size])
			.send()
			.await
			.unwrap()
			.status()
			.as_u16(),
	}
}

// Removing propagation to either ordinary or shutdown listeners must fail here.
#[rstest]
#[case::http1(Protocol::Http1, false, 1024)]
#[case::http1_shutdown(Protocol::Http1, true, 1024)]
#[case::http2(Protocol::Http2, false, 1024)]
#[case::http2_shutdown(Protocol::Http2, true, 1024)]
#[case::http1_empty_only(Protocol::Http1, true, 0)]
#[case::http2_empty_only(Protocol::Http2, true, 0)]
#[tokio::test]
async fn configured_body_budget_reaches_each_listener(
	#[case] protocol: Protocol,
	#[case] shutdown: bool,
	#[case] limit: usize,
) {
	// Arrange
	let handled = Arc::new(AtomicUsize::new(0));
	let (endpoint, client, _server) =
		listener(protocol, shutdown, Some(limit as u64), handled.clone()).await;
	let before = handled.load(Ordering::SeqCst);

	// Act
	let accepted = client
		.post(&endpoint)
		.body(vec![b'x'; limit])
		.send()
		.await
		.unwrap();
	let rejected = client
		.post(&endpoint)
		.body(vec![b'x'; limit + 1])
		.send()
		.await
		.unwrap();

	let stream = futures_util::stream::iter([Ok::<_, std::io::Error>(vec![b'x'; limit + 1])]);
	let streamed = client
		.post(&endpoint)
		.body(reqwest::Body::wrap_stream(stream))
		.send()
		.await
		.unwrap();

	// Assert
	assert_eq!(accepted.status(), 200);
	assert_eq!(accepted.text().await.unwrap(), limit.to_string());
	assert_eq!(
		streamed.status(),
		413,
		"unknown-length bodies obey the same budget"
	);
	assert_eq!(rejected.status(), 413);
	assert_eq!(
		handled.load(Ordering::SeqCst),
		before + 1,
		"oversized bodies must not reach the handler"
	);
}

#[rstest]
#[case::http1_default(Protocol::Http1, None, 10 * 1024 * 1024)]
#[case::http2_default(Protocol::Http2, None, 10 * 1024 * 1024)]
#[case::http1_larger(Protocol::Http1, Some(12 * 1024 * 1024), 12 * 1024 * 1024)]
#[case::http2_larger(Protocol::Http2, Some(12 * 1024 * 1024), 12 * 1024 * 1024)]
#[tokio::test]
async fn default_budget_is_preserved_and_can_be_increased(
	#[case] protocol: Protocol,
	#[case] configured: Option<u64>,
	#[case] limit: usize,
) {
	// Arrange
	let handled = Arc::new(AtomicUsize::new(0));
	let (endpoint, client, _server) = listener(protocol, true, configured, handled.clone()).await;
	let before = handled.load(Ordering::SeqCst);

	// Act
	let accepted = client
		.post(&endpoint)
		.body(vec![b'x'; limit])
		.send()
		.await
		.unwrap();
	let rejected = oversized_headers_status(protocol, &endpoint, &client, limit + 1).await;

	// Assert
	assert_eq!(accepted.status(), 200);
	assert_eq!(accepted.text().await.unwrap(), limit.to_string());
	assert_eq!(rejected, 413);
	assert_eq!(handled.load(Ordering::SeqCst), before + 1);
}
