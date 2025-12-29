# reinhardt-rest

**Export-only integration layer** for Reinhardt REST API framework.

## Overview

This crate serves as a **convenience layer** that combines multiple Reinhardt crates into a single import. It does not contain its own implementation or tests - all functionality is provided by the underlying specialized crates.

## Installation

Add `reinhardt` to your `Cargo.toml`:

```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["rest"] }

# Or use a preset:
# reinhardt = { version = "0.1.0-alpha.1", features = ["standard"] }  # Recommended
# reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }      # All features
```

Then import REST features:

```rust
use reinhardt::rest::{ApiResponse, ResponseBuilder, IntoApiResponse};
use reinhardt::rest::{JwtAuth, IsAuthenticated, AllowAny, User, SimpleUser};
use reinhardt::rest::{DefaultRouter, Router, Route};
use reinhardt::rest::{PaginatedResponse};
```

**Note:** REST features are included in the `standard` and `full` feature presets.

## Purpose

- **Unified Interface**: Single import point for REST API functionality
- **Re-export Layer**: Combines authentication, routing, browsable API, and response handling
- **No Implementation**: Pure export/aggregation crate
- **No Tests**: All functionality is tested in specialized crates

## Features

### Implemented ✓

#### Authentication (from `reinhardt-auth`)

- **JWT Authentication**: Stateless authentication using JSON Web Tokens
  - `JwtAuth` - JWT authentication backend
  - `Claims` - JWT claims structure
- **User Types**:
  - `User` - Base user trait
  - `SimpleUser` - Simple user implementation
  - `AnonymousUser` - Unauthenticated user representation
- **Permission Classes**:
  - `AllowAny` - Allow all users (authenticated or not)
  - `IsAuthenticated` - Require authentication
  - `IsAuthenticatedOrReadOnly` - Read-only for anonymous, full access for authenticated
  - `IsAdminUser` - Require admin privileges
- **REST-specific Utilities**:
  - `AuthResult<U>` - Result type for authentication operations
  - `AuthBackend` - Authentication backend trait

#### Routing (from `reinhardt-routers`)

- **Router Types**:
  - `DefaultRouter` - Default router with automatic ViewSet URL generation
  - `Router` - Base router trait
- **URL Patterns**:
  - `Route` - Individual route definition
  - `UrlPattern` - URL pattern matching

#### Browsable API (from `reinhardt-browsable-api`)

- **HTML Interface**: Interactive API explorer for development and testing
- **Automatic Documentation**: Self-documenting API endpoints

#### Response Handling

- **Response Types**:
  - `ApiResponse<T>` - DRF-style API response wrapper
    - Success responses (`success`, `success_with_status`)
    - Error responses (`error`, `validation_error`)
    - Standard HTTP responses (`not_found`, `unauthorized`, `forbidden`)
  - `ResponseBuilder<T>` - Fluent builder for API responses
- **Utilities**:
  - `IntoApiResponse<T>` - Trait for converting types to API responses
  - `PaginatedResponse` - Paginated response wrapper (from `reinhardt-pagination`)

#### Schema Generation (from `reinhardt-openapi`)

- **OpenAPI/Swagger**:
  - `OpenApiSchema` - OpenAPI 3.0 schema generation
  - `Components` - Reusable schema components
  - `Operation` - API operation definitions
  - `Parameter` - Request parameter definitions
  - `Server` - Server configuration
  - Auto-schema generation from Rust types
  - `SwaggerUI` - Interactive API documentation

#### Pagination (from `reinhardt-pagination`)

- **Pagination Strategies**:
  - `PageNumberPagination` - Page-based pagination
  - `LimitOffsetPagination` - Offset-based pagination
  - `CursorPagination` - Cursor-based pagination

#### Filtering (from `reinhardt-filters`)

- **Filter Backends**:
  - `SearchFilter` - Search across multiple fields
  - `OrderingFilter` - Sort results by fields
  - `QueryFilter` - Type-safe query filtering
  - `MultiTermSearch` - Multi-term search operations

#### Throttling/Rate Limiting (from `reinhardt-throttling`)

- **Throttling Classes**:
  - `AnonRateThrottle` - Rate limiting for anonymous users
  - `UserRateThrottle` - Rate limiting for authenticated users
  - `ScopedRateThrottle` - Per-endpoint rate limiting

#### Signals/Hooks (from `reinhardt-signals`)

- **Model Signals**:
  - `pre_save`, `post_save` - Model save signals
  - `pre_delete`, `post_delete` - Model delete signals
  - `m2m_changed` - Many-to-many relationship signals

## Testing

This crate does not contain tests. All functionality is tested in the underlying specialized crates:

- Authentication tests: `reinhardt-auth/tests/`
- Router tests: `reinhardt-routers/tests/`
- Browsable API tests: `reinhardt-browsable-api/tests/`
- Response handling tests: Documentation tests in `src/response.rs`
- Integration tests: `tests/integration/`

## Usage

```rust
use reinhardt::rest::{
    // Authentication
    JwtAuth, IsAuthenticated, AllowAny, User, SimpleUser,

    // Routing
    DefaultRouter, Router, Route,

    // Response handling
    ApiResponse, ResponseBuilder, IntoApiResponse,

    // Pagination
    PaginatedResponse,
};

// Create a successful response
let user = SimpleUser::new(1, "Alice");
let response = ApiResponse::success(user);

// Build a custom response
let response = ResponseBuilder::new()
    .data("Success")
    .status(201)
    .message("Resource created")
    .build();

// Convert Result to ApiResponse
let result: Result<String, String> = Ok("data".to_string());
let response = result.into_api_response();
```
