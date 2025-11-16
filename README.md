<div align="center">
  <img src="branding/logo.png" alt="Reinhardt Logo" width="200"/>

  <h1>Reinhardt</h1>

  <h3>🦀 Polylithic Batteries Included</h3>

  <p><strong>A composable full-stack API framework for Rust</strong></p>
  <p>Build with <em>all</em> the power of Django's batteries-included philosophy,<br/>
  or compose <em>only</em> what you need—your choice, your way.</p>

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

**Polylithic = Poly (many) + Lithic (building blocks)**
Unlike monolithic frameworks that force you to use everything, Reinhardt lets you compose your perfect stack from independent, well-tested components.

Reinhardt brings together the best of three worlds:

| Inspiration        | What We Borrowed                                       | What We Improved                                     |
|--------------------|--------------------------------------------------------|------------------------------------------------------|
| 🐍 **Django**      | Batteries-included philosophy, ORM design, admin panel | Feature flags for composable builds, Rust's type safety |
| 🎯 **Django REST** | Serializers, ViewSets, permissions                     | Compile-time validation, zero-cost abstractions      |
| ⚡ **FastAPI**      | DI system, automatic OpenAPI                           | Native Rust performance, no runtime overhead         |
| 🗄️ **SQLAlchemy** | QuerySet patterns, relationship handling               | Type-safe query builder, compile-time validation     |

**Result**: A framework that's familiar to Python developers, but with Rust's performance and safety guarantees.

## ✨ Features

### 🎯 Core Framework

- **Type-Safe ORM**: QuerySet API with compile-time query validation (using SeaQuery v1.0.0-rc1)
- **Powerful Serializers**: Automatic validation and transformation with serde and validator
- **Smart Routing**: Function-based and class-based route registration
- **Multi-Auth Support**: JWT, Token, Session, and Basic authentication with BaseUser/FullUser traits
- **ViewSets**: DRY principle for CRUD operations with ModelViewSet and ReadOnlyModelViewSet

### 🚀 FastAPI-Inspired Ergonomics

- **Parameter Extraction**: Access path and query parameters via `Request` fields
- **Dependency Injection**: DI system for managing application dependencies (coming soon)
- **Auto OpenAPI**: Generate OpenAPI 3.0 schemas from Rust types with `#[derive(Schema)]`
- **Function-based Endpoints**: Register functions as route handlers
- **Background Tasks**: Simple async task execution

### 🔋 Batteries Included

- **Admin Panel**: Django-style auto-generated admin interface with model management, filtering, and custom actions
- **Middleware System**: Request/response processing pipeline
- **Management Commands**: CLI tools for migrations, static files, and more
- **Pagination**: PageNumber, LimitOffset, and Cursor strategies
- **Filtering & Search**: Built-in SearchFilter and OrderingFilter for querysets
- **Rate Limiting**: Flexible throttling (AnonRateThrottle, UserRateThrottle, ScopedRateThrottle)
- **Signals**: Event-driven hooks (pre_save, post_save, pre_delete, post_delete, m2m_changed)

### 🌍 Advanced Features

- **GraphQL Support**: Build GraphQL APIs alongside REST with schema generation and subscriptions
- **WebSocket Support**: Real-time bidirectional communication with channels, rooms, and authentication
- **Internationalization**: Multi-language support
- **Static Files**: CDN integration, hashed storage, and compression
- **Browsable API**: HTML interface for API exploration

## Installation

Compose your perfect framework—Reinhardt offers three ready-made flavors:

### Reinhardt Micro - For Microservices

Lightweight and fast, perfect for simple APIs and microservices:

```toml
[dependencies]
reinhardt-micro = "0.1.0-alpha.1"
```

### Reinhardt Standard - Balanced Approach

The default configuration, suitable for most projects:

```toml
[dependencies]
reinhardt = "0.1.0-alpha.1"
# Equivalent to: reinhardt = { version = "0.1.0-alpha.1", features = ["standard"] }
```

### Reinhardt Full - Everything Included

All features enabled, Django-style batteries-included:

```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }
```

### Compose Your Own Configuration

Mix and match features to build your ideal framework:

```toml
[dependencies]
# Minimal setup with just routing and params
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal"] }

# Add database support
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "database"] }

# Standard with extra features
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "websockets", "graphql"] }
```

**📖 For a complete list of available feature flags and detailed configuration options, see the [Feature Flags Guide](docs/FEATURE_FLAGS.md).**

## Quick Start

### 1. Install Reinhardt Admin Tool

```bash
cargo install reinhardt-admin-cli
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

### 3. Setup Git Hooks (Recommended)

Run the setup script to install pre-commit hooks:

```bash
./scripts/setup-hooks.sh
```

This will automatically check code formatting and linting before each commit.

### 4. Verify Docker Setup (Required for Integration Tests)

Reinhardt uses Docker for TestContainers integration in database and infrastructure tests.

```bash
# Verify Docker is installed and running
docker version
docker ps

# Both commands should succeed without errors
```

**Important Notes:**

- **Docker Desktop must be running** before executing integration tests
- If you have both Docker and Podman installed, ensure `DOCKER_HOST` environment variable is **not** set to a Podman socket
- The project includes `.testcontainers.properties` to ensure Docker is used

**Troubleshooting:**

If you encounter "Cannot connect to the Docker daemon" errors during tests:

```bash
# Check if Docker is running
docker ps

# Check DOCKER_HOST environment variable
echo $DOCKER_HOST

# It should be empty or point to Docker socket:
# ✅ Correct: (empty) or unix:///var/run/docker.sock
# ❌ Incorrect: unix:///.../podman/... (needs to be unset)

# Unset DOCKER_HOST if pointing to Podman
unset DOCKER_HOST
```

### 5. Run the Development Server

```bash
# Using the runserver binary (recommended)
cargo run --bin runserver

# Or using manage command
cargo run --bin manage runserver

# Server will start at http://127.0.0.1:8000
```

**Auto-Reload Support:**

For automatic reloading on code changes (like Django's runserver):

```bash
# Install cargo-watch
cargo install cargo-watch

# Enable cargo-watch-reload feature in Cargo.toml
# [dependencies]
# reinhardt-commands = { version = "0.1.0-alpha.1", features = ["cargo-watch-reload"] }

# Run with auto-reload (detects changes, rebuilds, and restarts automatically)
cargo run --bin runserver

# Optional: Clear screen before each rebuild
cargo run --bin runserver -- --clear

# Optional: Disable auto-reload
cargo run --bin runserver -- --noreload
```

See [Feature Flags Guide](docs/FEATURE_FLAGS.md) for more auto-reload options.

### 6. Create Your First App

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

### 7. Register Routes

Edit your app's `urls.rs`:

```rust
use reinhardt_routers::UnifiedRouter;
use hyper::Method;
use crate::views;

pub fn url_patterns() -> UnifiedRouter {
    UnifiedRouter::new()
        .function("/users", Method::GET, views::list_users)
        .function("/users/{id}", Method::GET, views::get_user)
        .function("/users", Method::POST, views::create_user)
}
```

Include in `src/config/urls.rs`:

```rust
use reinhardt_routers::UnifiedRouter;
use std::sync::Arc;

pub fn url_patterns() -> Arc<UnifiedRouter> {
    let router = UnifiedRouter::new()
        .mount("/api/", users::urls::url_patterns());

    Arc::new(router)
}
```

For a complete step-by-step guide, see [Getting Started](docs/GETTING_STARTED.md).

## 🎓 Learn by Example

### With Database

Configure database in `settings/base.toml`:

```toml
debug = true
secret_key = "your-secret-key-for-development"

[database]
engine = "postgresql"
host = "localhost"
port = 5432
name = "mydb"
user = "postgres"
password = "postgres"
```

Settings are automatically loaded in `src/config/settings.rs`:

```rust
use reinhardt_conf::settings::prelude::*;
use reinhardt_core::Settings;

pub fn get_settings() -> Settings {
    let profile_str = env::var("REINHARDT_ENV").unwrap_or_else(|_| "local".to_string());
    let profile = Profile::from_str(&profile_str).unwrap_or(Profile::Development);

    let settings_dir = PathBuf::from("settings");

    SettingsBuilder::new()
        .profile(profile)
        .add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
        .add_source(TomlFileSource::new(settings_dir.join("base.toml")))
        .add_source(TomlFileSource::new(settings_dir.join(format!("{}.toml", profile_str))))
        .build()
        .expect("Failed to build settings")
        .into_typed()
        .expect("Failed to convert settings")
}
```

**Priority Order**: `{profile}.toml` > `base.toml` > Environment Variables > Defaults

See [Settings Documentation](docs/SETTINGS_DOCUMENT.md) for more details.

Define models in your app (e.g., `users/models.rs`):

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub username: String,
    pub is_active: bool,
}

impl User {
    pub async fn find_by_id(id: i64) -> Result<Self, Box<dyn std::error::Error>> {
        // Query database using SeaQuery
        // This is a simplified example
        todo!("Implement database query with SeaQuery")
    }

    pub async fn save(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Insert or update using SeaQuery
        todo!("Implement database save with SeaQuery")
    }

    pub async fn delete(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Delete from database
        todo!("Implement database delete with SeaQuery")
    }
}
```

**Note**: Reinhardt uses [SeaQuery v1.0.0-rc1](https://crates.io/crates/sea-query) for SQL operations. The `#[derive(Model)]` macro is planned for future release to reduce boilerplate.

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

Reinhardt provides Django-style user models with `BaseUser` and `FullUser` traits.

Define your user model in `users/models.rs`:

```rust
use reinhardt_auth::{BaseUser, FullUser, PermissionsMixin};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub is_active: bool,
    pub is_staff: bool,
    pub is_superuser: bool,
    pub last_login: Option<DateTime<Utc>>,
    pub date_joined: DateTime<Utc>,
}

impl BaseUser for User {
    type PrimaryKey = Uuid;

    fn get_username_field() -> &'static str { "username" }
    fn get_username(&self) -> &str { &self.username }
    fn password_hash(&self) -> Option<&str> { self.password_hash.as_deref() }
    fn set_password_hash(&mut self, hash: String) { self.password_hash = Some(hash); }
    fn last_login(&self) -> Option<DateTime<Utc>> { self.last_login }
    fn set_last_login(&mut self, time: DateTime<Utc>) { self.last_login = Some(time); }
    fn is_active(&self) -> bool { self.is_active }
}

impl FullUser for User {
    fn username(&self) -> &str { &self.username }
    fn email(&self) -> &str { &self.email }
    fn first_name(&self) -> &str { &self.first_name }
    fn last_name(&self) -> &str { &self.last_name }
    fn is_staff(&self) -> bool { self.is_staff }
    fn is_superuser(&self) -> bool { self.is_superuser }
    fn date_joined(&self) -> DateTime<Utc> { self.date_joined }
}
```

Use JWT authentication in your app's `views/profile.rs`:

```rust
use reinhardt_auth::{JwtAuth, BaseUser};
use reinhardt_http::{Request, Response, StatusCode};
use crate::models::User;

pub async fn get_profile(req: Request) -> Result<Response, Box<dyn std::error::Error>> {
    // Extract JWT token from Authorization header
    let auth_header = req.headers.get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or("Missing Authorization header")?;

    let token = auth_header.strip_prefix("Bearer ")
        .ok_or("Invalid Authorization header format")?;

    // Verify token and get user ID
    let jwt_auth = JwtAuth::new(b"your-secret-key");
    let claims = jwt_auth.verify_token(token)?;

    // Load user from database using claims.user_id
    let user = User::find_by_id(&claims.user_id).await?;

    // Check if user is active
    if !user.is_active() {
        return Err("User account is inactive".into());
    }

    // Return user profile as JSON
    let json = serde_json::to_string(&user)?;
    Ok(Response::new(StatusCode::OK, json.into()))
}
```

### With Parameter Extraction

In your app's `views/user.rs`:

```rust
use reinhardt_http::{Request, Response, StatusCode};
use crate::models::User;

pub async fn get_user(req: Request) -> Result<Response, Box<dyn std::error::Error>> {
    // Extract path parameter from request
    let id = req.path_params.get("id")
        .ok_or("Missing id parameter")?
        .parse::<i64>()
        .map_err(|_| "Invalid id format")?;

    // Extract query parameters (e.g., ?include_inactive=true)
    let include_inactive = req.query_params.get("include_inactive")
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);

    // Fetch user from database
    let user = User::find_by_id(id).await?;

    // Check active status if needed
    if !include_inactive && !user.is_active {
        return Err("User is inactive".into());
    }

    // Return as JSON
    let json = serde_json::to_string(&user)?;
    Ok(Response::new(StatusCode::OK, json.into()))
}
```

Register route with path parameter in `urls.rs`:

```rust
use reinhardt_routers::UnifiedRouter;
use hyper::Method;
use crate::views;

pub fn url_patterns() -> UnifiedRouter {
    UnifiedRouter::new()
        .function("/users/:id", Method::GET, views::get_user)
}
```

### With Serializers and Validation

In your app's `serializers/user.rs`:

```rust
use serde::{Serialize, Deserialize};
use validator::Validate;

#[derive(Serialize, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    #[validate(length(min = 8))]
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct UserResponse {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub is_active: bool,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        UserResponse {
            id: user.id,
            username: user.username,
            email: user.email,
            is_active: user.is_active,
        }
    }
}
```

In your app's `views/user.rs`:

```rust
use reinhardt_http::{Request, Response, StatusCode};
use crate::models::User;
use crate::serializers::{CreateUserRequest, UserResponse};
use validator::Validate;

pub async fn create_user(mut req: Request) -> Result<Response, Box<dyn std::error::Error>> {
    // Parse request body
    let body_bytes = std::mem::take(&mut req.body);
    let create_req: CreateUserRequest = serde_json::from_slice(&body_bytes)?;

    // Validate request
    create_req.validate()?;

    // Create user
    let mut user = User {
        id: 0, // Will be set by database
        username: create_req.username,
        email: create_req.email,
        password_hash: None,
        is_active: true,
        // ... other fields
    };

    // Hash password using BaseUser trait
    user.set_password(&create_req.password)?;

    // Save to database
    user.save().await?;

    // Convert to response
    let response_data = UserResponse::from(user);
    let json = serde_json::to_string(&response_data)?;

    Ok(Response::new(StatusCode::CREATED, json.into()))
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
| **Standard Features** |
| ORM (SeaQuery)        | Optional | ✅        | ✅      |
| Serializers           | ❌       | ✅        | ✅      |
| Authentication        | ❌       | ✅        | ✅      |
| Pagination            | ❌       | ✅        | ✅      |
| ViewSets              | ❌       | ✅        | ✅      |
| **Advanced Features** |
| Admin Panel           | ❌       | ❌        | ✅      |
| GraphQL               | ❌       | ❌        | ✅      |
| WebSockets            | ❌       | ❌        | ✅      |
| i18n                  | ❌       | ❌        | ✅      |
| **Planned Features**  |
| FastAPI-style DI      | ❌       | 🔜       | 🔜      |
| #[derive(Model)]      | ❌       | 🔜       | 🔜      |
| **Use Case**          |
| Microservices         | ✅       | ⚠️        | ❌      |
| REST APIs             | ✅       | ✅        | ✅      |
| Full Applications     | ❌       | ✅        | ✅      |
| Complex Systems       | ❌       | ⚠️        | ✅      |

**Legend**: ✅ Recommended • ⚠️ Possible but not optimal • ❌ Not recommended

**Need more granular control?** The [Feature Flags Guide](docs/FEATURE_FLAGS.md) provides detailed documentation on 70+ individual feature flags, allowing you to fine-tune your build beyond these presets.

## Components

Reinhardt includes the following core components:

### Core Framework

- **ORM**: Database abstraction layer using SeaQuery v1.0.0-rc1 for SQL operations
- **Serializers**: Type-safe data serialization and validation with serde and validator crates
- **Routers**: Function-based and class-based URL routing
- **Authentication**: JWT, Token, Session, and Basic authentication with BaseUser/FullUser traits
- **Middleware**: Request/response processing pipeline
- **Management Commands**: Django-style CLI for project management (`reinhardt-admin-cli`)

### REST API Features (reinhardt-rest)

- **Authentication**: JWT, Token, Session, and Basic authentication
- **Browsable API**: HTML interface for API exploration
- **Schema Generation**: OpenAPI/Swagger documentation
- **Pagination**: PageNumber, LimitOffset, and Cursor pagination
- **Filtering**: SearchFilter and OrderingFilter for querysets
- **Throttling**: Rate limiting (AnonRateThrottle, UserRateThrottle, ScopedRateThrottle)
- **Signals**: Event-driven hooks (pre_save, post_save, pre_delete, post_delete, m2m_changed)

### Advanced Features

- **Admin Panel**: Fully-featured admin interface (`reinhardt-admin-panel`) with model management, filtering, bulk actions, and audit logging
- **GraphQL**: Complete GraphQL support (`reinhardt-graphql`) with schema generation and subscription support
- **WebSockets**: Real-time communication (`reinhardt-websockets`) with channels, rooms, authentication, and Redis integration
- **Internationalization**: Multi-language support with translation catalogs
- **Static Files**: CDN integration, hashed storage, and compression

## Documentation

- 📚 [Getting Started Guide](docs/GETTING_STARTED.md) - Step-by-step tutorial for beginners
- 🎛️ [Feature Flags Guide](docs/FEATURE_FLAGS.md) - Optimize your build with granular feature control
- 📖 [API Reference](https://docs.rs/reinhardt) (Coming soon)
- 📝 [Tutorials](docs/tutorials/) - Learn by building real applications

**For AI Assistants**: See [CLAUDE.md](CLAUDE.md) for project-specific coding standards, testing guidelines, and development conventions.

## 💬 Getting Help

Reinhardt is a community-driven project. Here's where you can get help:

- 💬 **Discord**: Join our Discord server for real-time chat (coming soon)
- 💭 **GitHub Discussions**: [Ask questions and share ideas](https://github.com/kent8192/reinhardt-rs/discussions)
- 🐛 **Issues**: [Report bugs](https://github.com/kent8192/reinhardt-rs/issues)
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