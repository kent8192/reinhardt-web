//! FastAPI context manager (yield) dependency tests translated to Rust
//!
//! Based on: fastapi/tests/test_dependency_contextmanager.py
//!
//! These tests verify that:
//! 1. Dependencies can have setup and cleanup phases (yield pattern)
//! 2. Cleanup runs even when errors occur
//! 3. Nested context managers execute in the correct order
//! 4. State is properly shared between setup and cleanup

use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Shared state for tracking lifecycle
#[derive(Clone)]
struct SharedState {
	data: Arc<Mutex<HashMap<String, String>>>,
}

impl SharedState {
	fn new() -> Self {
		Self {
			data: Arc::new(Mutex::new(HashMap::new())),
		}
	}

	fn set(&self, key: &str, value: &str) {
		self.data
			.lock()
			.unwrap()
			.insert(key.to_string(), value.to_string());
	}

	fn get(&self, key: &str) -> String {
		self.data
			.lock()
			.unwrap()
			.get(key)
			.cloned()
			.unwrap_or_default()
	}
}

#[async_trait::async_trait]
impl Injectable for SharedState {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		if let Some(cached) = ctx.get_singleton::<SharedState>() {
			return Ok((*cached).clone());
		}
		let state = SharedState::new();
		ctx.set_singleton(state.clone());
		Ok(state)
	}
}

// AsyncGen dependency - simulates async generator with setup/cleanup
#[derive(Clone)]
struct AsyncGenDependency {
	state: SharedState,
	value: String,
}

impl AsyncGenDependency {
	async fn setup(state: SharedState) -> Self {
		state.set("/async", "asyncgen started");
		Self {
			state: state.clone(),
			value: "asyncgen started".to_string(),
		}
	}

	async fn cleanup(self) {
		self.state.set("/async", "asyncgen completed");
	}

	fn value(&self) -> &str {
		&self.value
	}
}

#[async_trait::async_trait]
impl Injectable for AsyncGenDependency {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let state = SharedState::inject(ctx).await?;
		Ok(AsyncGenDependency::setup(state).await)
	}
}

#[tokio::test]
async fn test_async_state_lifecycle() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let state = SharedState::inject(&ctx).await.unwrap();
	state.set("/async", "asyncgen not started");

	assert_eq!(state.get("/async"), "asyncgen not started");

	// Setup phase
	let dep = AsyncGenDependency::inject(&ctx).await.unwrap();
	assert_eq!(dep.value(), "asyncgen started");
	assert_eq!(state.get("/async"), "asyncgen started");

	// Cleanup phase
	dep.cleanup().await;
	assert_eq!(state.get("/async"), "asyncgen completed");
}

// Sync generator dependency
#[derive(Clone)]
struct SyncGenDependency {
	state: SharedState,
	value: String,
}

impl SyncGenDependency {
	fn setup(state: SharedState) -> Self {
		state.set("/sync", "generator started");
		Self {
			state: state.clone(),
			value: "generator started".to_string(),
		}
	}

	fn cleanup(self) {
		self.state.set("/sync", "generator completed");
	}

	fn value(&self) -> &str {
		&self.value
	}
}

#[async_trait::async_trait]
impl Injectable for SyncGenDependency {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let state = SharedState::inject(ctx).await?;
		Ok(SyncGenDependency::setup(state))
	}
}

#[tokio::test]
async fn test_sync_state_lifecycle() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let state = SharedState::inject(&ctx).await.unwrap();
	state.set("/sync", "generator not started");

	assert_eq!(state.get("/sync"), "generator not started");

	// Setup phase
	let dep = SyncGenDependency::inject(&ctx).await.unwrap();
	assert_eq!(dep.value(), "generator started");
	assert_eq!(state.get("/sync"), "generator started");

	// Cleanup phase
	dep.cleanup();
	assert_eq!(state.get("/sync"), "generator completed");
}

// Error tracking
#[derive(Clone)]
struct ErrorTracker {
	errors: Arc<Mutex<Vec<String>>>,
}

impl ErrorTracker {
	fn new() -> Self {
		Self {
			errors: Arc::new(Mutex::new(Vec::new())),
		}
	}

	fn add_error(&self, error: &str) {
		self.errors.lock().unwrap().push(error.to_string());
	}

	fn has_error(&self, error: &str) -> bool {
		self.errors.lock().unwrap().contains(&error.to_string())
	}

	fn clear(&self) {
		self.errors.lock().unwrap().clear();
	}
}

#[async_trait::async_trait]
impl Injectable for ErrorTracker {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		if let Some(cached) = ctx.get_singleton::<ErrorTracker>() {
			return Ok((*cached).clone());
		}
		let tracker = ErrorTracker::new();
		ctx.set_singleton(tracker.clone());
		Ok(tracker)
	}
}

// Custom errors
#[derive(Debug)]
struct AsyncDependencyError;
#[derive(Debug)]
struct SyncDependencyError;
#[derive(Debug)]
struct OtherDependencyError;

// AsyncGen with try/except/finally
#[derive(Clone)]
struct AsyncGenTryDependency {
	state: SharedState,
	error_tracker: ErrorTracker,
	value: String,
}

impl AsyncGenTryDependency {
	async fn setup(state: SharedState, error_tracker: ErrorTracker) -> Self {
		state.set("/async_raise", "asyncgen raise started");
		Self {
			state: state.clone(),
			error_tracker,
			value: "asyncgen raise started".to_string(),
		}
	}

	async fn cleanup_with_error(self, error: Option<&str>) {
		if let Some(err) = error {
			if err == "AsyncDependencyError" {
				self.error_tracker.add_error("/async_raise");
			}
		}
		// Finally block
		self.state.set("/async_raise", "asyncgen raise finalized");
	}

	fn value(&self) -> &str {
		&self.value
	}
}

#[async_trait::async_trait]
impl Injectable for AsyncGenTryDependency {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let state = SharedState::inject(ctx).await?;
		let error_tracker = ErrorTracker::inject(ctx).await?;
		Ok(AsyncGenTryDependency::setup(state, error_tracker).await)
	}
}

#[tokio::test]
async fn test_async_raise_other() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let state = SharedState::inject(&ctx).await.unwrap();
	let error_tracker = ErrorTracker::inject(&ctx).await.unwrap();

	state.set("/async_raise", "asyncgen raise not started");

	// Setup
	let dep = AsyncGenTryDependency::inject(&ctx).await.unwrap();
	assert_eq!(dep.value(), "asyncgen raise started");

	// Simulate OtherDependencyError (not caught by except block)
	dep.cleanup_with_error(Some("OtherDependencyError")).await;

	assert_eq!(state.get("/async_raise"), "asyncgen raise finalized");
	assert!(!error_tracker.has_error("/async_raise"));
}

#[tokio::test]
async fn test_async_raise_raises() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let state = SharedState::inject(&ctx).await.unwrap();
	let error_tracker = ErrorTracker::inject(&ctx).await.unwrap();

	state.set("/async_raise", "asyncgen raise not started");

	// Setup
	let dep = AsyncGenTryDependency::inject(&ctx).await.unwrap();
	assert_eq!(dep.value(), "asyncgen raise started");

	// Simulate AsyncDependencyError (caught by except block)
	dep.cleanup_with_error(Some("AsyncDependencyError")).await;

	assert_eq!(state.get("/async_raise"), "asyncgen raise finalized");
	assert!(error_tracker.has_error("/async_raise"));
	error_tracker.clear();
}

// Nested context managers (context_a and context_b)
#[derive(Clone)]
struct ContextA {
	state: SharedState,
}

impl ContextA {
	async fn setup(state: SharedState) -> Self {
		state.set("context_a", "started a");
		Self { state }
	}

	async fn cleanup(self) {
		self.state.set("context_a", "finished a");
	}

	fn get_state(&self) -> SharedState {
		self.state.clone()
	}
}

#[async_trait::async_trait]
impl Injectable for ContextA {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let state = SharedState::inject(ctx).await?;
		Ok(ContextA::setup(state).await)
	}
}

#[derive(Clone)]
struct ContextB {
	state: SharedState,
	context_a: Arc<ContextA>,
}

impl ContextB {
	async fn setup(context_a: ContextA) -> Self {
		let state = context_a.get_state();
		state.set("context_b", "started b");
		Self {
			state,
			context_a: Arc::new(context_a),
		}
	}

	async fn cleanup(self) {
		let context_a_state = self.state.get("context_a");
		self.state.set(
			"context_b",
			&format!("finished b with a: {}", context_a_state),
		);
		// Then cleanup context_a
		let context_a = Arc::try_unwrap(self.context_a).unwrap_or_else(|arc| (*arc).clone());
		context_a.cleanup().await;
	}

	fn get_state(&self) -> SharedState {
		self.state.clone()
	}
}

#[async_trait::async_trait]
impl Injectable for ContextB {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let context_a = ContextA::inject(ctx).await?;
		Ok(ContextB::setup(context_a).await)
	}
}

#[tokio::test]
async fn test_context_b() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let state = SharedState::inject(&ctx).await.unwrap();
	state.set("context_a", "not started a");
	state.set("context_b", "not started b");

	// Setup
	let context_b = ContextB::inject(&ctx).await.unwrap();

	assert_eq!(state.get("context_b"), "started b");
	assert_eq!(state.get("context_a"), "started a");

	// Cleanup
	context_b.cleanup().await;

	assert_eq!(state.get("context_b"), "finished b with a: started a");
	assert_eq!(state.get("context_a"), "finished a");
}

#[tokio::test]
async fn test_context_b_raise() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let state = SharedState::inject(&ctx).await.unwrap();
	state.set("context_a", "not started a");
	state.set("context_b", "not started b");

	// Setup
	let context_b = ContextB::inject(&ctx).await.unwrap();

	assert_eq!(state.get("context_b"), "started b");
	assert_eq!(state.get("context_a"), "started a");

	// Simulate error and cleanup
	context_b.cleanup().await;

	assert_eq!(state.get("context_b"), "finished b with a: started a");
	assert_eq!(state.get("context_a"), "finished a");
}

// Background tasks simulation
#[tokio::test]
async fn test_background_tasks() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let state = SharedState::inject(&ctx).await.unwrap();
	state.set("context_a", "not started a");
	state.set("context_b", "not started b");
	state.set("bg", "not set");

	// Setup
	let context_b = ContextB::inject(&ctx).await.unwrap();

	assert_eq!(state.get("context_b"), "started b");
	assert_eq!(state.get("context_a"), "started a");
	assert_eq!(state.get("bg"), "not set");

	// Simulate background task (runs after cleanup)
	let bg_state = context_b.get_state();
	let context_b_value = bg_state.get("context_b");
	let context_a_value = bg_state.get("context_a");

	// Cleanup
	context_b.cleanup().await;

	assert_eq!(state.get("context_b"), "finished b with a: started a");
	assert_eq!(state.get("context_a"), "finished a");

	// Background task runs with captured values
	bg_state.set(
		"bg",
		&format!("bg set - b: {} - a: {}", context_b_value, context_a_value),
	);
	assert_eq!(state.get("bg"), "bg set - b: started b - a: started a");
}

// Sync generator with try/except/finally
#[derive(Clone)]
struct SyncGenTryDependency {
	state: SharedState,
	error_tracker: ErrorTracker,
	value: String,
}

impl SyncGenTryDependency {
	fn setup(state: SharedState, error_tracker: ErrorTracker) -> Self {
		state.set("/sync_raise", "generator raise started");
		Self {
			state: state.clone(),
			error_tracker,
			value: "generator raise started".to_string(),
		}
	}

	fn cleanup_with_error(self, error: Option<&str>) {
		if let Some(err) = error {
			if err == "SyncDependencyError" {
				self.error_tracker.add_error("/sync_raise");
			}
		}
		// Finally block
		self.state.set("/sync_raise", "generator raise finalized");
	}

	fn value(&self) -> &str {
		&self.value
	}
}

#[async_trait::async_trait]
impl Injectable for SyncGenTryDependency {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let state = SharedState::inject(ctx).await?;
		let error_tracker = ErrorTracker::inject(ctx).await?;
		Ok(SyncGenTryDependency::setup(state, error_tracker))
	}
}

#[tokio::test]
async fn test_sync_raise_other() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let state = SharedState::inject(&ctx).await.unwrap();
	let error_tracker = ErrorTracker::inject(&ctx).await.unwrap();

	state.set("/sync_raise", "generator raise not started");

	// Setup
	let dep = SyncGenTryDependency::inject(&ctx).await.unwrap();
	assert_eq!(dep.value(), "generator raise started");

	// Simulate OtherDependencyError
	dep.cleanup_with_error(Some("OtherDependencyError"));

	assert_eq!(state.get("/sync_raise"), "generator raise finalized");
	assert!(!error_tracker.has_error("/sync_raise"));
}

#[tokio::test]
async fn test_sync_raise_raises() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let state = SharedState::inject(&ctx).await.unwrap();
	let error_tracker = ErrorTracker::inject(&ctx).await.unwrap();

	state.set("/sync_raise", "generator raise not started");

	// Setup
	let dep = SyncGenTryDependency::inject(&ctx).await.unwrap();
	assert_eq!(dep.value(), "generator raise started");

	// Simulate SyncDependencyError
	dep.cleanup_with_error(Some("SyncDependencyError"));

	assert_eq!(state.get("/sync_raise"), "generator raise finalized");
	assert!(error_tracker.has_error("/sync_raise"));
	error_tracker.clear();
}
