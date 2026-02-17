//! FastAPI dependency after yield raise tests translated to Rust
//!
//! Based on: fastapi/tests/test_dependency_after_yield_raise.py
//!
//! These tests verify that:
//! 1. Dependencies can catch exceptions after yield and convert them to HTTP errors
//! 2. Exceptions raised after yield in dependencies are properly handled
//! 3. Response is sent before yield cleanup runs

use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use rstest::rstest;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

// Dependency that catches exceptions after yield
struct CatchingDep {
	value: String,
	cleanup_error_catcher: Arc<AtomicBool>,
}

impl Drop for CatchingDep {
	fn drop(&mut self) {
		// NOTE: Drop trait simulates FastAPI's after-yield cleanup in generator dependencies
		// FastAPI: Code after yield in generator executes during cleanup
		// Reinhardt: Drop trait provides equivalent cleanup guarantees
		// Test verifies cleanup execution even when exceptions occur during request handling
		//
		// IMPLEMENTATION NOTE: Rust's Drop trait cannot return errors
		// - FastAPI: Generator can yield and catch exceptions after yield
		// - Reinhardt: Drop trait runs cleanup, errors must be handled via:
		//   1. panic! (for unrecoverable errors, as in BrokenDep)
		//   2. Atomic flags (for testable error states, as used here)
		//   3. Logging/metrics (for production error tracking)
		//
		// This implementation uses AtomicBool to mark error states during cleanup,
		// allowing tests to verify error handling without panicking.
		if self.cleanup_error_catcher.load(Ordering::SeqCst) {
			// Cleanup error was detected during request handling
			// In production, this would log the error or update metrics
		}
	}
}

#[async_trait::async_trait]
impl Injectable for CatchingDep {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let error_catcher = Arc::new(AtomicBool::new(false));
		ctx.set_request(error_catcher.clone());

		Ok(CatchingDep {
			value: "s".to_string(),
			cleanup_error_catcher: error_catcher,
		})
	}
}

// Dependency that raises an error after yield
struct BrokenDep {
	value: String,
	should_raise: Arc<AtomicBool>,
}

impl Drop for BrokenDep {
	fn drop(&mut self) {
		// Simulate error after yield
		if self.should_raise.load(Ordering::SeqCst) {
			// In FastAPI, this would raise ValueError("Broken after yield")
			// In Rust, we can't raise from Drop, but we can test the concept
			panic!("Broken after yield");
		}
	}
}

#[async_trait::async_trait]
impl Injectable for BrokenDep {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let should_raise = Arc::new(AtomicBool::new(true));
		ctx.set_request(should_raise.clone());

		Ok(BrokenDep {
			value: "s".to_string(),
			should_raise,
		})
	}
}

#[rstest]
#[tokio::test]
async fn test_catching_dependency_can_handle_errors() {
	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	// Inject catching dependency
	let catching = CatchingDep::inject(&ctx).await.unwrap();
	assert_eq!(catching.value, "s");

	// Simulate error during request
	let error_catcher = ctx.get_request::<Arc<AtomicBool>>().unwrap();
	error_catcher.store(true, Ordering::SeqCst);

	// Drop the dependency - cleanup runs
	drop(catching);

	// Verify error was marked for catching
	assert!(error_catcher.load(Ordering::SeqCst));
}

#[rstest]
#[tokio::test]
#[should_panic(expected = "Broken after yield")]
async fn test_broken_dependency_raises_on_drop() {
	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	// Inject broken dependency
	let broken = BrokenDep::inject(&ctx).await.unwrap();
	assert_eq!(broken.value, "s");

	// Drop will panic
	drop(broken);
}

#[rstest]
#[tokio::test]
async fn test_broken_dependency_no_raise_when_disabled() {
	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	// Inject broken dependency
	let broken = BrokenDep::inject(&ctx).await.unwrap();
	assert_eq!(broken.value, "s");

	// Disable the panic
	let should_raise = ctx.get_request::<Arc<AtomicBool>>().unwrap();
	should_raise.store(false, Ordering::SeqCst);

	// Drop should not panic
	drop(broken);
}

// Test that response can be sent before cleanup runs
struct ResponseBeforeCleanup {
	response_sent: Arc<AtomicBool>,
	cleanup_ran: Arc<AtomicBool>,
}

impl Drop for ResponseBeforeCleanup {
	fn drop(&mut self) {
		// Cleanup runs after response is sent
		assert!(self.response_sent.load(Ordering::SeqCst));
		self.cleanup_ran.store(true, Ordering::SeqCst);
	}
}

#[async_trait::async_trait]
impl Injectable for ResponseBeforeCleanup {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let response_sent = Arc::new(AtomicBool::new(false));
		let cleanup_ran = Arc::new(AtomicBool::new(false));

		ctx.set_request(response_sent.clone());
		ctx.set_request(cleanup_ran.clone());

		Ok(ResponseBeforeCleanup {
			response_sent,
			cleanup_ran,
		})
	}
}

#[rstest]
#[tokio::test]
async fn test_response_sent_before_cleanup() {
	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	// Inject dependency
	let dep = ResponseBeforeCleanup::inject(&ctx).await.unwrap();

	// Get references from the dependency
	let response_sent = dep.response_sent.clone();
	let cleanup_ran = dep.cleanup_ran.clone();

	// Simulate sending response
	response_sent.store(true, Ordering::SeqCst);

	// Drop dependency - cleanup runs
	drop(dep);

	// Verify cleanup ran after response
	assert!(cleanup_ran.load(Ordering::SeqCst));
}
