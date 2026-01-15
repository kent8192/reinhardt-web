# reinhardt-routers

Automatic URL routing configuration for Reinhardt framework

## Overview

`reinhardt-routers` provides Django-inspired URL routing functionality for Reinhardt applications. It automatically generates URL patterns for ViewSets, supports namespacing and versioning, and includes powerful URL reversal capabilities. This crate eliminates boilerplate code for defining common REST API URL patterns while maintaining type safety and flexibility.

## Installation

Add `reinhardt` to your `Cargo.toml`:

```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["urls-routers"] }

# For UnifiedRouter with DI and middleware:
# reinhardt = { version = "0.1.0-alpha.1", features = ["urls-routers", "urls-routers-unified"] }

# Or use a preset:
# reinhardt = { version = "0.1.0-alpha.1", features = ["standard"] }  # Recommended
# reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }      # All features
```

Then import router features:

```rust
use reinhardt::urls::routers::{DefaultRouter, UnifiedRouter, Router};
use reinhardt::urls::routers::{path, re_path, include_routes};
use reinhardt::urls::routers::{reverse, UrlReverser};
```

**Note:** Router features are included in the `standard` and `full` feature presets.

## Implemented ✓

### Core Router Types

- **`Router` trait**: Composable router interface for building composable routing systems
- **`DefaultRouter`**: Full-featured router implementation with automatic ViewSet URL generation
  - Automatic list/detail endpoint generation (`/resource/` and `/resource/{id}/`)
  - Support for custom ViewSet actions (both list and detail-level)
  - Request dispatching with path parameter extraction
  - Integration with `Handler` trait from `reinhardt-apps`
- **`UnifiedRouter`**: Hierarchical router with support for nested routing structures
  - Three API style unification (FastAPI-style, Django-style, DRF-style)
  - Nested router mounting with automatic prefix inheritance
  - DI context propagation from parent to child routers
  - Middleware stack composition (parent → child order)
  - Namespace-based route organization
  - Depth-first route resolution algorithm

### Route Definition

- **`Route`**: Path pattern and handler composition
  - Named routes with optional namespacing (`namespace:name` format)
  - Full name resolution for reverse URL lookup
  - Version extraction from namespace patterns
  - Namespace pattern matching for versioning strategies

### URL Pattern Matching

- **`PathPattern`**: Django-style URL pattern parser
  - Parameter extraction syntax (`/users/{id}/`)
  - Regex-based pattern matching with named groups
  - Parameter name validation and tracking
  - `is_match()`: Test if a path matches the pattern
  - `extract_params()`: Extract path parameters from a matched path
- **`PathMatcher`**: Efficient path matching engine
  - Multiple pattern registration
  - Path parameter extraction and mapping
  - First-match-wins routing strategy

### Helper Functions (Django-style API)

- **`path()`**: Create routes with simple parameter syntax
  - Similar to Django's `path()` function
  - Clean syntax for defining URL patterns
- **`re_path()`**: Create routes using regex patterns
  - Similar to Django's `re_path()` function
  - Converts Django-style regex patterns (`(?P<name>pattern)`) to Reinhardt format
  - Advanced parser using `nom` for complex regex handling
  - Supports nested groups and escaped characters
- **`include_routes()`**: Include route collections with prefix and namespace
  - Similar to Django's `include()` function
  - Namespace support for organizational hierarchy

### URL Reversal (Django-style reverse())

#### Runtime String-based Reversal

- **`UrlReverser`**: Name-to-URL resolution engine
  - Route registration and lookup by name
  - Parameter substitution in URL patterns
  - Namespace-aware URL resolution
  - Helper methods (`reverse()`, `reverse_with()`)
  - Route existence checking and name enumeration
- **`reverse()` function**: Standalone convenience function for URL reversal

#### Compile-time Type-safe Reversal

- **`UrlPattern` trait**: Define type-safe URL patterns at compile time
  - Constant pattern and name definitions
  - Zero-cost abstraction for URL definitions
- **`UrlPatternWithParams` trait**: Type-safe patterns with parameters
  - Compile-time parameter name validation
  - Parameter requirement enforcement
- **`reverse_typed()`**: Type-safe reversal for simple URLs (no parameters)
- **`reverse_typed_with_params()`**: Type-safe reversal with parameter validation
- **`UrlParams<T>` builder**: Fluent API for building type-safe URLs
  - Chainable parameter addition
  - Compile-time pattern checking
  - Runtime parameter validation

### ViewSet Integration

- **Automatic endpoint generation**: Generate standard REST endpoints from ViewSets
  - List endpoint (`GET /resource/`)
  - Create endpoint (`POST /resource/`)
  - Detail endpoint (`GET /resource/{id}/`)
  - Update endpoint (`PUT/PATCH /resource/{id}/`)
  - Delete endpoint (`DELETE /resource/{id}/`)
- **Custom action support**: Register and route custom ViewSet actions
  - Both list-level and detail-level actions
  - Custom URL path and name configuration
  - Automatic action handler wrapping
- **Action URL mapping**: Generate URL maps for ViewSet actions
  - Helper method `get_action_url_map()` for API discoverability

### Versioning Support

- **Namespace-based versioning**: Version APIs using URL namespaces
  - Version extraction from path patterns (`/v{version}/`)
  - Route filtering by namespace pattern
  - Available version enumeration
- **Pattern-based version detection**: Extract version numbers from URLs
  - Flexible pattern matching for different versioning schemes
  - Support for custom version formats

### Error Handling

- **`ReverseError`**: Comprehensive error types for URL reversal
  - `NotFound`: Route name not registered
  - `MissingParameter`: Required parameter not provided
  - `Validation`: Pattern parsing or parameter validation errors
- **`ReverseResult<T>`**: Type alias for reversal operations

### Hierarchical Routing (UnifiedRouter)

- **Nested router mounting**: Build hierarchical routing structures
  - `mount(prefix, child)`: Mount child router with automatic prefix inheritance
  - `mount_mut(&mut self, prefix, child)`: Mutable reference version
  - `group(namespace)`: Create namespace groups for organization
- **Builder pattern configuration**:
  - `with_prefix(prefix)`: Set URL prefix for router
  - `with_namespace(namespace)`: Set namespace for route naming
  - `with_di_context(context)`: Attach dependency injection context
  - `with_middleware(middleware)`: Add middleware to router
- **Automatic inheritance**:
  - DI context inherited from parent to child
  - Middleware stacks accumulated (parent → child order)
  - Prefix concatenation for nested paths
- **Route resolution**:
  - Depth-first search algorithm
  - Child routers checked before parent's own routes
  - Full middleware and DI context propagation

## Usage Examples

### DefaultRouter (Traditional)

```rust
use reinhardt::urls::routers::{DefaultRouter, Router, path, include_routes};
use reinhardt::views::viewsets::ViewSet;
use std::sync::Arc;

// Create a router
let mut router = DefaultRouter::new();

// Register a ViewSet (automatic endpoint generation)
let user_viewset = Arc::new(UserViewSet::new());
router.register_viewset("users", user_viewset);

// Add custom routes
router.add_route(
    path("/health/", Arc::new(HealthHandler))
        .with_name("health")
);

// Mount routes with namespace
let api_routes = vec![/* routes */];
router.mount("/api/v1", api_routes, Some("v1".to_string()));

// URL reversal
let user_url = router.reverse_with("users-detail", &[("id", "123")]).unwrap();
// => "/users/123/"
```

### UnifiedRouter (Hierarchical)

```rust
use reinhardt::urls::routers::UnifiedRouter;
use reinhardt::di::InjectionContext;
use reinhardt::middleware::AuthMiddleware;
use std::sync::Arc;

// Create main router
let app = UnifiedRouter::new()
    .with_middleware(Arc::new(LoggingMiddleware));

// Create API v1 router
let api_v1 = UnifiedRouter::new()
    .with_namespace("v1")
    .with_middleware(Arc::new(AuthMiddleware));

// Create users router
let users_router = UnifiedRouter::new()
    .viewset("users", Arc::new(UserViewSet::new()));

// Create posts router
let posts_router = UnifiedRouter::new()
    .viewset("posts", Arc::new(PostViewSet::new()));

// Build hierarchy
let app = app
    .mount("/api/v1",
        api_v1
            .mount("/users", users_router)
            .mount("/posts", posts_router)
    );

// Resulting URL structure:
// GET  /api/v1/users/       -> List users
// POST /api/v1/users/       -> Create user
// GET  /api/v1/users/{id}/  -> Get user
// PUT  /api/v1/users/{id}/  -> Update user
// DELETE /api/v1/users/{id}/ -> Delete user
// (same for /api/v1/posts/)

// Middleware stack for /api/v1/users/:
// 1. LoggingMiddleware (from app)
// 2. AuthMiddleware (from api_v1)
```

### Mixed API Styles with UnifiedRouter

```rust
use reinhardt::urls::routers::UnifiedRouter;
use hyper::Method;
use std::sync::Arc;

let router = UnifiedRouter::new()
    // FastAPI-style: Function-based endpoint
    .function("/health", Method::GET, health_check)

    // DRF-style: ViewSet with automatic CRUD
    .viewset("users", Arc::new(UserViewSet::new()))

    // Django-style: Class-based view
    .view("/about", Arc::new(AboutView));

// All three styles work seamlessly together!
```

## Dependencies

- `reinhardt-apps`: Handler trait and request/response types
- `reinhardt-viewsets`: ViewSet trait and action definitions
- `reinhardt-exception`: Error types and result handling
- `reinhardt-di` (optional, with `unified-router` feature): Dependency injection support
- `reinhardt-middleware` (optional, with `unified-router` feature): Middleware system
- `regex`: Pattern matching engine
- `nom`: Parser combinator library for regex conversion
- `async-trait`: Async trait support
- `hyper`: HTTP types (Method, Uri, etc.)

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
