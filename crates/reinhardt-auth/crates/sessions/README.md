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

- **Session expiration and cleanup** - Implemented via `cleanup_expired()` in DatabaseSessionBackend
- **Session key rotation** - Implemented via `cycle_key()` and `flush()` in Session API
- **Cross-site request forgery (CSRF) protection integration** - CSRF module available
- **Session serialization formats** - JSON via serde_json, MessagePack, CBOR, Bincode
- **Session storage migration tools** - Migration module available

#### Session Serialization Formats

- **JSON** (always available) - Human-readable, widely compatible via `serde_json`
- **MessagePack** (feature: `messagepack`) - Compact binary format, cross-platform via `rmp-serde`
- **CBOR** (feature: `cbor`) - RFC 7049 compliant binary format via `ciborium`
- **Bincode** (feature: `bincode`) - Fastest for Rust-to-Rust communication

#### Session Compression

- **CompressedSessionBackend** (feature: `compression`) - Automatic compression wrapper
  - Threshold-based compression (default: 512 bytes, configurable)
  - Only compresses data exceeding threshold to avoid overhead
  - **Zstd compression** (feature: `compression-zstd`) - Best balance of speed and ratio
  - **Gzip compression** (feature: `compression-gzip`) - Wide compatibility
  - **Brotli compression** (feature: `compression-brotli`) - Best compression ratio

#### Session Replication

- **ReplicatedSessionBackend** (feature: `replication`) - High availability with multi-backend replication
  - **AsyncReplication** - Eventual consistency, highest throughput
  - **SyncReplication** - Strong consistency, both backends updated in parallel
  - **AcknowledgedReplication** - Primary first, then secondary with acknowledgment
  - Configurable retry attempts and delays for failure handling

#### Session Analytics

- **InstrumentedSessionBackend** - Automatic session event tracking wrapper
- **LoggerAnalytics** - Tracing-based logging (always available)
- **PrometheusAnalytics** (feature: `analytics-prometheus`) - Prometheus metrics export
  - `session_created_total` - Total sessions created
  - `session_accessed_total` - Total session accesses
  - `session_access_latency_seconds` - Access latency histogram
  - `session_size_bytes` - Session data size histogram
  - `session_deleted_total` - Deletions by reason (explicit, expired, flushed)
  - `session_expired_total` - Total expired sessions

#### Multi-Tenant Session Isolation

- **TenantSessionBackend** (feature: `tenant`) - Tenant-specific session namespacing
  - Prefix-based keying: `tenant:{tenant_id}:session:{session_id}`
  - Configurable key prefix pattern
  - Maximum sessions per tenant limit
  - Strict isolation mode for security
  - **TenantSessionOperations** trait: `list_sessions()`, `count_sessions()`, `delete_all_sessions()`

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

### Storage Backends

- `database` - Enable database-backed sessions (requires `reinhardt-orm`)
- `file` - Enable file-backed sessions (requires `tokio` with fs feature)
- `cookie` - Enable cookie-backed sessions with encryption (requires `base64`, `aes-gcm`, `rand`)
- `jwt` - Enable JWT-based stateless sessions (requires `jsonwebtoken`)

### HTTP Integration

- `middleware` - Enable HTTP middleware support (requires `reinhardt-http`, `reinhardt-types`, `reinhardt-exception`)

### Serialization Formats

- `messagepack` - Enable MessagePack serialization (requires `rmp-serde`)
- `cbor` - Enable CBOR serialization (requires `ciborium`)
- `bincode` - Enable Bincode serialization (requires `bincode`)

### Compression

- `compression-zstd` - Enable Zstd compression (requires `zstd`)
- `compression-gzip` - Enable Gzip compression (requires `flate2`)
- `compression-brotli` - Enable Brotli compression (requires `brotli`)
- `compression` - Enable all compression algorithms

### Monitoring

- `analytics-prometheus` - Enable Prometheus metrics export (requires `prometheus`)
- `analytics` - Enable all analytics features

### High Availability

- `replication` - Enable session replication across multiple backends

### Multi-tenancy

- `tenant` - Enable multi-tenant session isolation

### All Features

- `full` - Enable all features

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

| Feature                     | Django | Reinhardt Sessions                           |
| --------------------------- | ------ | -------------------------------------------- |
| Session Backends            | ✓      | ✓                                            |
| Session Object              | ✓      | ✓                                            |
| In-Memory Backend           | ✓      | ✓                                            |
| Database Backend            | ✓      | ✓ (SQLite, PostgreSQL, MySQL)                |
| File Backend                | ✓      | ✓ (with file locking)                        |
| Cookie Backend              | ✓      | ✓ (AES-GCM encrypted)                        |
| Session Middleware          | ✓      | ✓                                            |
| TTL/Expiration              | ✓      | ✓                                            |
| Session Iteration           | ✓      | ✓ (keys, values, items)                      |
| Manual Modification Control | ✓      | ✓ (mark_modified, mark_unmodified)           |
| Multiple Serializers        | ✓      | ✓ (JSON, MessagePack, CBOR, Bincode)         |
| Session Compression         | -      | ✓ (Zstd, Gzip, Brotli)                       |
| Session Replication         | -      | ✓ (Async, Sync, Acknowledged)                |
| Session Analytics           | -      | ✓ (Logger, Prometheus)                       |
| Multi-Tenant Isolation      | -      | ✓ (Prefix-based namespacing)                 |
| Type Safety                 | -      | ✓ (Rust types)                               |
| Async Operations            | -      | ✓                                            |

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please see the main [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.
