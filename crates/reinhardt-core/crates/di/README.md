# reinhardt-di

FastAPI-inspired dependency injection system for Reinhardt.

## Overview

Provides a FastAPI-style dependency injection system with support for request-scoped and singleton-scoped dependency caching, automatic resolution of nested dependencies, and integration with authentication and database connections.

Delivers the FastAPI development experience in Rust with type-safe and async-first design.

## Core Concepts

### Dependency Scopes

- **Request Scope**: Dependencies cached per request (default)
- **Singleton Scope**: Dependencies shared across the entire application

### Automatic Injection

Types implementing `Default + Clone + Send + Sync + 'static` automatically implement the `Injectable` trait and can be used as dependencies.

## Implemented Features ✓

### Core Dependency Injection

- ✓ **`Depends<T>` Wrapper**: FastAPI-style dependency injection wrapper
  - `Depends::<T>::new()` - Cache enabled (default)
  - `Depends::<T>::no_cache()` - Cache disabled
  - `resolve(&ctx)` - Dependency resolution
  - `from_value(value)` - Generate from value for testing

- ✓ **Injectable Trait**: Define types that can be injected as dependencies
  - Auto-implementation: For types implementing `Default + Clone + Send + Sync + 'static`
  - Custom implementation: When complex initialization logic is needed

- ✓ **InjectionContext**: Context for dependency resolution
  - `get_request<T>()` / `set_request<T>()` - Request scope
  - `get_singleton<T>()` / `set_singleton<T>()` - Singleton scope
  - Generate new context per request

- ✓ **RequestScope**: Caching within requests
  - Type-based cache (using `TypeId` as key)
  - Thread-safe implementation (`Arc<RwLock<HashMap>>`)

- ✓ **SingletonScope**: Application-wide caching
  - Dependencies shared across all requests
  - Thread-safe implementation

### Advanced Features

- ✓ **Dependency Caching**: Automatic caching within request scope
  - Same dependency is generated only once even when requested multiple times
  - Cache is shared between nested dependencies
  - Cache enable/disable control available

- ✓ **Nested Dependencies**: Dependencies can depend on other dependencies
  - Automatic dependency graph resolution
  - Circular dependency detection and error handling

- ✓ **Dependency Overrides**: Dependency overrides for testing
  - Use different implementations for production and testing
  - Application-level override management
  - Support for overrides with sub-dependencies

- ✓ **Provider System**: Async factory pattern
  - `Provider` trait - Generic interface for providing dependencies
  - `ProviderFn` - Function-based provider
  - Any async closure can be used as a provider

### Error Handling

- ✓ **DiError**: Comprehensive error type
  - `NotFound` - Dependency not found
  - `CircularDependency` - Circular dependency detection
  - `ProviderError` - Provider errors
  - `TypeMismatch` - Type mismatch
  - `ScopeError` - Scope-related errors

### Integration Support

- ✓ **HTTP Integration**: Integration with HTTP requests/responses
  - Dependency injection from requests
  - Support for connection info injection

- ✓ **WebSocket Support**: Dependency injection into WebSocket connections
  - Use `Depends<T>` in WebSocket handlers

### Advanced Dependency Patterns ✓

#### Generator-based Dependencies (yield pattern)

- **Lifecycle Management**: Setup/teardown pattern
- **Context Manager**: Automatic resource cleanup
- **Error Handling**: Cleanup execution even on errors
- **Streaming Support**: Streaming response support
- **WebSocket Support**: Integration with WebSocket handlers

```rust
use reinhardt_di::{Injectable, InjectionContext};

#[derive(Clone)]
struct DatabaseConnection {
    // Setup
}

impl DatabaseConnection {
    async fn setup() -> Self {
        // Initialize connection
        DatabaseConnection { }
    }

    async fn cleanup(self) {
        // Close connection
    }
}
```

#### Dependency Classes (Class-based dependencies)

- **Callable Dependencies**: Struct-based dependencies with call methods
- **Async Callables**: Async dependency method support
- **Stateful Dependencies**: Dependencies with internal state
- **Method-based Injection**: Flexible dependency construction

```rust
#[derive(Clone)]
struct CallableDependency {
    prefix: String,
}

impl CallableDependency {
    fn call(&self, value: String) -> String {
        format!("{}{}", self.prefix, value)
    }
}
```

#### Parametrized Dependencies (Parameterized dependencies)

- **Path Parameter Integration**: Access to path parameters from dependencies
- **Shared Parameters**: Share path parameters between endpoints and dependencies
- **Type-safe Extraction**: Compile-time validated parameter passing

```rust
// Path parameter accessible in dependency
#[async_trait::async_trait]
impl Injectable for UserValidator {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        let user_id = UserId::inject(ctx).await?;
        Ok(UserValidator { user_id: user_id.0 })
    }
}
```

#### Schema Generation (Schema generation)

- **Dependency Deduplication**: Shared dependencies appear only once in schema
- **Transitive Dependencies**: Automatic caching of nested dependencies
- **Schema Optimization**: Efficient dependency graph representation

#### Security Overrides (Security overrides)

- **Security Dependencies**: OAuth2, JWT, and other authentication schemes
- **Security Scopes**: Scope-based access control
- **Override Support**: Test-friendly replacement of security dependencies

```rust
// Security dependency with scopes
#[async_trait::async_trait]
impl Injectable for UserData {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        let scopes = ctx.get_request::<SecurityScopes>()?;
        Ok(UserData { scopes: scopes.scopes })
    }
}
```

## Usage Examples

### Basic Usage

```rust
use reinhardt_di::{Depends, Injectable, InjectionContext, SingletonScope};
use std::sync::Arc;

#[derive(Clone, Default)]
struct Config {
    api_key: String,
    database_url: String,
}

#[tokio::main]
async fn main() {
    // Creating a singleton scope
    let singleton = Arc::new(SingletonScope::new());

    // Creating the request context
    let ctx = InjectionContext::new(singleton);

    // Dependency Resolution (Cache Enabled)
    let config = Depends::<Config>::new()
        .resolve(&ctx)
        .await
        .unwrap();

    println!("API Key: {}", config.api_key);
}
```

### Custom Injectable Implementation

```rust
use reinhardt_di::{Injectable, InjectionContext, DiResult};

struct Database {
    pool: DbPool,
}

#[async_trait::async_trait]
impl Injectable for Database {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        // Custom initialization logic
        let config = Config::inject(ctx).await?;
        let pool = create_pool(&config.database_url).await?;

        Ok(Database { pool })
    }
}
```

### Nested Dependencies

```rust
#[derive(Clone)]
struct ServiceA {
    db: Arc<Database>,
}

#[async_trait::async_trait]
impl Injectable for ServiceA {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        // Depends on Database
        let db = Database::inject(ctx).await?;
        Ok(ServiceA { db: Arc::new(db) })
    }
}

#[derive(Clone)]
struct ServiceB {
    service_a: Arc<ServiceA>,
    config: Config,
}

#[async_trait::async_trait]
impl Injectable for ServiceB {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        // Depends on ServiceA and Config (nested dependencies)
        let service_a = ServiceA::inject(ctx).await?;
        let config = Config::inject(ctx).await?;

        Ok(ServiceB {
            service_a: Arc::new(service_a),
            config,
        })
    }
}
```

### Dependency Overrides for Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct MockDatabase {
        // Mock implementation for testing
    }

    #[tokio::test]
    async fn test_with_mock_database() {
        let singleton = Arc::new(SingletonScope::new());
        let ctx = InjectionContext::new(singleton);

        // Set mock for testing
        let mock_db = MockDatabase { /* ... */ };
        ctx.set_request(mock_db);

        // Test code
    }
}
```

### Cache Control

```rust
// Cache enabled (default) - Returns the same instance
let config1 = Depends::<Config>::new().resolve(&ctx).await?;
let config2 = Depends::<Config>::new().resolve(&ctx).await?;
// config1 and config2 are the same instance

// Cache disabled - Creates new instance each time
let config3 = Depends::<Config>::no_cache().resolve(&ctx).await?;
let config4 = Depends::<Config>::no_cache().resolve(&ctx).await?;
// config3 and config4 are different instances
```

## Architecture

### Type-Based Caching

Dependency caching is managed using type (`TypeId`) as the key. This allows dependencies of the same type to be automatically cached.

### Scope Hierarchy

```
SingletonScope (Application level)
    ↓ Shared
InjectionContext (Request level)
    ↓ Holds
RequestScope (In-request cache)
```

### Thread Safety

- All scopes are thread-safe using `Arc<RwLock<HashMap>>`
- `Injectable` trait requires `Send + Sync`
- Safe to use in async code

## Testing Support

The testing framework includes a comprehensive test suite:

- **Unit Tests**: Unit tests for each component
- **Integration Tests**: Integration tests ported from FastAPI test cases
- **Feature Tests**:
  - Automatic Injectable implementation tests
  - Circular dependency detection tests
  - Cache behavior tests
  - Dependency override tests
  - Nested dependency tests

## Performance Considerations

- **Lazy Initialization**: Dependencies are not generated until needed
- **Cache Efficiency**: Same dependency is generated only once within request scope
- **Zero-Cost Abstractions**: Low-overhead design leveraging Rust's type system
- **Arc-based Sharing**: Efficient instance sharing using `Arc`

## Comparison with FastAPI

| Feature                  | FastAPI (Python) | reinhardt-di (Rust) |
| ------------------------ | ---------------- | ------------------- |
| Basic DI                 | ✓                | ✓                   |
| Request Scope            | ✓                | ✓                   |
| Singleton Scope          | ✓                | ✓                   |
| Dependency Caching       | ✓                | ✓                   |
| Nested Dependencies      | ✓                | ✓                   |
| Dependency Overrides     | ✓                | ✓                   |
| `yield` Pattern          | ✓                | ✓                   |
| Type Safety              | Runtime          | **Compile-time**    |
| Performance              | Dynamic          | **Static & Fast**   |

## License

This crate is part of the Reinhardt project and follows the same dual-license structure (MIT or Apache-2.0).
