//! API Client fixtures for E2E testing
//!
//! Provides helper functions for creating APIClient instances
//! that connect to test servers.
//!
//! ## Overview
//!
//! This module provides helper functions for creating `APIClient` instances
//! configured to connect to test servers. Use `api_client_from_url` with
//! the server URL obtained from `TestServerGuard`, or use [`test_server_client`]
//! to create a server-backed client that owns its server guard.
//!
//! ## Usage Examples
//!
//! ### Using api_client_from_url with TestServerGuard
//!
//! ```rust,no_run
//! use reinhardt_testkit::fixtures::{test_server_guard, api_client_from_url, TestServerGuard};
//! use reinhardt_urls::routers::ServerRouter;
//! use rstest::*;
//!
//! #[rstest]
//! #[tokio::test]
//! async fn test_api() {
//!     let router = ServerRouter::new();
//!     let server = test_server_guard(router).await;
//!     let client = api_client_from_url(&server.url);
//!     let response = client.get("/api/test").await.unwrap();
//!     assert_eq!(response.status_code(), 404);
//! }
//! ```
//!
//! ### Creating APIClient directly
//!
//! ```rust,no_run
//! use reinhardt_testkit::APIClient;
//!
//! # async fn example() {
//! let client = APIClient::with_base_url("http://localhost:8080");
//! let response = client.get("/api/test").await.unwrap();
//! # }
//! ```

use super::server::{TestServerGuard, test_server_guard};
use crate::client::{APIClient, APIClientBuilder};
use reinhardt_urls::routers::ServerRouter as Router;
use rstest::fixture;
use std::ops::Deref;
use std::sync::Arc;

/// A server-backed client that owns its test server's lifetime.
///
/// Requests reach the server over the network through [`APIClient`]. The client
/// drops before the [`TestServerGuard`], and dropping this value shuts down the
/// server, including during panic unwinding, unless a fork still shares the
/// guard. Server cancellation completes asynchronously as the Tokio runtime
/// polls the aborted tasks.
///
/// Shared client methods are available through [`Deref`]. Mutable client access
/// and splitting the client from its guard are intentionally unavailable so
/// requests cannot be redirected to an in-process handler.
///
/// Use [`TestServerClient::fork`] instead of [`APIClient::fork`] to derive an
/// independent client for another credential or subject. The fork shares this
/// client's server guard, so the server stays up until the last fork drops.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_testkit::fixtures::{test_server_client, TestServerClient};
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_api(#[future] test_server_client: TestServerClient) {
///     let client = test_server_client.await;
///     let response = client.get("/unregistered").await.unwrap();
///     assert_eq!(response.status_code(), 404);
/// }
/// ```
pub struct TestServerClient {
	// Declaration order ensures the client drops before its server guard.
	client: APIClient,
	// Shared with every fork; the guard shuts the server down when the last holder drops.
	server: Arc<TestServerGuard>,
}

impl TestServerClient {
	/// Create a server-backed client using the server's URL and default settings.
	///
	/// Takes ownership of the guard so the server stays alive until this client
	/// and all of its forks (see [`Self::fork`]) are dropped.
	pub fn new(server: TestServerGuard) -> Self {
		Self {
			client: APIClient::with_base_url(&server.url),
			server: Arc::new(server),
		}
	}

	/// Create a server-backed client using custom client settings.
	///
	/// Always replaces the builder's base URL with the server's URL and takes
	/// ownership of the guard, so the server stays alive until this client and
	/// all of its forks (see [`Self::fork`]) are dropped.
	///
	/// # Panics
	///
	/// Panics if the builder has a handler configured via
	/// [`APIClientBuilder::handler`], because in-process dispatch would bypass
	/// the test server.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_testkit::client::APIClientBuilder;
	/// use reinhardt_testkit::fixtures::{test_server_guard, TestServerClient};
	/// use reinhardt_urls::routers::ServerRouter;
	/// use std::time::Duration;
	///
	/// # #[tokio::main]
	/// # async fn main() {
	/// let server = test_server_guard(ServerRouter::new()).await;
	/// let builder = APIClientBuilder::new().timeout(Duration::from_secs(5));
	/// let client = TestServerClient::with_builder(server, builder);
	/// let response = client.get("/unregistered").await.unwrap();
	/// assert_eq!(response.status_code(), 404);
	/// # }
	/// ```
	pub fn with_builder(server: TestServerGuard, builder: APIClientBuilder) -> Self {
		assert!(
			!builder.has_framework_handler(),
			"TestServerClient requires a network client; handler-configured builders bypass the test server"
		);
		Self {
			client: builder.base_url(&server.url).build(),
			server: Arc::new(server),
		}
	}

	/// Derive an independent server-backed client that shares this client's server.
	///
	/// Behaves like [`APIClient::fork`] for client state: default headers, manual
	/// cookies, and the forced-auth user are snapshotted into separate locks, and
	/// the fork builds its own transport with the same settings. In addition, the
	/// fork holds a shared reference to the server guard, so the server keeps
	/// running until the last of this client and all of its forks drops. Dropping
	/// the original first does not cut off the forks.
	///
	/// This inherent method takes precedence over [`APIClient::fork`], which is
	/// otherwise reachable through [`Deref`] and would return a bare [`APIClient`]
	/// that does not keep the server alive.
	///
	/// # Panics
	///
	/// Panics if reqwest cannot build a transport, as with [`APIClient::fork`].
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_testkit::fixtures::{test_server_client, TestServerClient};
	/// use reinhardt_urls::routers::ServerRouter;
	///
	/// # #[tokio::main]
	/// # async fn main() {
	/// let base = test_server_client(ServerRouter::new()).await;
	/// let alice = base.fork().await;
	/// alice.set_header("Authorization", "Bearer alice").await.unwrap();
	/// // The server stays up because `alice` shares the guard.
	/// drop(base);
	/// let response = alice.get("/unregistered").await.unwrap();
	/// assert_eq!(response.status_code(), 404);
	/// # }
	/// ```
	pub async fn fork(&self) -> TestServerClient {
		TestServerClient {
			client: self.client.fork().await,
			server: Arc::clone(&self.server),
		}
	}

	/// Borrow the server guard to inspect its URL or shutdown coordinator.
	///
	/// The guard is shared with this client's forks and cannot be separated from
	/// them.
	pub fn server(&self) -> &TestServerGuard {
		&self.server
	}
}

impl Deref for TestServerClient {
	type Target = APIClient;

	fn deref(&self) -> &Self::Target {
		&self.client
	}
}

/// Create a server-backed client fixture with automatic server cleanup.
///
/// Uses [`test_server_guard`] with an empty [`Router`] by default. Override the
/// router with rstest's `#[with(...)]` attribute or pass it directly when calling
/// the function. Dropping the returned [`TestServerClient`] shuts down the server,
/// including during panic unwinding, once no [`TestServerClient::fork`] remains.
///
/// # Examples
///
/// Use the default fixture or override its router with `#[with(...)]`:
///
/// ```rust,no_run
/// use reinhardt_testkit::fixtures::{test_server_client, BasicHandler, TestServerClient};
/// use reinhardt_urls::routers::ServerRouter;
/// use rstest::*;
///
/// fn my_router() -> ServerRouter {
///     ServerRouter::new().handler("/hello", BasicHandler)
/// }
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_default_client(#[future] test_server_client: TestServerClient) {
///     let client = test_server_client.await;
///     let response = client.get("/unregistered").await.unwrap();
///     assert_eq!(response.status_code(), 404);
/// }
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_custom_client(
///     #[future]
///     #[from(test_server_client)]
///     #[with(my_router())]
///     client: TestServerClient,
/// ) {
///     let client = client.await;
///     let response = client.get("/hello").await.unwrap();
///     assert_eq!(response.status_code(), 200);
///     assert_eq!(response.text(), "OK");
/// }
/// ```
#[fixture]
pub async fn test_server_client(#[default(Router::new())] router: Router) -> TestServerClient {
	TestServerClient::new(test_server_guard(router).await)
}

/// Create an APIClient from a server URL string
///
/// This is a helper function for creating an `APIClient` when you already
/// have a server URL. Use this when you need more control over the server
/// setup or when working with existing test infrastructure.
///
/// # Arguments
///
/// * `url` - The base URL of the server to connect to (e.g., "http://localhost:8080")
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_testkit::fixtures::api_client_from_url;
///
/// # async fn example() {
/// let client = api_client_from_url("http://localhost:8080");
/// let response = client.get("/api/users").await.unwrap();
/// # }
/// ```
///
/// To inspect redirect responses, configure the network policy with the builder:
///
/// ```rust,no_run
/// use reinhardt_testkit::{APIClient, RedirectPolicy};
///
/// # async fn example() {
/// let client = APIClient::builder()
///     .base_url("http://localhost:8080")
///     .redirect_policy(RedirectPolicy::Never)
///     .build();
/// let response = client.get("/login").await.unwrap();
/// assert_eq!(response.status_code(), 302);
/// assert_eq!(response.header("Location"), Some("/dashboard"));
/// # }
/// ```
///
/// # Usage with TestServerGuard
///
/// ```rust,no_run
/// use reinhardt_testkit::fixtures::{test_server_guard, api_client_from_url, TestServerGuard};
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_with_custom_setup(#[future] test_server_guard: TestServerGuard) {
///     let server = test_server_guard.await;
///     let client = api_client_from_url(&server.url);
///     let response = client.get("/test").await.unwrap();
/// }
/// ```
pub fn api_client_from_url(url: &str) -> APIClient {
	APIClient::with_base_url(url)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::fixtures::server::BasicHandler;
	use reinhardt_http::{Handler, Request, Response};
	use rstest::*;
	use std::io::ErrorKind;
	use std::net::SocketAddr;
	use std::time::Duration;
	use tokio::net::TcpStream;
	use tokio::time::{sleep, timeout};

	#[fixture]
	fn hello_router() -> Router {
		Router::new().handler("/hello", BasicHandler)
	}

	/// Echoes the `x-fork` request header, or `none` when it is absent.
	struct ForkHeaderEchoHandler;

	#[async_trait::async_trait]
	impl Handler for ForkHeaderEchoHandler {
		async fn handle(&self, request: Request) -> reinhardt_core::exception::Result<Response> {
			let value = request
				.headers
				.get("x-fork")
				.and_then(|value| value.to_str().ok())
				.unwrap_or("none")
				.to_owned();
			Ok(Response::ok().with_body(value))
		}
	}

	#[fixture]
	fn fork_router() -> Router {
		Router::new()
			.handler("/hello", BasicHandler)
			.handler("/echo", ForkHeaderEchoHandler)
	}

	async fn assert_connection_refused(addr: SocketAddr) {
		for _ in 0..50 {
			match timeout(Duration::from_millis(100), TcpStream::connect(addr))
				.await
				.expect("TCP shutdown probe timed out")
			{
				Err(error) if error.kind() != ErrorKind::ConnectionReset => {
					assert_eq!(error.kind(), ErrorKind::ConnectionRefused);
					return;
				}
				// A pending connection can reset while cancellation completes.
				Err(_) | Ok(_) => {}
			}
			sleep(Duration::from_millis(10)).await;
		}
		panic!("Server at {addr} still accepts connections after client cleanup");
	}

	#[rstest]
	#[tokio::test]
	async fn test_server_client_default_router(#[future] test_server_client: TestServerClient) {
		// Arrange
		let client = test_server_client.await;

		// Act
		let response = client.get("/unregistered").await.unwrap();

		// Assert
		assert_eq!(response.status_code(), 404);
	}

	#[rstest]
	#[tokio::test]
	async fn test_server_client_router_override(
		#[future]
		#[from(test_server_client)]
		#[with(hello_router())]
		client: TestServerClient,
	) {
		// Arrange
		let client = client.await;

		// Act
		let response = client.get("/hello").await.unwrap();

		// Assert
		assert_eq!(response.status_code(), 200);
		assert_eq!(response.text(), "OK");
	}

	#[rstest]
	#[tokio::test]
	async fn test_server_client_drop_stops_server(#[future] test_server_client: TestServerClient) {
		// Arrange
		let client = test_server_client.await;
		let addr = client
			.server()
			.url
			.strip_prefix("http://")
			.unwrap()
			.parse()
			.unwrap();
		let _connection = TcpStream::connect(addr).await.unwrap();

		// Act
		drop(client);

		// Assert
		assert_connection_refused(addr).await;
	}

	#[rstest]
	#[tokio::test]
	async fn test_server_client_panicking_owner_stops_server(
		#[future] test_server_client: TestServerClient,
	) {
		// Arrange
		let client = test_server_client.await;
		let addr = client
			.server()
			.url
			.strip_prefix("http://")
			.unwrap()
			.parse()
			.unwrap();
		let _connection = TcpStream::connect(addr).await.unwrap();

		// Act
		let result = tokio::spawn(async move {
			let _client = client;
			panic!("TestServerClient owner panicked");
		})
		.await;

		// Assert
		assert!(result.unwrap_err().is_panic());
		assert_connection_refused(addr).await;
	}

	#[rstest]
	#[tokio::test]
	#[should_panic(
		expected = "TestServerClient requires a network client; handler-configured builders bypass the test server"
	)]
	async fn test_server_client_builder_rejects_handler(
		#[future] test_server_guard: TestServerGuard,
	) {
		// Arrange
		let server = test_server_guard.await;
		let builder = APIClientBuilder::new().handler(BasicHandler);

		// Act
		let _client = TestServerClient::with_builder(server, builder);
	}

	#[rstest]
	#[tokio::test]
	async fn test_server_client_builder_overrides_base_url(
		#[future]
		#[from(test_server_guard)]
		#[with(hello_router())]
		server: TestServerGuard,
	) {
		// Arrange
		let server = server.await;
		let builder = APIClientBuilder::new().base_url("http://testserver.invalid");
		let client = TestServerClient::with_builder(server, builder);

		// Act
		let response = client.get("/hello").await.unwrap();

		// Assert
		assert_eq!(response.status_code(), 200);
		assert_eq!(response.text(), "OK");
	}

	#[rstest]
	#[tokio::test]
	async fn test_server_client_fork_shares_server_guard(
		#[future]
		#[from(test_server_client)]
		#[with(hello_router())]
		client: TestServerClient,
	) {
		// Arrange
		let parent = client.await;

		// Act
		let fork: TestServerClient = parent.fork().await;

		// Assert
		assert!(Arc::ptr_eq(&parent.server, &fork.server));
		assert_eq!(fork.base_url(), parent.base_url());
	}

	#[rstest]
	#[case::parent_dropped_first(true)]
	#[case::fork_dropped_first(false)]
	#[tokio::test]
	async fn test_server_client_fork_keeps_server_until_last_holder_drops(
		#[future]
		#[from(test_server_client)]
		#[with(hello_router())]
		client: TestServerClient,
		#[case] parent_dropped_first: bool,
	) {
		// Arrange
		let parent = client.await;
		let addr = parent
			.server()
			.url
			.strip_prefix("http://")
			.unwrap()
			.parse()
			.unwrap();
		let fork = parent.fork().await;
		let (dropped_first, survivor) = if parent_dropped_first {
			(parent, fork)
		} else {
			(fork, parent)
		};
		let _connection = TcpStream::connect(addr).await.unwrap();

		// Act
		drop(dropped_first);
		let response = survivor.get("/hello").await.unwrap();

		// Assert
		assert_eq!(response.status_code(), 200);
		assert_eq!(response.text(), "OK");

		// Act
		drop(survivor);

		// Assert
		assert_connection_refused(addr).await;
	}

	#[rstest]
	#[tokio::test]
	async fn test_server_client_fork_state_is_independent(
		#[future]
		#[from(test_server_client)]
		#[with(fork_router())]
		client: TestServerClient,
	) {
		// Arrange
		let parent = client.await;
		parent.set_header("x-fork", "original").await.unwrap();
		let fork = parent.fork().await;

		// Act
		let snapshot = fork.get("/echo").await.unwrap();
		parent.set_header("x-fork", "parent").await.unwrap();
		fork.set_header("x-fork", "fork").await.unwrap();
		let parent_response = parent.get("/echo").await.unwrap();
		let fork_response = fork.get("/echo").await.unwrap();

		// Assert
		assert_eq!(snapshot.text(), "original");
		assert_eq!(parent_response.text(), "parent");
		assert_eq!(fork_response.text(), "fork");
	}

	#[test]
	fn test_api_client_from_url() {
		let client = api_client_from_url("http://localhost:8080");
		assert_eq!(client.base_url(), "http://localhost:8080");
	}

	#[test]
	fn test_api_client_from_url_with_path() {
		let client = api_client_from_url("http://example.com:3000");
		assert_eq!(client.base_url(), "http://example.com:3000");
	}
}
