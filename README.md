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

### 1. Install Reinhardt Admin Tool

```bash
cargo install reinhardt-admin
```

### 2. Create a New Project

```bash
# Create a RESTful API project
reinhardt-admin startproject my-api --template-type restful
cd my-api

# Or create a Model-Template-View (MTV) project
reinhardt-admin startproject my-web --template-type mtv
```

This generates a complete project structure:

```
my-api/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── apps.rs
│   ├── config/
│   │   ├── settings.rs
│   │   ├── settings/
│   │   │   ├── base.rs
│   │   │   ├── local.rs
│   │   │   ├── staging.rs
│   │   │   └── production.rs
│   │   ├── urls.rs
│   │   └── apps.rs
│   └── bin/
│       ├── runserver.rs
│       └── manage.rs
└── README.md
```

### 3. Run the Development Server

```bash
# Using the runserver binary (recommended)
cargo run --bin runserver

# Or using manage command
cargo run --bin manage runserver

# Server will start at http://127.0.0.1:8000
```

### 4. Create Your First App

```bash
# Create a new app
cargo run --bin manage startapp users --template-type restful
```

This creates an app structure:

```
users/
├── lib.rs
├── models.rs
├── models/
├── views.rs
├── views/
├── serializers.rs
├── serializers/
├── admin.rs
├── urls.rs
└── tests.rs
```

### 5. Register ViewSets

Edit your app's `urls.rs`:

```rust
use reinhardt_routers::UnifiedRouter;
use crate::views::UserViewSet;

pub fn url_patterns() -> UnifiedRouter {
    let router = UnifiedRouter::builder()
        .build();

    // Register ViewSet for automatic CRUD endpoints
    router.register_viewset("users", UserViewSet::new());

    router
}
```

Include in `src/config/urls.rs`:

```rust
use reinhardt::prelude::*;
use std::sync::Arc;

pub fn url_patterns() -> Arc<UnifiedRouter> {
    let router = UnifiedRouter::builder()
        .build();

    // Include app routers
    router.include_router("/api/", users::urls::url_patterns(), Some("users".to_string()));

    Arc::new(router)
}
```

For a complete step-by-step guide, see [Getting Started](docs/GETTING_STARTED.md).

## 🎓 Learn by Example

### With Database

Configure database in `src/config/settings/base.rs`:

```rust
use reinhardt::prelude::*;

pub struct Settings {
    pub database_url: String,
    pub debug: bool,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost/mydb".to_string()),
            debug: true,
        }
    }
}
```

Define models in your app (e.g., `users/models.rs`):

```rust
use reinhardt::prelude::*;

#[derive(Model, Serialize, Deserialize)]
#[reinhardt(table_name = "users")]
pub struct User {
    #[reinhardt(primary_key)]
    pub id: i64,
    pub email: String,
    pub name: String,
}
```

Register in `src/config/apps.rs`:

```rust
use reinhardt_macros::installed_apps;

installed_apps! {
    auth: "reinhardt.contrib.auth",
    contenttypes: "reinhardt.contrib.contenttypes",
    users: "users",
}

pub fn get_installed_apps() -> Vec<String> {
    InstalledApp::all_apps()
}
```

### With Authentication

Add to your app's `views/profile.rs`:

```rust
use reinhardt::prelude::*;
use crate::models::User;

#[endpoint(GET, "/profile")]
pub async fn get_profile(
    user: Authenticated<User>,
) -> Json<UserProfile> {
    Json(user.to_profile())
}
```

Re-export in your app's `views.rs`:

```rust
mod profile;
pub use profile::*;
```

Register URL in your app's `urls.rs`:

```rust
use reinhardt_routers::UnifiedRouter;
use crate::views;

pub fn url_patterns() -> UnifiedRouter {
    let router = UnifiedRouter::builder()
        .build();

    router.add_function_route("/profile", Method::GET, views::get_profile);

    router
}
```

### With Dependency Injection

In your app's `views/user.rs`:

```rust
use reinhardt::prelude::*;
use crate::models::User;

async fn get_db() -> Database {
    Database::from_env()
}

#[endpoint(GET, "/users/{id}")]
pub async fn get_user(
    Path(id): Path<i64>,
    Depends(db): Depends<Database, get_db>,
) -> Result<Json<User>> {
    let user = User::find_by_id(id, &db).await?;
    Ok(Json(user))
}
```

Re-export in your app's `views.rs`:

```rust
mod user;
pub use user::*;
```

### With Serializers and Validation

In your app's `serializers/user.rs`:

```rust
use reinhardt::prelude::*;

#[derive(Serialize, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 3, max = 50))]
    pub name: String,
}

#[derive(Serializer)]
#[serializer(model = "User")]
pub struct UserSerializer {
    pub id: i64,
    pub email: String,
    pub name: String,
}
```

Re-export in your app's `serializers.rs`:

```rust
mod user;
pub use user::*;
```

In your app's `views/user.rs`:

```rust
use reinhardt::prelude::*;
use crate::models::User;
use crate::serializers::{CreateUserRequest, UserSerializer};

#[endpoint(POST, "/users")]
pub async fn create_user(
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