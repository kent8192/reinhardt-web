# reinhardt-rest-core

Core abstractions and types for REST API functionality in Reinhardt framework

## Overview

`reinhardt-rest-core` provides the fundamental building blocks for building RESTful APIs in Reinhardt. It offers Django REST Framework-style response types, authentication abstractions, router re-exports, and OpenAPI schema support.

This crate serves as the foundation layer, re-exporting and composing types from other Reinhardt crates into a cohesive REST API development experience.

## Features

### Implemented ✓

#### Authentication System

- **AuthResult** - REST-specific authentication result type
  - `Authenticated(U)` - Successfully authenticated user
  - `Anonymous` - Unauthenticated access
  - `Failed(String)` - Authentication failure with error message
- **Permission Classes** (re-exported from `reinhardt-core::auth`)
  - `AllowAny` - No authentication required
  - `IsAuthenticated` - Requires authenticated user
  - `IsAuthenticatedOrReadOnly` - Read-only for anonymous, full access for authenticated
  - `IsAdminUser` - Requires admin privileges
- **User Types** (re-exported from `reinhardt-core::auth`)
  - `User` trait - Base user interface
  - `SimpleUser` - Minimal user implementation
  - `AnonymousUser` - Unauthenticated user representation
- **JWT Support** (optional, requires `jwt` feature)
  - `JwtAuth` - JWT authentication backend
  - `Claims` - JWT claims structure

#### Response Types

- **ApiResponse<T>** - Django REST Framework-style structured response
  - Success responses: `success()`, `success_with_status()`
  - Error responses: `error()`, `not_found()`, `unauthorized()`, `forbidden()`
  - Validation errors: `validation_error()`
  - JSON serialization: `to_json()`, `to_json_pretty()`
- **ResponseBuilder<T>** - Fluent API for building complex responses
  - Method chaining: `.data()`, `.error()`, `.errors()`, `.status()`, `.message()`
  - Flexible response construction
- **IntoApiResponse** trait - Convert `Result` and `Option` to `ApiResponse`
- **PaginatedResponse** - Paginated response wrapper (re-exported from `reinhardt-core::pagination`)

#### Router Integration

- **DefaultRouter** - Feature-rich router with ViewSet support (re-exported from `reinhardt-urls`)
- **Router** trait - Base router interface
- **Route** - URL route definition
- **UrlPattern** - URL pattern matching

#### OpenAPI Schema

- **Schema types** - OpenAPI 3.0.3 schema definitions (re-exported from `reinhardt-openapi`)
- **OPENAPI_VERSION** constant - Current OpenAPI version (3.0.3)

## Installation

```toml
[dependencies]
reinhardt-rest-core = "0.1.0-alpha.1"

# Optional: Enable JWT authentication
reinhardt-rest-core = { version = "0.1.0-alpha.1", features = ["jwt"] }
```

## Usage Examples

### Basic API Response

```rust
use reinhardt_rest_core::ApiResponse;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: i64,
    name: String,
    email: String,
}

// Success response
let user = User {
    id: 1,
    name: "Alice".to_string(),
    email: "alice@example.com".to_string(),
};
let response = ApiResponse::success(user);
// Returns: {"data": {"id": 1, "name": "Alice", "email": "alice@example.com"}, "status": 200}

// Error response
let error_response: ApiResponse<User> = ApiResponse::not_found();
// Returns: {"error": "Not found", "status": 404}

// Validation error response
let mut errors = std::collections::HashMap::new();
errors.insert("email".to_string(), vec!["Invalid email format".to_string()]);
errors.insert("age".to_string(), vec!["Must be 18 or older".to_string()]);
let validation_response: ApiResponse<User> = ApiResponse::validation_error(errors);
// Returns: {"error": "Validation failed", "errors": {"email": ["Invalid email format"], "age": ["Must be 18 or older"]}, "status": 400}
```

### ResponseBuilder Pattern

```rust
use reinhardt_rest_core::ResponseBuilder;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct CreateUserResponse {
    id: i64,
    message: String,
}

// Build complex response with fluent API
let response = ResponseBuilder::new()
    .data(CreateUserResponse {
        id: 42,
        message: "User created successfully".to_string(),
    })
    .status(201)
    .message("Resource created")
    .build();

// Convert to JSON
let json = response.to_json_pretty().unwrap();
```

### Authentication with AuthResult

```rust
use reinhardt_rest_core::AuthResult;

// Successful authentication
let auth_result = AuthResult::Authenticated("user123".to_string());
assert!(auth_result.is_authenticated());
assert_eq!(auth_result.user(), Some("user123".to_string()));

// Anonymous access
let anonymous = AuthResult::<String>::Anonymous;
assert!(!anonymous.is_authenticated());
assert_eq!(anonymous.user(), None);

// Authentication failure
let failed = AuthResult::<String>::Failed("Invalid credentials".to_string());
assert!(!failed.is_authenticated());
assert_eq!(failed.error(), Some("Invalid credentials"));
```

### Converting Result and Option to ApiResponse

```rust
use reinhardt_rest_core::{ApiResponse, IntoApiResponse};

// From Result
let result: Result<String, String> = Ok("Success".to_string());
let response: ApiResponse<String> = result.into_api_response();
// Returns: {"data": "Success", "status": 200}

let error_result: Result<String, String> = Err("Failed".to_string());
let error_response: ApiResponse<String> = error_result.into_api_response();
// Returns: {"error": "Failed", "status": 500}

// From Option
let some_value: Option<String> = Some("Found".to_string());
let response: ApiResponse<String> = some_value.into_api_response();
// Returns: {"data": "Found", "status": 200}

let none_value: Option<String> = None;
let not_found: ApiResponse<String> = none_value.into_api_response();
// Returns: {"error": "Not found", "status": 404}
```

### Router Usage

```rust,ignore
use reinhardt_rest_core::routers::DefaultRouter;
use std::sync::Arc;

let mut router = DefaultRouter::new();
router.register_viewset("users", Arc::new(UserViewSet));
// Routes automatically generated:
// GET    /users/       - List
// POST   /users/       - Create
// GET    /users/{id}/  - Retrieve
// PUT    /users/{id}/  - Update
// PATCH  /users/{id}/  - Partial Update
// DELETE /users/{id}/  - Destroy
```

## Feature Flags

- `jwt` - Enable JWT authentication support (includes `JwtAuth` and `Claims` types)

## Dependencies

- `reinhardt-core` - Core framework types (auth, pagination)
- `reinhardt-urls` - Router and URL pattern types
- `reinhardt-openapi` - OpenAPI schema definitions
- `reinhardt-auth` - JWT authentication (optional, with `jwt` feature)
- `serde` - Serialization framework
- `serde_json` - JSON serialization

## Related Crates

- `reinhardt-rest` - High-level REST API framework (includes this crate)
- `reinhardt-serializers` - Data serialization and validation
- `reinhardt-viewsets` - ViewSet implementations for CRUD operations
- `reinhardt-pagination` - Pagination strategies
- `reinhardt-filters` - Filtering and search functionality

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
