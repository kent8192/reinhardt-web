//! Tests for FastAPI-style Depends functionality

#![cfg(feature = "macros")]

use reinhardt_di::{Injectable, Injected, InjectionContext, SingletonScope, injectable};
use serial_test::serial;
use std::sync::atomic::{AtomicUsize, Ordering};

#[injectable]
#[derive(Clone, Default, Debug, PartialEq)]
struct CommonQueryParams {
	#[no_inject]
	q: Option<String>,
	#[no_inject]
	skip: usize,
	#[no_inject]
	limit: usize,
}

#[injectable]
#[derive(Clone, Default)]
struct Database {
	#[no_inject]
	connection_count: usize,
}

#[injectable]
#[derive(Clone, Default)]
struct Config {
	#[no_inject]
	api_key: String,
}

// Custom Injectable with instance counter (thread-safe using AtomicUsize)
static INSTANCE_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
struct CountedService {
	instance_id: usize,
}

#[async_trait::async_trait]
impl Injectable for CountedService {
	async fn inject(_ctx: &InjectionContext) -> reinhardt_di::DiResult<Self> {
		let instance_id = INSTANCE_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
		Ok(CountedService { instance_id })
	}
}

#[tokio::test]
async fn test_injected_with_cache_default() {
	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	// Cache is enabled by default
	let params1 = Injected::<CommonQueryParams>::resolve(&ctx).await.unwrap();
	let params2 = Injected::<CommonQueryParams>::resolve(&ctx).await.unwrap();

	// Returns the same instance
	assert_eq!(*params1, *params2);
}

#[tokio::test]
#[serial(counted_service)]
async fn test_injected_no_cache() {
	// Reset counter for this test
	INSTANCE_COUNTER.store(0, Ordering::SeqCst);

	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	// Cache disabled
	let service1 = Injected::<CountedService>::resolve_uncached(&ctx)
		.await
		.unwrap();
	let service2 = Injected::<CountedService>::resolve_uncached(&ctx)
		.await
		.unwrap();

	// Different instances are created (IDs are sequential)
	assert_ne!(service1.instance_id, service2.instance_id);
	assert_eq!(service1.instance_id + 1, service2.instance_id);
}

#[tokio::test]
#[serial(counted_service)]
async fn test_injected_with_cache_enabled() {
	// Reset counter for this test
	INSTANCE_COUNTER.store(0, Ordering::SeqCst);

	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	// Cache enabled (default)
	let service1 = Injected::<CountedService>::resolve(&ctx).await.unwrap();
	let service2 = Injected::<CountedService>::resolve(&ctx).await.unwrap();

	// Returns the same instance (same ID)
	assert_eq!(service1.instance_id, service2.instance_id);
}

#[tokio::test]
async fn test_injected_from_value() {
	let db = Database {
		connection_count: 10,
	};
	let injected = Injected::from_value(db);

	assert_eq!(injected.connection_count, 10);
}

#[tokio::test]
async fn test_injected_deref() {
	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	let params = Injected::<CommonQueryParams>::resolve(&ctx).await.unwrap();

	// Can access fields directly via Deref
	assert_eq!(params.skip, 0);
	assert_eq!(params.limit, 0);
}

#[tokio::test]
async fn test_fastapi_injected_clone() {
	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	let params1 = Injected::<CommonQueryParams>::resolve(&ctx).await.unwrap();
	let params2 = params1.clone();

	// Clone copies the reference (Arc::clone)
	assert_eq!(*params1, *params2);
}

// FastAPI-style usage example
#[tokio::test]
async fn test_fastapi_style_usage() {
	async fn endpoint_handler(
		config: Injected<Config>,
		params: Injected<CommonQueryParams>,
	) -> String {
		format!("API Key: {}, Skip: {}", config.api_key, params.skip)
	}

	let singleton = SingletonScope::new();
	let ctx = InjectionContext::builder(singleton).build();

	// Simulate endpoint usage
	let config = Injected::<Config>::resolve(&ctx).await.unwrap();
	let params = Injected::<CommonQueryParams>::resolve(&ctx).await.unwrap();

	let result = endpoint_handler(config, params).await;
	assert_eq!(result, "API Key: , Skip: 0");
}
