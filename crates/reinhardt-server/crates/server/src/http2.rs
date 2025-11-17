use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http2;
use hyper::service::Service;
use hyper_util::rt::TokioIo;
use reinhardt_core::http::{Request, Response};
use reinhardt_core::types::Handler;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

use crate::shutdown::ShutdownCoordinator;

/// HTTP/2 Server
pub struct Http2Server {
	handler: Arc<dyn Handler>,
}

impl Http2Server {
	/// Create a new HTTP/2 server with the given handler
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_server_core::Http2Server;
	/// use reinhardt_core::types::Handler;
	/// use reinhardt_core::http::{Request, Response};
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
	/// use reinhardt_server_core::Http2Server;
	/// use reinhardt_core::types::Handler;
	/// use reinhardt_core::http::{Request, Response};
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
	/// use reinhardt_server_core::{Http2Server, ShutdownCoordinator};
	/// use reinhardt_core::types::Handler;
	/// use reinhardt_core::http::{Request, Response};
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
	/// use reinhardt_server_core::Http2Server;
	/// use reinhardt_core::types::Handler;
	/// use reinhardt_core::http::{Request, Response};
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
		let service = RequestService { handler };

		http2::Builder::new(hyper_util::rt::TokioExecutor::new())
			.serve_connection(io, service)
			.await?;

		Ok(())
	}
}

/// Service implementation for hyper
struct RequestService {
	handler: Arc<dyn Handler>,
}

impl Service<hyper::Request<Incoming>> for RequestService {
	type Response = hyper::Response<Full<Bytes>>;
	type Error = Box<dyn std::error::Error + Send + Sync>;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn call(&self, req: hyper::Request<Incoming>) -> Self::Future {
		let handler = self.handler.clone();

		Box::pin(async move {
			// Extract request parts
			let (parts, body) = req.into_parts();

			// Read body
			let body_bytes = body.collect().await?.to_bytes();

			// Create reinhardt Request
			let request = Request::builder()
				.method(parts.method)
				.uri(parts.uri)
				.version(parts.version)
				.headers(parts.headers)
				.body(body_bytes)
				.build()
				.expect("Failed to build request");

			// Handle request
			let response = handler
				.handle(request)
				.await
				.unwrap_or_else(|_| Response::internal_server_error());

			// Convert to hyper response
			let mut hyper_response = hyper::Response::builder().status(response.status);

			// Add headers
			for (key, value) in response.headers.iter() {
				hyper_response = hyper_response.header(key, value);
			}

			Ok(hyper_response.body(Full::new(response.body))?)
		})
	}
}

/// Helper function to create and run an HTTP/2 server
///
/// This is a convenience function that creates an `Http2Server` and starts listening.
///
/// # Examples
///
/// ```no_run
/// use std::net::SocketAddr;
/// use reinhardt_server_core::serve_http2;
/// use reinhardt_core::types::Handler;
/// use reinhardt_core::http::{Request, Response};
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
/// use reinhardt_server_core::{serve_http2_with_shutdown, shutdown_signal, ShutdownCoordinator};
/// use reinhardt_core::types::Handler;
/// use reinhardt_core::http::{Request, Response};
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
