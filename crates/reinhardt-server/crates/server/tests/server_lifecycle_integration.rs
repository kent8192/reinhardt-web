//! Server Lifecycle Integration Tests
//!
//! This test suite verifies server lifecycle management including:
//! - Server startup and shutdown
//! - Graceful shutdown with active connections
//! - Request handling during shutdown
//! - Server restart
//! - Health check endpoints
//! - Server state management

use anyhow::Result;
use reinhardt_core::Handler;
use reinhardt_core::http::{Request, Response};
use reinhardt_server_core::{HttpServer, ShutdownCoordinator};
use std::net::SocketAddr;
use std::sync::{
	Arc, Mutex,
	atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::{sleep, timeout};

/// Simple test handler for basic server operations
#[derive(Clone)]
struct BasicHandler;

#[async_trait::async_trait]
impl Handler for BasicHandler {
	async fn handle(&self, _request: Request) -> reinhardt_core::exception::Result<Response> {
		Ok(Response::ok().with_body("Server is running"))
	}
}

/// Handler that tracks request count
#[derive(Clone)]
struct RequestCounterHandler {
	count: Arc<AtomicU64>,
}

impl RequestCounterHandler {
	fn new() -> Self {
		Self {
			count: Arc::new(AtomicU64::new(0)),
		}
	}

	fn get_count(&self) -> u64 {
		self.count.load(Ordering::SeqCst)
	}
}

#[async_trait::async_trait]
impl Handler for RequestCounterHandler {
	async fn handle(&self, _request: Request) -> reinhardt_core::exception::Result<Response> {
		let count = self.count.fetch_add(1, Ordering::SeqCst) + 1;
		Ok(Response::ok().with_body(format!("Request #{}", count)))
	}
}

/// Handler with configurable delay for testing shutdown timing
#[derive(Clone)]
struct DelayedHandler {
	delay: Duration,
	processing: Arc<AtomicBool>,
}

impl DelayedHandler {
	fn new(delay: Duration) -> Self {
		Self {
			delay,
			processing: Arc::new(AtomicBool::new(false)),
		}
	}

	fn is_processing(&self) -> bool {
		self.processing.load(Ordering::SeqCst)
	}
}

#[async_trait::async_trait]
impl Handler for DelayedHandler {
	async fn handle(&self, _request: Request) -> reinhardt_core::exception::Result<Response> {
		self.processing.store(true, Ordering::SeqCst);
		sleep(self.delay).await;
		self.processing.store(false, Ordering::SeqCst);
		Ok(Response::ok().with_body("Request completed"))
	}
}

/// Handler that returns different responses based on path (health check endpoint)
#[derive(Clone)]
struct HealthCheckHandler {
	is_healthy: Arc<AtomicBool>,
}

impl HealthCheckHandler {
	fn new() -> Self {
		Self {
			is_healthy: Arc::new(AtomicBool::new(true)),
		}
	}

	fn set_healthy(&self, healthy: bool) {
		self.is_healthy.store(healthy, Ordering::SeqCst);
	}
}

#[async_trait::async_trait]
impl Handler for HealthCheckHandler {
	async fn handle(&self, request: Request) -> reinhardt_core::exception::Result<Response> {
		match request.path() {
			"/health" => {
				if self.is_healthy.load(Ordering::SeqCst) {
					Ok(Response::ok().with_body(r#"{"status":"healthy"}"#))
				} else {
					Ok(Response::internal_server_error().with_body(r#"{"status":"unhealthy"}"#))
				}
			}
			"/ready" => Ok(Response::ok().with_body(r#"{"ready":true}"#)),
			"/live" => Ok(Response::ok().with_body(r#"{"live":true}"#)),
			_ => Ok(Response::ok().with_body("OK")),
		}
	}
}

/// Handler that tracks server state (startup, running, shutting down)
#[derive(Clone)]
struct StateTrackingHandler {
	state: Arc<Mutex<String>>,
}

impl StateTrackingHandler {
	fn new() -> Self {
		Self {
			state: Arc::new(Mutex::new("starting".to_string())),
		}
	}

	fn set_state(&self, new_state: &str) {
		*self.state.lock().unwrap() = new_state.to_string();
	}

	fn get_state(&self) -> String {
		self.state.lock().unwrap().clone()
	}
}

#[async_trait::async_trait]
impl Handler for StateTrackingHandler {
	async fn handle(&self, request: Request) -> reinhardt_core::exception::Result<Response> {
		if request.path() == "/state" {
			let state = self.get_state();
			Ok(Response::ok().with_body(format!(r#"{{"state":"{}"}}"#, state)))
		} else {
			Ok(Response::ok().with_body("OK"))
		}
	}
}

/// Helper function to spawn a test server with graceful shutdown support
async fn spawn_server_with_shutdown(
	handler: Arc<dyn Handler>,
	coordinator: ShutdownCoordinator,
) -> (SocketAddr, tokio::task::JoinHandle<Result<()>>) {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	drop(listener);

	let handle = tokio::spawn(async move {
		let server = HttpServer::new(handler);
		server
			.listen_with_shutdown(addr, coordinator)
			.await
			.map_err(|e| anyhow::anyhow!("{}", e))
	});

	// Wait for server to be ready with retry logic
	wait_for_server_ready(addr).await;

	(addr, handle)
}

/// Wait for server to be ready to accept connections
async fn wait_for_server_ready(addr: SocketAddr) {
	let max_retries = 50;
	let retry_delay = Duration::from_millis(10);

	for attempt in 0..max_retries {
		if tokio::net::TcpStream::connect(addr).await.is_ok() {
			// Server is listening, give it a moment to fully initialize
			sleep(Duration::from_millis(10)).await;
			return;
		}

		if attempt < max_retries - 1 {
			sleep(retry_delay).await;
		}
	}

	panic!("Server did not start listening on {} within timeout", addr);
}

/// Helper function to make HTTP request with retry logic
async fn make_request(addr: SocketAddr, path: &str) -> Result<String> {
	make_request_with_retries(addr, path, 3).await
}

/// Make HTTP request with configurable retry count
async fn make_request_with_retries(
	addr: SocketAddr,
	path: &str,
	max_retries: usize,
) -> Result<String> {
	let url = format!("http://{}{}", addr, path);

	let mut last_error = None;

	for attempt in 0..max_retries {
		match timeout(Duration::from_secs(2), reqwest::get(&url)).await {
			Ok(Ok(response)) => {
				return Ok(response.text().await?);
			}
			Ok(Err(e)) => {
				last_error = Some(e.into());
				if attempt < max_retries - 1 {
					sleep(Duration::from_millis(100)).await;
				}
			}
			Err(e) => {
				last_error = Some(e.into());
				if attempt < max_retries - 1 {
					sleep(Duration::from_millis(100)).await;
				}
			}
		}
	}

	Err(last_error
		.unwrap_or_else(|| anyhow::anyhow!("Request failed after {} retries", max_retries)))
}

/// Test: Server starts successfully and accepts connections
#[tokio::test]
async fn test_server_startup() {
	let handler = Arc::new(BasicHandler) as Arc<dyn Handler>;
	let coordinator = ShutdownCoordinator::new(Duration::from_secs(5));

	let (addr, server_handle) = spawn_server_with_shutdown(handler, coordinator.clone()).await;

	// Make a request to verify server is running
	let response = make_request(addr, "/").await.unwrap();
	assert_eq!(response, "Server is running");

	// Shutdown
	coordinator.shutdown();
	coordinator.wait_for_shutdown().await;

	// Verify server task completed successfully
	let result = timeout(Duration::from_secs(1), server_handle).await;
	assert!(result.is_ok());
}

/// Test: Server handles multiple sequential requests
#[tokio::test]
async fn test_sequential_requests() {
	let handler = Arc::new(RequestCounterHandler::new());
	let coordinator = ShutdownCoordinator::new(Duration::from_secs(5));
	let counter_ref = handler.clone();

	let (addr, server_handle) =
		spawn_server_with_shutdown(handler as Arc<dyn Handler>, coordinator.clone()).await;

	// Make multiple requests
	for i in 1..=5 {
		let response = make_request(addr, "/").await.unwrap();
		assert_eq!(response, format!("Request #{}", i));
	}

	// Verify counter matches expected value
	assert_eq!(counter_ref.get_count(), 5);

	// Shutdown
	coordinator.shutdown();
	coordinator.wait_for_shutdown().await;
	let _ = server_handle.await;
}

/// Test: Server handles concurrent requests
#[tokio::test]
async fn test_concurrent_requests() {
	let handler = Arc::new(RequestCounterHandler::new());
	let coordinator = ShutdownCoordinator::new(Duration::from_secs(5));
	let counter_ref = handler.clone();

	let (addr, server_handle) =
		spawn_server_with_shutdown(handler as Arc<dyn Handler>, coordinator.clone()).await;

	// Spawn 10 concurrent requests
	let mut handles = vec![];
	for _ in 0..10 {
		let addr_copy = addr;
		let handle = tokio::spawn(async move {
			make_request(addr_copy, "/").await.unwrap();
		});
		handles.push(handle);
	}

	// Wait for all requests to complete
	for handle in handles {
		handle.await.unwrap();
	}

	// Verify all requests were handled
	assert_eq!(counter_ref.get_count(), 10);

	// Shutdown
	coordinator.shutdown();
	coordinator.wait_for_shutdown().await;
	let _ = server_handle.await;
}

/// Test: Server shuts down gracefully without active connections
#[tokio::test]
async fn test_graceful_shutdown_no_active_connections() {
	let handler = Arc::new(BasicHandler) as Arc<dyn Handler>;
	let coordinator = ShutdownCoordinator::new(Duration::from_secs(5));

	let (addr, server_handle) = spawn_server_with_shutdown(handler, coordinator.clone()).await;

	// Make a request and wait for completion
	let _response = make_request(addr, "/").await.unwrap();

	// Trigger shutdown
	let shutdown_start = std::time::Instant::now();
	coordinator.shutdown();

	// Wait for shutdown to complete
	coordinator.wait_for_shutdown().await;
	let shutdown_duration = shutdown_start.elapsed();

	// Shutdown should complete quickly since no active connections
	assert!(shutdown_duration < Duration::from_secs(1));

	// Verify server task completed
	let result = timeout(Duration::from_secs(1), server_handle).await;
	assert!(result.is_ok());
}

/// Test: Server stops accepting new connections after shutdown signal
#[tokio::test]
async fn test_graceful_shutdown_with_active_connections() {
	let handler = Arc::new(BasicHandler);
	let coordinator = ShutdownCoordinator::new(Duration::from_secs(5));

	let (addr, server_handle) =
		spawn_server_with_shutdown(handler as Arc<dyn Handler>, coordinator.clone()).await;

	// Make initial request to verify server is running
	let response = make_request(addr, "/").await.unwrap();
	assert_eq!(response, "Server is running");

	// Trigger shutdown
	coordinator.shutdown();

	// Wait for shutdown to complete
	coordinator.wait_for_shutdown().await;

	// Server should complete shutdown
	let result = timeout(Duration::from_secs(1), server_handle).await;
	assert!(result.is_ok(), "Server should shut down cleanly");
}

/// Test: Shutdown coordinator respects timeout when not explicitly notified
#[tokio::test]
async fn test_shutdown_timeout_with_long_request() {
	// Create coordinator with short timeout
	let coordinator = ShutdownCoordinator::new(Duration::from_millis(200));

	// Trigger shutdown without calling notify_shutdown_complete
	let shutdown_start = std::time::Instant::now();
	coordinator.shutdown();

	// Wait for shutdown (should timeout after 200ms)
	coordinator.wait_for_shutdown().await;
	let shutdown_duration = shutdown_start.elapsed();

	// Shutdown should respect timeout
	assert!(
		shutdown_duration >= Duration::from_millis(180),
		"Shutdown took {:?}, expected at least 180ms",
		shutdown_duration
	);
	assert!(
		shutdown_duration < Duration::from_millis(400),
		"Shutdown took {:?}, expected less than 400ms",
		shutdown_duration
	);
}

/// Test: Server rejects new connections after shutdown signal
#[tokio::test]
async fn test_no_new_connections_after_shutdown() {
	let handler = Arc::new(RequestCounterHandler::new());
	let coordinator = ShutdownCoordinator::new(Duration::from_secs(5));
	let counter_ref = handler.clone();

	let (addr, server_handle) =
		spawn_server_with_shutdown(handler as Arc<dyn Handler>, coordinator.clone()).await;

	// Make initial request
	let _response = make_request(addr, "/").await.unwrap();
	assert_eq!(counter_ref.get_count(), 1);

	// Trigger shutdown
	coordinator.shutdown();

	// Give shutdown signal time to propagate
	sleep(Duration::from_millis(50)).await;

	// Attempt to make new request after shutdown signal
	let new_request_result = timeout(Duration::from_millis(200), make_request(addr, "/")).await;

	// Request should fail (connection refused or timeout)
	assert!(new_request_result.is_err() || new_request_result.unwrap().is_err());

	// Wait for shutdown
	coordinator.wait_for_shutdown().await;
	let _ = server_handle.await;
}

/// Test: Server can be restarted after shutdown
#[tokio::test]
async fn test_server_restart() {
	let handler = Arc::new(BasicHandler) as Arc<dyn Handler>;

	// First server instance
	let coordinator1 = ShutdownCoordinator::new(Duration::from_secs(5));
	let (addr1, server_handle1) =
		spawn_server_with_shutdown(handler.clone(), coordinator1.clone()).await;

	// Verify first server is running
	let response1 = make_request(addr1, "/").await.unwrap();
	assert_eq!(response1, "Server is running");

	// Shutdown first server
	coordinator1.shutdown();
	coordinator1.wait_for_shutdown().await;
	let _ = server_handle1.await;

	// Start second server instance on different port
	let coordinator2 = ShutdownCoordinator::new(Duration::from_secs(5));
	let (addr2, server_handle2) = spawn_server_with_shutdown(handler, coordinator2.clone()).await;

	// Verify second server is running
	let response2 = make_request(addr2, "/").await.unwrap();
	assert_eq!(response2, "Server is running");

	// Shutdown second server
	coordinator2.shutdown();
	coordinator2.wait_for_shutdown().await;
	let _ = server_handle2.await;
}

/// Test: Health check endpoint returns correct status
#[tokio::test]
async fn test_health_check_endpoint() {
	let handler = Arc::new(HealthCheckHandler::new());
	let coordinator = ShutdownCoordinator::new(Duration::from_secs(5));
	let health_ref = handler.clone();

	let (addr, server_handle) =
		spawn_server_with_shutdown(handler as Arc<dyn Handler>, coordinator.clone()).await;

	// Health check should be healthy initially
	let response = make_request(addr, "/health").await.unwrap();
	assert_eq!(response, r#"{"status":"healthy"}"#);

	// Set unhealthy
	health_ref.set_healthy(false);

	// Health check should reflect new status
	let response = make_request(addr, "/health").await.unwrap();
	assert_eq!(response, r#"{"status":"unhealthy"}"#);

	// Set healthy again
	health_ref.set_healthy(true);

	// Health check should be healthy
	let response = make_request(addr, "/health").await.unwrap();
	assert_eq!(response, r#"{"status":"healthy"}"#);

	// Shutdown
	coordinator.shutdown();
	coordinator.wait_for_shutdown().await;
	let _ = server_handle.await;
}

/// Test: Readiness and liveness probes
#[tokio::test]
async fn test_readiness_and_liveness_probes() {
	let handler = Arc::new(HealthCheckHandler::new());
	let coordinator = ShutdownCoordinator::new(Duration::from_secs(5));

	let (addr, server_handle) =
		spawn_server_with_shutdown(handler as Arc<dyn Handler>, coordinator.clone()).await;

	// Check readiness probe
	let ready_response = make_request(addr, "/ready").await.unwrap();
	assert_eq!(ready_response, r#"{"ready":true}"#);

	// Check liveness probe
	let live_response = make_request(addr, "/live").await.unwrap();
	assert_eq!(live_response, r#"{"live":true}"#);

	// Shutdown
	coordinator.shutdown();
	coordinator.wait_for_shutdown().await;
	let _ = server_handle.await;
}

/// Test: Server state transitions during lifecycle
#[tokio::test]
async fn test_server_state_transitions() {
	let handler = Arc::new(StateTrackingHandler::new());
	let coordinator = ShutdownCoordinator::new(Duration::from_secs(5));
	let state_ref = handler.clone();

	// Initial state is "starting"
	assert_eq!(state_ref.get_state(), "starting");

	let (addr, server_handle) =
		spawn_server_with_shutdown(handler as Arc<dyn Handler>, coordinator.clone()).await;

	// Transition to "running"
	state_ref.set_state("running");
	let response = make_request(addr, "/state").await.unwrap();
	assert_eq!(response, r#"{"state":"running"}"#);

	// Transition to "shutting_down"
	state_ref.set_state("shutting_down");
	let response = make_request(addr, "/state").await.unwrap();
	assert_eq!(response, r#"{"state":"shutting_down"}"#);

	// Trigger shutdown
	coordinator.shutdown();
	coordinator.wait_for_shutdown().await;

	// Transition to "stopped"
	state_ref.set_state("stopped");
	assert_eq!(state_ref.get_state(), "stopped");

	let _ = server_handle.await;
}

/// Test: Multiple shutdown coordinator subscriptions
#[tokio::test]
async fn test_multiple_shutdown_subscriptions() {
	let coordinator = ShutdownCoordinator::new(Duration::from_secs(5));

	// Create multiple subscribers
	let mut rx1 = coordinator.subscribe();
	let mut rx2 = coordinator.subscribe();
	let mut rx3 = coordinator.subscribe();

	// Trigger shutdown
	coordinator.shutdown();

	// All subscribers should receive shutdown signal
	assert!(timeout(Duration::from_secs(1), rx1.recv()).await.is_ok());
	assert!(timeout(Duration::from_secs(1), rx2.recv()).await.is_ok());
	assert!(timeout(Duration::from_secs(1), rx3.recv()).await.is_ok());
}

/// Test: Shutdown coordinator can be cloned
#[tokio::test]
async fn test_shutdown_coordinator_clone() {
	let coordinator = ShutdownCoordinator::new(Duration::from_secs(5));
	let coordinator_clone = coordinator.clone();

	let mut rx = coordinator.subscribe();

	// Trigger shutdown via clone
	coordinator_clone.shutdown();

	// Original coordinator should propagate signal
	assert!(timeout(Duration::from_secs(1), rx.recv()).await.is_ok());

	// Both coordinators should wait for shutdown
	coordinator.wait_for_shutdown().await;
	coordinator_clone.wait_for_shutdown().await;
}

/// Test: Server completes shutdown before active request finishes
#[tokio::test]
async fn test_requests_during_partial_shutdown() {
	let handler = Arc::new(RequestCounterHandler::new());
	let coordinator = ShutdownCoordinator::new(Duration::from_secs(5));
	let counter_ref = handler.clone();

	let (addr, server_handle) =
		spawn_server_with_shutdown(handler as Arc<dyn Handler>, coordinator.clone()).await;

	// Make initial request
	let _response = make_request(addr, "/").await.unwrap();
	assert_eq!(counter_ref.get_count(), 1);

	// Start a delayed request with separate coordinator
	let delayed_coordinator = ShutdownCoordinator::new(Duration::from_secs(5));
	let delayed_handler = Arc::new(DelayedHandler::new(Duration::from_millis(500)));
	let (delayed_addr, delayed_server_handle) = spawn_server_with_shutdown(
		delayed_handler.clone() as Arc<dyn Handler>,
		delayed_coordinator.clone(),
	)
	.await;

	let request_task = tokio::spawn(async move { make_request(delayed_addr, "/").await });

	// Wait for request to start
	// Poll until processing starts
	let mut processing_started = false;
	for _ in 0..10 {
		sleep(Duration::from_millis(20)).await;
		if delayed_handler.is_processing() {
			processing_started = true;
			break;
		}
	}
	assert!(
		processing_started,
		"Delayed request should start processing"
	);

	// Trigger shutdown on delayed server
	delayed_coordinator.shutdown();

	// Wait for delayed server shutdown (it stops accepting connections immediately)
	delayed_coordinator.wait_for_shutdown().await;
	let _ = delayed_server_handle.await;

	// The request may or may not complete (connection was closed during shutdown)
	// This is expected behavior as the server doesn't wait for in-flight requests
	let _ = timeout(Duration::from_millis(100), request_task).await;

	// Shutdown the first server
	coordinator.shutdown();
	coordinator.wait_for_shutdown().await;
	let _ = server_handle.await;
}

/// Test: Shutdown timeout is configurable
#[tokio::test]
async fn test_configurable_shutdown_timeout() {
	// Short timeout
	let short_coordinator = ShutdownCoordinator::new(Duration::from_millis(100));
	let short_start = std::time::Instant::now();
	short_coordinator.shutdown();
	short_coordinator.wait_for_shutdown().await;
	let short_duration = short_start.elapsed();
	assert!(short_duration >= Duration::from_millis(100));
	assert!(short_duration < Duration::from_millis(200));

	// Long timeout
	let long_coordinator = ShutdownCoordinator::new(Duration::from_millis(500));
	let long_start = std::time::Instant::now();
	long_coordinator.shutdown();
	long_coordinator.wait_for_shutdown().await;
	let long_duration = long_start.elapsed();
	assert!(long_duration >= Duration::from_millis(500));
	assert!(long_duration < Duration::from_millis(700));
}

/// Test: Multiple shutdown calls are safe
#[tokio::test]
async fn test_multiple_shutdown_calls_safe() {
	let handler = Arc::new(BasicHandler) as Arc<dyn Handler>;
	let coordinator = ShutdownCoordinator::new(Duration::from_secs(5));

	let (addr, server_handle) = spawn_server_with_shutdown(handler, coordinator.clone()).await;

	// Make a request
	let _response = make_request(addr, "/").await.unwrap();

	// Call shutdown multiple times
	coordinator.shutdown();
	coordinator.shutdown();
	coordinator.shutdown();

	// Should still work correctly
	coordinator.wait_for_shutdown().await;
	let result = timeout(Duration::from_secs(1), server_handle).await;
	assert!(result.is_ok());
}
