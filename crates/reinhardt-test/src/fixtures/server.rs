//! Server test fixtures with automatic graceful shutdown.
//!
//! This module provides rstest fixtures for testing HTTP servers with automatic
//! cleanup via RAII pattern.

use reinhardt_routers::UnifiedRouter as Router;
use reinhardt_server::{HttpServer, ShutdownCoordinator};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

/// Test server guard with automatic graceful shutdown.
///
/// This guard automatically performs graceful shutdown when dropped, ensuring
/// proper cleanup of server resources even if the test panics.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_test::fixtures::*;
/// use reinhardt_routers::UnifiedRouter as Router;
/// use hyper::Method;
/// use std::sync::Arc;
/// use rstest::*;
///
/// #[fixture]
/// fn test_router() -> Arc<Router> {
///     Arc::new(Router::new())
/// }
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_example(
///     #[from(test_router)] router: Arc<Router>,
///     #[future] test_server_guard: TestServerGuard
/// ) {
///     let server = test_server_guard.await;
///     let response = reqwest::get(&format!("{}/test", server.url))
///         .await
///         .unwrap();
///     assert_eq!(response.status(), 200);
///     // Automatic graceful shutdown when server goes out of scope
/// }
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
	/// 4. Waits 100ms for the server to start
	///
	/// # Arguments
	///
	/// * `router` - Router to use for handling requests
	async fn new(router: Arc<Router>) -> Self {
		let shutdown_timeout = Duration::from_secs(5);
		// Bind to random port
		let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
		let listener = TcpListener::bind(addr).await.unwrap();
		let actual_addr = listener.local_addr().unwrap();
		let url = format!("http://{}", actual_addr);
		drop(listener);

		// Create shutdown coordinator
		let coordinator = Arc::new(ShutdownCoordinator::new(shutdown_timeout));

		// Spawn server
		let server_coordinator = (*coordinator).clone();
		let server_task = tokio::spawn(async move {
			let server = HttpServer::new(router);
			let _ = server
				.listen_with_shutdown(actual_addr, server_coordinator)
				.await;
		});

		// Wait for server to start
		tokio::time::sleep(Duration::from_millis(100)).await;

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

		// Abort the server task
		// The ShutdownCoordinator will handle graceful shutdown,
		// but we need to ensure the task is terminated
		if let Some(task) = self.server_task.take() {
			task.abort();
		}
	}
}

/// Create a test server guard with the given router.
///
/// This is a helper function (not an rstest fixture) that creates a test server
/// with automatic graceful shutdown. Use it directly in your tests.
///
/// # Examples
///
/// ```no_run
/// use reinhardt_test::fixtures::*;
/// use reinhardt_routers::UnifiedRouter as Router;
/// use std::sync::Arc;
///
/// #[tokio::test]
/// async fn test_server() {
///     let router = Arc::new(Router::new());
///     let server = test_server_guard(router).await;
///     let response = reqwest::get(&format!("{}/hello", server.url))
///         .await
///         .unwrap();
///     assert_eq!(response.status(), 200);
///     // Automatic cleanup on drop
/// }
/// ```
pub async fn test_server_guard(router: Arc<Router>) -> TestServerGuard {
	TestServerGuard::new(router).await
}
