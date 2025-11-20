# views-core

Core view abstractions and base classes for the Reinhardt framework

## Overview

`views-core` provides the fundamental view abstractions, traits, and base classes for building views in the Reinhardt framework. It defines the `View` trait and provides generic API view structures inspired by Django REST Framework.

## Features

### Implemented ✓

#### Core View Trait

- **View trait** - Base abstraction for all views
  - `async fn dispatch(&self, request: Request) -> Result<Response>` - Main entry point for request processing
  - `fn allowed_methods(&self) -> Vec<&'static str>` - Define allowed HTTP methods (defaults to GET, HEAD, OPTIONS)
  - Async support via `async_trait`
  - Thread-safe (`Send + Sync`)

#### Generic API View Structures

**Note:** These structures currently have basic definitions but lack full dispatch implementation. For production use, prefer `ListView` and `DetailView` from `reinhardt-views` or `ViewSet` from `reinhardt-viewsets`.

- **ListAPIView** - List endpoint structure (struct only)
- **CreateAPIView** - Create endpoint structure (struct only)
- **UpdateAPIView** - Update endpoint structure (struct only)
- **DestroyAPIView** - Delete endpoint structure (struct only)
- **ListCreateAPIView** - Combined list/create structure (struct only)
- **RetrieveUpdateAPIView** - Combined retrieve/update structure (struct only)
- **RetrieveDestroyAPIView** - Combined retrieve/delete structure (struct only)
- **RetrieveUpdateDestroyAPIView** - Combined retrieve/update/delete structure (struct only)

#### OpenAPI Support

- **OpenAPI schema generation** - Basic schema structures
- Integration with `reinhardt-openapi` for API documentation

#### Browsable API Support

- **Browsable API renderer** - Basic HTML rendering infrastructure
- Integration with `reinhardt-rest` for interactive API exploration

#### Admin Integration

- **Admin views** - Re-exports from `reinhardt-contrib`
- Django-style admin interface support

### Planned Features

- **Full Generic API View Implementation** - Complete dispatch logic for all generic views
- **View Mixins** - Reusable view components (SingleObjectMixin, MultipleObjectMixin, etc.)
- **Template View Support** - Template-based rendering for HTML views
- **Form View Support** - Form handling and validation in views
- **View Decorators** - Middleware-like decorators for views
- **View Composition** - Advanced view composition patterns

## Installation

```toml
[dependencies]
views-core = "0.1.0-alpha.1"
```

## Usage Examples

### Basic View Implementation

```rust
use views_core::{View, generic};
use async_trait::async_trait;
use reinhardt_core::exception::Result;
use reinhardt_core::http::{Request, Response};

struct MyView;

#[async_trait]
impl View for MyView {
    async fn dispatch(&self, request: Request) -> Result<Response> {
        // Handle request based on method
        match request.method.as_str() {
            "GET" => Ok(Response::ok().with_body("GET response")),
            "POST" => Ok(Response::created().with_body("POST response")),
            _ => Ok(Response::bad_request().with_body("Method not allowed")),
        }
    }

    fn allowed_methods(&self) -> Vec<&'static str> {
        vec!["GET", "POST", "OPTIONS"]
    }
}
```

### Custom Allowed Methods

```rust
use views_core::View;
use async_trait::async_trait;
use reinhardt_core::exception::Result;
use reinhardt_core::http::{Request, Response};

struct ReadOnlyView;

#[async_trait]
impl View for ReadOnlyView {
    async fn dispatch(&self, request: Request) -> Result<Response> {
        // Only handle GET requests
        if request.method == hyper::Method::GET {
            Ok(Response::ok().with_body("Read-only data"))
        } else {
            Ok(Response::bad_request().with_body("Method not allowed"))
        }
    }

    fn allowed_methods(&self) -> Vec<&'static str> {
        vec!["GET", "HEAD", "OPTIONS"]
    }
}
```

### Using Generic API View Structures (Basic)

```rust
use views_core::generic::ListAPIView;

// Create a basic list view structure
let list_view = ListAPIView::new();

// Note: Full dispatch implementation not yet available
// For production use, prefer reinhardt-views::ListView or reinhardt-viewsets::ViewSet
```

### Handling Multiple HTTP Methods

```rust
use views_core::View;
use async_trait::async_trait;
use reinhardt_core::exception::Result;
use reinhardt_core::http::{Request, Response};

struct UserView;

#[async_trait]
impl View for UserView {
    async fn dispatch(&self, request: Request) -> Result<Response> {
        match request.method.as_str() {
            "GET" => self.get(request).await,
            "POST" => self.post(request).await,
            "PUT" => self.put(request).await,
            "DELETE" => self.delete(request).await,
            _ => Ok(Response::bad_request().with_body("Method not allowed")),
        }
    }

    fn allowed_methods(&self) -> Vec<&'static str> {
        vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"]
    }
}

impl UserView {
    async fn get(&self, _request: Request) -> Result<Response> {
        Ok(Response::ok().with_json(&serde_json::json!({
            "users": ["Alice", "Bob"]
        }))?)
    }

    async fn post(&self, request: Request) -> Result<Response> {
        // Parse request body and create user
        Ok(Response::created().with_body("User created"))
    }

    async fn put(&self, request: Request) -> Result<Response> {
        // Update user
        Ok(Response::ok().with_body("User updated"))
    }

    async fn delete(&self, _request: Request) -> Result<Response> {
        // Delete user
        Ok(Response::no_content())
    }
}
```

## API Reference

### View Trait

```rust
#[async_trait]
pub trait View: Send + Sync {
    /// Main entry point for request processing
    async fn dispatch(&self, request: Request) -> Result<Response>;

    /// Returns the list of HTTP methods allowed by this view
    /// Default: ["GET", "HEAD", "OPTIONS"]
    fn allowed_methods(&self) -> Vec<&'static str> {
        vec!["GET", "HEAD", "OPTIONS"]
    }
}
```

**Key Characteristics:**
- Async by default (via `async_trait`)
- Thread-safe (`Send + Sync`)
- Returns `Result<Response>` for error handling
- Extensible via custom method handlers

### Generic API Views

**Current Status:** Struct definitions only (no dispatch implementation)

**Available Structures:**
- `ListAPIView` - For listing resources
- `CreateAPIView` - For creating resources
- `UpdateAPIView` - For updating resources
- `DestroyAPIView` - For deleting resources
- `ListCreateAPIView` - Combined list and create
- `RetrieveUpdateAPIView` - Combined retrieve and update
- `RetrieveDestroyAPIView` - Combined retrieve and delete
- `RetrieveUpdateDestroyAPIView` - Combined retrieve, update, and delete

**Usage:**
```rust
let view = ListAPIView::new();  // Creates struct, but dispatch not implemented
```

**Recommendation:** For full functionality, use:
- `reinhardt-views::ListView` - Complete list view implementation
- `reinhardt-views::DetailView` - Complete detail view implementation
- `reinhardt-viewsets::ViewSet` - Complete CRUD implementation

## Architecture

### View Pattern

The `View` trait follows a dispatch pattern similar to Django's class-based views:

1. **Request enters via `dispatch()`** - Single entry point
2. **Method-based routing** - Dispatch to specific method handlers (get, post, etc.)
3. **Response generation** - Return HTTP response
4. **Error handling** - Automatic error propagation via `Result`

### Design Principles

- **Single Responsibility** - Each view handles one resource or action
- **Async-First** - Native async/await support
- **Type Safety** - Compile-time type checking
- **Composability** - Views can be composed and extended

## Integration

### With reinhardt-views

For complete view implementations:
```rust
use reinhardt_views::{ListView, DetailView};
// Full-featured views with pagination, serialization, etc.
```

### With reinhardt-viewsets

For CRUD operations:
```rust
use reinhardt_viewsets::ViewSet;
// Complete CRUD with automatic routing
```

### With reinhardt-urls

For URL routing:
```rust
use reinhardt_urls::Router;
use views_core::View;

router.add_route("/api/users", Arc::new(MyView));
```

## Dependencies

- `reinhardt-core` - Core types (Request, Response, Result)
- `async-trait` - Async trait support
- `reinhardt-openapi` - OpenAPI schema generation (optional)
- `reinhardt-contrib` - Admin views (optional)

## Testing

The crate includes unit tests for:
- View trait implementation
- Generic API view structure creation
- Default method behavior

Run tests with:
```bash
cargo test
```

## Migration Guide

**From Partial Generic Views:**

If you're currently using the generic API view structures:

```rust
// Before (struct only, no dispatch)
use views_core::generic::ListAPIView;
let view = ListAPIView::new();

// After (full implementation)
use reinhardt_views::ListView;
use reinhardt_serializers::JsonSerializer;

let view = ListView::<MyModel, JsonSerializer<MyModel>>::new()
    .with_objects(data)
    .with_paginate_by(10);
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
