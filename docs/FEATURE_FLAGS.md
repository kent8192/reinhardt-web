# Feature Flags Guide

Reinhardt uses feature flags to give you fine-grained control over which components are included in your build. This allows you to optimize compilation time, binary size, and dependencies based on your project's needs.

## Overview

Reinhardt provides three pre-configured feature sets plus individual flags for custom configurations:

- **minimal** - Core routing and dependency injection only
- **standard** (default) - Balanced setup for most REST APIs
- **full** - Everything included, Django-style batteries-included

## Quick Start

### Using Pre-configured Feature Sets

```toml
# Minimal (microservices)
[dependencies]
reinhardt = { version = "0.1.0", default-features = false, features = ["minimal"] }

# Standard (default - most common)
[dependencies]
reinhardt = "0.1.0"
# Or explicitly:
# reinhardt = { version = "0.1.0", features = ["standard"] }

# Full (all features)
[dependencies]
reinhardt = { version = "0.1.0", features = ["full"] }
```

## Feature Set Comparison

| Feature Category                   | Minimal  | Standard  | Full    |
| ---------------------------------- | -------- | --------- | ------- |
| **Binary Size**                    | ~5-10 MB | ~20-30 MB | ~50+ MB |
| **Compile Time**                   | Fast     | Medium    | Slower  |
| **Core Features**                  |
| Routing & Views                    | ✅       | ✅        | ✅      |
| Parameters (`Path`, `Query`, etc.) | ✅       | ✅        | ✅      |
| Dependency Injection               | ✅       | ✅        | ✅      |
| **Database**                       |
| ORM                                | ❌       | ✅        | ✅      |
| Migrations                         | ❌       | ❌        | ✅      |
| Content Types                      | ❌       | ❌        | ✅      |
| **REST API**                       |
| Serializers                        | ❌       | ✅        | ✅      |
| ViewSets                           | ❌       | ✅        | ✅      |
| Parsers                            | ❌       | ✅        | ✅      |
| Renderers                          | ❌       | ✅        | ✅      |
| Pagination                         | ❌       | ✅        | ✅      |
| Filtering                          | ❌       | ✅        | ✅      |
| Throttling                         | ❌       | ✅        | ✅      |
| Versioning                         | ❌       | ✅        | ✅      |
| **Security**                       |
| Authentication                     | ❌       | ✅        | ✅      |
| Permissions                        | ❌       | ✅        | ✅      |
| **Advanced**                       |
| Admin Panel                        | ❌       | ❌        | ✅      |
| Forms                              | ❌       | ❌        | ✅      |
| Templates                          | ❌       | ❌        | ✅      |
| GraphQL                            | ❌       | ❌        | ✅      |
| WebSockets                         | ❌       | ❌        | ✅      |
| Internationalization               | ❌       | ❌        | ✅      |
| Mail                               | ❌       | ❌        | ✅      |
| Sessions                           | ❌       | ❌        | ✅      |
| Static Files                       | ❌       | ❌        | ✅      |
| Storage                            | ❌       | ❌        | ✅      |

## Individual Feature Flags

For custom configurations, you can mix and match individual features:

### Core Features

```toml
[dependencies]
reinhardt = { version = "0.1.0", default-features = false, features = [
    "minimal",      # Views, params, and DI
] }
```

**Included in `minimal`:**

- `reinhardt-views` - View functions and classes
- `reinhardt-params` - Path, Query, Header, Cookie, Json, Form extractors
- `reinhardt-di` - Dependency injection system

### Database Features

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = [
    "standard",
    "database",     # Adds ORM, migrations, and content types
] }
```

**`database` includes:**

- `reinhardt-orm` - ORM and QuerySet API
- `reinhardt-migrations` - Database migration system
- `reinhardt-contenttypes` - Generic foreign keys
- `reinhardt-db` - Low-level database operations

### REST API Features

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = [
    "minimal",
    "api",          # Serializers and ViewSets
] }
```

**`api` includes:**

- `reinhardt-serializers` - Data serialization and validation
- `reinhardt-viewsets` - CRUD views for models

### Authentication & Security

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = [
    "standard",
    "auth",         # JWT, Token, Session authentication
] }
```

**`auth` includes:**

- `reinhardt-auth` - Authentication backends and permissions

### Additional Features

#### Admin Panel

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["admin"] }
```

**Includes:**

- `reinhardt-forms` - Form handling and validation
- `reinhardt-templates` - Template rendering
- (Note: `reinhardt-admin` is planned for future release)

#### Forms

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["forms"] }
```

**Includes:**

- `reinhardt-forms` - Standalone form handling

#### GraphQL

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["graphql"] }
```

**Includes:**

- `reinhardt-graphql` - GraphQL schema and resolvers

#### Templates

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["templates"] }
```

**Includes:**

- `reinhardt-templates` - Template engine
- `reinhardt-template` - Template utilities

#### WebSockets

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["websockets"] }
```

**Includes:**

- `reinhardt-websockets` - WebSocket support

#### Caching

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["cache"] }
```

**Includes:**

- `reinhardt-cache` - Caching backends (Redis, in-memory)

#### Internationalization (i18n)

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["i18n"] }
```

**Includes:**

- `reinhardt-i18n` - Translation and localization

#### Email

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["mail"] }
```

**Includes:**

- `reinhardt-mail` - Email sending utilities

#### Sessions

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["sessions"] }
```

**Includes:**

- `reinhardt-sessions` - Session management

#### Static Files

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["static-files"] }
```

**Includes:**

- `reinhardt-static` - Static file serving

#### Storage

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["storage"] }
```

**Includes:**

- `reinhardt-storage` - File storage backends (S3, local)

#### Contrib (All contrib apps)

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["contrib"] }
```

**Includes:**

- `reinhardt-contrib` - All contrib applications

### Parent Crate Features

These provide access to major subsystems:

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = [
    "conf",         # Configuration system
    "core",         # Core utilities
    "rest",         # REST framework
    "di",           # Dependency injection (already in minimal)
    "test",         # Testing utilities
] }
```

## Common Configurations

### Microservice API

Lightweight, fast, minimal dependencies:

```toml
[dependencies]
reinhardt = { version = "0.1.0", default-features = false, features = ["minimal"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

### REST API with Database

Standard REST API with database support:

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["standard", "database"] }
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-native-tls"] }
```

### Full-Stack Application

Everything included:

```toml
[dependencies]
reinhardt = { version = "0.1.0", features = ["full"] }
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-native-tls"] }
```

### Custom Configuration

Pick exactly what you need:

```toml
[dependencies]
reinhardt = { version = "0.1.0", default-features = false, features = [
    "minimal",      # Core routing and DI
    "database",     # Database support
    "api",          # Serializers and ViewSets
    "auth",         # Authentication
    "cache",        # Caching
] }
```

## Feature Dependencies

Some features automatically enable others:

- `standard` → includes `minimal`
- `full` → includes `standard` (and therefore `minimal`)
- `database` → enables `reinhardt-orm`, `reinhardt-migrations`, `reinhardt-contenttypes`, `reinhardt-db`
- `api` → enables `reinhardt-serializers`, `reinhardt-viewsets`

## Optimization Tips

### Reduce Binary Size

1. Use `minimal` and add only what you need
2. Enable link-time optimization in `Cargo.toml`:

```toml
[profile.release]
lto = true
codegen-units = 1
strip = true
```

### Faster Compilation

1. Use fewer features
2. Use `minimal` for development
3. Enable parallel compilation:

```toml
[profile.dev]
codegen-units = 16
```

### Minimal Dependencies

For the smallest possible binary:

```toml
[dependencies]
reinhardt = { version = "0.1.0", default-features = false, features = ["minimal"] }
```

Then use `cargo tree` to verify:

```bash
cargo tree --features minimal
```

## Feature Flag Reference

### Complete List

| Flag           | Description                     |
| -------------- | ------------------------------- |
| `minimal`      | Core routing, params, and DI    |
| `standard`     | Balanced setup (default)        |
| `full`         | All features                    |
| `database`     | ORM, migrations, content types  |
| `api`          | Serializers and ViewSets        |
| `auth`         | Authentication and permissions  |
| `admin`        | Admin panel (forms + templates) |
| `forms`        | Form handling                   |
| `graphql`      | GraphQL support                 |
| `templates`    | Template engine                 |
| `websockets`   | WebSocket support               |
| `cache`        | Caching backends                |
| `i18n`         | Internationalization            |
| `mail`         | Email utilities                 |
| `sessions`     | Session management              |
| `static-files` | Static file serving             |
| `storage`      | File storage backends           |
| `contrib`      | All contrib apps                |
| `conf`         | Configuration system            |
| `core`         | Core utilities                  |
| `rest`         | REST framework                  |
| `di`           | Dependency injection            |
| `test`         | Testing utilities               |

## Troubleshooting

### Feature Not Found

If you get "feature not found" errors:

1. Check the spelling of the feature name
2. Verify you're using the correct version of Reinhardt
3. Some features may not be available yet (check the roadmap)

### Compilation Errors

If features cause compilation errors:

1. Make sure all required dependencies are included
2. Check that feature combinations are compatible
3. Update to the latest version: `cargo update reinhardt`

### Missing Functionality

If expected functionality is missing:

1. Verify the feature is enabled: `cargo tree --features <feature>`
2. Check the [API documentation](https://docs.rs/reinhardt)
3. Consult the [tutorials](tutorials/README.md)

## Further Reading

- [Getting Started Guide](GETTING_STARTED.md)
- [API Reference](https://docs.rs/reinhardt)
- [Tutorials](tutorials/README.md)

---

Need help? Check the [GitHub Discussions](https://github.com/your-org/reinhardt/discussions) or [open an issue](https://github.com/your-org/reinhardt/issues).
