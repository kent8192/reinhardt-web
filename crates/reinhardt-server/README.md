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
  - Efficient TCP connection management
  - Automatic request/response conversion
  - Built-in error handling

- **WebSocket Support** (feature = "websocket"): WebSocket server implementation
  - tokio-tungstenite based WebSocket server
  - Custom message handler support
  - Connection lifecycle hooks (on_connect, on_disconnect)
  - Text and binary message handling
  - Automatic connection management

- **GraphQL Support** (feature = "graphql"): GraphQL endpoint integration
  - async-graphql integration
  - Schema builder for Query and Mutation roots
  - POST request handling for GraphQL queries
  - JSON response serialization
  - Error handling for GraphQL errors

### Planned

- HTTP/2 support
- Middleware pipeline
- Connection pooling
- Graceful shutdown
- Request timeouts
- Rate limiting

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
reinhardt-server = "0.1.0"
```

### Optional Features

Enable specific features based on your needs:

```toml
[dependencies]
reinhardt-server = { version = "0.1.0", features = ["websocket", "graphql"] }
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

### WebSocket Server

```rust
use reinhardt_server::{serve_websocket, WebSocketHandler};

struct MyWebSocketHandler;

impl WebSocketHandler for MyWebSocketHandler {
    async fn on_connect(&self, peer: SocketAddr) {
        println!("Client connected: {}", peer);
    }

    async fn on_text(&self, peer: SocketAddr, text: String) -> Option<String> {
        Some(format!("Echo: {}", text))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    serve_websocket("127.0.0.1:8080", MyWebSocketHandler).await?;
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
