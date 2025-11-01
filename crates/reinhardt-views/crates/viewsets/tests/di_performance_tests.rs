//! Performance Tests for Dependency Injection in ViewSets
//!
//! Tests the performance characteristics of DI system including:
//! - Injection speed
//! - Cache hit rates
//! - Memory efficiency
//! - Concurrent performance

use async_trait::async_trait;
use bytes::Bytes;
use hyper::{HeaderMap, Method, Uri, Version};
use reinhardt_apps::{Handler, Request, Response, Result};
use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use reinhardt_macros::{Injectable as InjectableDerive, endpoint};
use reinhardt_viewsets::{Action, ViewSet, ViewSetHandler};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::Instant;

// ============================================================================
// Category 1: Injection Speed (3 tests)
// ============================================================================

#[tokio::test]
async fn test_injection_speed_simple_service() {
	#[derive(Clone)]
	struct FastService {
		id: usize,
	}

	impl Default for FastService {
		fn default() -> Self {
			Self { id: 42 }
		}
	}

	#[derive(Clone, InjectableDerive)]
	struct FastViewSet {
		#[inject]
		service: FastService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Warm up
	for _ in 0..10 {
		let _ = FastViewSet::inject(&ctx).await.unwrap();
	}

	// Measure injection speed
	let start = Instant::now();
	let iterations = 1000;

	for _ in 0..iterations {
		let _ = FastViewSet::inject(&ctx).await.unwrap();
	}

	let elapsed = start.elapsed();
	let avg_time = elapsed / iterations;

	// Should complete in reasonable time (< 1ms per injection with caching)
	assert!(avg_time.as_micros() < 1000, "Injection took {:?}", avg_time);
}

#[tokio::test]
async fn test_injection_speed_multiple_dependencies() {
	#[derive(Clone)]
	struct Service1 {
		data: i32,
	}
	impl Default for Service1 {
		fn default() -> Self {
			Self { data: 1 }
		}
	}

	#[derive(Clone)]
	struct Service2 {
		data: i32,
	}
	impl Default for Service2 {
		fn default() -> Self {
			Self { data: 2 }
		}
	}

	#[derive(Clone)]
	struct Service3 {
		data: i32,
	}
	impl Default for Service3 {
		fn default() -> Self {
			Self { data: 3 }
		}
	}

	#[derive(Clone, InjectableDerive)]
	struct MultiDepsViewSet {
		#[inject]
		s1: Service1,
		#[inject]
		s2: Service2,
		#[inject]
		s3: Service3,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Warm up
	for _ in 0..10 {
		let _ = MultiDepsViewSet::inject(&ctx).await.unwrap();
	}

	// Measure injection speed with multiple dependencies
	let start = Instant::now();
	let iterations = 1000;

	for _ in 0..iterations {
		let _ = MultiDepsViewSet::inject(&ctx).await.unwrap();
	}

	let elapsed = start.elapsed();
	let avg_time = elapsed / iterations;

	// Multiple dependencies should still be fast due to caching
	assert!(avg_time.as_micros() < 2000, "Injection took {:?}", avg_time);
}

#[tokio::test]
async fn test_injection_speed_nested_dependencies() {
	#[derive(Clone)]
	struct InnerService {
		value: i32,
	}
	impl Default for InnerService {
		fn default() -> Self {
			Self { value: 10 }
		}
	}

	#[derive(Clone, InjectableDerive)]
	struct MiddleService {
		#[inject]
		inner: InnerService,
	}

	#[derive(Clone, InjectableDerive)]
	struct OuterService {
		#[inject]
		middle: MiddleService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	// Warm up
	for _ in 0..10 {
		let _ = OuterService::inject(&ctx).await.unwrap();
	}

	// Measure nested injection speed
	let start = Instant::now();
	let iterations = 1000;

	for _ in 0..iterations {
		let _ = OuterService::inject(&ctx).await.unwrap();
	}

	let elapsed = start.elapsed();
	let avg_time = elapsed / iterations;

	// Nested dependencies should benefit from caching
	assert!(avg_time.as_micros() < 3000, "Injection took {:?}", avg_time);
}

// ============================================================================
// Category 2: Cache Performance (3 tests)
// ============================================================================

#[tokio::test]
async fn test_cache_hit_rate() {
	static CONSTRUCTION_COUNTER: LazyLock<AtomicUsize> = LazyLock::new(|| AtomicUsize::new(0));

	#[derive(Clone)]
	struct CachedService {
		id: usize,
	}

	impl Default for CachedService {
		fn default() -> Self {
			let count = CONSTRUCTION_COUNTER.fetch_add(1, Ordering::SeqCst);
			Self { id: count }
		}
	}

	#[derive(Clone, InjectableDerive)]
	struct CacheTestViewSet {
		#[inject]
		service: CachedService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let iterations = 100;
	let start_count = CONSTRUCTION_COUNTER.load(Ordering::SeqCst);

	for _ in 0..iterations {
		let viewset = CacheTestViewSet::inject(&ctx).await.unwrap();
		// All should have the same service ID due to caching
		assert_eq!(viewset.service.id, start_count);
	}

	let end_count = CONSTRUCTION_COUNTER.load(Ordering::SeqCst);
	let constructions = end_count - start_count;

	// Should only construct once (100% cache hit rate after first)
	assert_eq!(
		constructions, 1,
		"Expected 1 construction, got {}",
		constructions
	);
}

#[tokio::test]
async fn test_cache_miss_with_no_cache() {
	static NO_CACHE_COUNTER: LazyLock<AtomicUsize> = LazyLock::new(|| AtomicUsize::new(1000));

	#[derive(Clone)]
	struct FreshService {
		id: usize,
	}

	impl Default for FreshService {
		fn default() -> Self {
			let count = NO_CACHE_COUNTER.fetch_add(1, Ordering::SeqCst);
			Self { id: count }
		}
	}

	#[derive(Clone, InjectableDerive)]
	struct NoCacheViewSet {
		#[inject(cache = false)]
		service: FreshService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let iterations = 10;
	let start_count = NO_CACHE_COUNTER.load(Ordering::SeqCst);

	for _ in 0..iterations {
		let _ = NoCacheViewSet::inject(&ctx).await.unwrap();
	}

	let end_count = NO_CACHE_COUNTER.load(Ordering::SeqCst);
	let constructions = end_count - start_count;

	// Should construct fresh instance each time
	assert_eq!(
		constructions, iterations,
		"Expected {} constructions, got {}",
		iterations, constructions
	);
}

#[tokio::test]
async fn test_mixed_cache_behavior() {
	static MIXED_CACHED: LazyLock<AtomicUsize> = LazyLock::new(|| AtomicUsize::new(2000));
	static MIXED_FRESH: LazyLock<AtomicUsize> = LazyLock::new(|| AtomicUsize::new(3000));

	#[derive(Clone)]
	struct CachedService {
		id: usize,
	}
	impl Default for CachedService {
		fn default() -> Self {
			Self {
				id: MIXED_CACHED.fetch_add(1, Ordering::SeqCst),
			}
		}
	}

	#[derive(Clone)]
	struct FreshService {
		id: usize,
	}
	impl Default for FreshService {
		fn default() -> Self {
			Self {
				id: MIXED_FRESH.fetch_add(1, Ordering::SeqCst),
			}
		}
	}

	#[derive(Clone, InjectableDerive)]
	struct MixedViewSet {
		#[inject]
		cached: CachedService,
		#[inject(cache = false)]
		fresh: FreshService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let iterations = 10;
	let cached_start = MIXED_CACHED.load(Ordering::SeqCst);
	let fresh_start = MIXED_FRESH.load(Ordering::SeqCst);

	for _ in 0..iterations {
		let _ = MixedViewSet::inject(&ctx).await.unwrap();
	}

	let cached_constructions = MIXED_CACHED.load(Ordering::SeqCst) - cached_start;
	let fresh_constructions = MIXED_FRESH.load(Ordering::SeqCst) - fresh_start;

	// Cached should construct once, fresh should construct every time
	assert_eq!(cached_constructions, 1);
	assert_eq!(fresh_constructions, iterations);
}

// ============================================================================
// Category 3: Memory Efficiency (2 tests)
// ============================================================================

#[tokio::test]
async fn test_memory_reuse_with_caching() {
	use std::sync::LazyLock;
	use std::sync::atomic::{AtomicUsize, Ordering};

	static MEMORY_COUNTER: LazyLock<AtomicUsize> = LazyLock::new(|| AtomicUsize::new(3500));

	#[derive(Clone)]
	struct LargeService {
		id: usize,
		data: Vec<u8>,
	}

	impl Default for LargeService {
		fn default() -> Self {
			Self {
				id: MEMORY_COUNTER.fetch_add(1, Ordering::SeqCst),
				data: vec![0; 1024], // 1KB per instance
			}
		}
	}

	#[derive(Clone, InjectableDerive)]
	struct MemoryTestViewSet {
		#[inject]
		service: LargeService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = InjectionContext::new(singleton);

	let start_count = MEMORY_COUNTER.load(Ordering::SeqCst);

	// Create many instances - should all use the same cached service
	let mut viewsets = Vec::new();
	for _ in 0..100 {
		viewsets.push(MemoryTestViewSet::inject(&ctx).await.unwrap());
	}

	let end_count = MEMORY_COUNTER.load(Ordering::SeqCst);
	let constructions = end_count - start_count;

	// Should only construct once (all use cached instance)
	assert_eq!(
		constructions, 1,
		"Expected 1 construction, got {}",
		constructions
	);

	// All viewsets should have the same service ID
	let first_id = viewsets[0].service.id;
	for vs in &viewsets[1..] {
		assert_eq!(
			vs.service.id, first_id,
			"All services should have the same ID from cache"
		);
	}
}

#[tokio::test]
async fn test_context_cleanup() {
	static CLEANUP_COUNTER: LazyLock<AtomicUsize> = LazyLock::new(|| AtomicUsize::new(4000));

	#[derive(Clone)]
	struct CleanupService {
		id: usize,
	}

	impl Default for CleanupService {
		fn default() -> Self {
			Self {
				id: CLEANUP_COUNTER.fetch_add(1, Ordering::SeqCst),
			}
		}
	}

	#[derive(Clone, InjectableDerive)]
	struct CleanupViewSet {
		#[inject]
		service: CleanupService,
	}

	let start_count = CLEANUP_COUNTER.load(Ordering::SeqCst);

	// Create and drop multiple contexts
	for _ in 0..5 {
		let singleton = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton);
		let _ = CleanupViewSet::inject(&ctx).await.unwrap();
		// Context and its cache are dropped here
	}

	let end_count = CLEANUP_COUNTER.load(Ordering::SeqCst);
	let constructions = end_count - start_count;

	// Each context creates its own cached instance
	assert_eq!(
		constructions, 5,
		"Expected 5 constructions (one per context)"
	);
}

// ============================================================================
// Category 4: Concurrent Performance (2 tests)
// ============================================================================

#[tokio::test]
async fn test_concurrent_injection_performance() {
	#[derive(Clone)]
	struct ConcurrentService {
		id: usize,
	}

	impl Default for ConcurrentService {
		fn default() -> Self {
			Self { id: 100 }
		}
	}

	#[derive(Clone, InjectableDerive)]
	struct ConcurrentViewSet {
		#[inject]
		service: ConcurrentService,
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = Arc::new(InjectionContext::new(singleton));

	let start = Instant::now();
	let concurrency = 100;

	let mut handles = vec![];
	for _ in 0..concurrency {
		let ctx_clone = ctx.clone();
		handles.push(tokio::spawn(async move {
			ConcurrentViewSet::inject(&ctx_clone).await.unwrap()
		}));
	}

	let mut results = Vec::new();
	for handle in handles {
		results.push(handle.await.unwrap());
	}

	let elapsed = start.elapsed();

	// All concurrent injections should complete quickly
	assert!(
		elapsed.as_millis() < 1000,
		"Concurrent injections took {:?}",
		elapsed
	);

	// All should get the same cached service
	let first_id = results[0].service.id;
	for vs in &results[1..] {
		assert_eq!(vs.service.id, first_id);
	}
}

#[tokio::test]
async fn test_concurrent_handler_performance() {
	static HANDLER_COUNTER: LazyLock<AtomicUsize> = LazyLock::new(|| AtomicUsize::new(5000));

	#[derive(Clone)]
	struct HandlerService {
		id: usize,
	}

	impl Default for HandlerService {
		fn default() -> Self {
			Self {
				id: HANDLER_COUNTER.fetch_add(1, Ordering::SeqCst),
			}
		}
	}

	#[derive(Clone)]
	struct HandlerViewSet;

	impl HandlerViewSet {
		#[endpoint]
		async fn handle_impl(
			&self,
			_request: Request,
			_action: Action,
			#[inject] service: HandlerService,
		) -> DiResult<Response> {
			Ok(Response::ok().with_body(format!("id:{}", service.id)))
		}
	}

	#[async_trait]
	impl ViewSet for HandlerViewSet {
		fn get_basename(&self) -> &str {
			"handler"
		}

		fn supports_di(&self) -> bool {
			true
		}

		async fn dispatch(&self, _request: Request, _action: Action) -> Result<Response> {
			Err(reinhardt_apps::Error::Internal(
				"use dispatch_with_context".to_string(),
			))
		}

		async fn dispatch_with_context(
			&self,
			request: Request,
			action: Action,
			ctx: &InjectionContext,
		) -> Result<Response> {
			self.handle_impl(request, action, ctx)
				.await
				.map_err(|e| reinhardt_apps::Error::Internal(format!("DI error: {}", e)))
		}
	}

	let singleton = Arc::new(SingletonScope::new());
	let ctx = Arc::new(InjectionContext::new(singleton));

	let mut action_map = HashMap::new();
	action_map.insert(Method::GET, "handle".to_string());

	let handler = Arc::new(
		ViewSetHandler::new(Arc::new(HandlerViewSet), action_map, None, None).with_di_context(ctx),
	);

	let start = Instant::now();
	let concurrency = 50;

	let mut handles = vec![];
	for _ in 0..concurrency {
		let h = handler.clone();
		handles.push(tokio::spawn(async move {
			let req = Request::new(
				Method::GET,
				Uri::from_static("/handler/"),
				Version::HTTP_11,
				HeaderMap::new(),
				Bytes::new(),
			);
			h.handle(req).await.unwrap()
		}));
	}

	let mut responses = Vec::new();
	for handle in handles {
		responses.push(handle.await.unwrap());
	}

	let elapsed = start.elapsed();

	// Concurrent requests should complete in reasonable time
	assert!(
		elapsed.as_millis() < 2000,
		"Concurrent requests took {:?}",
		elapsed
	);

	// All responses should have the same ID (cached)
	let bodies: Vec<String> = responses
		.into_iter()
		.map(|r| String::from_utf8(r.body.to_vec()).unwrap())
		.collect();

	let first_body = &bodies[0];
	for body in &bodies[1..] {
		assert_eq!(body, first_body);
	}
}
