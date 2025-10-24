# reinhardt-urls

URL routing and proxy utilities for Reinhardt framework

## Overview

`reinhardt-urls` provides comprehensive URL routing and lazy loading proxy functionality for Reinhardt applications, inspired by Django's URL system. This parent crate integrates routers, routing macros, and lazy loading proxy utilities to provide powerful URL management capabilities.

## Features

### Implemented ✓

This parent crate re-exports functionality from the following sub-crates:

- **Routers** (`reinhardt-routers`): Automatic URL routing configuration
  - Django-inspired URL routing
  - Automatic ViewSet URL generation
  - Namespacing and versioning support
  - URL reversal capabilities
  - PathPattern for URL pattern matching
  - DefaultRouter with automatic endpoint generation
  - Custom action support (list and detail-level)

- **Routers Macros** (`reinhardt-routers-macros`): Routing-related procedural macros
  - Compile-time route validation
  - Type-safe URL pattern generation
  - Route registration macros

- **Proxy** (`reinhardt-proxy`): Lazy loading proxy system
  - Django-style SimpleLazyObject implementation
  - Thread-safe lazy evaluation
  - Integration with ORM for lazy model loading
  - Automatic initialization on first access
  - Support for complex initialization logic
  - Advanced proxy features:
    - Association proxies (SQLAlchemy-style)
    - Scalar proxies with comparison operations
    - Collection proxies for relationship management
    - Query filtering and join operations
    - Lazy/eager loading strategies
    - Relationship caching

- **Advanced URL Pattern Matching**:
  - Compile-time path validation via `path!` macro
  - Runtime pattern matching with parameter extraction
  - Path constraint validation (snake_case parameters, no double slashes, etc.)
  - Regex-based URL matching with named capture groups

### Planned

- Route middleware support

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
reinhardt-urls = "0.1.0"
```

### Optional Features

Enable specific sub-crates based on your needs:

```toml
[dependencies]
reinhardt-urls = { version = "0.1.0", features = ["routers", "proxy"] }
```

Available features:

- `routers` (default): URL routing system
- `routers-macros` (default): Routing macros
- `proxy` (default): Lazy loading proxy

## Usage

### URL Routing

```rust
use reinhardt_urls::{Router, DefaultRouter, Route};

// Create a router
let mut router = DefaultRouter::new();

// Register ViewSet
router.register("users", UserViewSet::new());

// Add custom routes
router.add_route(Route::new("/custom/", custom_handler));

// Match incoming requests
if let Some((handler, params)) = router.match_request(&request) {
    handler.handle(request, params).await?;
}
```

### URL Reversal

```rust
use reinhardt_urls::reverse;

// Reverse URL by name
let url = reverse("user-detail", &[("id", "123")]);
// Returns: /users/123/

// With namespace
let url = reverse("api:v1:user-list", &[]);
// Returns: /api/v1/users/
```

### Lazy Loading Proxy

```rust
use reinhardt_proxy::SimpleLazyObject;

// Create lazy object
let lazy_user = SimpleLazyObject::new(|| {
    // Expensive initialization
    User::from_database(user_id)
});

// Access triggers initialization
let name = lazy_user.name; // Initialization happens here
```

## Sub-crates

This parent crate contains the following sub-crates:

```
reinhardt-urls/
├── Cargo.toml          # Parent crate definition
├── src/
│   └── lib.rs          # Re-exports from sub-crates
└── crates/
    ├── routers/         # URL routing system
    ├── routers-macros/  # Routing procedural macros
    └── proxy/           # Lazy loading proxy
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
