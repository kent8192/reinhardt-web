# Reinhardt API Reference

Welcome to the Reinhardt API reference documentation. This guide provides comprehensive information about Reinhardt's APIs, modules, and components.

> **Note**: Full API documentation is available at [docs.rs/reinhardt](https://docs.rs/reinhardt) (coming soon).

## Core Modules

### reinhardt-core

Core utilities and fundamental types used throughout the framework.

**Key Components:**

- Request/Response types
- Error handling
- HTTP primitives

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-core) (coming soon)

### reinhardt-views

View functions and class-based views for handling HTTP requests.

**Key Components:**

- Function-based views
- Class-based views
- Generic views (ListView, DetailView, CreateView, UpdateView, DeleteView)

**Example:**

```rust
use reinhardt_views::{View, TemplateView};

async fn my_view(request: Request) -> Result<Response> {
    // Handle request
}
```

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-views) (coming soon)

### reinhardt-params

FastAPI-style parameter extractors for type-safe request data extraction.

**Key Components:**

- `Path<T>` - Extract path parameters
- `Query<T>` - Extract query parameters
- `Header<T>` - Extract headers
- `Cookie<T>` - Extract cookies
- `Json<T>` - Parse JSON body
- `Form<T>` - Parse form data

**Example:**

```rust
use reinhardt_params::{Path, Query, Json};

#[derive(Deserialize)]
struct UserQuery {
    page: Option<u32>,
    limit: Option<u32>,
}

async fn get_user(
    Path(user_id): Path<i64>,
    Query(params): Query<UserQuery>,
) -> Result<Response> {
    // user_id and params are type-safe
}
```

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-params) (coming soon)

### reinhardt-di

Dependency injection system inspired by FastAPI.

**Key Components:**

- `Injectable` trait
- `Depends<T>` - Inject dependencies
- `InjectionContext` - DI container
- Singleton and request scopes

**Example:**

```rust
use reinhardt_di::{Injectable, Depends};

#[derive(Clone)]
struct Database {
    pool: DbPool,
}

#[async_trait]
impl Injectable for Database {
    async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
        Ok(Database { pool: get_pool().await? })
    }
}

async fn handler(db: Depends<Database>) -> Result<Response> {
    // db is automatically injected
}
```

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-di) (coming soon)
- [Tutorial: Dependency Injection](../tutorials/en/07-dependency-injection.md)

## Database & ORM

### reinhardt-orm

Django-style ORM with QuerySet API.

**Key Components:**

- `Model` trait
- `QuerySet` - Chainable queries
- `Manager` - Model manager
- Field types (CharField, IntegerField, DateTimeField, etc.)
- Relationships (ForeignKey, ManyToMany)

**Example:**

```rust
use reinhardt_orm::{Model, QuerySet};

#[derive(Model)]
struct User {
    id: i64,
    username: String,
    email: String,
}

// Query examples
let users = User::objects()
    .filter(age__gte = 18)
    .order_by("-created")
    .limit(10)
    .all()
    .await?;
```

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-orm) (coming soon)

### reinhardt-migrations

Database migration system.

**Key Components:**

- Migration files
- Schema operations
- Migration runner

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-migrations) (coming soon)

### reinhardt-db

Low-level database operations and connection management.

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-db) (coming soon)

## REST API Components

### reinhardt-serializers

Data serialization, deserialization, and validation.

**Key Components:**

- `Serializer` trait
- `ModelSerializer` - Auto-generate from models
- Field validators
- Nested serializers

**Example:**

```rust
use reinhardt_serializers::{Serializer, ModelSerializer};

#[derive(Serialize, Deserialize)]
struct UserSerializer {
    id: i64,
    username: String,
    email: String,
}

impl Serializer<User> for UserSerializer {
    fn validate(&self, instance: &User) -> ValidationResult {
        // Custom validation
    }
}
```

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-serializers) (coming soon)
- [Tutorial: Serialization](../tutorials/en/rest/1-serialization.md)

### reinhardt-viewsets

CRUD views for models with automatic routing.

**Key Components:**

- `ModelViewSet` - Full CRUD
- `ReadOnlyModelViewSet` - Read-only views
- Custom actions

**Example:**

```rust
use reinhardt_viewsets::ModelViewSet;

let viewset = ModelViewSet::<User, UserSerializer>::new();
```

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-viewsets) (coming soon)
- [Tutorial: ViewSets and Routers](../tutorials/en/rest/6-viewsets-and-routers.md)

### reinhardt-routers

Automatic URL routing for ViewSets.

**Key Components:**

- `Router` trait
- `DefaultRouter` - Standard REST routing
- URL pattern generation

**Example:**

```rust
use reinhardt_routers::{DefaultRouter, Router};

let mut router = DefaultRouter::new();
router.register("users", user_viewset);
router.register("posts", post_viewset);
```

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-routers) (coming soon)

### reinhardt-pagination

Pagination for large datasets.

**Key Components:**

- `PageNumberPagination`
- `LimitOffsetPagination`
- `CursorPagination`

**Example:**

```rust
use reinhardt_pagination::PageNumberPagination;

let pagination = PageNumberPagination::new(25); // 25 items per page
```

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-pagination) (coming soon)

### reinhardt-filters

Query filtering for ViewSets.

**Key Components:**

- `SearchFilter`
- `OrderingFilter`
- Custom filters

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-filters) (coming soon)

### reinhardt-throttling

Rate limiting for API endpoints.

**Key Components:**

- `AnonRateThrottle`
- `UserRateThrottle`
- `ScopedRateThrottle`

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-throttling) (coming soon)

## Authentication & Security

### reinhardt-auth

Authentication backends and permission system.

**Key Components:**

- JWT authentication
- Token authentication
- Session authentication
- Basic authentication
- Permission classes (`IsAuthenticated`, `IsAdminUser`, etc.)

**Example:**

```rust
use reinhardt_auth::{JWTAuth, IsAuthenticated};

// Configure in settings
let auth = JWTAuth::new(secret_key);
```

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-auth) (coming soon)
- [Tutorial: Authentication & Permissions](../tutorials/en/rest/4-authentication-and-permissions.md)

## Additional Components

### reinhardt-forms

Form handling and validation.

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-forms) (coming soon)

### reinhardt-templates

Template engine for rendering HTML.

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-templates) (coming soon)

### reinhardt-cache

Caching backends (Redis, in-memory).

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-cache) (coming soon)

### reinhardt-sessions

Session management.

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-sessions) (coming soon)

### reinhardt-mail

Email utilities.

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-mail) (coming soon)

### reinhardt-static

Static file serving.

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-static) (coming soon)

### reinhardt-storage

File storage backends (S3, local).

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-storage) (coming soon)

### reinhardt-websockets

WebSocket support.

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-websockets) (coming soon)

### reinhardt-graphql

GraphQL schema and resolvers.

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-graphql) (coming soon)

### reinhardt-i18n

Internationalization and localization.

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-i18n) (coming soon)

## Configuration

### reinhardt-conf

Configuration system and settings management.

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-conf) (coming soon)

### reinhardt-settings

Application settings.

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-settings) (coming soon)

## Testing

### reinhardt-test

Testing utilities and helpers.

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-test) (coming soon)

## Meta Packages

### reinhardt

Main package that re-exports all components based on feature flags.

**Documentation:**

- [Main documentation](https://docs.rs/reinhardt) (coming soon)
- [Feature Flags Guide](../FEATURE_FLAGS.md)

### reinhardt-micro

Lightweight version for microservices.

**Documentation:**

- [Module documentation](https://docs.rs/reinhardt-micro) (coming soon)

## Common Patterns

### Error Handling

```rust
use reinhardt::prelude::*;

async fn my_handler() -> Result<Response, Error> {
    let data = fetch_data().await?;
    Ok(JsonResponse::new(data))
}
```

### Middleware

```rust
use reinhardt_middleware::Middleware;

struct LoggingMiddleware;

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn process_request(&self, request: Request) -> Result<Request> {
        println!("Request: {} {}", request.method(), request.uri());
        Ok(request)
    }
}
```

### Custom Validators

```rust
use reinhardt_serializers::{ValidationError, ValidationResult};

fn validate_email(email: &str) -> ValidationResult {
    if !email.contains('@') {
        return Err(vec![ValidationError::new("email", "Invalid email format")]);
    }
    Ok(())
}
```

## Quick Links

- [Getting Started Guide](../GETTING_STARTED.md)
- [Tutorials](../tutorials/README.md)
- [Feature Flags](../FEATURE_FLAGS.md)
- [GitHub Repository](https://github.com/your-org/reinhardt)

## Contributing

Found an error in the documentation? Want to improve it?

- [Report documentation issues](https://github.com/your-org/reinhardt/issues)
- [Contribute to docs](https://github.com/your-org/reinhardt/blob/main/CONTRIBUTING.md)

---

**Note**: This is a high-level overview. For detailed API documentation with all methods, types, and examples, visit [docs.rs/reinhardt](https://docs.rs/reinhardt) once published.
