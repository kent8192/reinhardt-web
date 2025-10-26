<div align="center">
  <img src="branding/logo.png" alt="Reinhardt Logo" width="200"/>

  <h1>Reinhardt</h1>

  <h3>🦀 Polylithic Batteries Included</h3>

  <p><strong>A modular full-stack API framework for Rust</strong></p>
  <p>All the power of Django's batteries-included philosophy,<br/>
  with the flexibility to include only what you need.</p>

[![Crates.io](https://img.shields.io/crates/v/reinhardt.svg)](https://crates.io/crates/reinhardt)
[![Documentation](https://docs.rs/reinhardt/badge.svg)](https://docs.rs/reinhardt)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

</div>

---

## 📍 Quick Navigation

You may be looking for:

- 🚀 [Quick Start](#quick-start) - Get up and running in 5 minutes
- 📦 [Installation Options](#installation) - Choose your flavor: Micro, Standard, or Full
- 📚 [Getting Started Guide](docs/GETTING_STARTED.md) - Step-by-step tutorial
- 🎛️ [Feature Flags](docs/FEATURE_FLAGS.md) - Fine-tune your build
- 📖 [API Documentation](https://docs.rs/reinhardt) - Complete API reference
- 💬 [Community & Support](#getting-help) - Get help from the community

## Why Reinhardt?

Reinhardt brings together the best of three worlds:

| Inspiration        | What We Borrowed                                       | What We Improved                                     |
|--------------------|--------------------------------------------------------|------------------------------------------------------|
| 🐍 **Django**      | Batteries-included philosophy, ORM design, admin panel | Feature flags for modular builds, Rust's type safety |
| 🎯 **Django REST** | Serializers, ViewSets, permissions                     | Compile-time validation, zero-cost abstractions      |
| ⚡ **FastAPI**      | DI system, automatic OpenAPI                           | Native Rust performance, no runtime overhead         |
| 🗄️ **SQLAlchemy** | QuerySet patterns, relationship handling               | Type-safe query builder, compile-time validation     |

**Result**: A framework that's familiar to Python developers, but with Rust's performance and safety guarantees.

## ✨ Features

### 🎯 Core Framework

- **Type-Safe ORM**: QuerySet API with compile-time query validation
- **Powerful Serializers**: Automatic validation and transformation
- **Flexible ViewSets**: DRY principle for CRUD operations
- **Smart Routing**: Automatic URL configuration from ViewSets
- **Multi-Auth Support**: JWT, Token, Session, and Basic authentication

### 🚀 FastAPI-Inspired Ergonomics

- **Parameter Extraction**: Type-safe `Path<T>`, `Query<T>`, `Header<T>`, `Cookie<T>`, `Json<T>`, `Form<T>` extractors
- **Dependency Injection**: FastAPI-style DI system with `Depends<T>`, request scoping, and caching
- **Auto OpenAPI**: Generate OpenAPI 3.0 schemas from Rust types with `#[derive(Schema)]`
- **Function-based Endpoints**: Ergonomic `#[endpoint]` macro for defining API routes (coming soon)
- **Background Tasks**: Simple async task execution

### 🔋 Batteries Included

- **Admin Panel**: Django-style auto-generated admin interface (coming soon)
- **Middleware System**: Request/response processing pipeline
- **Management Commands**: CLI tools for migrations, static files, and more
- **Pagination**: PageNumber, LimitOffset, and Cursor strategies
- **Filtering & Search**: Built-in SearchFilter and OrderingFilter for querysets
- **Rate Limiting**: Flexible throttling (AnonRateThrottle, UserRateThrottle, ScopedRateThrottle)
- **Signals**: Event-driven hooks (pre_save, post_save, pre_delete, post_delete, m2m_changed)

### 🌍 Advanced Features

- **GraphQL Support**: Build GraphQL APIs alongside REST (coming soon)
- **WebSocket Support**: Real-time bidirectional communication (coming soon)
- **Internationalization**: Multi-language support
- **Static Files**: CDN integration, hashed storage, and compression
- **Browsable API**: HTML interface for API exploration

## Installation

Reinhardt offers three flavors to match your project's scale:

### Reinhardt Micro - For Microservices

Lightweight and fast, perfect for simple APIs and microservices:

```toml
[dependencies]
reinhardt-micro = "0.1.0"
```

### Reinhardt Standard - Balanced Approach

The default configuration, suitable for most projects:

```toml
[dependencies]
reinhardt = "0.1.0"
# Equivalent to: reinhardt = { version = "0.1.0", features = ["standard"] }
```

### Reinhardt Full - Everything Included

All features enabled, Django-style batteries-included:

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["full"] }
```

### Custom Configuration

Mix and match features as needed:

```toml
[dependencies]
# Minimal setup with just routing and params
reinhardt = { version = "0.1.0", default-features = false, features = ["minimal"] }

# Add database support
reinhardt = { version = "0.1.0", default-features = false, features = ["minimal", "database"] }

# Standard with extra features
reinhardt = { version = "0.1.0", features = ["standard", "websockets", "graphql"] }
```

## Quick Start

### Basic CRUD API

```rust
use reinhardt::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: i64,
    name: String,
    email: String,
}

#[derive(Debug, Clone)]
struct UserSerializer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a router
    let mut router = DefaultRouter::new();

    // Create and register a ViewSet for CRUD operations
    let user_viewset: Arc<ModelViewSet<User, UserSerializer>> =
        Arc::new(ModelViewSet::new("users"));
    router.register_viewset("users", user_viewset);

    // Start the server
    println!("Server running on http://127.0.0.1:8000");
    reinhardt::serve("127.0.0.1:8000", router).await?;

    Ok(())
}
```

This creates a full CRUD API with the following endpoints:

- `GET /users/` - List all users
- `POST /users/` - Create a new user
- `GET /users/{id}/` - Retrieve a user
- `PUT /users/{id}/` - Update a user
- `DELETE /users/{id}/` - Delete a user

## 🎓 Learn by Example

### With Database

```rust
use reinhardt::prelude::*;

#[derive(Model, Serialize, Deserialize)]
#[reinhardt(table_name = "users")]
struct User {
    #[reinhardt(primary_key)]
    id: i64,
    email: String,
    name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let db = Database::connect("postgres://localhost/mydb").await?;
    let router = DefaultRouter::new().with_database(db);

    reinhardt::serve("127.0.0.1:8000", router).await?;
    Ok(())
}
```

### With Authentication

```rust
use reinhardt::prelude::*;

#[endpoint(GET, "/profile")]
async fn get_profile(
    user: Authenticated<User>,
) -> Json<UserProfile> {
    Json(user.to_profile())
}
```

### With Dependency Injection

```rust
use reinhardt::prelude::*;

async fn get_db() -> Database {
    Database::from_env()
}

#[endpoint(GET, "/users/{id}")]
async fn get_user(
    Path(id): Path<i64>,
    Depends(db): Depends<Database, get_db>,
) -> Result<Json<User>> {
    let user = User::find_by_id(id, &db).await?;
    Ok(Json(user))
}
```

### With Serializers and Validation

```rust
use reinhardt::prelude::*;

#[derive(Serialize, Deserialize, Validate)]
struct CreateUserRequest {
    #[validate(email)]
    email: String,
    #[validate(length(min = 3, max = 50))]
    name: String,
}

#[derive(Serializer)]
#[serializer(model = "User")]
struct UserSerializer {
    id: i64,
    email: String,
    name: String,
}

#[endpoint(POST, "/users")]
async fn create_user(
    Json(req): Json<CreateUserRequest>,
    db: Depends<Database>,
) -> Result<Json<UserSerializer>> {
    req.validate()?;
    let user = User::create(&req, &db).await?;
    Ok(Json(UserSerializer::from(user)))
}
```

## Choosing the Right Flavor

| Feature      | Micro    | Standard  | Full    |
|--------------|----------|-----------|---------|
| Binary Size  | ~5-10 MB | ~20-30 MB | ~50+ MB |
| Compile Time | Fast     | Medium    | Slower  |
| **Core Features**     |
| Routing               | ✅       | ✅        | ✅      |
| Parameter Extraction  | ✅       | ✅        | ✅      |
| Dependency Injection  | ✅       | ✅        | ✅      |
| **Standard Features** |
| ORM                   | Optional | ✅        | ✅      |
| Serializers           | ❌       | ✅        | ✅      |
| ViewSets              | ❌       | ✅        | ✅      |
| Authentication        | ❌       | ✅        | ✅      |
| Pagination            | ❌       | ✅        | ✅      |
| **Advanced Features** |
| Admin Panel           | ❌       | ❌        | ✅      |
| GraphQL               | ❌       | ❌        | ✅      |
| WebSockets            | ❌       | ❌        | ✅      |
| i18n                  | ❌       | ❌        | ✅      |
| **Use Case**          |
| Microservices         | ✅       | ⚠️        | ❌      |
| REST APIs             | ✅       | ✅        | ✅      |
| Full Applications     | ❌       | ✅        | ✅      |
| Complex Systems       | ❌       | ⚠️        | ✅      |

**Legend**: ✅ Recommended • ⚠️ Possible but not optimal • ❌ Not recommended

## Components

Reinhardt includes the following core components:

### Core Framework

- **ORM**: Database abstraction layer with QuerySet API
- **Serializers**: Type-safe data serialization and validation
- **ViewSets**: Composable views for API endpoints
- **Routers**: Automatic URL routing configuration
- **Authentication**: JWT auth, permissions system
- **Middleware**: Request/response processing pipeline
- **Management Commands**: Django-style CLI for project management (`reinhardt-commands`)

### REST API Features (reinhardt-rest)

- **Authentication**: JWT, Token, Session, and Basic authentication
- **Routing**: Automatic URL routing for ViewSets
- **Browsable API**: HTML interface for API exploration
- **Schema Generation**: OpenAPI/Swagger documentation
- **Pagination**: PageNumber, LimitOffset, and Cursor pagination
- **Filtering**: SearchFilter and OrderingFilter for querysets
- **Throttling**: Rate limiting (AnonRateThrottle, UserRateThrottle, ScopedRateThrottle)
- **Signals**: Event-driven hooks (pre_save, post_save, pre_delete, post_delete, m2m_changed)

### FastAPI Inspired Features

- **Parameter Extraction**: Type-safe `Path<T>`, `Query<T>`, `Header<T>`, `Cookie<T>`, `Json<T>`, `Form<T>` extractors
- **Dependency Injection**: FastAPI-style DI system with `Depends<T>`, request scoping, and caching
- **Auto Schema Generation**: Derive OpenAPI schemas from Rust types with `#[derive(Schema)]`
- **Function-based Endpoints**: Ergonomic `#[endpoint]` macro for defining API routes (coming soon)
- **Background Tasks**: Simple background task execution

## Documentation

- 📚 [Getting Started Guide](docs/GETTING_STARTED.md) - Step-by-step tutorial for beginners
- 🎛️ [Feature Flags Guide](docs/FEATURE_FLAGS.md) - Optimize your build with granular feature control
- 📖 [API Reference](https://docs.rs/reinhardt) (Coming soon)
- 📝 [Tutorials](docs/tutorials/) - Learn by building real applications

## 💬 Getting Help

Reinhardt is a community-driven project. Here's where you can get help:

- 💬 **Discord**: Join our Discord server for real-time chat (coming soon)
- 💭 **GitHub Discussions**: [Ask questions and share ideas](https://github.com/yourusername/reinhardt/discussions)
- 🐛 **Issues**: [Report bugs](https://github.com/yourusername/reinhardt/issues)
- 📖 **Documentation**: [Read the guides](docs/)

Before asking, please check:

- ✅ [Getting Started Guide](docs/GETTING_STARTED.md)
- ✅ [Examples](examples/)
- ✅ Existing GitHub Issues and Discussions

## 🤝 Contributing

We love contributions! Please read our [Contributing Guide](CONTRIBUTING.md) to get started.

**Quick links**:

- [Development Setup](CONTRIBUTING.md#development-setup)
- [Testing Guidelines](CONTRIBUTING.md#testing-guidelines)
- [Commit Guidelines](CONTRIBUTING.md#commit-guidelines)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Third-Party Attribution

This project is inspired by:

- [Django](https://www.djangoproject.com/) (BSD 3-Clause License)
- [Django REST Framework](https://www.django-rest-framework.org/) (BSD 3-Clause License)
- [FastAPI](https://fastapi.tiangolo.com/) (MIT License)
- [SQLAlchemy](https://www.sqlalchemy.org/) (MIT License)

See [THIRD-PARTY-NOTICES](THIRD-PARTY-NOTICES) for full attribution.

**Note:** This project is not affiliated with or endorsed by the Django Software Foundation, Encode OSS Ltd., Sebastián Ramírez (FastAPI author), or Michael Bayer (SQLAlchemy author).