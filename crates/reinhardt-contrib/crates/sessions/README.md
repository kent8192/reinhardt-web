# reinhardt-sessions

Django-inspired session management for Reinhardt.

## Overview

Session framework for maintaining state across HTTP requests. This crate provides a session backend trait and implementations for storing session data in various storage systems.

## Features

### Implemented ✓

#### Core Session Backend

- **SessionBackend Trait** - Async trait defining session storage operations (load, save, delete, exists)
- **SessionError** - Error types for session operations (cache errors, serialization errors)
- **Generic Session Storage** - Type-safe session data storage with `serde` support

#### Cache-Based Backends

- **InMemorySessionBackend** - In-memory session storage using `InMemoryCache`
  - Fast, volatile storage (sessions lost on restart)
  - TTL (Time-To-Live) support for automatic expiration
  - Suitable for development and single-instance deployments
- **CacheSessionBackend** - Generic cache-based session backend
  - Works with any `Cache` trait implementation
  - Supports external cache systems (Redis, Memcached, etc.)
  - Configurable TTL for session expiration
  - Horizontal scalability for distributed systems

#### Dependency Injection Support

- Integration with `reinhardt-di` for dependency injection
- Session backend registration and resolution

#### High-Level Session API

- **Session<B>** struct - Django-style session object with dictionary-like interface
  - Type-safe with generic backend parameter `B: SessionBackend`
  - Dictionary-like methods: `get()`, `set()`, `delete()`, `contains_key()`
  - Session iteration methods: `keys()`, `values()`, `items()`
  - Manual session clearing: `clear()`
  - Manual modification tracking: `mark_modified()`, `mark_unmodified()`
  - Session modification tracking: `is_modified()`, `is_accessed()`
  - Session key management: `get_or_create_key()`, `generate_key()`
  - Session lifecycle: `flush()` (clear and new key), `cycle_key()` (keep data, new key)
  - Automatic persistence: `save()` method with TTL support (default: 3600 seconds)
  - Comprehensive doctests and unit tests (36 total tests)

#### Storage Backends

- **DatabaseSessionBackend** (feature: `database`) - Persistent session storage in database
  - Session model with expiration timestamps
  - Automatic session cleanup with `cleanup_expired()`
  - SQLite, PostgreSQL, and MySQL support via sqlx
  - Table creation with `create_table()`
  - Indexed expiration dates for efficient cleanup
  - 9 comprehensive tests
- **FileSessionBackend** (feature: `file`) - File-based session storage
  - Session files stored in configurable directory (default: `/tmp/reinhardt_sessions`)
  - File locking using `fs2` for concurrent access safety
  - JSON serialization with TTL support
  - Automatic expired session cleanup on access
  - 11 comprehensive tests
- **CookieSessionBackend** (feature: `cookie`) - Encrypted session data in cookies
  - AES-256-GCM encryption for session data
  - HMAC-SHA256 signing for tamper detection
  - Automatic size limitation checking (4KB max)
  - Secure client-side storage
  - 11 comprehensive tests

#### HTTP Middleware

- **SessionMiddleware** (feature: `middleware`) - HTTP middleware for session management
  - Automatic session loading from cookies
  - Automatic session saving on response
  - Cookie configuration: name, path, domain
  - Security settings: secure, httponly, samesite
  - TTL and max-age support
- **HttpSessionConfig** - Comprehensive middleware configuration
- **SameSite** enum - Cookie SameSite attribute (Strict, Lax, None)

#### Session Management Features

- Session expiration and cleanup
- Session key rotation
- Cross-site request forgery (CSRF) protection integration
- Session serialization formats (JSON, MessagePack, etc.)
- Session storage migration tools

#### Advanced Features

- Session replication for high availability
- Session analytics and monitoring
- Custom session serializers
- Session compression for large data
- Multi-tenant session isolation

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
reinhardt-sessions = "0.1.0-alpha.1"

# With optional features
reinhardt-sessions = { version = "0.1.0-alpha.1", features = ["database", "file", "cookie", "middleware"] }
```

## Quick Start

### Using Session with InMemorySessionBackend

```rust
use reinhardt_sessions::Session;
use reinhardt_sessions::backends::InMemorySessionBackend;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = InMemorySessionBackend::new();
    let mut session = Session::new(backend);

    // Set session data
    session.set("user_id", 42)?;
    session.set("username", "alice")?;

    // Get session data
    let user_id: i32 = session.get("user_id")?.unwrap();
    assert_eq!(user_id, 42);

    // Check if key exists
    assert!(session.contains_key("username"));

    // Delete a key
    session.delete("username");

    // Save session
    session.save().await?;

    Ok(())
}
```

### Using SessionBackend directly

```rust
use reinhardt_sessions::backends::{InMemorySessionBackend, SessionBackend};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a session backend
    let backend = InMemorySessionBackend::new();

    // Store session data
    let session_data = json!({
        "user_id": 42,
        "username": "alice",
        "authenticated": true,
    });

    backend.save("session_key_123", &session_data, Some(3600)).await?;

    // Retrieve session data
    let retrieved: Option<serde_json::Value> = backend.load("session_key_123").await?;
    assert!(retrieved.is_some());

    // Check if session exists
    assert!(backend.exists("session_key_123").await?);

    // Delete session
    backend.delete("session_key_123").await?;

    Ok(())
}
```

### Using CacheSessionBackend with Custom Cache

```rust
use reinhardt_sessions::backends::{CacheSessionBackend, SessionBackend};
use reinhardt_utils::cache::InMemoryCache;
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create cache and backend
    let cache = Arc::new(InMemoryCache::new());
    let backend = CacheSessionBackend::new(cache);

    // Store user preferences
    let preferences = json!({
        "theme": "dark",
        "language": "en",
        "notifications": true,
    });

    backend.save("pref_session_789", &preferences, Some(7200)).await?;

    // Load preferences
    let loaded: Option<serde_json::Value> = backend.load("pref_session_789").await?;
    assert_eq!(loaded.unwrap()["theme"], "dark");

    Ok(())
}
```

## Feature Flags

- `database` - Enable database-backed sessions (requires `reinhardt-orm`)
- `file` - Enable file-backed sessions (requires `tokio` with fs feature)
- `cookie` - Enable cookie-backed sessions with encryption (requires `base64`, `aes-gcm`, `rand`)
- `middleware` - Enable HTTP middleware support (requires `reinhardt-http`, `reinhardt-types`, `reinhardt-exception`)

## Architecture

### SessionBackend Trait

The core of the session framework is the `SessionBackend` trait, which defines the interface for session storage:

```rust
#[async_trait]
pub trait SessionBackend: Send + Sync {
    /// Load session data by key
    async fn load<T>(&self, session_key: &str) -> Result<Option<T>, SessionError>
    where
        T: for<'de> Deserialize<'de> + Send;

    /// Save session data with optional TTL (in seconds)
    async fn save<T>(
        &self,
        session_key: &str,
        data: &T,
        ttl: Option<u64>,
    ) -> Result<(), SessionError>
    where
        T: Serialize + Send + Sync;

    /// Delete session by key
    async fn delete(&self, session_key: &str) -> Result<(), SessionError>;

    /// Check if session exists
    async fn exists(&self, session_key: &str) -> Result<bool, SessionError>;
}
```

### Type Safety

All session backends use Rust's type system to ensure type-safe serialization and deserialization:

- Generic type parameters allow storing any `serde`-compatible data
- Compile-time type checking prevents runtime type errors
- Automatic serialization/deserialization handling

## Django Comparison

This crate is inspired by Django's session framework:

| Feature                     | Django | Reinhardt Sessions                 |
| --------------------------- | ------ | ---------------------------------- |
| Session Backends            | ✓      | ✓                                  |
| Session Object              | ✓      | ✓                                  |
| In-Memory Backend           | ✓      | ✓                                  |
| Database Backend            | ✓      | ✓ (SQLite, PostgreSQL, MySQL)      |
| File Backend                | ✓      | ✓ (with file locking)              |
| Cookie Backend              | ✓      | ✓ (AES-GCM encrypted)              |
| Session Middleware          | ✓      | ✓                                  |
| TTL/Expiration              | ✓      | ✓                                  |
| Session Iteration           | ✓      | ✓ (keys, values, items)            |
| Manual Modification Control | ✓      | ✓ (mark_modified, mark_unmodified) |
| Type Safety                 | -      | ✓ (Rust types)                     |
| Async Operations            | -      | ✓                                  |

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please see the main [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.
