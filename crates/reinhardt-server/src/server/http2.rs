use bytes::Bytes;
use http_body_util::Full;
use hyper::StatusCode;
use hyper::body::Incoming;
use hyper::server::conn::http2;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use reinhardt_http::Handler;
use reinhardt_http::{Request, Response};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

use crate::shutdown::ShutdownCoordinator;

use super::body::{RequestBodyPlan, collect_request_body, request_body_plan_collecting_unsized};

/// HTTP/2 Server
///
/// Note: HTTP/2 connections currently bypass the DI context and middleware
/// pipeline. The handler is invoked directly without middleware composition
/// or dependency injection integration. Full middleware and DI context
/// integration is tracked separately.
pub struct Http2Server {
	handler: Arc<dyn Handler>,
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
		}
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
			let (stream, _) = listener.accept().await?;
			let handler = self.handler.clone();

			tokio::task::spawn(async move {
				if let Err(err) = Self::handle_connection(stream, handler).await {
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
					let (stream, _) = result?;
					let handler = self.handler.clone();
					let mut conn_shutdown = coordinator.subscribe();

					tokio::task::spawn(async move {
						// Handle connection with shutdown support
						tokio::select! {
							result = Self::handle_connection(stream, handler) => {
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
		let io = TokioIo::new(stream);
		let service = service_fn(move |req| {
			let handler = handler.clone();

			handle_request(req, handler, DEFAULT_MAX_BODY_SIZE)
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
	max_body_size: u64,
) -> Result<hyper::Response<Full<Bytes>>, BoxError> {
	// Extract request parts
	let (parts, body) = req.into_parts();

	let body_bytes =
		match request_body_plan_collecting_unsized(&parts.method, &parts.headers, max_body_size) {
			RequestBodyPlan::Empty => Bytes::new(),
			RequestBodyPlan::Collect => match collect_request_body(body, max_body_size).await {
				Ok(body) => body,
				Err(error) if error.is_too_large() => return Ok(request_body_too_large_response()),
				Err(error) => return Err(error.into_box_error()),
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
		None,
	);

	// Handle request
	let response = handler
		.as_ref()
		.handle(request)
		.await
		.unwrap_or_else(Response::from);

	Ok(into_hyper_response(response))
}

fn into_hyper_response(response: Response) -> hyper::Response<Full<Bytes>> {
	let status = response.status;
	let headers = response.headers;
	let mut hyper_response = hyper::Response::new(Full::new(response.body));
	if status != StatusCode::OK {
		*hyper_response.status_mut() = status;
	}
	if !headers.is_empty() {
		*hyper_response.headers_mut() = headers;
	}
	hyper_response
}

fn request_body_too_large_response() -> hyper::Response<Full<Bytes>> {
	hyper::Response::builder()
		.status(StatusCode::PAYLOAD_TOO_LARGE)
		.body(Full::new(Bytes::from_static(b"Request body too large")))
		.expect("Failed to build 413 response")
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
}
