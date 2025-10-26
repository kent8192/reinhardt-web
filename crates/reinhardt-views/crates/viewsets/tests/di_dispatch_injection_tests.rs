//! Tests for dispatch-level dependency injection with #[inject]

use async_trait::async_trait;
use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
use reinhardt_apps::{Handler, Request, Response, Result};
use reinhardt_di::{DiError, DiResult, InjectionContext, SingletonScope};
use reinhardt_macros::endpoint;
use reinhardt_viewsets::{Action, ViewSet, ViewSetHandler};
use std::collections::HashMap;
use std::sync::Arc;

/// Mock database dependency
#[derive(Clone)]
struct Database {
    connection_string: String,
}

impl Default for Database {
    fn default() -> Self {
        Self {
            connection_string: "postgres://localhost/testdb".to_string(),
        }
    }
}

/// Mock logging service dependency
#[derive(Clone)]
struct Logger {
    log_level: String,
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            log_level: "INFO".to_string(),
        }
    }
}

/// ViewSet with DI in dispatch method
#[derive(Clone)]
struct OrderViewSet {
    name: String,
}

impl OrderViewSet {
    fn new() -> Self {
        Self {
            name: "orders".to_string(),
        }
    }

    #[endpoint]
    async fn dispatch_with_context_impl(
        &self,
        _request: Request,
        action: Action,
        #[inject] db: Database,
        #[inject] logger: Logger,
    ) -> DiResult<Response> {
        use reinhardt_viewsets::ActionType;

        let body = match action.action_type {
            ActionType::List => format!(
                r#"{{"action":"list","db":"{}","logger":"{}"}}"#,
                db.connection_string, logger.log_level
            ),
            ActionType::Retrieve => format!(
                r#"{{"action":"retrieve","db":"{}","logger":"{}"}}"#,
                db.connection_string, logger.log_level
            ),
            _ => return Err(DiError::NotFound("Action not found".to_string())),
        };

        Ok(Response::ok().with_body(body))
    }
}

#[async_trait]
impl ViewSet for OrderViewSet {
    fn get_basename(&self) -> &str {
        &self.name
    }

    fn supports_di(&self) -> bool {
        true
    }

    async fn dispatch(&self, _request: Request, _action: Action) -> Result<Response> {
        // This should not be called when DI is supported
        Err(reinhardt_apps::Error::Internal(
            "Should use dispatch_with_context".to_string(),
        ))
    }

    async fn dispatch_with_context(
        &self,
        request: Request,
        action: Action,
        ctx: &reinhardt_di::InjectionContext,
    ) -> Result<Response> {
        self.dispatch_with_context_impl(request, action, ctx)
            .await
            .map_err(|e| reinhardt_apps::Error::Internal(format!("DI error: {}", e)))
    }
}

#[tokio::test]
async fn test_dispatch_injection_list_action() {
    // Create DI context
    let singleton = Arc::new(SingletonScope::new());
    let ctx = Arc::new(InjectionContext::new(singleton));

    // Create ViewSet
    let viewset = OrderViewSet::new();

    // Create action map
    let mut action_map = HashMap::new();
    action_map.insert(Method::GET, "list".to_string());

    // Create handler with DI context
    let handler =
        ViewSetHandler::new(Arc::new(viewset), action_map, None, None).with_di_context(ctx);

    // Create request
    let request = Request::new(
        Method::GET,
        Uri::from_static("/orders/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    // Handle request
    let response = handler.handle(request).await.unwrap();

    // Verify response
    assert_eq!(response.status, StatusCode::OK);

    let body = String::from_utf8(response.body.to_vec()).unwrap();
    assert!(body.contains("\"action\":\"list\""));
    assert!(body.contains("postgres://localhost/testdb"));
    assert!(body.contains("INFO"));
}

#[tokio::test]
async fn test_dispatch_injection_retrieve_action() {
    // Create DI context
    let singleton = Arc::new(SingletonScope::new());
    let ctx = Arc::new(InjectionContext::new(singleton));

    // Create ViewSet
    let viewset = OrderViewSet::new();

    // Create action map
    let mut action_map = HashMap::new();
    action_map.insert(Method::GET, "retrieve".to_string());

    // Create handler with DI context
    let handler =
        ViewSetHandler::new(Arc::new(viewset), action_map, None, None).with_di_context(ctx);

    // Create request
    let request = Request::new(
        Method::GET,
        Uri::from_static("/orders/123/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    // Handle request
    let response = handler.handle(request).await.unwrap();

    // Verify response
    assert_eq!(response.status, StatusCode::OK);

    let body = String::from_utf8(response.body.to_vec()).unwrap();
    assert!(body.contains("\"action\":\"retrieve\""));
    assert!(body.contains("postgres://localhost/testdb"));
    assert!(body.contains("INFO"));
}

#[tokio::test]
async fn test_dispatch_injection_without_context_fails() {
    // Create ViewSet
    let viewset = OrderViewSet::new();

    // Create action map
    let mut action_map = HashMap::new();
    action_map.insert(Method::GET, "list".to_string());

    // Create handler WITHOUT DI context
    let handler = ViewSetHandler::new(Arc::new(viewset), action_map, None, None);

    // Create request
    let request = Request::new(
        Method::GET,
        Uri::from_static("/orders/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    // Handle request - should fail
    let result = handler.handle(request).await;

    // Verify it fails with appropriate error
    assert!(result.is_err());
    let error = result.err().unwrap();
    assert!(
        error
            .to_string()
            .contains("ViewSet requires DI context but none was provided")
    );
}

/// ViewSet with cache control in dispatch
#[derive(Clone)]
struct CacheTestViewSet;

impl CacheTestViewSet {
    #[endpoint]
    async fn dispatch_with_context_impl(
        &self,
        _request: Request,
        _action: Action,
        #[inject] cached: Database,
        #[inject(cache = false)] fresh: Database,
    ) -> DiResult<Response> {
        let body = format!(
            r#"{{"cached":"{}","fresh":"{}"}}"#,
            cached.connection_string, fresh.connection_string
        );
        Ok(Response::ok().with_body(body))
    }
}

#[async_trait]
impl ViewSet for CacheTestViewSet {
    fn get_basename(&self) -> &str {
        "cache_test"
    }

    fn supports_di(&self) -> bool {
        true
    }

    async fn dispatch(&self, _request: Request, _action: Action) -> Result<Response> {
        Err(reinhardt_apps::Error::Internal(
            "Should use dispatch_with_context".to_string(),
        ))
    }

    async fn dispatch_with_context(
        &self,
        request: Request,
        action: Action,
        ctx: &reinhardt_di::InjectionContext,
    ) -> Result<Response> {
        self.dispatch_with_context_impl(request, action, ctx)
            .await
            .map_err(|e| reinhardt_apps::Error::Internal(format!("DI error: {}", e)))
    }
}

#[tokio::test]
async fn test_dispatch_injection_cache_control() {
    // Create DI context
    let singleton = Arc::new(SingletonScope::new());
    let ctx = Arc::new(InjectionContext::new(singleton));

    // Create ViewSet
    let viewset = CacheTestViewSet;

    // Create action map
    let mut action_map = HashMap::new();
    action_map.insert(Method::GET, "test".to_string());

    // Create handler with DI context
    let handler =
        ViewSetHandler::new(Arc::new(viewset), action_map, None, None).with_di_context(ctx);

    // Create request
    let request = Request::new(
        Method::GET,
        Uri::from_static("/cache_test/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    // Handle request
    let response = handler.handle(request).await.unwrap();

    // Verify response
    assert_eq!(response.status, StatusCode::OK);

    let body = String::from_utf8(response.body.to_vec()).unwrap();
    assert!(body.contains("postgres://localhost/testdb"));
}

/// ViewSet without DI support (backward compatibility test)
#[derive(Clone)]
struct LegacyViewSet;

#[async_trait]
impl ViewSet for LegacyViewSet {
    fn get_basename(&self) -> &str {
        "legacy"
    }

    async fn dispatch(&self, _request: Request, _action: Action) -> Result<Response> {
        Ok(Response::ok().with_body("legacy_response"))
    }
}

#[tokio::test]
async fn test_backward_compatibility_without_di() {
    // Create ViewSet (no DI)
    let viewset = LegacyViewSet;

    // Create action map
    let mut action_map = HashMap::new();
    action_map.insert(Method::GET, "test".to_string());

    // Create handler WITHOUT DI context
    let handler = ViewSetHandler::new(Arc::new(viewset), action_map, None, None);

    // Create request
    let request = Request::new(
        Method::GET,
        Uri::from_static("/legacy/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    // Handle request - should work without DI
    let response = handler.handle(request).await.unwrap();

    // Verify response
    assert_eq!(response.status, StatusCode::OK);

    let body = String::from_utf8(response.body.to_vec()).unwrap();
    assert_eq!(body, "legacy_response");
}
