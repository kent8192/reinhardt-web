# reinhardt-pool

Connection pooling utilities

## Overview

Connection pool management for database connections and other resources.

Provides efficient connection reuse, automatic cleanup, and configurable pool sizing.

## Features

### Implemented ✓

#### Core Connection Pool

- **Multi-database support**: PostgreSQL, MySQL, SQLite connection pools
  - `ConnectionPool::new_postgres()` - Create PostgreSQL connection pool
  - `ConnectionPool::new_mysql()` - Create MySQL connection pool
  - `ConnectionPool::new_sqlite()` - Create SQLite connection pool
- **Connection acquisition**: Acquire connections from pool with event emission
- **Pooled connections**: Wrapper type with automatic return-to-pool on drop
- **Pool recreation**: Recreate pools with same configuration for all database types
- **Inner pool access**: Direct access to underlying sqlx pool when needed

#### Pool Configuration

- **Flexible sizing**: Configurable min/max connection limits
  - `max_connections` - Maximum number of connections
  - `min_connections` - Minimum idle connections to maintain
  - `max_size` - Overall pool size limit
  - `min_idle` - Optional minimum idle connections
- **Timeout management**: Configurable connection and acquisition timeouts
  - `connection_timeout` - Timeout for creating new connections
  - `acquire_timeout` - Timeout for acquiring from pool
  - `idle_timeout` - Optional timeout for idle connections
- **Lifecycle settings**: Connection lifetime and idle timeout configuration
  - `max_lifetime` - Optional maximum connection lifetime
- **Health checks**: Optional test-before-acquire validation
  - `test_before_acquire` - Validate connections before use
- **Builder pattern**: `PoolOptions` for ergonomic configuration with method chaining

#### Event System

- **Connection lifecycle events**: Track connection state changes
  - `ConnectionAcquired` - Connection checked out from pool
  - `ConnectionReturned` - Connection returned to pool
  - `ConnectionCreated` - New connection established
  - `ConnectionClosed` - Connection terminated
  - `ConnectionTestFailed` - Health check failure
  - `ConnectionInvalidated` - Hard invalidation (connection unusable)
  - `ConnectionSoftInvalidated` - Soft invalidation (can complete current operation)
  - `ConnectionReset` - Connection reset
- **Event listeners**: Subscribe to pool events via `PoolEventListener` trait
- **Async event handling**: Non-blocking event emission
- **Built-in logger**: `EventLogger` for simple event logging
- **Timestamped events**: All events include UTC timestamps
- **Serializable events**: Events support serde serialization

#### Connection Management

- **Connection invalidation**:
  - Hard invalidation via `invalidate()` - connection immediately unusable
  - Soft invalidation via `soft_invalidate()` - can complete current operation
- **Connection reset**: Reset connection state via `reset()`
- **Connection ID tracking**: Unique UUID for each pooled connection
- **Automatic cleanup**: Connections automatically returned on drop with event emission

#### Pool Management

- **Multi-pool management**: `PoolManager` for managing multiple named pools
  - `add_pool()` - Register a named pool
  - `get_pool()` - Retrieve pool by name with type safety
  - `remove_pool()` - Unregister a pool
- **Type-safe pool storage**: Generic pool storage with downcasting
- **Shared configuration**: Common config across managed pools

#### Dependency Injection Support

- **Database service wrapper**: `DatabaseService` for DI frameworks
- **Database URL type**: `DatabaseUrl` wrapper for type-safe URLs
- **Pool type placeholders**: `MySqlPool`, `PostgresPool`, `SqlitePool` types
- **Manager types**: Dedicated manager types for each database backend

#### Error Handling

- **Comprehensive error types**: Detailed error variants
  - `PoolClosed` - Pool has been closed
  - `Timeout` - Operation timeout
  - `PoolExhausted` - Max connections reached
  - `InvalidConnection` - Connection validation failed
  - `Database` - sqlx database errors
  - `Config` - Configuration validation errors
  - `Connection` - Connection-specific errors
  - `PoolNotFound` - Named pool not found
- **Type-safe results**: `PoolResult<T>` type alias
- **Error propagation**: Automatic conversion from sqlx errors

### Planned

Currently all planned features are implemented.
