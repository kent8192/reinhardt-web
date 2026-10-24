use super::response_body::{
	ServerResponseBody, into_hyper_response, request_body_too_large_response,
};
use bytes::Bytes;
use hyper::body::Incoming;
use hyper::server::conn::http2;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use reinhardt_http::Handler;
use reinhardt_http::{Request, Response};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

use crate::shutdown::ShutdownCoordinator;

use super::body::{RequestBodyPlan, collect_request_body, request_body_plan_collecting_unsized};
use super::{PreBodyAdmitted, RequestAdmission, RequestHead};

/// HTTP/2 Server
///
/// Note: HTTP/2 connections currently bypass the DI context and middleware
/// pipeline. The handler is invoked directly without middleware composition
/// or dependency injection integration. Full middleware and DI context
/// integration is tracked separately.
pub struct Http2Server {
	handler: Arc<dyn Handler>,
	request_admission: Option<Arc<dyn RequestAdmission>>,
}

impl Http2Server {
	/// Create a new HTTP/2 server with the given handler
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_server::server::Http2Server;
	/// use reinhardt_http::Handler;
	/// use reinhardt_http::{Request, Response};
	///
	/// struct MyHandler;
	///
	/// #[async_trait::async_trait]
	/// impl Handler for MyHandler {
	///     async fn handle(&self, _req: Request) -> reinhardt_core::exception::Result<Response> {
	///         Ok(Response::ok().with_body("Hello from HTTP/2"))
	///     }
	/// }
	///
	/// let server = Http2Server::new(MyHandler);
	/// ```
	pub fn new<H: Handler + 'static>(handler: H) -> Self {
		Self {
			handler: Arc::new(handler),
			request_admission: None,
		}
	}

	/// Check request metadata and acquire an owned resource before reading the body.
	pub fn with_request_admission(mut self, policy: Arc<dyn RequestAdmission>) -> Self {
		self.request_admission = Some(policy);
		self
	}

	/// Start the server and listen on the given address
	///
	/// This method starts the HTTP/2 server and begins accepting connections.
	/// It runs indefinitely until an error occurs.
	///
	/// # Examples
	///
	/// ```no_run
	/// use std::net::SocketAddr;
	/// use reinhardt_server::server::Http2Server;
	/// use reinhardt_http::Handler;
	/// use reinhardt_http::{Request, Response};
	///
	/// struct MyHandler;
	///
	/// #[async_trait::async_trait]
	/// impl Handler for MyHandler {
	///     async fn handle(&self, _req: Request) -> reinhardt_core::exception::Result<Response> {
	///         Ok(Response::ok())
	///     }
	/// }
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let server = Http2Server::new(MyHandler);
	/// let addr: SocketAddr = "127.0.0.1:8080".parse()?;
	/// server.listen(addr).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn listen(self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
		let listener = TcpListener::bind(addr).await?;
		println!("HTTP/2 server listening on http://{}", addr);

		loop {
			let (stream, remote_addr) = listener.accept().await?;
			let handler = self.handler.clone();
			let request_admission = self.request_admission.clone();

			tokio::task::spawn(async move {
				if let Err(err) = Self::handle_connection_admitted(
					stream,
					remote_addr,
					handler,
					request_admission,
				)
				.await
				{
					eprintln!("Error handling HTTP/2 connection: {:?}", err);
				}
			});
		}
	}

	/// Start the server with graceful shutdown support
	///
	/// This method starts the HTTP/2 server and listens for shutdown signals.
	/// When a shutdown signal is received, it stops accepting new connections
	/// and waits for existing connections to complete.
	///
	/// # Examples
	///
	/// ```no_run
	/// use std::net::SocketAddr;
	/// use std::time::Duration;
	/// use reinhardt_server::server::{Http2Server, ShutdownCoordinator};
	/// use reinhardt_http::Handler;
	/// use reinhardt_http::{Request, Response};
	///
	/// struct MyHandler;
	///
	/// #[async_trait::async_trait]
	/// impl Handler for MyHandler {
	///     async fn handle(&self, _req: Request) -> reinhardt_core::exception::Result<Response> {
	///         Ok(Response::ok())
	///     }
	/// }
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let server = Http2Server::new(MyHandler);
	/// let addr: SocketAddr = "127.0.0.1:8080".parse()?;
	/// let coordinator = ShutdownCoordinator::new(Duration::from_secs(30));
	/// server.listen_with_shutdown(addr, coordinator).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn listen_with_shutdown(
		self,
		addr: SocketAddr,
		coordinator: ShutdownCoordinator,
	) -> Result<(), Box<dyn std::error::Error>> {
		let listener = TcpListener::bind(addr).await?;
		println!("HTTP/2 server listening on http://{}", addr);

		let mut shutdown_rx = coordinator.subscribe();

		loop {
			tokio::select! {
				// Accept new connection
				result = listener.accept() => {
					let (stream, remote_addr) = result?;
					let handler = self.handler.clone();
					let request_admission = self.request_admission.clone();
					let mut conn_shutdown = coordinator.subscribe();

					tokio::task::spawn(async move {
						// Handle connection with shutdown support
						tokio::select! {
							result = Self::handle_connection_admitted(stream, remote_addr, handler, request_admission) => {
								if let Err(err) = result {
									eprintln!("Error handling HTTP/2 connection: {:?}", err);
								}
							}
							_ = conn_shutdown.recv() => {
								// Connection interrupted by shutdown
							}
						}
					});
				}
				// Shutdown signal received
				_ = shutdown_rx.recv() => {
					println!("Shutdown signal received, stopping HTTP/2 server...");
					break;
				}
			}
		}

		// Notify that server has stopped accepting connections
		coordinator.notify_shutdown_complete();

		Ok(())
	}

	/// Handle a single TCP connection by processing HTTP/2 requests
	///
	/// This is an internal method used by the server to process individual connections.
	/// Every request on the connection carries the peer address of `stream` as its
	/// `remote_addr`, as with [`HttpServer`](crate::server::HttpServer). The connection
	/// is not served if its peer address cannot be read.
	///
	/// # Examples
	///
	/// ```no_run
	/// use std::sync::Arc;
	/// use std::net::SocketAddr;
	/// use tokio::net::TcpStream;
	/// use reinhardt_server::server::Http2Server;
	/// use reinhardt_http::Handler;
	/// use reinhardt_http::{Request, Response};
	///
	/// struct MyHandler;
	///
	/// #[async_trait::async_trait]
	/// impl Handler for MyHandler {
	///     async fn handle(&self, _req: Request) -> reinhardt_core::exception::Result<Response> {
	///         Ok(Response::ok())
	///     }
	/// }
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let handler = Arc::new(MyHandler);
	/// let addr: SocketAddr = "127.0.0.1:8080".parse()?;
	/// let stream = TcpStream::connect(addr).await?;
	/// Http2Server::handle_connection(stream, handler).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn handle_connection(
		stream: TcpStream,
		handler: Arc<dyn Handler>,
	) -> Result<(), Box<dyn std::error::Error>> {
		let remote_addr = stream.peer_addr()?;
		Self::handle_connection_admitted(stream, remote_addr, handler, None).await
	}

	/// Serve HTTP/2 requests on `stream`, attributing each one to `remote_addr`.
	///
	/// The listen loops pass the address returned by `accept`, so admission and
	/// per-peer controls such as [`RateLimitHandler`](crate::server::RateLimitHandler)
	/// see the same client identity as over HTTP/1.1.
	async fn handle_connection_admitted(
		stream: TcpStream,
		remote_addr: SocketAddr,
		handler: Arc<dyn Handler>,
		request_admission: Option<Arc<dyn RequestAdmission>>,
	) -> Result<(), Box<dyn std::error::Error>> {
		let io = TokioIo::new(stream);
		let service = service_fn(move |req| {
			let handler = handler.clone();
			let request_admission = request_admission.clone();

			handle_request(
				req,
				handler,
				request_admission,
				remote_addr,
				DEFAULT_MAX_BODY_SIZE,
			)
		});

		http2::Builder::new(hyper_util::rt::TokioExecutor::new())
			.serve_connection(io, service)
			.await?;

		Ok(())
	}
}

/// Default maximum request body size (10 MB)
const DEFAULT_MAX_BODY_SIZE: u64 = 10 * 1024 * 1024;
type BoxError = Box<dyn std::error::Error + Send + Sync>;

async fn handle_request(
	req: hyper::Request<Incoming>,
	handler: Arc<dyn Handler>,
	request_admission: Option<Arc<dyn RequestAdmission>>,
	remote_addr: SocketAddr,
	max_body_size: u64,
) -> Result<hyper::Response<ServerResponseBody>, BoxError> {
	// Extract request parts
	let (parts, body) = req.into_parts();
	let is_head = parts.method == hyper::Method::HEAD;
	let admission = match request_admission {
		Some(policy) => match policy
			.admit(RequestHead {
				method: &parts.method,
				uri: &parts.uri,
				headers: &parts.headers,
				remote_addr: Some(remote_addr),
			})
			.await
		{
			Ok(admission) => Some(admission),
			Err(response) => return Ok(into_hyper_response(response)),
		},
		None => None,
	};
	let max_body_size = admission
		.as_ref()
		.map_or(max_body_size, |admission| admission.max_body_size);
	let receive_timeout = admission
		.as_ref()
		.map(|admission| admission.receive_timeout);

	let body_bytes =
		match request_body_plan_collecting_unsized(&parts.method, &parts.headers, max_body_size) {
			RequestBodyPlan::Empty => Bytes::new(),
			RequestBodyPlan::Collect => match async {
				if let Some(receive_timeout) = receive_timeout {
					timeout(receive_timeout, collect_request_body(body, max_body_size))
						.await
						.map_err(|_| ())
				} else {
					Ok(collect_request_body(body, max_body_size).await)
				}
			}
			.await
			{
				Err(()) => {
					return Ok(into_hyper_response(Response::new(
						hyper::StatusCode::REQUEST_TIMEOUT,
					)));
				}
				Ok(Ok(body)) => body,
				Ok(Err(error)) if error.is_too_large() => {
					return Ok(request_body_too_large_response());
				}
				Ok(Err(error)) => return Err(error.into_box_error()),
			},
			RequestBodyPlan::RejectTooLarge => return Ok(request_body_too_large_response()),
		};

	// Create reinhardt Request
	let request = Request::from_hyper_parts(
		parts.method,
		parts.uri,
		parts.version,
		parts.headers,
		body_bytes,
		false,
		Some(remote_addr),
	);
	if admission.is_some() {
		request.extensions.insert(PreBodyAdmitted);
	}

	// Handle request
	let response = handler
		.as_ref()
		.handle(request)
		.await
		.unwrap_or_else(Response::from);

	let mut response = into_hyper_response(response);
	if is_head {
		// HTTP/2 does not suppress a pending body for HEAD automatically. Drop
		// the transferred producer now while preserving representation headers.
		*response.body_mut() =
			ServerResponseBody::Buffered(http_body_util::Full::new(Bytes::new()));
	}
	Ok(response)
}

/// Helper function to create and run an HTTP/2 server
///
/// This is a convenience function that creates an `Http2Server` and starts listening.
///
/// # Examples
///
/// ```no_run
/// use std::net::SocketAddr;
/// use reinhardt_server::server::serve_http2;
/// use reinhardt_http::Handler;
/// use reinhardt_http::{Request, Response};
///
/// struct MyHandler;
///
/// #[async_trait::async_trait]
/// impl Handler for MyHandler {
///     async fn handle(&self, _req: Request) -> reinhardt_core::exception::Result<Response> {
///         Ok(Response::ok().with_body("Hello from HTTP/2!"))
///     }
/// }
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let addr: SocketAddr = "127.0.0.1:3000".parse()?;
/// serve_http2(addr, MyHandler).await?;
/// # Ok(())
/// # }
/// ```
pub async fn serve_http2<H: Handler + 'static>(
	addr: SocketAddr,
	handler: H,
) -> Result<(), Box<dyn std::error::Error>> {
	let server = Http2Server::new(handler);
	server.listen(addr).await
}

/// Helper function to create and run an HTTP/2 server with graceful shutdown
///
/// This function sets up an HTTP/2 server with shutdown signal handling and graceful shutdown support.
///
/// # Examples
///
/// ```no_run
/// use std::net::SocketAddr;
/// use std::time::Duration;
/// use reinhardt_server::server::{serve_http2_with_shutdown, shutdown_signal, ShutdownCoordinator};
/// use reinhardt_http::Handler;
/// use reinhardt_http::{Request, Response};
///
/// struct MyHandler;
///
/// #[async_trait::async_trait]
/// impl Handler for MyHandler {
///     async fn handle(&self, _req: Request) -> reinhardt_core::exception::Result<Response> {
///         Ok(Response::ok().with_body("Hello from HTTP/2!"))
///     }
/// }
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let addr: SocketAddr = "127.0.0.1:3000".parse()?;
/// let coordinator = ShutdownCoordinator::new(Duration::from_secs(30));
///
/// tokio::select! {
///     result = serve_http2_with_shutdown(addr, MyHandler, coordinator.clone()) => {
///         result?;
///     }
///     _ = shutdown_signal() => {
///         coordinator.shutdown();
///         coordinator.wait_for_shutdown().await;
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub async fn serve_http2_with_shutdown<H: Handler + 'static>(
	addr: SocketAddr,
	handler: H,
	coordinator: ShutdownCoordinator,
) -> Result<(), Box<dyn std::error::Error>> {
	let server = Http2Server::new(handler);
	server.listen_with_shutdown(addr, coordinator).await
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::server::{RateLimitSettings, create_rate_limit_handler_from_settings};
	use http_body_util::Empty;
	use hyper::StatusCode;
	use hyper_util::rt::TokioExecutor;
	use rstest::rstest;
	use std::sync::Mutex;
	use std::sync::atomic::{AtomicUsize, Ordering};
	use tokio::task::JoinHandle;

	struct TestHandler;

	#[async_trait::async_trait]
	impl Handler for TestHandler {
		async fn handle(&self, _request: Request) -> reinhardt_core::exception::Result<Response> {
			Ok(Response::ok().with_body("Hello from HTTP/2!"))
		}
	}

	#[tokio::test]
	async fn test_http2_server_creation() {
		let _server = Http2Server::new(TestHandler);
		// Just verify server can be created without panicking
	}

	struct ThreeByteAdmission;

	#[async_trait::async_trait]
	impl RequestAdmission for ThreeByteAdmission {
		async fn admit(
			&self,
			_head: RequestHead<'_>,
		) -> Result<super::super::AdmittedRequest, Response> {
			Ok(super::super::AdmittedRequest::new(
				3,
				std::time::Duration::from_secs(1),
				(),
			))
		}
	}

	struct CountedHandler(Arc<AtomicUsize>);

	#[async_trait::async_trait]
	impl Handler for CountedHandler {
		async fn handle(&self, request: Request) -> reinhardt_core::exception::Result<Response> {
			assert!(request.extensions.get::<PreBodyAdmitted>().is_some());
			self.0.fetch_add(1, Ordering::SeqCst);
			Ok(Response::ok())
		}
	}

	#[tokio::test]
	async fn http2_request_admission_limits_body_before_handler() {
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let address = listener.local_addr().unwrap();
		let handled = Arc::new(AtomicUsize::new(0));
		let server_task = tokio::spawn({
			let handled = handled.clone();
			async move {
				let (stream, remote_addr) = listener.accept().await.unwrap();
				Http2Server::handle_connection_admitted(
					stream,
					remote_addr,
					Arc::new(CountedHandler(handled)),
					Some(Arc::new(ThreeByteAdmission)),
				)
				.await
				.unwrap();
			}
		});
		let client = reqwest::Client::builder()
			.http2_prior_knowledge()
			.build()
			.unwrap();
		let url = format!("http://{address}/upload");
		let rejected = client.post(&url).body("four").send().await.unwrap();
		assert_eq!(rejected.status(), hyper::StatusCode::PAYLOAD_TOO_LARGE);
		assert_eq!(handled.load(Ordering::SeqCst), 0);
		let accepted = client.post(&url).body("yes").send().await.unwrap();
		assert_eq!(accepted.status(), hyper::StatusCode::OK);
		assert_eq!(handled.load(Ordering::SeqCst), 1);
		server_task.abort();
		let _ = server_task.await;
	}

	/// Records the `remote_addr` of every request it handles.
	struct PeerRecordingHandler(Arc<Mutex<Vec<Option<SocketAddr>>>>);

	#[async_trait::async_trait]
	impl Handler for PeerRecordingHandler {
		async fn handle(&self, request: Request) -> reinhardt_core::exception::Result<Response> {
			self.0.lock().unwrap().push(request.remote_addr);
			Ok(Response::ok())
		}
	}

	/// How the test server hands an accepted connection to `Http2Server`.
	#[derive(Clone, Copy, Debug)]
	enum Entry {
		/// As the `listen` loops do: with the address returned by `accept`.
		AcceptedAddress,
		/// Through the public `handle_connection`, which reads the stream's peer.
		HandleConnection,
	}

	/// Aborts the accept loop when dropped, including when an assertion fails.
	struct ServerTask(JoinHandle<()>);

	impl Drop for ServerTask {
		fn drop(&mut self) {
			self.0.abort();
		}
	}

	/// Serves every connection accepted on an ephemeral loopback port.
	async fn spawn_server(entry: Entry, handler: Arc<dyn Handler>) -> (SocketAddr, ServerTask) {
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let address = listener.local_addr().unwrap();
		let task = tokio::spawn(async move {
			loop {
				let (stream, accepted) = listener.accept().await.unwrap();
				let handler = handler.clone();
				tokio::spawn(async move {
					let _ = match entry {
						Entry::AcceptedAddress => {
							Http2Server::handle_connection_admitted(stream, accepted, handler, None)
								.await
						}
						Entry::HandleConnection => {
							Http2Server::handle_connection(stream, handler).await
						}
					};
				});
			}
		});
		(address, ServerTask(task))
	}

	/// Sends `GET /` over a new HTTP/2 connection whose client-IP headers claim
	/// `claimed_ip`, returning the client's local address and the response status.
	async fn get_over_new_connection(
		server: SocketAddr,
		claimed_ip: &str,
	) -> (SocketAddr, StatusCode) {
		let stream = TcpStream::connect(server).await.unwrap();
		let local_addr = stream.local_addr().unwrap();
		let (mut sender, connection) =
			hyper::client::conn::http2::handshake(TokioExecutor::new(), TokioIo::new(stream))
				.await
				.unwrap();
		let connection = tokio::spawn(connection);
		let request = hyper::Request::builder()
			.uri(format!("http://{server}/"))
			.header("x-forwarded-for", claimed_ip)
			.header("x-real-ip", claimed_ip)
			.body(Empty::<Bytes>::new())
			.unwrap();
		let status = sender.send_request(request).await.unwrap().status();
		connection.abort();
		(local_addr, status)
	}

	#[rstest]
	#[case::accepted_address(Entry::AcceptedAddress)]
	#[case::handle_connection(Entry::HandleConnection)]
	#[tokio::test]
	async fn request_remote_addr_is_the_connection_peer(#[case] entry: Entry) {
		// Arrange
		let seen = Arc::new(Mutex::new(Vec::new()));
		let (server, _task) =
			spawn_server(entry, Arc::new(PeerRecordingHandler(seen.clone()))).await;

		// Act
		let (first_client, first_status) = get_over_new_connection(server, "203.0.113.7").await;
		let (second_client, second_status) = get_over_new_connection(server, "203.0.113.7").await;

		// Assert
		assert_eq!(first_status, StatusCode::OK);
		assert_eq!(second_status, StatusCode::OK);
		// Each direct client keeps its own socket identity, and the claimed
		// client-IP headers do not replace it.
		assert_ne!(first_client, second_client);
		assert_eq!(
			*seen.lock().unwrap(),
			vec![Some(first_client), Some(second_client)]
		);
	}

	#[rstest]
	#[tokio::test]
	async fn per_ip_rate_limit_keys_on_the_connection_peer() {
		// Arrange
		// The loopback peer is a trusted proxy, so each claimed client IP gets its
		// own bucket. Without the peer address every request would fall back to
		// one shared bucket and the second client would be rejected.
		let settings = RateLimitSettings {
			max_requests: 1,
			trusted_proxies: vec!["127.0.0.1".to_string()],
			..RateLimitSettings::default()
		};
		let limiter = create_rate_limit_handler_from_settings(Arc::new(TestHandler), &settings);
		let (server, _task) = spawn_server(Entry::AcceptedAddress, Arc::new(limiter)).await;

		// Act
		let (_, first) = get_over_new_connection(server, "198.51.100.1").await;
		let (_, other_client) = get_over_new_connection(server, "198.51.100.2").await;
		let (_, repeated) = get_over_new_connection(server, "198.51.100.1").await;

		// Assert
		assert_eq!(first, StatusCode::OK);
		assert_eq!(other_client, StatusCode::OK);
		assert_eq!(repeated, StatusCode::TOO_MANY_REQUESTS);
	}
}
