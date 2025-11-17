//! FastAPI dependency cache tests translated to Rust
//!
//! Based on: fastapi/tests/test_dependency_cache.py
//!
//! These tests verify that:
//! 1. Dependencies are cached within a request by default
//! 2. Multiple uses of the same dependency return the same instance
//! 3. Nested dependencies share cached values

use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// Counter dependency - increments each time it's instantiated
#[derive(Clone)]
struct Counter {
	value: usize,
	counter_ref: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl Injectable for Counter {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// Check cache first
		if let Some(cached) = ctx.get_request::<Counter>() {
			return Ok((*cached).clone());
		}

		// Get or create counter ref from singleton scope
		let counter_ref = if let Some(cached_ref) = ctx.get_singleton::<Arc<AtomicUsize>>() {
			(*cached_ref).clone()
		} else {
			let new_ref = Arc::new(AtomicUsize::new(0));
			ctx.set_singleton(new_ref.clone());
			new_ref
		};

		// Increment counter and create new instance
		let value = counter_ref.fetch_add(1, Ordering::SeqCst) + 1;
		let counter = Counter { value, counter_ref };

		// Store in cache
		ctx.set_request(counter.clone());

		Ok(counter)
	}
}

// SuperDependency that depends on Counter
#[derive(Clone)]
struct SuperDep {
	counter_value: usize,
}

#[async_trait::async_trait]
impl Injectable for SuperDep {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let counter = Counter::inject(ctx).await?;
		Ok(SuperDep {
			counter_value: counter.value,
		})
	}
}

#[tokio::test]
async fn test_dependency_cached_within_request() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	// First injection
	let counter1 = Counter::inject(&ctx).await.unwrap();
	assert_eq!(counter1.value, 1);

	// Second injection - should return cached value
	let counter2 = Counter::inject(&ctx).await.unwrap();
	assert_eq!(counter2.value, 1); // Same value, not 2

	// Counter should only have incremented once
	assert_eq!(counter1.counter_ref.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_nested_dependencies_share_cache() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	// Inject SuperDep (which depends on Counter)
	let super_dep = SuperDep::inject(&ctx).await.unwrap();

	// Also inject Counter directly
	let counter = Counter::inject(&ctx).await.unwrap();

	// Both should have the same counter value because cache is shared
	assert_eq!(super_dep.counter_value, counter.value);
	assert_eq!(counter.value, 1);

	// Counter should only have incremented once
	assert_eq!(counter.counter_ref.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_separate_requests_have_separate_caches() {
	let singleton = Arc::new(SingletonScope::new());

	// Request 1
	let ctx1 = InjectionContext::builder(singleton.clone()).build();
	let counter1 = Counter::inject(&ctx1).await.unwrap();
	assert_eq!(counter1.value, 1);

	// Request 2 - new context, new cache
	let ctx2 = InjectionContext::builder(singleton.clone()).build();
	let counter2 = Counter::inject(&ctx2).await.unwrap();
	assert_eq!(counter2.value, 2); // Different value

	// Counter should have incremented twice
	assert_eq!(counter1.counter_ref.load(Ordering::SeqCst), 2);
}

// Test without caching - direct instantiation
#[derive(Clone)]
struct NoCacheCounter {
	value: usize,
}

static NO_CACHE_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[async_trait::async_trait]
impl Injectable for NoCacheCounter {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		// No caching - always create new instance
		let value = NO_CACHE_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
		Ok(NoCacheCounter { value })
	}
}

#[tokio::test]
async fn test_no_cache_creates_new_instances() {
	// Reset global counter
	NO_CACHE_COUNTER.store(0, Ordering::SeqCst);

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	// First injection
	let counter1 = NoCacheCounter::inject(&ctx).await.unwrap();
	assert_eq!(counter1.value, 1);

	// Second injection - should create new instance (no caching)
	let counter2 = NoCacheCounter::inject(&ctx).await.unwrap();
	assert_eq!(counter2.value, 2); // Different value

	// Global counter should have incremented twice
	assert_eq!(NO_CACHE_COUNTER.load(Ordering::SeqCst), 2);
}

// Multiple nested dependencies using the same cached dependency
#[derive(Clone)]
struct ServiceA {
	counter_value: usize,
}

#[derive(Clone)]
struct ServiceB {
	counter_value: usize,
}

#[derive(Clone)]
struct AggregateService {
	service_a_value: usize,
	service_b_value: usize,
}

#[async_trait::async_trait]
impl Injectable for ServiceA {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let counter = Counter::inject(ctx).await?;
		Ok(ServiceA {
			counter_value: counter.value,
		})
	}
}

#[async_trait::async_trait]
impl Injectable for ServiceB {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let counter = Counter::inject(ctx).await?;
		Ok(ServiceB {
			counter_value: counter.value,
		})
	}
}

#[async_trait::async_trait]
impl Injectable for AggregateService {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let service_a = ServiceA::inject(ctx).await?;
		let service_b = ServiceB::inject(ctx).await?;
		Ok(AggregateService {
			service_a_value: service_a.counter_value,
			service_b_value: service_b.counter_value,
		})
	}
}

#[tokio::test]
async fn test_multiple_services_share_cached_dependency() {
	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::builder(singleton).build();

	// Inject aggregate service that depends on ServiceA and ServiceB,
	// both of which depend on Counter
	let aggregate = AggregateService::inject(&ctx).await.unwrap();

	// All services should see the same counter value
	assert_eq!(aggregate.service_a_value, 1);
	assert_eq!(aggregate.service_b_value, 1);

	// Verify counter was only created once by checking the first Counter
	let counter = Counter::inject(&ctx).await.unwrap();
	assert_eq!(counter.counter_ref.load(Ordering::SeqCst), 1);
}
