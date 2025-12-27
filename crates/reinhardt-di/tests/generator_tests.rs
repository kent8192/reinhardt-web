//! Tests for generator-based dependency resolution (generator feature)

#![cfg(feature = "generator")]

use reinhardt_di::generator::{DependencyGenerator, DependencyStream, RequestScopedGenerator};
use rstest::*;
use std::pin::Pin;

/// Test lazy dependency resolution using generator
#[rstest]
#[tokio::test]
async fn test_generator_lazy_resolution() {
	let mut generator = DependencyGenerator::new(|co| {
		Box::pin(async move {
			// Simulate lazy database connection
			let db = "Database".to_string();
			co.yield_(db).await;

			// Simulate lazy cache connection
			let cache = "Cache".to_string();
			co.yield_(cache).await;

			// Simulate lazy service initialization
			let service = "Service".to_string();
			co.yield_(service).await;

			"completed"
		}) as Pin<Box<dyn std::future::Future<Output = &str> + Send>>
	});

	// Consume dependencies one by one (lazy evaluation)
	assert_eq!(generator.next().await, Some("Database".to_string()));
	assert_eq!(generator.next().await, Some("Cache".to_string()));
	assert_eq!(generator.next().await, Some("Service".to_string()));
	assert_eq!(generator.next().await, None);
}

/// Test streaming dependencies from generator
#[rstest]
#[tokio::test]
async fn test_generator_streaming_dependencies() {
	let generator = DependencyGenerator::new(|co| {
		Box::pin(async move {
			// Stream dependencies as they become available
			for i in 1..=5 {
				let dep = format!("Dependency{}", i);
				co.yield_(dep).await;
			}
		}) as Pin<Box<dyn std::future::Future<Output = ()> + Send>>
	});

	// Collect all streamed dependencies
	let deps = generator.collect().await;
	assert_eq!(deps.len(), 5);
	assert_eq!(deps[0], "Dependency1");
	assert_eq!(deps[4], "Dependency5");
}

/// Test generator error handling
#[rstest]
#[tokio::test]
async fn test_generator_error_handling() {
	let mut generator = DependencyGenerator::new(|co| {
		Box::pin(async move {
			// Yield some successful dependencies
			co.yield_(Ok::<String, String>("Database".to_string()))
				.await;
			co.yield_(Ok("Cache".to_string())).await;

			// Simulate error
			co.yield_(Err("Connection failed".to_string())).await;

			"completed"
		}) as Pin<Box<dyn std::future::Future<Output = &str> + Send>>
	});

	// Process dependencies with error handling
	let result1 = generator.next().await;
	assert!(result1.is_some());
	assert!(result1.unwrap().is_ok());

	let result2 = generator.next().await;
	assert!(result2.is_some());
	assert!(result2.unwrap().is_ok());

	let result3 = generator.next().await;
	assert!(result3.is_some());
	assert!(result3.unwrap().is_err());

	let result4 = generator.next().await;
	assert!(result4.is_none());
}

/// Test generator cleanup and finalization
#[rstest]
#[tokio::test]
async fn test_generator_cleanup() {
	// Test DependencyStream
	let mut stream = DependencyStream::new(|co| {
		Box::pin(async move {
			co.yield_("resource1".to_string()).await;
			co.yield_("resource2".to_string()).await;
		}) as Pin<Box<dyn std::future::Future<Output = ()> + Send>>
	});

	// Consume all resources
	let r1 = stream.next().await;
	assert_eq!(r1, Some("resource1".to_string()));

	let r2 = stream.next().await;
	assert_eq!(r2, Some("resource2".to_string()));

	// Stream is exhausted
	let r3 = stream.next().await;
	assert_eq!(r3, None);

	// Test RequestScopedGenerator cleanup
	let request_stream = DependencyStream::new(|co| {
		Box::pin(async move {
			co.yield_("dep1".to_string()).await;
			co.yield_("dep2".to_string()).await;
		}) as Pin<Box<dyn std::future::Future<Output = ()> + Send>>
	});

	let mut request_gen = RequestScopedGenerator::new("req-123".to_string(), request_stream);

	assert_eq!(request_gen.request_id(), "req-123");
	assert_eq!(request_gen.resolve_next().await, Some("dep1".to_string()));
	assert_eq!(request_gen.resolve_next().await, Some("dep2".to_string()));
	assert_eq!(request_gen.resolve_next().await, None);
}
