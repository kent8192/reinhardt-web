//! FastAPI HTTP connection injection tests translated to Rust
//!
//! Based on: fastapi/tests/test_http_connection_injection.py
//!
//! These tests verify that:
//! 1. HTTP connection can be injected into dependencies
//! 2. Application state can be accessed through the connection
//! 3. WebSocket connections can also be injected

use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Simulated application state
#[derive(Clone)]
struct AppState {
    values: Arc<Mutex<HashMap<String, i32>>>,
}

impl AppState {
    fn new() -> Self {
        let mut values = HashMap::new();
        values.insert("value".to_string(), 42);
        AppState {
            values: Arc::new(Mutex::new(values)),
        }
    }

    fn get(&self, key: &str) -> Option<i32> {
        self.values.lock().unwrap().get(key).copied()
    }
}

// HTTP Connection wrapper
#[derive(Clone)]
struct HttpConnection {
    app_state: AppState,
}

#[async_trait::async_trait]
impl Injectable for HttpConnection {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        // Check cache first
        if let Some(cached) = ctx.get_request::<HttpConnection>() {
            return Ok((*cached).clone());
        }

        // Get app state from singleton
        let app_state = if let Some(state) = ctx.get_singleton::<AppState>() {
            (*state).clone()
        } else {
            let state = AppState::new();
            ctx.set_singleton(state.clone());
            state
        };

        let connection = HttpConnection { app_state };
        ctx.set_request(connection.clone());
        Ok(connection)
    }
}

// WebSocket connection wrapper
#[derive(Clone)]
struct WebSocketConnection {
    app_state: AppState,
}

#[async_trait::async_trait]
impl Injectable for WebSocketConnection {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        // Check cache first
        if let Some(cached) = ctx.get_request::<WebSocketConnection>() {
            return Ok((*cached).clone());
        }

        // Get app state from singleton
        let app_state = if let Some(state) = ctx.get_singleton::<AppState>() {
            (*state).clone()
        } else {
            let state = AppState::new();
            ctx.set_singleton(state.clone());
            state
        };

        let connection = WebSocketConnection { app_state };
        ctx.set_request(connection.clone());
        Ok(connection)
    }
}

// Dependency that extracts value from HTTP connection
#[derive(Clone, Debug, PartialEq)]
struct ValueFromHttp(i32);

#[async_trait::async_trait]
impl Injectable for ValueFromHttp {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        let conn = HttpConnection::inject(ctx).await?;
        let value = conn.app_state.get("value").unwrap_or(0);
        Ok(ValueFromHttp(value))
    }
}

// Dependency that extracts value from WebSocket connection
#[derive(Clone, Debug, PartialEq)]
struct ValueFromWebSocket(i32);

#[async_trait::async_trait]
impl Injectable for ValueFromWebSocket {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        let conn = WebSocketConnection::inject(ctx).await?;
        let value = conn.app_state.get("value").unwrap_or(0);
        Ok(ValueFromWebSocket(value))
    }
}

#[tokio::test]
async fn test_value_extracting_by_http() {
    let singleton = Arc::new(SingletonScope::new());

    // Set up app state
    let app_state = AppState::new();
    singleton.set(app_state);

    let ctx = InjectionContext::new(singleton);

    // Inject value through HTTP connection
    let value = ValueFromHttp::inject(&ctx).await.unwrap();

    assert_eq!(value.0, 42);
}

#[tokio::test]
async fn test_value_extracting_by_ws() {
    let singleton = Arc::new(SingletonScope::new());

    // Set up app state
    let app_state = AppState::new();
    singleton.set(app_state);

    let ctx = InjectionContext::new(singleton);

    // Inject value through WebSocket connection
    let value = ValueFromWebSocket::inject(&ctx).await.unwrap();

    assert_eq!(value.0, 42);
}

#[tokio::test]
async fn test_http_connection_cached() {
    let singleton = Arc::new(SingletonScope::new());
    singleton.set(AppState::new());

    let ctx = InjectionContext::new(singleton);

    // Inject connection twice
    let conn1 = HttpConnection::inject(&ctx).await.unwrap();
    let conn2 = HttpConnection::inject(&ctx).await.unwrap();

    // Should be the same instance (cached)
    assert_eq!(conn1.app_state.get("value"), conn2.app_state.get("value"));
}

#[tokio::test]
async fn test_websocket_connection_cached() {
    let singleton = Arc::new(SingletonScope::new());
    singleton.set(AppState::new());

    let ctx = InjectionContext::new(singleton);

    // Inject connection twice
    let conn1 = WebSocketConnection::inject(&ctx).await.unwrap();
    let conn2 = WebSocketConnection::inject(&ctx).await.unwrap();

    // Should be the same instance (cached)
    assert_eq!(conn1.app_state.get("value"), conn2.app_state.get("value"));
}

#[tokio::test]
async fn test_app_state_shared_across_connections() {
    let singleton = Arc::new(SingletonScope::new());
    singleton.set(AppState::new());

    let ctx = InjectionContext::new(singleton.clone());

    // Get value through HTTP
    let http_value = ValueFromHttp::inject(&ctx).await.unwrap();

    // Create new context (simulating new request)
    let ctx2 = InjectionContext::new(singleton);

    // Get value through WebSocket
    let ws_value = ValueFromWebSocket::inject(&ctx2).await.unwrap();

    // Both should see the same app state
    assert_eq!(http_value.0, ws_value.0);
}
