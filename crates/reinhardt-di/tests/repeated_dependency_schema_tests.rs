//! FastAPI repeated dependency schema tests translated to Rust
//!
//! Based on: fastapi/tests/test_repeated_dependency_schema.py
//!
//! These tests verify that:
//! 1. When a dependency is used multiple times (directly and transitively),
//!    it should only appear once in the schema
//! 2. Shared dependencies are properly cached and reused

use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::sync::Arc;

// Simulates Header extraction
#[derive(Clone, Debug, PartialEq)]
struct SomeHeader(String);

#[async_trait::async_trait]
impl Injectable for SomeHeader {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        // Check cache first
        if let Some(cached) = ctx.get_request::<SomeHeader>() {
            return Ok((*cached).clone());
        }

        // In a real implementation, this would extract from HTTP headers
        let header = SomeHeader("test-value".to_string());
        ctx.set_request(header.clone());
        Ok(header)
    }
}

// Dependency that depends on SomeHeader
#[derive(Clone, Debug, PartialEq)]
struct DerivedDependency(String);

#[async_trait::async_trait]
impl Injectable for DerivedDependency {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        let header = SomeHeader::inject(ctx).await?;
        Ok(DerivedDependency(format!("{}123", header.0)))
    }
}

#[tokio::test]
async fn test_repeated_dependency_uses_cache() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    // Set initial header value
    ctx.set_request(SomeHeader("hello".to_string()));

    // Inject both dependencies
    let dep1 = SomeHeader::inject(&ctx).await.unwrap();
    let dep2 = DerivedDependency::inject(&ctx).await.unwrap();

    // Verify values
    assert_eq!(dep1.0, "hello");
    assert_eq!(dep2.0, "hello123");
}

#[tokio::test]
async fn test_header_extracted_only_once() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    // Set header
    ctx.set_request(SomeHeader("test".to_string()));

    // Inject header multiple times
    let header1 = SomeHeader::inject(&ctx).await.unwrap();
    let header2 = SomeHeader::inject(&ctx).await.unwrap();

    // Both should be the same (cached)
    assert_eq!(header1, header2);
}

#[tokio::test]
async fn test_derived_dependency_uses_cached_header() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    // Set header
    ctx.set_request(SomeHeader("value".to_string()));

    // Inject header first
    let header = SomeHeader::inject(&ctx).await.unwrap();

    // Then inject derived dependency
    let derived = DerivedDependency::inject(&ctx).await.unwrap();

    // Derived should use the cached header
    assert_eq!(derived.0, format!("{}123", header.0));
}

// Test with multiple endpoints using the same dependencies
struct Endpoint1Result {
    dep1: SomeHeader,
    dep2: DerivedDependency,
}

struct Endpoint2Result {
    header: SomeHeader,
}

#[tokio::test]
async fn test_multiple_endpoints_share_dependencies() {
    let singleton = Arc::new(SingletonScope::new());

    // Request 1 - uses both dependencies
    let ctx1 = InjectionContext::new(singleton.clone());
    ctx1.set_request(SomeHeader("req1".to_string()));

    let result1 = Endpoint1Result {
        dep1: SomeHeader::inject(&ctx1).await.unwrap(),
        dep2: DerivedDependency::inject(&ctx1).await.unwrap(),
    };

    assert_eq!(result1.dep1.0, "req1");
    assert_eq!(result1.dep2.0, "req1123");

    // Request 2 - uses only header
    let ctx2 = InjectionContext::new(singleton.clone());
    ctx2.set_request(SomeHeader("req2".to_string()));

    let result2 = Endpoint2Result {
        header: SomeHeader::inject(&ctx2).await.unwrap(),
    };

    assert_eq!(result2.header.0, "req2");
}
