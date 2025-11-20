# reinhardt-types

Core type definitions and abstractions for the Reinhardt framework

## Overview

`reinhardt-types` provides fundamental traits and types used across the Reinhardt framework. It defines the core abstractions for request handling, middleware processing, and composable middleware chains with performance optimizations.

## Features

### Implemented ✓

- **Handler trait** - Core abstraction for async request processing
  - `async fn handle(&self, request: Request) -> Result<Response>`
  - Blanket implementation for `Arc<T>` to enable `Arc<dyn Handler>`
- **Middleware trait** - Request/response pipeline processing
  - `async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response>`
  - `fn should_continue(&self, request: &Request) -> bool` - Conditional execution
- **MiddlewareChain** - Composable middleware system with automatic chaining
  - Builder pattern: `with_middleware()` for method chaining
  - Mutable API: `add_middleware()` for imperative style
  - Performance optimizations:
    - O(k) complexity where k ≤ n (skips unnecessary middleware)
    - Short-circuiting with `Response::with_stop_chain(true)`
- **Type aliases** - Re-export of `Request` and `Response` from `reinhardt-http`
- **Async trait support** - Full async/await support via `async_trait`
- **Zero-cost abstractions** - All traits compile to efficient code with no runtime overhead

## Installation

```toml
[dependencies]
reinhardt-types = { version = "0.1.0-alpha.1", features = ["http"] }
```

**Note:** The `http` feature is required to use the `Handler` and `Middleware` traits.

## Usage Examples

### Basic Handler

```rust
use reinhardt_types::{Handler, Request, Response};
use async_trait::async_trait;
use reinhardt_exception::Result;

struct HelloHandler;

#[async_trait]
impl Handler for HelloHandler {
    async fn handle(&self, _request: Request) -> Result<Response> {
        Ok(Response::ok().with_body("Hello, World!"))
    }
}

// Use the handler
let handler = HelloHandler;
let request = Request::get("/");
let response = handler.handle(request).await?;
```

### Middleware

```rust
use reinhardt_types::{Middleware, Handler, Request, Response};
use async_trait::async_trait;
use reinhardt_exception::Result;
use std::sync::Arc;

struct LoggingMiddleware;

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response> {
        println!("Request: {} {}", request.method, request.uri);
        let response = next.handle(request).await?;
        println!("Response: {}", response.status);
        Ok(response)
    }
}
```

### Conditional Middleware

Optimize performance by skipping middleware when not needed:

```rust
use reinhardt_types::{Middleware, Handler, Request, Response};
use async_trait::async_trait;
use reinhardt_exception::Result;
use std::sync::Arc;

struct AuthMiddleware;

#[async_trait]
impl Middleware for AuthMiddleware {
    async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response> {
        // Check authentication
        if let Some(token) = request.headers.get("Authorization") {
            // Validate token...
            next.handle(request).await
        } else {
            Ok(Response::unauthorized().with_body("Authentication required"))
        }
    }

    // Only execute for /api/* endpoints
    fn should_continue(&self, request: &Request) -> bool {
        request.uri.path().starts_with("/api/")
    }
}
```

**Performance Benefits:**
- Achieves O(k) complexity instead of O(n), where k is the number of middleware that should run
- Skips unnecessary middleware based on request properties
- Common use cases:
  - Skip authentication for public endpoints
  - Skip CORS for same-origin requests
  - Skip compression for small responses
  - Skip rate limiting for admin requests

### Middleware Chain

Compose multiple middleware with automatic chaining:

```rust
use reinhardt_types::{MiddlewareChain, Handler, Middleware, Request, Response};
use std::sync::Arc;

// Create handler
let handler = Arc::new(MyHandler);

// Build middleware chain
let chain = MiddlewareChain::new(handler)
    .with_middleware(Arc::new(LoggingMiddleware))
    .with_middleware(Arc::new(AuthMiddleware))
    .with_middleware(Arc::new(CorsMiddleware));

// Process request through the chain
let response = chain.handle(request).await?;
```

**Execution Order:**
Middleware are applied in the order they were added:
1. `LoggingMiddleware` runs first
2. `AuthMiddleware` runs second
3. `CorsMiddleware` runs third
4. Handler executes
5. Response flows back through middleware in reverse order

### Short-Circuit Middleware

Stop processing early with `Response::with_stop_chain(true)`:

```rust
use reinhardt_types::{Middleware, Handler, Request, Response};
use async_trait::async_trait;
use reinhardt_exception::Result;
use std::sync::Arc;

struct RateLimitMiddleware {
    max_requests: usize,
}

#[async_trait]
impl Middleware for RateLimitMiddleware {
    async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response> {
        if self.is_rate_limited(&request) {
            // Stop the chain immediately without calling next handlers
            return Ok(Response::new(429, "Too Many Requests")
                .with_stop_chain(true));
        }
        next.handle(request).await
    }
}
```

**Use Cases:**
- Rate limiting (return 429 without processing)
- Authentication failures (return 401 without accessing resources)
- Request validation (return 400 before reaching business logic)
- Circuit breaker patterns (fail fast on system errors)

### Mutable Middleware Chain

Alternative API for imperative style:

```rust
use reinhardt_types::{MiddlewareChain, Handler};
use std::sync::Arc;

let handler = Arc::new(MyHandler);
let mut chain = MiddlewareChain::new(handler);

// Add middleware imperatively
chain.add_middleware(Arc::new(LoggingMiddleware));
chain.add_middleware(Arc::new(AuthMiddleware));

// Use the chain
let response = chain.handle(request).await?;
```

## Feature Flags

- `http` - Enables `Handler` and `Middleware` traits (requires `reinhardt-http`)
  - **Required** for most Reinhardt applications

## Architecture

### Handler Trait

The `Handler` trait is the core abstraction for request processing in Reinhardt:

```rust
#[async_trait]
pub trait Handler: Send + Sync {
    async fn handle(&self, request: Request) -> Result<Response>;
}
```

**Design Principles:**
- Async by default (via `async_trait`)
- Thread-safe (`Send + Sync`)
- Returns `Result` for error propagation
- Simple interface with single method

### Middleware Trait

The `Middleware` trait enables composable request/response processing:

```rust
#[async_trait]
pub trait Middleware: Send + Sync {
    async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response>;

    fn should_continue(&self, request: &Request) -> bool {
        true  // Default: always execute
    }
}
```

**Key Features:**
- Composition pattern (not inheritance)
- Explicit control flow with `next` handler
- Conditional execution with `should_continue()`
- Performance optimizations via early filtering

### MiddlewareChain

The `MiddlewareChain` composes multiple middleware into a single handler:

**Internal Implementation:**
1. Filters middleware using `should_continue()` - O(k) where k ≤ n
2. Builds nested handler chain via composition
3. Supports short-circuiting with `Response::should_stop_chain()`
4. Executes handlers from outermost to innermost

**Performance Characteristics:**
- Middleware filtering: O(n) single pass
- Execution: O(k) where k is number of active middleware
- Memory: O(n) for middleware storage, O(k) for active chain
- Short-circuit: Stops immediately without further processing

## Dependencies

- `reinhardt-http` - Request and Response types (with `http` feature)
- `reinhardt-exception` - Error handling and `Result` type
- `async-trait` - Async trait support
- `bytes` - Efficient byte buffer handling

## Testing

The crate includes comprehensive unit tests covering:
- Basic handler behavior
- Middleware processing
- Empty middleware chains
- Single and multiple middleware chains
- Conditional middleware execution
- Short-circuit behavior
- Response stop_chain flag

Run tests with:
```bash
cargo test --features http
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
