# Reinhardt Micro

A lightweight microservice framework for Rust, providing the minimal subset of Reinhardt functionality needed for building simple APIs and microservices.

## Why Reinhardt Micro?

Django is known for being "batteries-included", but this comes at a cost: simple services end up with unnecessary complexity and dependencies. Reinhardt Micro solves this problem by providing a **minimal, composable** framework that scales with your needs.

## Features

### Implemented ✓

- 🪶 **Lightweight** - Minimal dependencies (87 lines of code), fast compilation (~10s), small binaries (~5-10 MB)
- 🚀 **Fast** - Built on Tokio and Hyper, async from the ground up
- 🔒 **Type-safe** - Full Rust type system with Path, Query, Json extractors
- 🎯 **Focused** - Only routing (`App`, `route`), parameter extraction (`reinhardt-params`), and DI (`reinhardt-di`)
- 📦 **Composable** - Feature flags for incremental adoption (`routing`, `params`, `di`, `schema`, `database`)
- **App builder API** - Simple `App::new().route().serve()` pattern
- **Handler integration** - Works with any `Handler` trait implementation
- **Function-based endpoint macros** - FastAPI-style `#[get]`, `#[post]`, `#[put]`, `#[patch]`, `#[delete]` decorators
- **Built-in middleware shortcuts** - Quick imports for common middleware (CORS, CSRF, Compression, Logging, etc.)

## Quick Start

Add Reinhardt Micro to your `Cargo.toml`:

```toml
[dependencies]
reinhardt-micro = "0.1.0"
tokio = { version = "1", features = ["full"] }
```

### Example 1: Basic API with Endpoint Macros

```rust
use reinhardt_micro::prelude::*;

#[tokio::main]
async fn main() {
    let app = App::new()
        .route("/", hello)
        .route("/users/:id", get_user)
        .route("/users", create_user);

    app.serve("127.0.0.1:8000").await.unwrap();
}

async fn hello() -> &'static str {
    "Hello, World!"
}

#[get("/users/:id")]
async fn get_user(Path(id): Path<u64>) -> String {
    format!("User ID: {}", id)
}

#[derive(Deserialize)]
struct CreateUser {
    name: String,
    email: String,
}

#[post("/users")]
async fn create_user(Json(user): Json<CreateUser>) -> String {
    format!("Created user: {}", user.name)
}
```

### Example 2: With Middleware

```rust
use reinhardt_micro::prelude::*;
use reinhardt_micro::middleware::*;

#[tokio::main]
async fn main() {
    let app = App::new()
        .middleware(CorsMiddleware::permissive())
        .middleware(LoggingMiddleware::new())
        .middleware(GZipMiddleware::new())
        .route("/api/users", list_users);

    app.serve("127.0.0.1:8000").await.unwrap();
}

#[get("/api/users")]
async fn list_users() -> &'static str {
    "[{\"id\": 1, \"name\": \"Alice\"}]"
}
```

## Feature Flags

Reinhardt Micro uses feature flags to keep the core lightweight:

```toml
[dependencies]reinhardt-micro = { version = "0.1.0", default-features = false, features = ["routing", "params"] }
```

Available features:

- `routing` (default): Basic routing functionality
- `params` (default): Type-safe parameter extraction (Path, Query, Json, etc.)
- `di` (default): Dependency injection system
- `schema` (default): OpenAPI schema generation
- `database`: ORM integration (optional)

## Comparison with Full Reinhardt

| Feature              | Reinhardt Micro | Reinhardt (Standard) | Reinhardt (Full) |
|----------------------|-----------------|----------------------|------------------|
| Binary Size          | ~5-10 MB        | ~20-30 MB            | ~50+ MB          |
| Compile Time         | Fast            | Medium               | Slow             |
| Routing              | ✅               | ✅                    | ✅                |
| Parameter Extraction | ✅               | ✅                    | ✅                |
| Dependency Injection | ✅               | ✅                    | ✅                |
| ORM                  | Optional        | ✅                    | ✅                |
| Admin Panel          | ❌               | ❌                    | ✅                |
| Authentication       | ❌               | ✅                    | ✅                |
| Migrations           | ❌               | ✅                    | ✅                |
| Forms                | ❌               | ❌                    | ✅                |
| Templates            | ❌               | ❌                    | ✅                |

## When to Use

**Use Reinhardt Micro when:**

- Building microservices or serverless functions
- You need fast compilation and small binaries
- You prefer function-based endpoints over class-based views
- You want to add features incrementally

**Use Full Reinhardt when:**

- Building monolithic applications
- You need Django-style admin panel and ORM
- You want all batteries included from the start

## Migration Path

Start with Reinhardt Micro and upgrade to full Reinhardt as your needs grow:

```toml
# Start micro
[dependencies]reinhardt-micro = "0.1.0"

# Upgrade to standard
[dependencies]reinhardt = { version = "0.1.0", default-features = false, features = ["minimal"] }

# Full framework
[dependencies]reinhardt = "0.1.0"  # or features = ["full"]
```

## Examples

See the [examples directory](examples/) for more examples:

- Simple REST API
- JSON CRUD operations
- Dependency injection
- Database integration

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../../LICENSE-MIT))

at your option.