# reinhardt-server

HTTP server implementation for Reinhardt framework

## Overview

`reinhardt-server` provides a high-performance HTTP server implementation for Reinhardt applications, built on Hyper with support for WebSockets and GraphQL. This crate serves as a parent crate that integrates server-related functionality.

## Features

### Implemented ✓

This parent crate re-exports functionality from the `server` sub-crate:

- **Core HTTP Server**: High-performance HTTP/1.1 server
  - Async request processing with Tokio runtime
  - Custom handler support via Handler trait
  - Middleware pipeline for request/response processing
  - Builder pattern for middleware configuration
  - Efficient TCP connection management
  - Automatic request/response conversion
  - Built-in error handling

- **WebSocket Support** (feature = "websocket"): WebSocket server implementation
  - tokio-tungstenite based WebSocket server
  - Custom message handler support
  - Broadcast support for multiple clients
  - Client connection management with automatic registration/unregistration
  - Connection lifecycle hooks (on_connect, on_disconnect)
  - Text and binary message handling
  - Automatic connection management

- **GraphQL Support** (feature = "graphql"): GraphQL endpoint integration
  - async-graphql integration
  - Schema builder for Query and Mutation roots
  - POST request handling for GraphQL queries
  - JSON response serialization
  - Error handling for GraphQL errors

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
reinhardt-server = "0.1.0-alpha.1"
```

### Optional Features

Enable specific features based on your needs:

```toml
[dependencies]
reinhardt-server = { version = "0.1.0-alpha.1", features = ["websocket", "graphql"] }
```

Available features:

- `server` (default): Core HTTP server
- `websocket`: WebSocket support
- `graphql`: GraphQL endpoint support

## Usage

### Basic HTTP Server

```rust
use reinhardt_server::{serve, HttpServer};
use reinhardt_http::{Request, Response};
use std::sync::Arc;

async fn my_handler(req: Request) -> Result<Response, Error> {
    Response::ok().with_body("Hello, world!")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let handler = Arc::new(my_handler);
    serve("127.0.0.1:8000", handler).await?;
    Ok(())
}
```

### HTTP Server with Middleware

```rust
use reinhardt_server::HttpServer;
use reinhardt_types::{Handler, Middleware};
use reinhardt_http::{Request, Response};
use std::sync::Arc;

struct MyHandler;

#[async_trait::async_trait]
impl Handler for MyHandler {
    async fn handle(&self, _req: Request) -> Result<Response, Error> {
        Ok(Response::ok().with_body("Hello from handler!"))
    }
}

struct LoggingMiddleware;

#[async_trait::async_trait]
impl Middleware for LoggingMiddleware {
    async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response, Error> {
        println!("Request: {} {}", request.method, request.uri);
        let response = next.handle(request).await?;
        println!("Response: {}", response.status);
        Ok(response)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let handler = Arc::new(MyHandler);
    let middleware = Arc::new(LoggingMiddleware);

    let server = HttpServer::new(handler)
        .with_middleware(middleware);

    server.listen("127.0.0.1:8000".parse()?).await?;
    Ok(())
}
```

### WebSocket Server

```rust
use reinhardt_server::{WebSocketServer, WebSocketHandler};
use std::sync::Arc;

struct EchoHandler;

#[async_trait::async_trait]
impl WebSocketHandler for EchoHandler {
    async fn handle_message(&self, message: String) -> Result<String, String> {
        Ok(format!("Echo: {}", message))
    }

    async fn on_connect(&self) {
        println!("Client connected");
    }

    async fn on_disconnect(&self) {
        println!("Client disconnected");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let handler = Arc::new(EchoHandler);
    let server = WebSocketServer::new(handler);
    server.listen("127.0.0.1:9001".parse()?).await?;
    Ok(())
}
```

### WebSocket Server with Broadcast

```rust
use reinhardt_server::{WebSocketServer, WebSocketHandler};
use std::sync::Arc;

struct ChatHandler;

#[async_trait::async_trait]
impl WebSocketHandler for ChatHandler {
    async fn handle_message(&self, message: String) -> Result<String, String> {
        Ok(format!("Received: {}", message))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let handler = Arc::new(ChatHandler);
    let server = WebSocketServer::new(handler)
        .with_broadcast(100); // Enable broadcast with capacity of 100 messages

    // Clone broadcast manager to send messages from other tasks
    let broadcast_manager = server.broadcast_manager().unwrap().clone();

    // Spawn a task to send periodic broadcasts
    tokio::spawn(async move {
        loop {
// tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            broadcast_manager.broadcast("Server announcement!".to_string()).await;
        }
    });

    server.listen("127.0.0.1:9001".parse()?).await?;
    Ok(())
}
```

### GraphQL Server

```rust
use reinhardt_server::graphql_handler;
use async_graphql::{Object, Schema, EmptyMutation, EmptySubscription};

struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn hello(&self) -> String {
        "Hello, GraphQL!".to_string()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .finish();

    let handler = graphql_handler(schema);
    serve("127.0.0.1:8000", handler).await?;
    Ok(())
}
```

## Sub-crates

This parent crate contains the following sub-crate:

```
reinhardt-server/
├── Cargo.toml          # Parent crate definition
├── src/
│   └── lib.rs          # Re-exports from sub-crate
└── crates/
    └── server/         # HTTP server implementation
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
