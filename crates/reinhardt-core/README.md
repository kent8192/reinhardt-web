# reinhardt-core

Core components for Reinhardt framework

## Overview

`reinhardt-core` provides the fundamental building blocks for the Reinhardt framework. It contains essential types, traits, error handling, signals, security primitives, validators, and backend abstractions that other crates depend on.

This crate serves as the foundation for the entire Reinhardt ecosystem, providing core abstractions and utilities used throughout the framework.

## Features

### Implemented ✓

This parent crate re-exports functionality from the following sub-crates:

- **Types** (`reinhardt-types`): Core type definitions
  - Handler trait for request processing
  - Middleware trait for request/response pipelines
  - MiddlewareChain for composable middleware
  - Type aliases and async trait support

- **Exception** (`reinhardt-exception`): Exception handling and error types
  - Django-style exception hierarchy
  - HTTP status code exceptions (401, 403, 404, 500, etc.)
  - Validation error handling
  - Database exception types
  - Custom error types (ImproperlyConfigured, ParseError, etc.)

- **Signals** (`reinhardt-signals`): Event-driven hooks for lifecycle events
  - Type-safe signal system for decoupled communication
  - Lifecycle signals for models, migrations, requests
  - Async and sync signal dispatch patterns
  - Signal composition and middleware
  - Performance monitoring

- **Macros** (`reinhardt-macros`): Procedural macros for code generation
  - `#[handler]` macro for endpoint definitions
  - `#[middleware]` macro for middleware implementations
  - `#[injectable]` macro for dependency injection

- **Security** (`reinhardt-security`): Security primitives and utilities
  - Password hashing and verification
  - CSRF protection
  - XSS prevention
  - Secure random generation
  - Constant-time comparisons

- **Validators** (`reinhardt-validators`): Data validation utilities
  - Email validation
  - URL validation
  - Length validators
  - Range validators
  - Custom validator support

- **Serializers** (`reinhardt-serializers`): Serialization and deserialization
  - Django REST Framework-inspired field types
  - Validation system with field and object validators
  - Recursive serialization with circular reference detection
  - Arena allocation for high-performance serialization

- **Messages** (`reinhardt-messages`): Flash messages and user notifications
  - Message levels (Debug, Info, Success, Warning, Error)
  - Storage backends (Memory, Session, Cookie, Fallback)
  - Middleware integration

- **Pagination** (`reinhardt-pagination`): Pagination strategies
  - PageNumberPagination for page-based pagination
  - LimitOffsetPagination for SQL-style pagination
  - CursorPagination for efficient large dataset pagination
  - Database cursor pagination with O(k) performance

- **Parsers** (`reinhardt-parsers`): Request body parsing
  - JSON, XML, YAML, Form, MultiPart parsers
  - File upload handling
  - Content-type negotiation

- **Negotiation** (`reinhardt-negotiation`): Content negotiation
  - Media type selection based on Accept headers
  - Language negotiation (Accept-Language)
  - Encoding negotiation (Accept-Encoding)

- **Dependency Injection** (`reinhardt-di`): FastAPI-style DI system
  - Automatic dependency resolution
  - Parameter injection
  - Cache control

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
reinhardt-core = "0.1.0-alpha.1"
```

### Optional Features

Enable specific sub-crates based on your needs:

```toml
[dependencies]
reinhardt-core = { version = "0.1.0-alpha.1", features = ["signals", "macros", "security"] }
```

Available features:

- `types` (default): Core type definitions
- `exception` (default): Error handling
- `signals` (default): Event system
- `macros` (default): Procedural macros
- `security` (default): Security primitives
- `validators` (default): Data validation
- `serializers` (default): Serialization utilities
- `http`: HTTP types and traits (requires `types`)
- `messages`: Flash messaging system
- `di`: Dependency injection with parameter extraction
- `negotiation`: Content negotiation
- `parsers`: Request body parsers
- `pagination`: Pagination strategies

## Usage

### Handler and Middleware

```rust
// Import from sub-crates
use reinhardt::core::types::{Handler, Middleware};
use reinhardt::http::{Request, Response};
use reinhardt::core::exception::Result;
use async_trait::async_trait;

// Define a handler
async fn my_handler(req: Request) -> Result<Response> {
    Response::ok().with_body("Hello, world!")
}

// Define middleware
struct LoggingMiddleware;

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn process_request(&self, req: Request) -> Result<Request> {
        println!("Processing request: {:?}", req.uri());
        Ok(req)
    }
}
```

### Error Handling

```rust
use reinhardt::core::exception::{Error, Result};

fn validate_user(authenticated: bool, authorized: bool) -> Result<()> {
    if !authenticated {
        return Err(Error::Authentication("Invalid credentials".into()));
    }
    if !authorized {
        return Err(Error::Authorization("Permission denied".into()));
    }
    Ok(())
}
```

### Signals

```rust
use reinhardt::core::signals::{Signal, SignalDispatcher};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct User {
    name: String,
}

// Connect a receiver to the signal
async fn setup_signal() {
    let signal = Signal::<User>::new();

    signal.connect(|user: Arc<User>| async move {
        println!("User created: {}", user.name);
        Ok(())
    });

    // Send signal
    let user = User { name: "Alice".to_string() };
    signal.send(user).await.unwrap();
}
```

## Sub-crates

This parent crate contains the following sub-crates:

```
reinhardt-core/
├── Cargo.toml          # Parent crate definition
├── src/
│   └── lib.rs          # Re-exports from sub-crates
└── crates/
    ├── types/          # Core type definitions
    ├── exception/      # Error handling
    ├── signals/        # Event system
    ├── macros/         # Procedural macros
    ├── security/       # Security primitives
    ├── validators/     # Data validation
    ├── serializers/    # Serialization utilities
    ├── messages/       # Flash messaging
    ├── pagination/     # Pagination strategies
    ├── parsers/        # Request body parsers
    └── negotiation/    # Content negotiation
```

**Note**: `reinhardt-di` and `reinhardt-http` are workspace-level crates located at `crates/reinhardt-di` and `crates/reinhardt-http`, not sub-crates of `reinhardt-core`. They are re-exported by `reinhardt-core` for convenience.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
