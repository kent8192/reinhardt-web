# reinhardt-throttling

Rate limiting and throttling for Reinhardt framework

## Overview

Comprehensive rate limiting system to prevent API abuse and ensure fair resource allocation. Provides multiple throttling strategies with flexible backend storage options.

## Implemented ✓

### Core Throttle System

- **`Throttle` Trait**: Async trait defining the throttling interface
  - `allow_request`: Check if a request should be allowed
  - `wait_time`: Get the time to wait before retrying
  - `get_rate`: Retrieve the configured rate limit
- **`ThrottleError`**: Error handling for throttling operations
- **`ThrottleResult<T>`**: Result type alias for throttle operations

### Throttle Implementations

#### Anonymous User Throttling

- **`AnonRateThrottle`**: Rate limiting for anonymous users (by IP address)
  - Constructor with memory backend: `new(rate, window_secs)`
  - Constructor with custom backend: `with_backend(rate, window_secs, backend)`
  - Automatically prefixes keys with `throttle:anon:`
  - Example: Allow 60 requests per hour for anonymous users

#### Authenticated User Throttling

- **`UserRateThrottle`**: Rate limiting for authenticated users (by user ID)
  - Constructor with memory backend: `new(rate, window_secs)`
  - Constructor with custom backend: `with_backend(rate, window_secs, backend)`
  - Automatically prefixes keys with `throttle:user:`
  - Per-user rate limiting with window expiration
  - Example: Allow 100 requests per 60 seconds per user

#### Scoped Throttling

- **`ScopedRateThrottle`**: Per-endpoint or custom scope rate limiting
  - Constructor with memory backend: `new()`
  - Constructor with custom backend: `with_backend(backend)`
  - Builder pattern for adding scopes: `add_scope(scope, rate, window)`
  - Key format: `scope:identifier` (e.g., `"api:user1"`, `"upload:user2"`)
  - Different rate limits per scope
  - Unscoped requests are not throttled
  - Example: API scope with 100/min, Upload scope with 10/min

#### Burst Rate Throttling

- **`BurstRateThrottle`**: Dual-window throttling with burst and sustained rates
  - Separate burst rate (short window) and sustained rate (long window)
  - Constructor: `new(backend, burst_rate, sustained_rate, burst_duration, sustained_duration)`
  - Both rate limits must be satisfied for requests to pass
  - Example: 10 requests per second burst, 100 requests per minute sustained

#### Tiered Rate Throttling

- **`TieredRateThrottle`**: User-level based throttling (e.g., free vs premium)
  - `Tier` struct for defining tier configurations
  - Constructor: `new(backend, default_tier)`
  - Add tiers: `add_tier(tier)`
  - Get tier: `get_tier(tier_name)`
  - Key format: `tier_name:user_id`
  - Fallback to default tier for unknown tier names
  - Example: Free tier 100/hour, Premium tier 1000/hour

### Backend Storage

#### Memory Backend

- **`MemoryBackend`**: In-memory rate limit tracking
  - Default constructor: `new()`
  - With time provider: `with_time_provider(time_provider)`
  - HashMap-based storage with automatic window expiration
  - Thread-safe using `Arc<RwLock<HashMap>>`
  - Implements `Default` trait
  - Perfect for single-server deployments

#### Redis Backend

- **`RedisBackend`**: Distributed rate limiting (feature-gated: `redis-backend`)
  - Constructor: `new(url)` with Redis connection URL
  - Uses Redis INCR and EXPIRE commands
  - Supports distributed rate limiting across multiple servers
  - Async connection pooling with multiplexed connections

#### Backend Trait

- **`ThrottleBackend`**: Common interface for all backends
  - `increment(key, window)`: Increment counter with expiration
  - `get_count(key)`: Get current request count
  - `increment_duration(key, window)`: Duration-based increment
  - `get_wait_time(key)`: Get time until rate limit resets

### Testing Utilities

- **`TimeProvider` Trait**: Time abstraction for testability
  - `now()`: Get current time
- **`SystemTimeProvider`**: Real system time implementation
  - Uses `tokio::time::Instant::now()`
- **`MockTimeProvider`**: Controllable time for testing
  - `new(start_time)`: Create with initial time
  - `advance(duration)`: Move time forward
  - `set_time(time)`: Set absolute time
  - Thread-safe with `Arc<RwLock<Instant>>`

## Planned

### Advanced Features

- Distributed consensus for rate limit synchronization
- Graceful degradation under backend failure
- Rate limit warmup and cooldown strategies
- Adaptive rate limiting based on system load
- Rate limit analytics and reporting
- Token bucket algorithm implementation
- Leaky bucket algorithm implementation

### Backend Extensions

- Memcached backend support
- Database-backed rate limiting
- Multi-tier caching with fallback
- Custom backend plugin system

### Additional Throttle Types

- Concurrent request throttling
- Bandwidth throttling
- Geo-based rate limiting
- Time-of-day based rate limiting
- Dynamic rate adjustment

## Feature Flags

- `redis-backend`: Enable Redis backend support (requires Redis client dependency)

## Usage Examples

See the documentation tests in each module for detailed usage examples:

- Memory backend throttling
- Redis-backed distributed throttling
- Time-based testing with MockTimeProvider
- Multiple throttle strategies
