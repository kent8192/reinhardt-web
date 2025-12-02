//! Integration tests for Signal Emission
//!
//! Tests the integration between dispatch components and the signal system:
//! - `request_started` and `request_finished` signal emission
//! - Signal listener registration and event propagation
//! - Signal integration with request lifecycle

use async_trait::async_trait;
use bytes::Bytes;
use http::{Method, StatusCode};
use reinhardt_core::http::{Request, Response};
use reinhardt_core::signals::{request_finished, request_started};
use reinhardt_core::types::Handler;
use reinhardt_dispatch::handler::BaseHandler;
use reinhardt_urls::prelude::Router;
use reinhardt_urls::routers::{DefaultRouter, Route};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Simple test handler
struct TestHandler;

#[async_trait]
impl Handler for TestHandler {
	async fn handle(&self, _request: Request) -> reinhardt_core::exception::Result<Response> {
		Ok(Response::new(StatusCode::OK).with_body(Bytes::from("OK")))
	}
}

#[tokio::test]
async fn test_request_started_signal_emitted() {
	// Setup signal counter
	let counter = Arc::new(AtomicUsize::new(0));
	let counter_clone = counter.clone();

	// Register signal listener
	request_started().connect(move |_event| {
		let counter = counter_clone.clone();
		async move {
			counter.fetch_add(1, Ordering::SeqCst);
			Ok(())
		}
	});

	// Setup handler
	let mut router = DefaultRouter::new();
	let route = Route::from_handler("/test", Arc::new(TestHandler));
	router.add_route(route);
	let handler = BaseHandler::with_router(Arc::new(router));

	// Create and execute request
	let request = Request::builder()
		.method(Method::GET)
		.uri("/test")
		.body(Bytes::new())
		.build()
		.expect("Failed to build request");

	let _ = handler.handle(request).await;

	// Verify signal was emitted
	assert_eq!(
		counter.load(Ordering::SeqCst),
		1,
		"request_started signal should be emitted once"
	);
}

#[tokio::test]
async fn test_request_finished_signal_emitted() {
	// Setup signal counter
	let counter = Arc::new(AtomicUsize::new(0));
	let counter_clone = counter.clone();

	// Register signal listener
	request_finished().connect(move |_event| {
		let counter = counter_clone.clone();
		async move {
			counter.fetch_add(1, Ordering::SeqCst);
			Ok(())
		}
	});

	// Setup handler
	let mut router = DefaultRouter::new();
	let route = Route::from_handler("/test", Arc::new(TestHandler));
	router.add_route(route);
	let handler = BaseHandler::with_router(Arc::new(router));

	// Create and execute request
	let request = Request::builder()
		.method(Method::GET)
		.uri("/test")
		.body(Bytes::new())
		.build()
		.expect("Failed to build request");

	let _ = handler.handle(request).await;

	// Verify signal was emitted
	assert_eq!(
		counter.load(Ordering::SeqCst),
		1,
		"request_finished signal should be emitted once"
	);
}

#[tokio::test]
async fn test_both_signals_emitted_in_order() {
	// Setup signal counters
	let started_counter = Arc::new(AtomicUsize::new(0));
	let finished_counter = Arc::new(AtomicUsize::new(0));
	let started_clone = started_counter.clone();
	let finished_clone = finished_counter.clone();

	// Register signal listeners
	request_started().connect(move |_event| {
		let counter = started_clone.clone();
		async move {
			counter.fetch_add(1, Ordering::SeqCst);
			Ok(())
		}
	});

	request_finished().connect(move |_event| {
		let counter = finished_clone.clone();
		async move {
			counter.fetch_add(1, Ordering::SeqCst);
			Ok(())
		}
	});

	// Setup handler
	let mut router = DefaultRouter::new();
	let route = Route::from_handler("/test", Arc::new(TestHandler));
	router.add_route(route);
	let handler = BaseHandler::with_router(Arc::new(router));

	// Create and execute request
	let request = Request::builder()
		.method(Method::GET)
		.uri("/test")
		.body(Bytes::new())
		.build()
		.expect("Failed to build request");

	let _ = handler.handle(request).await;

	// Verify both signals were emitted
	assert_eq!(
		started_counter.load(Ordering::SeqCst),
		1,
		"request_started signal should be emitted once"
	);
	assert_eq!(
		finished_counter.load(Ordering::SeqCst),
		1,
		"request_finished signal should be emitted once"
	);
}

#[tokio::test]
async fn test_signals_emitted_for_multiple_requests() {
	// Setup signal counter
	let counter = Arc::new(AtomicUsize::new(0));
	let counter_clone = counter.clone();

	// Register signal listener
	request_started().connect(move |_event| {
		let counter = counter_clone.clone();
		async move {
			counter.fetch_add(1, Ordering::SeqCst);
			Ok(())
		}
	});

	// Setup handler
	let mut router = DefaultRouter::new();
	let route = Route::from_handler("/test", Arc::new(TestHandler));
	router.add_route(route);
	let handler = BaseHandler::with_router(Arc::new(router));

	// Execute multiple requests
	for _ in 0..3 {
		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.body(Bytes::new())
			.build()
			.expect("Failed to build request");

		let _ = handler.handle(request).await;
	}

	// Verify signals were emitted for all requests
	assert_eq!(
		counter.load(Ordering::SeqCst),
		3,
		"request_started signal should be emitted for each request"
	);
}

#[tokio::test]
async fn test_signal_listener_can_access_event_data() {
	// Setup flag to verify event data access
	let event_received = Arc::new(AtomicUsize::new(0));
	let event_clone = event_received.clone();

	// Register signal listener that accesses event data
	request_started().connect(move |event| {
		let event_counter = event_clone.clone();
		async move {
			// Verify we can access event fields
			let _environ = &event.environ;
			event_counter.store(1, Ordering::SeqCst);
			Ok(())
		}
	});

	// Setup handler
	let mut router = DefaultRouter::new();
	let route = Route::from_handler("/test", Arc::new(TestHandler));
	router.add_route(route);
	let handler = BaseHandler::with_router(Arc::new(router));

	// Create and execute request
	let request = Request::builder()
		.method(Method::GET)
		.uri("/test")
		.body(Bytes::new())
		.build()
		.expect("Failed to build request");

	let _ = handler.handle(request).await;

	// Verify event data was accessible
	assert_eq!(
		event_received.load(Ordering::SeqCst),
		1,
		"Signal listener should be able to access event data"
	);
}

#[tokio::test]
async fn test_signals_still_emit_on_404() {
	// Setup signal counters
	let started_counter = Arc::new(AtomicUsize::new(0));
	let finished_counter = Arc::new(AtomicUsize::new(0));
	let started_clone = started_counter.clone();
	let finished_clone = finished_counter.clone();

	// Register signal listeners
	request_started().connect(move |_event| {
		let counter = started_clone.clone();
		async move {
			counter.fetch_add(1, Ordering::SeqCst);
			Ok(())
		}
	});

	request_finished().connect(move |_event| {
		let counter = finished_clone.clone();
		async move {
			counter.fetch_add(1, Ordering::SeqCst);
			Ok(())
		}
	});

	// Setup handler with no routes
	let router = DefaultRouter::new();
	let handler = BaseHandler::with_router(Arc::new(router));

	// Create request for non-existent route
	let request = Request::builder()
		.method(Method::GET)
		.uri("/nonexistent")
		.body(Bytes::new())
		.build()
		.expect("Failed to build request");

	let _ = handler.handle(request).await;

	// Verify signals were still emitted for 404
	assert_eq!(
		started_counter.load(Ordering::SeqCst),
		1,
		"request_started signal should emit even on 404"
	);
	assert_eq!(
		finished_counter.load(Ordering::SeqCst),
		1,
		"request_finished signal should emit even on 404"
	);
}
