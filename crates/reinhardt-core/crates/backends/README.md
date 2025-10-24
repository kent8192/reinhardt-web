# reinhardt-backends

Shared backend infrastructure for Reinhardt framework

## Overview

This crate provides a unified backend system for storing and retrieving data across different components of the Reinhardt framework, including throttling, caching, and session storage.

## Features

### Core Features

- **Backend Trait**: Generic key-value interface with TTL support
- **MemoryBackend**: High-performance in-memory storage with automatic expiration
- **RedisBackend**: Distributed storage using Redis (feature-gated)

### Key Capabilities

- Async-first design using `async-trait`
- Automatic expiration with TTL support
- Type-safe serialization/deserialization with `serde`
- Thread-safe concurrent access
- Increment operations for counters

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
reinhardt-backends = { workspace = true }

# With Redis support
reinhardt-backends = { workspace = true, features = ["redis-backend"] }
```

## Usage Examples

### Memory Backend

```rust
use reinhardt_backends::{Backend, MemoryBackend};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let backend = MemoryBackend::new();

    // Store with TTL
    backend.set("user:123", "active", Some(Duration::from_secs(3600))).await.unwrap();

    // Retrieve
    let value: Option<String> = backend.get("user:123").await.unwrap();
    assert_eq!(value, Some("active".to_string()));

    // Counter operations
    let count = backend.increment("api:calls", Some(Duration::from_secs(60))).await.unwrap();
    println!("API call count: {}", count);
}
```

### Redis Backend

```rust
use reinhardt_backends::{Backend, RedisBackend};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let backend = RedisBackend::new("redis://localhost:6379").await.unwrap();

    // Same API as MemoryBackend
    backend.set("session:abc", vec![1, 2, 3], Some(Duration::from_secs(3600))).await.unwrap();

    let data: Option<Vec<u8>> = backend.get("session:abc").await.unwrap();
    assert_eq!(data, Some(vec![1, 2, 3]));
}
```

### Shared Backend Pattern

```rust
use reinhardt_backends::{Backend, MemoryBackend};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Create a shared backend
    let backend = Arc::new(MemoryBackend::new());

    // Use in throttling
    let throttle_backend = backend.clone();

    // Use in cache
    let cache_backend = backend.clone();

    // Use in session storage
    let session_backend = backend.clone();

    // All components share the same state
}
```

## API Documentation

### Backend Trait

```rust
#[async_trait]
pub trait Backend: Send + Sync {
    async fn set<V>(&self, key: &str, value: V, ttl: Option<Duration>) -> BackendResult<()>;
    async fn get<V>(&self, key: &str) -> BackendResult<Option<V>>;
    async fn delete(&self, key: &str) -> BackendResult<bool>;
    async fn exists(&self, key: &str) -> BackendResult<bool>;
    async fn increment(&self, key: &str, ttl: Option<Duration>) -> BackendResult<i64>;
    async fn clear(&self) -> BackendResult<()>;
}
```

### Memory Backend

- **Thread-safe**: Uses `DashMap` for concurrent access
- **Auto-cleanup**: Expired entries are removed automatically
- **Zero-cost**: No external dependencies when using memory backend

### Redis Backend

- **Distributed**: State shared across multiple servers
- **Persistent**: Data survives application restarts
- **Scalable**: Redis handles millions of operations per second

## Feature Flags

- `memory` (default): Enable in-memory backend
- `redis-backend`: Enable Redis backend

## Testing

```bash
# Run memory backend tests
cargo test --package reinhardt-backends

# Run Redis tests (requires Redis server)
cargo test --package reinhardt-backends --features redis-backend -- --ignored
```

## Performance

### Memory Backend

- **Throughput**: ~1M ops/sec (single-threaded)
- **Latency**: <1μs for get/set operations
- **Memory**: O(n) where n is the number of keys

### Redis Backend

- **Throughput**: ~100K ops/sec (depends on Redis)
- **Latency**: ~1-5ms (network + Redis)
- **Memory**: Managed by Redis

## Integration Examples

### Throttling Integration

```rust
use reinhardt_backends::{Backend, MemoryBackend};
use std::sync::Arc;

pub struct Throttle {
    backend: Arc<dyn Backend>,
    rate: String,
}

impl Throttle {
    pub fn new(backend: Arc<dyn Backend>, rate: &str) -> Self {
        Self {
            backend,
            rate: rate.to_string(),
        }
    }

    pub async fn allow(&self, key: &str) -> bool {
        let count = self.backend.increment(key, Some(std::time::Duration::from_secs(60))).await.unwrap();
        count <= 100 // Allow 100 requests per minute
    }
}
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.