# reinhardt-routers

Automatic URL routing configuration for Reinhardt framework

## Overview

`reinhardt-routers` provides Django-inspired URL routing functionality for Reinhardt applications. It automatically generates URL patterns for ViewSets, supports namespacing and versioning, and includes powerful URL reversal capabilities. This crate eliminates boilerplate code for defining common REST API URL patterns while maintaining type safety and flexibility.

## Implemented ✓

### Core Router Types

- **`Router` trait**: Composable router interface for building modular routing systems
- **`DefaultRouter`**: Full-featured router implementation with automatic ViewSet URL generation
  - Automatic list/detail endpoint generation (`/resource/` and `/resource/{id}/`)
  - Support for custom ViewSet actions (both list and detail-level)
  - Request dispatching with path parameter extraction
  - Integration with `Handler` trait from `reinhardt-apps`

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

## Planned

### Advanced Features

- **`SimpleRouter`**: Lightweight router for basic routing needs
  - Minimal overhead for simple applications
  - Subset of DefaultRouter functionality
- **Nested routing**: Nested resource URL patterns
  - Parent-child resource relationships
  - Automatic nested URL generation
- **Route groups**: Group routes with shared middleware/configuration
- **Route caching**: Performance optimization for large route tables
- **Custom converters**: Type-specific path parameter converters
  - Integer, UUID, slug converters
  - Custom validation rules

### Developer Experience

- **Route introspection**: Runtime route analysis and debugging
- **OpenAPI integration**: Automatic OpenAPI schema generation from routes
- **Route visualization**: Generate route maps for documentation

## Usage Example

```rust
use reinhardt_routers::{DefaultRouter, Router, path, include_routes};
use reinhardt_viewsets::ViewSet;
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

// Include routes with namespace
let api_routes = vec![/* routes */];
router.include("/api/v1", api_routes, Some("v1".to_string()));

// URL reversal
let user_url = router.reverse_with("users-detail", &[("id", "123")]).unwrap();
// => "/users/123/"
```

## Dependencies

- `reinhardt-apps`: Handler trait and request/response types
- `reinhardt-viewsets`: ViewSet trait and action definitions
- `reinhardt-exception`: Error types and result handling
- `regex`: Pattern matching engine
- `nom`: Parser combinator library for regex conversion
- `async-trait`: Async trait support

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
