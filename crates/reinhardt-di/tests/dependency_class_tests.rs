//! FastAPI class-based dependency tests translated to Rust
//!
//! Based on: fastapi/tests/test_dependency_class.py
//!
//! These tests verify that:
//! 1. Callable structs can be used as dependencies
//! 2. Async methods work as dependencies
//! 3. Dependencies can have internal state
//! 4. Generator-like patterns (setup/teardown) work with dependencies

use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// Callable struct dependency (like Python's __call__)
#[derive(Clone)]
struct CallableDependency {
    prefix: String,
}

impl CallableDependency {
    fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
        }
    }

    fn call(&self, value: String) -> String {
        format!("{}{}", self.prefix, value)
    }
}

#[async_trait::async_trait]
impl Injectable for CallableDependency {
    async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
        Ok(CallableDependency::new("Hello, "))
    }
}

#[tokio::test]
async fn test_callable_dependency() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    let dep = CallableDependency::inject(&ctx).await.unwrap();
    let result = dep.call("World".to_string());

    assert_eq!(result, "Hello, World");
}

// Async callable struct
#[derive(Clone)]
struct AsyncCallableDependency {
    multiplier: i32,
}

impl AsyncCallableDependency {
    fn new(multiplier: i32) -> Self {
        Self { multiplier }
    }

    async fn call(&self, value: i32) -> i32 {
        // Simulate async work
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        value * self.multiplier
    }
}

#[async_trait::async_trait]
impl Injectable for AsyncCallableDependency {
    async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
        Ok(AsyncCallableDependency::new(10))
    }
}

#[tokio::test]
async fn test_async_callable_dependency() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    let dep = AsyncCallableDependency::inject(&ctx).await.unwrap();
    let result = dep.call(5).await;

    assert_eq!(result, 50);
}

// Class with internal state that changes
#[derive(Clone)]
struct StatefulDependency {
    instance_id: usize,
    counter_ref: Arc<AtomicUsize>,
}

impl StatefulDependency {
    fn new(counter_ref: Arc<AtomicUsize>) -> Self {
        let instance_id = counter_ref.fetch_add(1, Ordering::SeqCst);
        Self {
            instance_id,
            counter_ref,
        }
    }

    fn get_id(&self) -> usize {
        self.instance_id
    }
}

#[async_trait::async_trait]
impl Injectable for StatefulDependency {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        // Cache within request
        if let Some(cached) = ctx.get_request::<StatefulDependency>() {
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

        let dep = StatefulDependency::new(counter_ref);
        ctx.set_request(dep.clone());
        Ok(dep)
    }
}

#[tokio::test]
async fn test_stateful_dependency_cached() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    let dep1 = StatefulDependency::inject(&ctx).await.unwrap();
    let dep2 = StatefulDependency::inject(&ctx).await.unwrap();

    // Same instance within request
    assert_eq!(dep1.get_id(), dep2.get_id());
    assert_eq!(dep1.get_id(), 0);
}

#[tokio::test]
async fn test_stateful_dependency_separate_requests() {
    let singleton = Arc::new(SingletonScope::new());

    let ctx1 = InjectionContext::new(singleton.clone());
    let dep1 = StatefulDependency::inject(&ctx1).await.unwrap();

    let ctx2 = InjectionContext::new(singleton.clone());
    let dep2 = StatefulDependency::inject(&ctx2).await.unwrap();

    // Different instances across requests
    assert_ne!(dep1.get_id(), dep2.get_id());
    assert_eq!(dep1.get_id(), 0);
    assert_eq!(dep2.get_id(), 1);
}

// Method-based dependency (using a method of a struct)
#[derive(Clone)]
struct ServiceWithMethods {
    base_value: i32,
}

impl ServiceWithMethods {
    fn new(base_value: i32) -> Self {
        Self { base_value }
    }

    async fn method_dependency(&self) -> i32 {
        self.base_value + 100
    }
}

#[async_trait::async_trait]
impl Injectable for ServiceWithMethods {
    async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
        Ok(ServiceWithMethods::new(42))
    }
}

#[tokio::test]
async fn test_method_dependency() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    let service = ServiceWithMethods::inject(&ctx).await.unwrap();
    let result = service.method_dependency().await;

    assert_eq!(result, 142);
}

// Generator-like pattern (setup/teardown simulation)
#[derive(Clone)]
struct ResourceDependency {
    resource_id: String,
    setup_done: bool,
}

impl ResourceDependency {
    async fn setup() -> Self {
        // Simulate resource acquisition
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        Self {
            resource_id: "resource_123".to_string(),
            setup_done: true,
        }
    }

    fn get_resource(&self) -> &str {
        &self.resource_id
    }

    async fn teardown(self) {
        // Simulate resource cleanup
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        // Resource cleaned up
    }
}

#[async_trait::async_trait]
impl Injectable for ResourceDependency {
    async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
        Ok(ResourceDependency::setup().await)
    }
}

#[tokio::test]
async fn test_resource_dependency_lifecycle() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    let resource = ResourceDependency::inject(&ctx).await.unwrap();

    assert!(resource.setup_done);
    assert_eq!(resource.get_resource(), "resource_123");

    // Simulate cleanup
    resource.teardown().await;
}

// Dependency that combines multiple patterns
#[derive(Clone)]
struct ComplexDependency {
    callable_dep: Arc<CallableDependency>,
    async_dep: Arc<AsyncCallableDependency>,
}

#[async_trait::async_trait]
impl Injectable for ComplexDependency {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        let callable_dep = CallableDependency::inject(ctx).await?;
        let async_dep = AsyncCallableDependency::inject(ctx).await?;

        Ok(ComplexDependency {
            callable_dep: Arc::new(callable_dep),
            async_dep: Arc::new(async_dep),
        })
    }
}

impl ComplexDependency {
    async fn process(&self, text: String, number: i32) -> String {
        let formatted = self.callable_dep.call(text);
        let multiplied = self.async_dep.call(number).await;
        format!("{} = {}", formatted, multiplied)
    }
}

#[tokio::test]
async fn test_complex_dependency() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    let complex = ComplexDependency::inject(&ctx).await.unwrap();
    let result = complex.process("Result".to_string(), 7).await;

    assert_eq!(result, "Hello, Result = 70");
}

// Singleton class dependency (shared across all requests)
#[derive(Clone)]
struct SingletonService {
    instance_id: usize,
    counter_ref: Arc<AtomicUsize>,
}

impl SingletonService {
    fn new(counter_ref: Arc<AtomicUsize>) -> Self {
        let instance_id = counter_ref.fetch_add(1, Ordering::SeqCst);
        Self {
            instance_id,
            counter_ref,
        }
    }

    fn get_id(&self) -> usize {
        self.instance_id
    }
}

#[async_trait::async_trait]
impl Injectable for SingletonService {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        // Check singleton scope first
        if let Some(cached) = ctx.get_singleton::<SingletonService>() {
            return Ok((*cached).clone());
        }

        // Create counter ref for this test
        let counter_ref = Arc::new(AtomicUsize::new(0));
        let service = SingletonService::new(counter_ref);
        ctx.set_singleton(service.clone());
        Ok(service)
    }
}

#[tokio::test]
async fn test_singleton_service_shared() {
    let singleton = Arc::new(SingletonScope::new());

    // Request 1
    let ctx1 = InjectionContext::new(singleton.clone());
    let service1 = SingletonService::inject(&ctx1).await.unwrap();

    // Request 2
    let ctx2 = InjectionContext::new(singleton.clone());
    let service2 = SingletonService::inject(&ctx2).await.unwrap();

    // Same instance across requests (singleton)
    assert_eq!(service1.get_id(), service2.get_id());
    assert_eq!(service1.get_id(), 0);

    // Counter only incremented once
    assert_eq!(service1.counter_ref.load(Ordering::SeqCst), 1);
}
