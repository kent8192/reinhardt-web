//! Tests for FastAPI-style Depends functionality

use reinhardt_di::{Depends, Injectable, InjectionContext, SingletonScope};
use serial_test::serial;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone, Default, Debug, PartialEq)]
struct CommonQueryParams {
    q: Option<String>,
    skip: usize,
    limit: usize,
}

#[derive(Clone, Default)]
struct Database {
    connection_count: usize,
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
async fn test_depends_with_cache_default() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    // Cache is enabled by default
    let params1 = Depends::<CommonQueryParams>::new()
        .resolve(&ctx)
        .await
        .unwrap();
    let params2 = Depends::<CommonQueryParams>::new()
        .resolve(&ctx)
        .await
        .unwrap();

    // Returns the same instance
    assert_eq!(*params1, *params2);
}

#[tokio::test]
#[serial(counted_service)]
async fn test_depends_no_cache() {
    // Reset counter for this test
    INSTANCE_COUNTER.store(0, Ordering::SeqCst);

    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    // Cache disabled
    let service1 = Depends::<CountedService>::no_cache()
        .resolve(&ctx)
        .await
        .unwrap();
    let service2 = Depends::<CountedService>::no_cache()
        .resolve(&ctx)
        .await
        .unwrap();

    // Different instances are created (IDs are sequential)
    assert_ne!(service1.instance_id, service2.instance_id);
    assert_eq!(service1.instance_id + 1, service2.instance_id);
}

#[tokio::test]
#[serial(counted_service)]
async fn test_depends_with_cache_enabled() {
    // Reset counter for this test
    INSTANCE_COUNTER.store(0, Ordering::SeqCst);

    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    // Cache enabled (default)
    let service1 = Depends::<CountedService>::new()
        .resolve(&ctx)
        .await
        .unwrap();
    let service2 = Depends::<CountedService>::new()
        .resolve(&ctx)
        .await
        .unwrap();

    // Returns the same instance (same ID)
    assert_eq!(service1.instance_id, service2.instance_id);
}

#[tokio::test]
async fn test_depends_from_value() {
    let db = Database {
        connection_count: 10,
    };
    let depends = Depends::from_value(db);

    assert_eq!(depends.connection_count, 10);
}

#[tokio::test]
async fn test_depends_deref() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    let params = Depends::<CommonQueryParams>::new()
        .resolve(&ctx)
        .await
        .unwrap();

    // Can access fields directly via Deref
    assert_eq!(params.skip, 0);
    assert_eq!(params.limit, 0);
}

#[tokio::test]
async fn test_fastapi_depends_clone() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    let params1 = Depends::<CommonQueryParams>::new()
        .resolve(&ctx)
        .await
        .unwrap();
    let params2 = params1.clone();

    // Clone copies the reference (Arc::clone)
    assert_eq!(*params1, *params2);
}

// FastAPI-style usage example
#[tokio::test]
async fn test_fastapi_style_usage() {
    #[derive(Clone, Default)]
    struct Config {
        api_key: String,
    }

    async fn endpoint_handler(
        config: Depends<Config>,
        params: Depends<CommonQueryParams>,
    ) -> String {
        format!("API Key: {}, Skip: {}", config.api_key, params.skip)
    }

    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    // Simulate endpoint usage
    let config = Depends::<Config>::new().resolve(&ctx).await.unwrap();
    let params = Depends::<CommonQueryParams>::new()
        .resolve(&ctx)
        .await
        .unwrap();

    let result = endpoint_handler(config, params).await;
    assert_eq!(result, "API Key: , Skip: 0");
}
