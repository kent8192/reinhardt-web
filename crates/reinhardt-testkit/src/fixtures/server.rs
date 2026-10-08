//! Server test fixtures with automatic shutdown.
//!
//! This module provides rstest fixtures for testing HTTP servers with automatic
//! cleanup via RAII pattern.

// `RateLimitConfig` is deprecated in favor of the `RateLimitSettings` fragment;
// these fixtures still build it directly during the 0.2 compatibility window.
#![allow(deprecated)]

use reinhardt_di::InjectionContext;
use reinhardt_http::Handler;
use reinhardt_http::{Request, Response};
use reinhardt_server::{
	HttpServer, RateLimitConfig, RateLimitHandler, ShutdownCoordinator, TimeoutHandler,
};
use reinhardt_urls::routers::ServerRouter as Router;
use rstest::fixture;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::task::{JoinHandle, JoinSet};

#[cfg(feature = "websockets")]
use reinhardt_server::WebSocketServer;

#[cfg(feature = "graphql")]
use reinhardt_server::GraphQLHandler;

/// Test server guard with automatic connection cleanup.
///
/// Dropping the guard cancels the listener and all accepted HTTP connections,
/// including pending handlers and the resources they own, even if the test panics.
/// Cancellation completes asynchronously as the Tokio runtime polls the aborted
/// tasks. In-flight requests are cancelled instead of being drained to completion.
///
/// # Examples
///
/// ```
/// use reinhardt_testkit::fixtures::*;
/// use reinhardt_urls::routers::ServerRouter as Router;
///
/// # #[tokio::main]
/// # async fn main() {
///     let router = Router::new().handler("/test", BasicHandler);
///     let server = test_server_guard(router).await;
///     let response = reqwest::get(&format!("{}/test", server.url))
///         .await
///         .unwrap();
///     assert_eq!(response.status(), 200);
///     // Automatic connection cleanup when server goes out of scope
/// # }
/// ```
pub struct TestServerGuard {
	/// Server URL (e.g., "http://127.0.0.1:12345")
	pub url: String,
	/// Shutdown coordinator for graceful shutdown
	pub coordinator: Arc<ShutdownCoordinator>,
	/// Server task handle
	server_task: Option<JoinHandle<()>>,
}

impl TestServerGuard {
	/// Create a new test server guard.
	///
	/// This function:
	/// 1. Binds to a random port (127.0.0.1:0)
	/// 2. Creates a ShutdownCoordinator
	/// 3. Spawns the server task
	/// 4. Probes the server port until it accepts connections
	///
	/// # Arguments
	///
	/// * `router` - Router to use for handling requests
	async fn new(router: Router) -> Self {
		let shutdown_timeout = Duration::from_secs(5);
		// Bind to random port and keep the listener to avoid TOCTOU race
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let actual_addr = listener.local_addr().unwrap();
		let url = format!("http://{}", actual_addr);

		// Create shutdown coordinator
		let coordinator = Arc::new(ShutdownCoordinator::new(shutdown_timeout));

		// Spawn server using the already-bound listener to avoid port race
		let server_coordinator = (*coordinator).clone();
		let handler: Arc<dyn Handler> = Arc::new(router);
		let server = HttpServer::new(handler);
		let mut shutdown_rx = server_coordinator.subscribe();
		let server_task = tokio::spawn(async move {
			// Dropping the accept loop must also cancel every accepted connection.
			let mut connections = JoinSet::new();
			loop {
				tokio::select! {
					result = listener.accept() => {
						match result {
							Ok((stream, socket_addr)) => {
								let handler_clone = server.handler();
								connections.spawn(async move {
									if let Err(e) =
										HttpServer::handle_connection(stream, socket_addr, handler_clone, None)
											.await
									{
										eprintln!("Error handling connection: {:?}", e);
									}
								});
							}
							Err(e) => {
								eprintln!("Error accepting connection: {:?}", e);
								break;
							}
						}
					}
					_ = shutdown_rx.recv() => {
						break;
					}
					// Reap completed tasks so long-lived fixtures do not retain them.
					_ = connections.join_next(), if !connections.is_empty() => {}
				}
			}
		});

		// Probe server readiness with TCP connect attempts instead of fixed sleep.
		// This avoids flaky failures when the system is under heavy load.
		wait_for_server_ready(actual_addr)
			.await
			.expect("Test server failed to become ready");

		Self {
			url,
			coordinator,
			server_task: Some(server_task),
		}
	}
}

impl Drop for TestServerGuard {
	fn drop(&mut self) {
		// Trigger shutdown signal
		self.coordinator.shutdown();

		// Aborting the accept loop drops its JoinSet and cancels active connections.
		if let Some(task) = self.server_task.take() {
			task.abort();
		}
	}
}

/// Create a test server fixture with automatic connection cleanup.
///
/// By default, the fixture uses an empty [`Router`]. Override the router with
/// rstest's `#[with(...)]` attribute, or pass it directly when calling the function.
/// A downstream fixture named `router` is not resolved implicitly: rstest resolves
/// fixture dependencies at the definition site.
/// See [`TestServerGuard`] for the cancellation behavior on drop.
///
/// # Examples
///
/// Call the function directly with a custom router:
///
/// ```no_run
/// use reinhardt_testkit::fixtures::*;
/// use reinhardt_urls::routers::ServerRouter as Router;
///
/// #[tokio::test]
/// async fn test_server() {
///     let router = Router::new().handler("/hello", BasicHandler);
///     let server = test_server_guard(router).await;
///     let response = reqwest::get(&format!("{}/hello", server.url))
///         .await
///         .unwrap();
///     assert_eq!(response.status(), 200);
///     // Automatic cleanup on drop
/// }
/// ```
///
/// Use the default fixture or override its router with `#[with(...)]`:
///
/// ```no_run
/// use reinhardt_testkit::fixtures::{test_server_guard, BasicHandler, TestServerGuard};
/// use reinhardt_urls::routers::ServerRouter;
/// use rstest::*;
///
/// fn my_router() -> ServerRouter {
///     ServerRouter::new().handler("/hello", BasicHandler)
/// }
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_default_server(#[future] test_server_guard: TestServerGuard) {
///     let server = test_server_guard.await;
///     let response = reqwest::get(format!("{}/unregistered", server.url)).await.unwrap();
///     assert_eq!(response.status(), 404);
/// }
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_custom_server(
///     #[future]
///     #[from(test_server_guard)]
///     #[with(my_router())]
///     server: TestServerGuard,
/// ) {
///     let server = server.await;
///     let response = reqwest::get(format!("{}/hello", server.url)).await.unwrap();
///     assert_eq!(response.status(), 200);
///     assert_eq!(response.text().await.unwrap(), "OK");
/// }
/// ```
#[fixture]
pub async fn test_server_guard(#[default(Router::new())] router: Router) -> TestServerGuard {
	TestServerGuard::new(router).await
}

// ============================================================================
// Basic Test Handlers
// ============================================================================

/// Basic handler for testing purposes that returns "OK"
#[derive(Clone)]
pub struct BasicHandler;

#[async_trait::async_trait]
impl Handler for BasicHandler {
	async fn handle(&self, _request: Request) -> reinhardt_core::exception::Result<Response> {
		Ok(Response::ok().with_body("OK"))
	}
}

// ============================================================================
// Client Fixtures
// ============================================================================

/// HTTP client fixture for testing HTTP requests
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::*;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_with_client(http_client: reqwest::Client) {
///     let response = http_client
///         .get("http://localhost:8080/api/test")
///         .send()
///         .await
///         .unwrap();
///     assert_eq!(response.status(), 200);
/// }
/// ```
#[fixture]
pub fn http_client() -> reqwest::Client {
	reqwest::Client::builder()
		.timeout(Duration::from_secs(10))
		.build()
		.expect("Failed to create HTTP client")
}
// ============================================================================
// HTTP/1.1 Server Fixtures
// ============================================================================

/// HTTP/1.1 test server fixture
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::*;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_http1_server(#[future] http1_server: TestServer) {
///     let server = http1_server.await;
///     let client = reqwest::Client::new();
///     let response = client.get(&server.url).send().await.unwrap();
///     assert_eq!(response.status(), 200);
/// }
/// ```
#[fixture]
pub async fn http1_server() -> TestServer {
	let handler = Arc::new(BasicHandler);
	TestServer::builder()
		.handler(handler)
		.build()
		.await
		.expect("Failed to create HTTP/1.1 server")
}

// ============================================================================
// HTTP/2 Server Fixtures
// ============================================================================

/// HTTP/2 test server fixture
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::*;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_http2_server(#[future] http2_server: TestServer) {
///     let server = http2_server.await;
///     // Test with HTTP/2 client
/// }
/// ```
#[fixture]
pub async fn http2_server() -> TestServer {
	let handler = Arc::new(BasicHandler);
	TestServer::builder()
		.handler(handler)
		.http2(true)
		.build()
		.await
		.expect("Failed to create HTTP/2 server")
}

// ============================================================================
// Middleware Server Fixtures
// ============================================================================

/// Server fixture with timeout middleware
///
/// Default timeout: 5 seconds
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::*;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_timeout(#[future] server_with_timeout: TestServer) {
///     let server = server_with_timeout.await;
///     // Timeout test
/// }
/// ```
#[fixture]
pub async fn server_with_timeout(
	#[default(Duration::from_secs(5))] timeout: Duration,
) -> TestServer {
	let handler = Arc::new(BasicHandler);
	let timeout_handler = Arc::new(TimeoutHandler::new(handler, timeout));
	TestServer::builder()
		.handler(timeout_handler)
		.build()
		.await
		.expect("Failed to create server with timeout")
}

/// Server fixture with rate limit middleware
///
/// Default rate limit: 100 requests/minute
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::*;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_rate_limit(#[future] server_with_rate_limit: TestServer) {
///     let server = server_with_rate_limit.await;
///     // Rate limit test
/// }
/// ```
#[fixture]
pub async fn server_with_rate_limit(#[default(100)] limit: u32) -> TestServer {
	let handler = Arc::new(BasicHandler);
	let config = RateLimitConfig::per_minute(limit as usize);
	let rate_limit_handler = Arc::new(RateLimitHandler::new(handler, config));
	TestServer::builder()
		.handler(rate_limit_handler)
		.build()
		.await
		.expect("Failed to create server with rate limit")
}

/// Server fixture with middleware chain (Timeout + RateLimit)
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::*;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_middleware_chain(#[future] server_with_middleware_chain: TestServer) {
///     let server = server_with_middleware_chain.await;
///     // Middleware chain test
/// }
/// ```
#[fixture]
pub async fn server_with_middleware_chain() -> TestServer {
	let handler = Arc::new(BasicHandler);
	let timeout_handler = Arc::new(TimeoutHandler::new(handler, Duration::from_secs(5)));
	let config = RateLimitConfig::per_minute(100);
	let rate_limit_handler = Arc::new(RateLimitHandler::new(timeout_handler, config));

	TestServer::builder()
		.handler(rate_limit_handler)
		.build()
		.await
		.expect("Failed to create server with middleware chain")
}

/// Server fixture with DI context
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::*;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_di_context(#[future] server_with_di: (TestServer, Arc<InjectionContext>)) {
///     let (server, di_context) = server_with_di.await;
///     // DI context test
/// }
/// ```
#[fixture]
pub async fn server_with_di() -> (TestServer, Arc<InjectionContext>) {
	use reinhardt_di::SingletonScope;

	let handler = Arc::new(BasicHandler);
	let di_context = Arc::new(InjectionContext::builder(Arc::new(SingletonScope::new())).build());

	let server = TestServer::builder()
		.handler(handler)
		.di_context(di_context.clone())
		.build()
		.await
		.expect("Failed to create server with DI context");

	(server, di_context)
}

// ============================================================================
// WebSocket Server Fixtures (feature: websocket)
// ============================================================================

#[cfg(feature = "websockets")]
/// WebSocket-enabled server fixture
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::*;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_websocket_server(#[future] websocket_server: TestServer) {
///     let server = websocket_server.await;
///     // WebSocket test
/// }
/// ```
#[fixture]
pub async fn websocket_server() -> TestServer {
	use reinhardt_server::WebSocketHandler;

	#[derive(Clone)]
	struct EchoHandler;

	#[async_trait::async_trait]
	impl WebSocketHandler for EchoHandler {
		async fn handle_message(&self, message: String) -> Result<String, String> {
			Ok(message) // Echo back
		}

		async fn on_connect(&self) {}
		async fn on_disconnect(&self) {}
	}

	let ws_handler = Arc::new(EchoHandler);
	TestServer::builder()
		.websocket_handler(ws_handler)
		.build()
		.await
		.expect("Failed to create WebSocket server")
}

#[cfg(feature = "websockets")]
/// WebSocket client fixture
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::*;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_websocket_client(websocket_client: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>) {
///     // WebSocket client test
/// }
/// ```
#[fixture]
pub async fn websocket_client(
	#[from(websocket_server)]
	#[future]
	server: TestServer,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
	let server = server.await;
	let ws_url = server.url.replace("http://", "ws://");
	let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url)
		.await
		.expect("Failed to connect WebSocket");
	ws_stream
}

// ============================================================================
// GraphQL Server Fixtures (feature: graphql)
// ============================================================================

#[cfg(feature = "graphql")]
/// GraphQL-enabled server fixture
///
/// # Examples
///
/// ```no_run
/// use reinhardt_testkit::fixtures::*;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_graphql_server(#[future] graphql_server: TestServer) {
///     let server = graphql_server.await;
///     // GraphQL test
/// }
/// ```
#[cfg(feature = "graphql")]
#[fixture]
pub async fn graphql_server() -> TestServer {
	use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema};

	struct Query;

	#[Object]
	impl Query {
		async fn hello(&self) -> &'static str {
			"Hello, GraphQL!"
		}
	}

	let schema = Schema::build(Query, EmptyMutation, EmptySubscription).finish();
	let graphql_handler = Arc::new(GraphQLHandler::new(schema));

	TestServer::builder()
		.handler(graphql_handler)
		.build()
		.await
		.expect("Failed to create GraphQL server")
}

// ============================================================================
// TestServer Structure with Builder Pattern
// ============================================================================

/// Test server with automatic shutdown.
///
/// For HTTP/1.1 fixtures, dropping the guard also cancels accepted connections
/// and pending handlers. Their resources are released asynchronously when the
/// Tokio runtime polls the aborted tasks.
pub struct TestServer {
	/// Server URL (e.g., "http://127.0.0.1:12345")
	pub url: String,
	/// Server address
	pub addr: SocketAddr,
	/// Shutdown coordinator
	pub coordinator: Arc<ShutdownCoordinator>,
	/// Server task handle
	server_task: Option<JoinHandle<()>>,
}

impl TestServer {
	/// Create a new TestServerBuilder
	pub fn builder() -> TestServerBuilder {
		TestServerBuilder::new()
	}
}

impl Drop for TestServer {
	fn drop(&mut self) {
		// Trigger shutdown signal
		self.coordinator.shutdown();

		// Abort the server task
		if let Some(task) = self.server_task.take() {
			task.abort();
		}
	}
}

/// Builder for TestServer
pub struct TestServerBuilder {
	handler: Option<Arc<dyn Handler>>,
	#[cfg(feature = "websockets")]
	websocket_handler: Option<Arc<dyn reinhardt_server::WebSocketHandler>>,
	di_context: Option<Arc<InjectionContext>>,
	http2: bool,
	shutdown_timeout: Duration,
}

impl TestServerBuilder {
	fn new() -> Self {
		Self {
			handler: None,
			#[cfg(feature = "websockets")]
			websocket_handler: None,
			di_context: None,
			http2: false,
			shutdown_timeout: Duration::from_secs(5),
		}
	}

	/// Set the handler for HTTP requests
	pub fn handler(mut self, handler: Arc<dyn Handler>) -> Self {
		self.handler = Some(handler);
		self
	}

	#[cfg(feature = "websockets")]
	/// Set the WebSocket handler
	pub fn websocket_handler(
		mut self,
		handler: Arc<dyn reinhardt_server::WebSocketHandler>,
	) -> Self {
		self.websocket_handler = Some(handler);
		self
	}

	/// Set the DI context
	pub fn di_context(mut self, context: Arc<InjectionContext>) -> Self {
		self.di_context = Some(context);
		self
	}

	/// Enable HTTP/2
	pub fn http2(mut self, enabled: bool) -> Self {
		self.http2 = enabled;
		self
	}

	/// Set shutdown timeout
	pub fn shutdown_timeout(mut self, timeout: Duration) -> Self {
		self.shutdown_timeout = timeout;
		self
	}

	/// Build the TestServer
	pub async fn build(self) -> Result<TestServer, Box<dyn std::error::Error>> {
		// Bind to random port and keep the listener to avoid TOCTOU race
		let listener = TcpListener::bind("127.0.0.1:0").await?;
		let actual_addr = listener.local_addr()?;
		let url = format!("http://{}", actual_addr);

		// Create shutdown coordinator
		let coordinator = Arc::new(ShutdownCoordinator::new(self.shutdown_timeout));

		// Spawn server based on configuration
		let server_coordinator = (*coordinator).clone();

		#[cfg(feature = "websockets")]
		let websocket_handler = self.websocket_handler;

		let handler = self.handler;
		let di_context = self.di_context;
		let http2 = self.http2;

		let server_task = tokio::spawn(async move {
			// For WebSocket and HTTP/2 servers, we must drop the listener and re-bind
			// because their APIs only accept SocketAddr. This has a small TOCTOU window
			// but these server types are rarely used in parallel tests.
			#[cfg(feature = "websockets")]
			if let Some(ws_handler) = websocket_handler {
				drop(listener);
				let server = WebSocketServer::from_arc(ws_handler);
				let _ = server
					.listen_with_shutdown(actual_addr, server_coordinator)
					.await;
				return;
			}

			if let Some(h) = handler {
				if http2 {
					drop(listener);
					let server = reinhardt_server::Http2Server::new(h);
					let _ = server
						.listen_with_shutdown(actual_addr, server_coordinator)
						.await;
				} else {
					// Use the already-bound listener directly to avoid TOCTOU race
					let server = HttpServer::new(h);
					let mut shutdown_rx = server_coordinator.subscribe();
					// Own connections so both shutdown and task abortion release them.
					let mut connections = JoinSet::new();
					loop {
						tokio::select! {
							result = listener.accept() => {
								match result {
									Ok((stream, socket_addr)) => {
										let handler_clone = server.handler();
										let di_ctx = di_context.clone();
										connections.spawn(async move {
											if let Err(e) =
												HttpServer::handle_connection(stream, socket_addr, handler_clone, di_ctx)
													.await
											{
												eprintln!("Error handling connection: {:?}", e);
											}
										});
									}
									Err(e) => {
										eprintln!("Error accepting connection: {:?}", e);
										break;
									}
								}
							}
							_ = shutdown_rx.recv() => {
								break;
							}
							_ = connections.join_next(), if !connections.is_empty() => {}
						}
					}
				}
			}
		});

		// Probe server readiness with TCP connect attempts instead of fixed sleep.
		// This avoids flaky failures when the system is under heavy load.
		wait_for_server_ready(actual_addr)
			.await
			.expect("Test server failed to become ready");

		Ok(TestServer {
			url,
			addr: actual_addr,
			coordinator,
			server_task: Some(server_task),
		})
	}
}

// ============================================================================
// Server Readiness Probe
// ============================================================================

/// Maximum number of TCP readiness probe attempts
const SERVER_READY_MAX_ATTEMPTS: u32 = 20;

/// Interval between TCP readiness probe attempts
const SERVER_READY_PROBE_INTERVAL_MS: u64 = 50;

/// Probe the server address with TCP connects until it accepts a connection.
///
/// This replaces a fixed `sleep(100ms)` with an active readiness check,
/// eliminating flaky test failures caused by slow server startup under load.
///
/// # Errors
///
/// Returns an error if the server does not accept a TCP connection within
/// the configured number of probe attempts.
async fn wait_for_server_ready(addr: SocketAddr) -> Result<(), std::io::Error> {
	for attempt in 1..=SERVER_READY_MAX_ATTEMPTS {
		// Try to establish a TCP connection to verify the server is accepting
		match tokio::net::TcpStream::connect(addr).await {
			Ok(_) => return Ok(()),
			Err(_) if attempt < SERVER_READY_MAX_ATTEMPTS => {
				tokio::time::sleep(Duration::from_millis(SERVER_READY_PROBE_INTERVAL_MS)).await;
			}
			Err(e) => {
				return Err(std::io::Error::new(
					std::io::ErrorKind::TimedOut,
					format!(
						"Server at {} not ready after {} attempts: {}",
						addr, SERVER_READY_MAX_ATTEMPTS, e
					),
				));
			}
		}
	}

	Err(std::io::Error::new(
		std::io::ErrorKind::TimedOut,
		format!(
			"Server at {} not ready after {} attempts",
			addr, SERVER_READY_MAX_ATTEMPTS
		),
	))
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::*;

	#[fixture]
	fn hello_router() -> Router {
		Router::new().handler("/hello", BasicHandler)
	}

	#[fixture]
	async fn hello_scenario(
		#[future]
		#[from(test_server_guard)]
		#[with(hello_router())]
		server: TestServerGuard,
	) -> TestServerGuard {
		server.await
	}

	#[rstest]
	#[tokio::test]
	async fn test_server_guard_fixture_default_router(
		#[future] test_server_guard: TestServerGuard,
	) {
		// Arrange
		let server = test_server_guard.await;
		let client = http_client();

		// Act
		let response = client
			.get(format!("{}/unregistered", server.url))
			.send()
			.await
			.expect("Failed to request unregistered route");

		// Assert
		assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
	}

	#[rstest]
	#[tokio::test]
	async fn test_server_guard_fixture_router_override(
		#[future]
		#[from(test_server_guard)]
		#[with(hello_router())]
		server: TestServerGuard,
	) {
		// Arrange
		let server = server.await;
		let client = http_client();

		// Act
		let response = client
			.get(format!("{}/hello", server.url))
			.send()
			.await
			.expect("Failed to request overridden router");

		// Assert
		assert_eq!(response.status(), reqwest::StatusCode::OK);
		assert_eq!(
			response.text().await.expect("Failed to read response"),
			"OK"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_server_guard_fixture_composition(#[future] hello_scenario: TestServerGuard) {
		// Arrange
		let server = hello_scenario.await;
		let client = http_client();

		// Act
		let response = client
			.get(format!("{}/hello", server.url))
			.send()
			.await
			.expect("Failed to request composed server fixture");

		// Assert
		assert_eq!(response.status(), reqwest::StatusCode::OK);
		assert_eq!(
			response.text().await.expect("Failed to read response"),
			"OK"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_basic_handler_returns_ok() {
		// Arrange
		let handler = BasicHandler;
		let request = Request::builder()
			.method(hyper::Method::GET)
			.uri("/")
			.build()
			.expect("Failed to build request");

		// Act
		let response = handler.handle(request).await;

		// Assert
		assert!(response.is_ok(), "Expected Ok response from BasicHandler");
		let resp = response.unwrap();
		assert_eq!(resp.status, hyper::StatusCode::OK);
	}

	#[rstest]
	#[tokio::test]
	async fn test_test_server_guard_starts() {
		// Arrange
		let router = Router::new();

		// Act
		let server = test_server_guard(router).await;

		// Assert
		assert!(
			server.url.starts_with("http://127.0.0.1:"),
			"Expected URL to start with 'http://127.0.0.1:', got: {}",
			server.url
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_test_server_builder_default() {
		// Arrange
		let handler: Arc<dyn Handler> = Arc::new(BasicHandler);

		// Act
		let result = TestServer::builder().handler(handler).build().await;

		// Assert
		assert!(
			result.is_ok(),
			"Expected TestServer::builder().handler().build() to succeed"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_test_server_url_format() {
		// Arrange
		let handler: Arc<dyn Handler> = Arc::new(BasicHandler);

		// Act
		let server = TestServer::builder()
			.handler(handler)
			.build()
			.await
			.expect("Failed to build TestServer");

		// Assert
		assert!(
			server.url.starts_with("http://127.0.0.1:"),
			"Expected URL format 'http://127.0.0.1:<port>', got: {}",
			server.url
		);
		assert!(
			server.addr.port() > 0,
			"Expected non-zero port, got: {}",
			server.addr.port()
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_test_server_responds_to_request() {
		// Arrange
		let handler: Arc<dyn Handler> = Arc::new(BasicHandler);
		let server = TestServer::builder()
			.handler(handler)
			.build()
			.await
			.expect("Failed to build TestServer");
		let client = reqwest::Client::new();

		// Act
		let response = client.get(&server.url).send().await;

		// Assert
		assert!(response.is_ok(), "Expected GET request to succeed");
		let resp = response.unwrap();
		assert_eq!(resp.status(), reqwest::StatusCode::OK);
	}

	#[rstest]
	fn test_http_client_fixture() {
		// Arrange & Act
		let client = http_client();

		// Assert
		// Verify client was created successfully by making a type assertion
		let _: &reqwest::Client = &client;
	}

	#[rstest]
	#[tokio::test]
	async fn test_test_server_shutdown_timeout() {
		// Arrange
		let handler: Arc<dyn Handler> = Arc::new(BasicHandler);
		let custom_timeout = Duration::from_secs(10);

		// Act
		let result = TestServer::builder()
			.handler(handler)
			.shutdown_timeout(custom_timeout)
			.build()
			.await;

		// Assert
		assert!(
			result.is_ok(),
			"Expected TestServer with custom shutdown timeout to build successfully"
		);
	}

	#[rstest]
	#[tokio::test]
	async fn test_wait_for_server_ready() {
		// Arrange
		let listener = TcpListener::bind("127.0.0.1:0")
			.await
			.expect("Failed to bind listener");
		let addr = listener.local_addr().expect("Failed to get local addr");

		// Act
		let result = wait_for_server_ready(addr).await;

		// Assert
		assert!(
			result.is_ok(),
			"Expected wait_for_server_ready to succeed for a bound address"
		);
	}
}
