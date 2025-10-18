//! FastAPI WebSocket dependencies tests translated to Rust
//!
//! Based on: fastapi/tests/test_ws_dependencies.py
//!
//! These tests verify that:
//! 1. WebSocket endpoints can use dependencies
//! 2. Dependencies are executed in correct order (app -> router -> endpoint)
//! 3. Multiple routers with dependencies work correctly

use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::sync::{Arc, Mutex};

// Dependency list that tracks execution order
#[derive(Clone, Debug)]
struct DependencyList {
    items: Arc<Mutex<Vec<String>>>,
}

impl DependencyList {
    fn new() -> Self {
        DependencyList {
            items: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn append(&self, name: String) {
        self.items.lock().unwrap().push(name);
    }

    fn get_items(&self) -> Vec<String> {
        self.items.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl Injectable for DependencyList {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        // Check cache first (important for sharing across dependencies)
        if let Some(cached) = ctx.get_request::<DependencyList>() {
            return Ok((*cached).clone());
        }

        let list = DependencyList::new();
        ctx.set_request(list.clone());
        Ok(list)
    }
}

// App-level dependency
struct AppDependency;

#[async_trait::async_trait]
impl Injectable for AppDependency {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        let list = DependencyList::inject(ctx).await?;
        list.append("app".to_string());
        Ok(AppDependency)
    }
}

// Router-level dependency
struct RouterDependency;

#[async_trait::async_trait]
impl Injectable for RouterDependency {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        let list = DependencyList::inject(ctx).await?;
        list.append("router".to_string());
        Ok(RouterDependency)
    }
}

// Router2-level dependency (for include_router)
struct Router2Dependency;

#[async_trait::async_trait]
impl Injectable for Router2Dependency {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        let list = DependencyList::inject(ctx).await?;
        list.append("router2".to_string());
        Ok(Router2Dependency)
    }
}

// Endpoint-level dependency
struct IndexDependency;

#[async_trait::async_trait]
impl Injectable for IndexDependency {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        let list = DependencyList::inject(ctx).await?;
        list.append("index".to_string());
        Ok(IndexDependency)
    }
}

// Router index dependency
struct RouterIndexDependency;

#[async_trait::async_trait]
impl Injectable for RouterIndexDependency {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        let list = DependencyList::inject(ctx).await?;
        list.append("routerindex".to_string());
        Ok(RouterIndexDependency)
    }
}

// Prefix router dependency
struct PrefixRouterDependency;

#[async_trait::async_trait]
impl Injectable for PrefixRouterDependency {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        let list = DependencyList::inject(ctx).await?;
        list.append("prefix_router".to_string());
        Ok(PrefixRouterDependency)
    }
}

// Prefix router 2 dependency
struct PrefixRouter2Dependency;

#[async_trait::async_trait]
impl Injectable for PrefixRouter2Dependency {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        let list = DependencyList::inject(ctx).await?;
        list.append("prefix_router2".to_string());
        Ok(PrefixRouter2Dependency)
    }
}

// Prefix router index dependency
struct RouterPrefixIndexDependency;

#[async_trait::async_trait]
impl Injectable for RouterPrefixIndexDependency {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        let list = DependencyList::inject(ctx).await?;
        list.append("routerprefixindex".to_string());
        Ok(RouterPrefixIndexDependency)
    }
}

#[tokio::test]
async fn test_index_dependencies() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    // Simulate WebSocket endpoint "/" with dependencies: app, index
    let _app_dep = AppDependency::inject(&ctx).await.unwrap();
    let _index_dep = IndexDependency::inject(&ctx).await.unwrap();

    let list = DependencyList::inject(&ctx).await.unwrap();
    let items = list.get_items();

    assert_eq!(items, vec!["app", "index"]);
}

#[tokio::test]
async fn test_router_index_dependencies() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    // Simulate WebSocket endpoint "/router" with dependencies: app, router2, router, routerindex
    let _app_dep = AppDependency::inject(&ctx).await.unwrap();
    let _router2_dep = Router2Dependency::inject(&ctx).await.unwrap();
    let _router_dep = RouterDependency::inject(&ctx).await.unwrap();
    let _routerindex_dep = RouterIndexDependency::inject(&ctx).await.unwrap();

    let list = DependencyList::inject(&ctx).await.unwrap();
    let items = list.get_items();

    assert_eq!(items, vec!["app", "router2", "router", "routerindex"]);
}

#[tokio::test]
async fn test_prefix_router_index_dependencies() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    // Simulate WebSocket endpoint "/prefix/" with dependencies: app, prefix_router2, prefix_router, routerprefixindex
    let _app_dep = AppDependency::inject(&ctx).await.unwrap();
    let _prefix_router2_dep = PrefixRouter2Dependency::inject(&ctx).await.unwrap();
    let _prefix_router_dep = PrefixRouterDependency::inject(&ctx).await.unwrap();
    let _routerprefixindex_dep = RouterPrefixIndexDependency::inject(&ctx).await.unwrap();

    let list = DependencyList::inject(&ctx).await.unwrap();
    let items = list.get_items();

    assert_eq!(
        items,
        vec![
            "app",
            "prefix_router2",
            "prefix_router",
            "routerprefixindex"
        ]
    );
}

// Test that dependency list is shared across all dependencies in same request
#[tokio::test]
async fn test_dependency_list_shared() {
    let singleton = Arc::new(SingletonScope::new());
    let ctx = InjectionContext::new(singleton);

    // Inject multiple dependencies
    let _app_dep = AppDependency::inject(&ctx).await.unwrap();
    let _router_dep = RouterDependency::inject(&ctx).await.unwrap();

    // Get the list twice - should be the same instance
    let list1 = DependencyList::inject(&ctx).await.unwrap();
    let list2 = DependencyList::inject(&ctx).await.unwrap();

    // Should have the same items
    assert_eq!(list1.get_items(), list2.get_items());
    assert_eq!(list1.get_items(), vec!["app", "router"]);
}

// Test different requests have different dependency lists
#[tokio::test]
async fn test_different_requests_different_lists() {
    let singleton = Arc::new(SingletonScope::new());

    // Request 1
    let ctx1 = InjectionContext::new(singleton.clone());
    let _app_dep1 = AppDependency::inject(&ctx1).await.unwrap();
    let list1 = DependencyList::inject(&ctx1).await.unwrap();

    // Request 2
    let ctx2 = InjectionContext::new(singleton);
    let _router_dep2 = RouterDependency::inject(&ctx2).await.unwrap();
    let list2 = DependencyList::inject(&ctx2).await.unwrap();

    // Should have different items
    assert_eq!(list1.get_items(), vec!["app"]);
    assert_eq!(list2.get_items(), vec!["router"]);
}
