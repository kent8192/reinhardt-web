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

- **Backends** (`reinhardt-backends`): Backend abstractions
  - Cache backend traits
  - Session backend traits
  - Memory backend implementation
  - Redis backend implementation

### Planned

- Additional middleware types
- Enhanced security features
- More validator types
- Additional backend implementations

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
reinhardt-core = "0.1.0"
```

### Optional Features

Enable specific sub-crates based on your needs:

```toml
[dependencies]
reinhardt-core = { version = "0.1.0", features = ["signals", "macros", "security"] }
```

Available features:

- `types` (default): Core type definitions
- `exception` (default): Error handling
- `signals` (default): Event system
- `macros` (default): Procedural macros
- `security` (default): Security primitives
- `validators` (default): Data validation
- `backends` (default): Backend abstractions
- `redis-backend`: Redis backend implementation

## Usage

### Handler and Middleware

```rust
use reinhardt_core::{Handler, Middleware, Request, Response, Result};

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
use reinhardt_core::{Error, Result};

fn validate_user() -> Result<()> {
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
use reinhardt_core::{Signal, SignalDispatcher};

// Define a signal
static USER_CREATED: Signal<User> = Signal::new();

// Connect a receiver
USER_CREATED.connect(|user| {
    println!("User created: {}", user.name);
});

// Send signal
USER_CREATED.send(user)?;
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
    └── backends/       # Backend abstractions
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
