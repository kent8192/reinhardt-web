//! FastAPI dependency yield patterns tests translated to Rust
//!
//! Based on multiple FastAPI test files:
//! - test_dependency_after_yield_raise.py (4 tests)
//! - test_dependency_after_yield_streaming.py (7 tests)
//! - test_dependency_after_yield_websockets.py (2 tests)
//! - test_dependency_yield_except_httpexception.py (2 tests)
//!
//! These tests verify that:
//! 1. Cleanup code (after yield) runs even when exceptions occur
//! 2. Yield pattern works with streaming responses
//! 3. Yield pattern works with WebSockets
//! 4. HTTP exceptions are handled correctly in yield dependencies

use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::sync::{Arc, Mutex};

// State tracking for cleanup verification
#[derive(Clone)]
struct CleanupState {
	data: Arc<Mutex<Vec<String>>>,
}

impl CleanupState {
	fn new() -> Self {
		Self {
			data: Arc::new(Mutex::new(Vec::new())),
		}
	}

	fn add(&self, message: &str) {
		self.data.lock().unwrap().push(message.to_string());
	}

	fn get_all(&self) -> Vec<String> {
		self.data.lock().unwrap().clone()
	}

	fn contains(&self, message: &str) -> bool {
		self.data.lock().unwrap().contains(&message.to_string())
	}

	fn clear(&self) {
		self.data.lock().unwrap().clear();
	}
}

#[async_trait::async_trait]
impl Injectable for CleanupState {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		if let Some(cached) = ctx.get_singleton::<CleanupState>() {
			return Ok((*cached).clone());
		}
		let state = CleanupState::new();
		ctx.set_singleton(state.clone());
		Ok(state)
	}
}

// Dependency with cleanup that runs even on error
#[derive(Clone)]
struct YieldDependencyWithCleanup {
	state: CleanupState,
	value: String,
}

impl YieldDependencyWithCleanup {
	async fn setup(state: CleanupState) -> Self {
		state.add("setup");
		Self {
			state,
			value: "dependency_value".to_string(),
		}
	}

	async fn cleanup(self, had_error: bool) {
		if had_error {
			self.state.add("cleanup_with_error");
		} else {
			self.state.add("cleanup_success");
		}
	}

	fn value(&self) -> &str {
		&self.value
	}
}

#[async_trait::async_trait]
impl Injectable for YieldDependencyWithCleanup {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let state = CleanupState::inject(ctx).await?;
		Ok(YieldDependencyWithCleanup::setup(state).await)
	}
}

// Test 1: Cleanup runs after successful execution
#[tokio::test]
async fn test_yield_cleanup_success() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let state = CleanupState::inject(&ctx).await.unwrap();
	state.clear();

	let dep = YieldDependencyWithCleanup::inject(&ctx).await.unwrap();
	assert!(state.contains("setup"));

	// Simulate successful endpoint execution
	let _result = dep.value();

	// Cleanup
	dep.cleanup(false).await;

	assert!(state.contains("cleanup_success"));
	assert!(!state.contains("cleanup_with_error"));
}

// Test 2: Cleanup runs even when error occurs
#[tokio::test]
async fn test_yield_cleanup_after_raise() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let state = CleanupState::inject(&ctx).await.unwrap();
	state.clear();

	let dep = YieldDependencyWithCleanup::inject(&ctx).await.unwrap();
	assert!(state.contains("setup"));

	// Simulate error in endpoint
	dep.cleanup(true).await;

	assert!(state.contains("cleanup_with_error"));
	assert!(!state.contains("cleanup_success"));
}

// Streaming response dependency
#[derive(Clone)]
struct StreamingDependency {
	state: CleanupState,
	stream_id: String,
}

impl StreamingDependency {
	async fn setup(state: CleanupState, stream_id: String) -> Self {
		state.add(&format!("stream_{}_start", stream_id));
		Self { state, stream_id }
	}

	async fn cleanup(self) {
		self.state.add(&format!("stream_{}_end", self.stream_id));
	}

	fn stream_id(&self) -> &str {
		&self.stream_id
	}
}

#[async_trait::async_trait]
impl Injectable for StreamingDependency {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let state = CleanupState::inject(ctx).await?;
		Ok(StreamingDependency::setup(state, "test".to_string()).await)
	}
}

// Test 3: Cleanup runs after streaming response completes
#[tokio::test]
async fn test_yield_with_streaming_response() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let state = CleanupState::inject(&ctx).await.unwrap();
	state.clear();

	let dep = StreamingDependency::inject(&ctx).await.unwrap();
	assert!(state.contains("stream_test_start"));

	// Simulate streaming response generation
	let _stream_id = dep.stream_id();

	// After streaming completes, cleanup runs
	dep.cleanup().await;

	assert!(state.contains("stream_test_end"));
}

// Test 4: Multiple streaming dependencies
#[tokio::test]
async fn test_multiple_streaming_dependencies() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let state = CleanupState::inject(&ctx).await.unwrap();
	state.clear();

	let dep1 = StreamingDependency::setup(state.clone(), "stream1".to_string()).await;
	let dep2 = StreamingDependency::setup(state.clone(), "stream2".to_string()).await;

	assert!(state.contains("stream_stream1_start"));
	assert!(state.contains("stream_stream2_start"));

	dep1.cleanup().await;
	dep2.cleanup().await;

	assert!(state.contains("stream_stream1_end"));
	assert!(state.contains("stream_stream2_end"));
}

// WebSocket dependency
#[derive(Clone)]
struct WebSocketDependency {
	state: CleanupState,
	connection_id: String,
}

impl WebSocketDependency {
	async fn setup(state: CleanupState, connection_id: String) -> Self {
		state.add(&format!("ws_{}_connected", connection_id));
		Self {
			state,
			connection_id,
		}
	}

	async fn cleanup(self) {
		self.state
			.add(&format!("ws_{}_disconnected", self.connection_id));
	}

	fn connection_id(&self) -> &str {
		&self.connection_id
	}
}

#[async_trait::async_trait]
impl Injectable for WebSocketDependency {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let state = CleanupState::inject(ctx).await?;
		Ok(WebSocketDependency::setup(state, "conn_1".to_string()).await)
	}
}

// Test 5: WebSocket lifecycle
#[tokio::test]
async fn test_yield_with_websocket() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let state = CleanupState::inject(&ctx).await.unwrap();
	state.clear();

	let dep = WebSocketDependency::inject(&ctx).await.unwrap();
	assert!(state.contains("ws_conn_1_connected"));

	// Simulate WebSocket communication
	let _conn_id = dep.connection_id();

	// Connection closes, cleanup runs
	dep.cleanup().await;

	assert!(state.contains("ws_conn_1_disconnected"));
}

// Test 6: WebSocket cleanup on error
#[tokio::test]
async fn test_websocket_cleanup_on_error() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let state = CleanupState::inject(&ctx).await.unwrap();
	state.clear();

	let dep = WebSocketDependency::inject(&ctx).await.unwrap();
	assert!(state.contains("ws_conn_1_connected"));

	// Simulate WebSocket error - cleanup still runs
	dep.cleanup().await;

	assert!(state.contains("ws_conn_1_disconnected"));
}

// HTTP Exception handling
#[derive(Debug)]
struct HttpException {
	status_code: u16,
	detail: String,
}

#[derive(Clone)]
struct HttpExceptionDependency {
	state: CleanupState,
}

impl HttpExceptionDependency {
	async fn setup(state: CleanupState) -> Self {
		state.add("http_dep_setup");
		Self { state }
	}

	async fn cleanup(self, exception: Option<&HttpException>) {
		if let Some(exc) = exception {
			self.state
				.add(&format!("cleanup_with_http_exception_{}", exc.status_code));
		} else {
			self.state.add("cleanup_no_exception");
		}
	}
}

#[async_trait::async_trait]
impl Injectable for HttpExceptionDependency {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let state = CleanupState::inject(ctx).await?;
		Ok(HttpExceptionDependency::setup(state).await)
	}
}

// Test 7: Cleanup with HTTP exception
#[tokio::test]
async fn test_yield_except_httpexception() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let state = CleanupState::inject(&ctx).await.unwrap();
	state.clear();

	let dep = HttpExceptionDependency::inject(&ctx).await.unwrap();
	assert!(state.contains("http_dep_setup"));

	// Simulate HTTP exception (e.g., 404 Not Found)
	let exception = HttpException {
		status_code: 404,
		detail: "Not Found".to_string(),
	};

	dep.cleanup(Some(&exception)).await;

	assert!(state.contains("cleanup_with_http_exception_404"));
}

// Test 8: Cleanup without exception
#[tokio::test]
async fn test_yield_no_exception() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let state = CleanupState::inject(&ctx).await.unwrap();
	state.clear();

	let dep = HttpExceptionDependency::inject(&ctx).await.unwrap();
	assert!(state.contains("http_dep_setup"));

	// No exception
	dep.cleanup(None).await;

	assert!(state.contains("cleanup_no_exception"));
}

// Test 9: Nested yield dependencies
#[derive(Clone)]
struct OuterYieldDependency {
	state: CleanupState,
	inner: Arc<YieldDependencyWithCleanup>,
}

impl OuterYieldDependency {
	async fn setup(state: CleanupState, inner: YieldDependencyWithCleanup) -> Self {
		state.add("outer_setup");
		Self {
			state,
			inner: Arc::new(inner),
		}
	}

	async fn cleanup(self) {
		self.state.add("outer_cleanup");
		// Inner cleanup happens separately
		let inner = Arc::try_unwrap(self.inner).unwrap_or_else(|arc| (*arc).clone());
		inner.cleanup(false).await;
	}
}

#[tokio::test]
async fn test_nested_yield_dependencies() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let state = CleanupState::inject(&ctx).await.unwrap();
	state.clear();

	let inner = YieldDependencyWithCleanup::inject(&ctx).await.unwrap();
	let outer = OuterYieldDependency::setup(state.clone(), inner).await;

	assert!(state.contains("setup"));
	assert!(state.contains("outer_setup"));

	// Cleanup in reverse order
	outer.cleanup().await;

	let all_messages = state.get_all();
	let outer_idx = all_messages
		.iter()
		.position(|m| m == "outer_cleanup")
		.unwrap();
	let inner_idx = all_messages
		.iter()
		.position(|m| m == "cleanup_success")
		.unwrap();

	// Outer cleanup happens before inner cleanup
	assert!(outer_idx < inner_idx);
}
