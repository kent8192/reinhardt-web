# reinhardt-cache

Caching framework and backends for Reinhardt.

## Overview

Flexible caching framework with support for multiple backends including memory, Redis, and file-based caching. Provides cache decorators, low-level cache API, and integration with views and querysets for automatic caching.

## Features

### Core Cache API - Implemented ✓

- **`Cache` trait**: Async-first trait for cache operations with generic type support
  - `get<T>()`: Retrieve values from cache with automatic deserialization
  - `set<T>()`: Store values with optional TTL (Time-To-Live)
  - `delete()`: Remove individual cache entries
  - `has_key()`: Check cache key existence
  - `clear()`: Remove all entries from cache
  - `get_many()`: Batch retrieval of multiple cache keys
  - `set_many()`: Batch storage of multiple values
  - `delete_many()`: Batch deletion of multiple keys
  - `incr()`: Atomic increment for numeric values
  - `decr()`: Atomic decrement for numeric values

### Cache Backends - Implemented ✓

- **InMemoryCache**: Thread-safe in-memory cache backend
  - Built on `Arc<RwLock<HashMap>>` for concurrent access
  - Automatic expiration with TTL support
  - `with_default_ttl()`: Configure default expiration time
  - `cleanup_expired()`: Manual cleanup of expired entries
  - JSON serialization via serde for type safety

- **RedisCache**: Redis-backed distributed cache (requires `redis-backend` feature)
  - Connection pooling with `ConnectionManager` for efficient connection reuse
  - `with_default_ttl()`: Default TTL configuration
  - `with_key_prefix()`: Namespace support for multi-tenant scenarios
  - Automatic key prefixing for organized cache entries
  - Full Redis integration with all core operations implemented
  - Batch operations (`get_many`, `set_many`, `delete_many`) for improved performance
  - Atomic operations (`incr`, `decr`) using Redis native commands

### Cache Key Management - Implemented ✓

- **CacheKeyBuilder**: Utility for generating versioned cache keys
  - `new()`: Create builder with custom prefix
  - `with_version()`: Version-based cache invalidation
  - `build()`: Generate prefixed and versioned keys
  - `build_many()`: Batch key generation
  - Format: `{prefix}:{version}:{key}`

### HTTP Middleware - Implemented ✓

- **CacheMiddleware**: Automatic HTTP response caching
  - Request method filtering (GET-only by default via `cache_get_only`)
  - Response status code filtering (2xx-only by default via `cache_success_only`)
  - Cache-Control header parsing (max-age, no-cache, no-store directives)
  - Configurable cache timeout with `CacheMiddlewareConfig`
  - Query parameter-aware cache key generation
  - Full response caching (status, headers, body)

- **CacheMiddlewareConfig**: Middleware configuration
  - `with_default_timeout()`: Set default cache duration
  - `with_key_prefix()`: Configure cache namespace
  - `cache_all_methods()`: Enable caching for non-GET requests
  - `cache_all_responses()`: Cache non-2xx responses
  - Custom Cache-Control header name support

### Dependency Injection Support - Implemented ✓

- **CacheService**: High-level service with DI integration
  - Automatic injection via `reinhardt-di`
  - Integrated `CacheKeyBuilder` for automatic key prefixing
  - Methods: `get()`, `set()`, `delete()` with automatic key building
  - Access to underlying cache via `cache()` method
  - Access to key builder via `key_builder()` method

- **RedisConfig**: Redis configuration for DI (requires `redis-backend` feature)
  - `new()`: Custom Redis URL configuration
  - `localhost()`: Quick localhost setup
  - Automatic injection from singleton scope
  - Fallback to localhost if not configured

- **Injectable trait implementations**:
  - `InMemoryCache`: Uses default singleton-based injection
  - `CacheKeyBuilder`: Custom default ("app" prefix, version 1)
  - `RedisCache`: Injects with `RedisConfig` dependency
  - `CacheService`: Composes cache and key builder via DI

### Feature Flags - Implemented ✓

- `redis-backend`: Enable Redis support (optional dependency)
- `memcached-backend`: Enable Memcached support (optional dependency)
- `all-backends`: Enable all backend implementations

## Planned Features

### Cache Backends

- **File-based cache**: Persistent file system cache
- **Memcached backend**: Memcached integration (dependency declared but not implemented)
- **Hybrid cache**: Multi-tier caching (memory + distributed)

### Advanced Caching

- **Per-view caching**: View-level cache decorators
- **Template fragment caching**: Selective template output caching
- **QuerySet caching**: Automatic ORM query result caching
- **Cache warming**: Pre-populate cache on startup
- **Cache tags**: Tag-based invalidation for related entries

### Cache Strategies

- **Write-through**: Synchronous cache updates
- **Write-behind**: Asynchronous cache updates
- **Cache-aside**: Application-managed caching
- **Read-through**: Automatic cache population on miss

### Monitoring & Management

- **Cache statistics**: Hit/miss rates, entry counts, memory usage
- **Cache inspection**: List keys, view entries, export cache state
- **Automatic cleanup**: Background task for expired entry removal
- **Event hooks**: Pre/post cache operations callbacks

### Redis Backend Completion

- **Full Redis integration**: Complete implementation of Redis operations
- **Connection pooling**: Efficient connection management
- **Redis Cluster support**: Distributed Redis deployments
- **Redis Sentinel support**: High availability configurations
- **Pub/Sub support**: Cache invalidation via Redis channels

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
reinhardt-cache = { workspace = true }

# With Redis support
reinhardt-cache = { workspace = true, features = ["redis-backend"] }

# With all backends
reinhardt-cache = { workspace = true, features = ["all-backends"] }
```

## Usage Examples

### Basic In-Memory Caching

```rust
use reinhardt_cache::{Cache, InMemoryCache};
use std::time::Duration;

let cache = InMemoryCache::new();

// Set a value with TTL
cache.set("user:123", &user_data, Some(Duration::from_secs(300))).await?;

// Get the value
let user: Option<UserData> = cache.get("user:123").await?;

// Delete the value
cache.delete("user:123").await?;
```

### Using Cache Key Builder

```rust
use reinhardt_cache::CacheKeyBuilder;

let builder = CacheKeyBuilder::new("myapp").with_version(2);

// Build a single key
let key = builder.build("user:123"); // "myapp:2:user:123"

// Build multiple keys
let keys = builder.build_many(&["user:1", "user:2"]);
```

### HTTP Response Caching Middleware

```rust
use reinhardt_cache::{CacheMiddleware, CacheMiddlewareConfig, InMemoryCache};
use std::sync::Arc;
use std::time::Duration;

let cache = Arc::new(InMemoryCache::new());
let config = CacheMiddlewareConfig::new()
    .with_default_timeout(Duration::from_secs(600))
    .with_key_prefix("api_cache");

let middleware = CacheMiddleware::with_config(cache, config);
// Add middleware to your application
```

### Dependency Injection

```rust
use reinhardt_cache::CacheService;
use reinhardt_di::{Injectable, InjectionContext};

// Inject CacheService
let service = CacheService::inject(&ctx).await?;

// Use with automatic key building
service.set("session", &session_data, Some(Duration::from_secs(3600))).await?;
let session: Option<SessionData> = service.get("session").await?;
```

### Redis Cache (Feature-Gated)

```rust
use reinhardt_cache::{Cache, RedisCache, RedisConfig};
use std::time::Duration;

// Via DI
let config = RedisConfig::new("redis://localhost:6379");
ctx.set_singleton(config);

// Direct instantiation
let cache = RedisCache::new("redis://localhost:6379")
    .await?
    .with_default_ttl(Duration::from_secs(300))
    .with_key_prefix("myapp");

// Use the cache
cache.set("user:123", &user_data, Some(Duration::from_secs(3600))).await?;
let user: Option<UserData> = cache.get("user:123").await?;

// Batch operations
let mut values = HashMap::new();
values.insert("key1".to_string(), "value1".to_string());
values.insert("key2".to_string(), "value2".to_string());
cache.set_many(values, None).await?;

// Atomic operations
cache.incr("counter", 1).await?;
cache.decr("counter", 1).await?;
```

## Architecture

### Cache Entry Structure

- Serialized values stored as `Vec<u8>` (JSON via serde)
- Optional expiration timestamp (`SystemTime`)
- Automatic expiration checking on retrieval

### Thread Safety

- `Arc<RwLock<HashMap>>` for concurrent access in `InMemoryCache`
- Read-heavy optimization with `RwLock`
- All cache implementations are `Send + Sync`

### Error Handling

- Unified error types via `reinhardt-exception::Error`
- Serialization errors wrapped as `Error::Serialization`
- All operations return `Result<T, Error>`

## Testing

All features have comprehensive test coverage including:

- Unit tests for all cache operations
- TTL expiration behavior tests
- Batch operation tests
- Middleware integration tests
- DI injection tests
- Key builder functionality tests

Run tests with:

```bash
cargo test -p reinhardt-cache
cargo test -p reinhardt-cache --features redis-backend
cargo test -p reinhardt-cache --features all-backends
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
