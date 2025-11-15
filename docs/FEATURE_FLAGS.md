# Feature Flags Guide

## Table of Contents

- [Overview](#overview)
- [Basic Usage](#basic-usage)
- [Bundle Features](#bundle-features)
  - [Minimal - For Microservices](#minimal---for-microservices)
  - [Standard - Balanced Configuration](#standard---balanced-configuration)
  - [Full - All Features](#full---all-features)
  - [Preset Configurations](#preset-configurations)
- [Feature Flags by Category](#feature-flags-by-category)
  - [Database](#database)
  - [Authentication](#authentication)
  - [Cache](#cache)
  - [API Features](#api-features)
  - [Middleware](#middleware)
  - [Other Features](#other-features)
- [Feature Flags for Major Crates](#feature-flags-for-major-crates)
- [Feature Flag Dependency Map](#feature-flag-dependency-map)
- [Usage Examples and Best Practices](#usage-examples-and-best-practices)
- [Build Time and Binary Size Comparison](#build-time-and-binary-size-comparison)
- [Troubleshooting](#troubleshooting)
- [Quick Reference](#quick-reference)

---

## Overview

Reinhardt employs a **highly granular feature flag system**, allowing you to build with only the functionality you need. This provides several benefits:

### Benefits

- **Reduced Compile Time**: Significantly shorter build times by excluding unnecessary features
- **Smaller Binary Size**: Smaller executables as unused code is not included
- **Minimized Dependencies**: Only required external crates are included in the build
- **Flexible Configuration**: Optimal configuration for any use case, from microservices to full-featured applications

### Feature Flag Granularity

Reinhardt's feature flags have **3 levels of granularity**:

1. **Bundle Features**: Large groups like `minimal`, `standard`, `full`
2. **Feature Group Features**: Functional units like `database`, `auth`, `cache`
3. **Individual Features**: Fine-grained functionality like `jwt`, `redis-backend`, `cors`

With **over 70 feature flags** defined, extremely flexible configuration is possible.

---

## Basic Usage

### Default Configuration (standard)

If nothing is specified, the `standard` configuration is enabled:

```toml
[dependencies]
reinhardt = "0.1.0-alpha.1"
# This is equivalent to:
# reinhardt = { version = "0.1.0-alpha.1", features = ["standard"] }
```

### Selecting a Specific Configuration

```toml
[dependencies]
# minimal configuration
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal"] }

# full configuration
reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }
```

### Custom Configuration

```toml
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  default-features = false,  # Disable defaults
  features = [
    "minimal",        # Base minimal configuration
    "database",       # Database functionality
    "db-postgres",    # PostgreSQL support
    "auth-jwt",       # JWT authentication
    "cache",          # Cache
    "redis-backend",  # Redis backend
  ]
}
```

---

## Bundle Features

Bundle features are convenient presets that enable multiple features at once.

### Minimal - For Microservices

**Feature name**: `minimal`

**Use case**: Lightweight microservices or simple APIs

**Enabled features**:
- Parameter extraction (`reinhardt-params`)
- Dependency injection (`reinhardt-di`)

**Binary size**: ~5-10 MB
**Compile time**: Fast

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal"] }
```

**Suitable use cases**:
- ✅ Simple REST APIs
- ✅ Microservice architecture
- ✅ When fast startup time is required
- ❌ When database access is needed
- ❌ When complex authentication is required

---

### Standard - Balanced Configuration

**Feature name**: `standard` (default)

**Use case**: Balanced configuration suitable for most projects

**Enabled features**:
- All of `minimal`
- ORM (`reinhardt-orm`)
- Serializers (`reinhardt-serializers`)
- ViewSets (`reinhardt-viewsets`)
- Authentication (`reinhardt-auth`)
- Middleware (`reinhardt-middleware`)
- Pagination (`reinhardt-pagination`)
- Filtering (`reinhardt-filters`)
- Throttling (`reinhardt-throttling`)
- Signals (`reinhardt-signals`)
- Parsers (`reinhardt-parsers`)
- Renderers (`reinhardt-renderers`)
- Versioning (`reinhardt-versioning`)
- Metadata (`reinhardt-metadata`)
- Content negotiation (`reinhardt-negotiation`)
- REST API core (`reinhardt-rest`)

**Binary size**: ~20-30 MB
**Compile time**: Medium

**Usage example**:
```toml
[dependencies]
reinhardt = "0.1.0-alpha.1"
# Or explicitly
reinhardt = { version = "0.1.0-alpha.1", features = ["standard"] }
```

**Suitable use cases**:
- ✅ General REST APIs
- ✅ Applications using databases
- ✅ APIs requiring authentication
- ✅ APIs requiring pagination or filtering
- ⚠️ GraphQL or WebSocket not included (requires separate enablement)

---

### Full - All Features

**Feature name**: `full`

**Use case**: Django-style batteries-included, all features enabled

**Enabled features**:
- All of `standard`
- Database (`database`)
- Admin panel (`admin`)
- GraphQL (`graphql`)
- WebSocket (`websockets`)
- Cache (`cache`)
- Internationalization (`i18n`)
- Email sending (`mail`)
- Session management (`sessions`)
- Static file serving (`static-files`)
- Storage system (`storage`)
- Contrib apps (`contrib`)

**Binary size**: ~50+ MB
**Compile time**: Slow

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }
```

**Suitable use cases**:
- ✅ Large web applications
- ✅ Systems with complex requirements
- ✅ Providing both GraphQL and REST APIs
- ✅ When real-time features (WebSocket) are needed
- ✅ When multi-language support is required
- ❌ Microservices (overpowered)
- ❌ When minimizing compile time is desired

---

### Preset Configurations

Preset configurations optimized for specific use cases are also available.

#### api-only - REST API Only

REST API-only configuration without templates or forms.

**Enabled features**:
- All of `minimal`
- Serializers, ViewSets, authentication
- Parsers, renderers, versioning
- Metadata, content negotiation
- REST API core
- Pagination, filtering, throttling

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["api-only"] }
```

#### graphql-server - GraphQL Server

GraphQL API-centric server configuration.

**Enabled features**:
- All of `minimal`
- GraphQL (`reinhardt-graphql`)
- Authentication (`reinhardt-auth`)
- Database (`database`)

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["graphql-server"] }
```

#### websocket-server - WebSocket Server

Real-time communication-centric server configuration.

**Enabled features**:
- All of `minimal`
- WebSocket (`reinhardt-websockets`)
- Authentication (`reinhardt-auth`)
- Cache (`reinhardt-cache`)

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["websocket-server"] }
```

#### cli-tools - CLI/Background Jobs

Configuration for CLI tools and background processing.

**Enabled features**:
- Database (`database`)
- Migrations (`reinhardt-migrations`)
- Tasks (`reinhardt-tasks`)
- Email sending (`reinhardt-mail`)

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["cli-tools"] }
```

#### test-utils - Test Utilities

Configuration for test environments.

**Enabled features**:
- Test utilities (`reinhardt-test`)
- Database (`database`)

**Usage example**:
```toml
[dev-dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["test-utils"] }
```

---

## Feature Flags by Category

### Database

#### database

Enables general database functionality.

**Enabled crates**:
- `reinhardt-orm` - ORM functionality
- `reinhardt-migrations` - Migrations
- `reinhardt-contenttypes` - Content types
- `reinhardt-db` - Database foundation

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "database"] }
```

#### Database-Specific Features

Enable support for specific databases:

| Feature | Database | Description |
|---------|----------|-------------|
| `db-postgres` | PostgreSQL | PostgreSQL support |
| `db-mysql` | MySQL | MySQL support |
| `db-sqlite` | SQLite | SQLite support (lightweight, file-based) |
| `db-mongodb` | MongoDB | MongoDB support (NoSQL) |
| `db-cockroachdb` | CockroachDB | CockroachDB support (distributed SQL) |

**Usage example**:
```toml
[dependencies]
# Using PostgreSQL
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "database", "db-postgres"] }

# Multiple database support
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "database", "db-postgres", "db-sqlite"] }
```

**Note**:
- The `database` feature automatically enables PostgreSQL (`reinhardt-db` default)
- For other databases, explicitly specify the corresponding feature

---

### Authentication

#### auth

Enables basic authentication functionality.

**Enabled crates**:
- `reinhardt-auth` - Authentication foundation

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "auth"] }
```

#### Authentication Method Features

Enable specific authentication methods:

| Feature | Method | Description |
|---------|--------|-------------|
| `auth-jwt` | JWT | JSON Web Token authentication |
| `auth-session` | Session | Session-based authentication |
| `auth-oauth` | OAuth | OAuth authentication |
| `auth-token` | Token | Token authentication |

**Usage example**:
```toml
[dependencies]
# JWT authentication only
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "auth-jwt"] }

# JWT + session authentication
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "auth-jwt", "auth-session"] }
```

**Note**:
- Individual authentication method features (`auth-jwt`, etc.) automatically enable `auth`
- `auth-session` automatically enables the `sessions` feature

---

### Cache

#### cache

Enables cache functionality foundation.

**Enabled crates**:
- `reinhardt-cache` - Cache system

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "cache"] }
```

#### Cache Backend Features

| Feature | Backend | Description |
|---------|---------|-------------|
| `redis-backend` | Redis | Redis cache backend |
| `redis-cluster` | Redis Cluster | Redis cluster support |
| `redis-sentinel` | Redis Sentinel | Redis sentinel support |
| `memcached-backend` | Memcached | Memcached backend |

**Usage example**:
```toml
[dependencies]
# Redis cache
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "cache", "redis-backend"] }

# Redis cluster support
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "cache", "redis-backend", "redis-cluster"] }
```

**Dependencies**:
- External crates: `redis`, `deadpool-redis` (when using Redis)
- External crates: `memcache-async`, `tokio-util` (when using Memcached)

---

### API Features

#### api

Enables basic API-related functionality.

**Enabled crates**:
- `reinhardt-serializers` - Serializers
- `reinhardt-viewsets` - ViewSets

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "api"] }
```

#### Serialization Formats

| Feature | Format | Description |
|---------|--------|-------------|
| `serialize-json` | JSON | JSON format (enabled by default) |
| `serialize-xml` | XML | XML format |
| `serialize-yaml` | YAML | YAML format |

**Usage example**:
```toml
[dependencies]
# JSON + YAML support
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "serialize-yaml"] }
```

**Note**:
- `serialize-json` is enabled by default in `reinhardt-serializers`
- XML/YAML require explicit specification

---

### Middleware

#### middleware

Enables basic middleware functionality.

**Enabled crates**:
- `reinhardt-middleware` - Middleware foundation

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "middleware"] }
```

**Note**: `middleware` automatically enables `sessions`.

#### Individual Middleware Features

Enable specific middleware functionality only:

| Feature | Functionality | Description |
|---------|---------------|-------------|
| `middleware-cors` | CORS | Cross-Origin Resource Sharing |
| `middleware-compression` | Compression | Response compression (gzip, etc.) |
| `middleware-security` | Security | Security headers, etc. |
| `middleware-rate-limit` | Rate limiting | Request count limiting |

**Usage example**:
```toml
[dependencies]
# CORS + rate limiting only
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal", "middleware-cors", "middleware-rate-limit"] }
```

---

### Other Features

#### admin - Admin Panel

Django-style auto-generated admin panel.

**Enabled crates**:
- `reinhardt-forms` - Form processing
- `reinhardt-template` - Template engine

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "admin"] }
```

**Note**: The `reinhardt-admin` crate is currently under development and excluded from the `admin` feature.

---

#### graphql - GraphQL

GraphQL API support.

**Enabled crates**:
- `reinhardt-graphql` - GraphQL schema and resolvers

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "graphql"] }
```

**Features**:
- GraphQL schema generation
- Resolver definitions
- Subscription support

---

#### websockets - WebSocket

Real-time bidirectional communication.

**Enabled crates**:
- `reinhardt-websockets` - WebSocket server

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "websockets"] }
```

**Features**:
- WebSocket channels
- Room management
- Authentication integration
- Redis integration (pub/sub)

---

#### i18n - Internationalization

Multi-language support.

**Enabled crates**:
- `reinhardt-i18n` - Translation catalogs and locale management

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "i18n"] }
```

**Features**:
- Translation catalogs (gettext format)
- Locale switching
- Plural form support
- Timezone support

---

#### mail - Email Sending

Email sending functionality.

**Enabled crates**:
- `reinhardt-mail` - Email sending and templates

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "mail"] }
```

**Features**:
- SMTP sending
- Template emails
- Attachments
- HTML emails

---

#### sessions - Session Management

Session management functionality.

**Enabled crates**:
- `reinhardt-sessions` - Session storage

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "sessions"] }
```

**Features**:
- Multiple backends (database, file, Cookie, JWT)
- Secure session ID generation
- Session middleware integration

---

#### static-files - Static File Serving

Static file serving and management.

**Enabled crates**:
- `reinhardt-static` - Static file handler

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "static-files"] }
```

**Features**:
- CDN integration
- Hashed storage
- Compression support
- Cache control

---

#### storage - Storage System

File storage abstraction.

**Enabled crates**:
- `reinhardt-storage` - Storage backends

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "storage"] }
```

**Features**:
- Local file system
- S3-compatible storage
- Storage backend switching

---

#### tasks - Tasks/Background Jobs

Asynchronous task processing.

**Enabled crates**:
- `reinhardt-tasks` - Task queue and workers

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "tasks"] }
```

**Features**:
- Task queue
- Scheduled execution
- Retry functionality
- Background workers

---

#### shortcuts - Django-style Shortcuts

Django-style convenience functions.

**Enabled crates**:
- `reinhardt-shortcuts` - Shortcut functions

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "shortcuts"] }
```

**Features**:
- `get_object_or_404()` - Get object or 404 error
- `redirect()` - Redirect
- `render()` - Template rendering

---

#### contrib - Contrib Apps Aggregation

Enables all contrib apps at once.

**Enabled crates**:
- `reinhardt-contrib` - Contrib aggregation crate (auth, contenttypes, sessions, messages, static, mail, graphql, websockets, i18n)

**Usage example**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["standard", "contrib"] }
```

**Note**: Individual contrib features can be enabled (see `reinhardt-contrib` crate feature flags).

---

## Feature Flags for Major Crates

### reinhardt-micro

**Purpose**: Lightweight microservice configuration

**Default**: `["routing", "params", "di"]`

**Available features**:

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `routing` | Routing functionality | reinhardt-routers |
| `params` | Parameter extraction | reinhardt-params |
| `di` | Dependency injection | reinhardt-di |
| `database` | Database support | reinhardt-db |
| `compression` | Compression middleware | - |
| `cors` | CORS middleware | - |
| `rate-limit` | Rate limiting | - |
| `security` | Security middleware | - |

**Temporarily disabled features**:
- `schema` (OpenAPI schema generation) - Working on utoipa API compatibility

**Usage example**:
```toml
[dependencies]
reinhardt-micro = { version = "0.1.0-alpha.1", features = ["routing", "params", "di", "database"] }
```

---

### reinhardt-db

**Purpose**: Database layer integration crate

**Default**: `["backends", "pool", "postgres", "orm", "migrations", "hybrid", "associations"]`

**Available features**:

#### Module Features

| Feature | Description | Enabled Crates |
|---------|-------------|----------------|
| `backends` | Backend implementations | reinhardt-backends |
| `pool` | Connection pooling | reinhardt-backends-pool, reinhardt-pool, reinhardt-di |
| `orm` | ORM functionality | reinhardt-orm |
| `migrations` | Migrations | reinhardt-migrations |
| `hybrid` | Hybrid functionality | reinhardt-hybrid |
| `associations` | Association functionality | reinhardt-associations |

#### Database Features

| Feature | Database | Dependent Crates |
|---------|----------|------------------|
| `postgres` | PostgreSQL | sqlx/postgres, tokio-postgres |
| `sqlite` | SQLite | sqlx/sqlite, rusqlite |
| `mysql` | MySQL | sqlx/mysql, mysql_async |
| `mongodb-backend` | MongoDB | mongodb, tokio |
| `cockroachdb-backend` | CockroachDB | Same as postgres (protocol compatible) |
| `all-databases` | All databases | All of the above |

**Usage example**:
```toml
[dependencies]
# PostgreSQL only (default)
reinhardt-db = "0.1.0-alpha.1"

# SQLite and PostgreSQL
reinhardt-db = { version = "0.1.0-alpha.1", features = ["postgres", "sqlite"] }

# All database support
reinhardt-db = { version = "0.1.0-alpha.1", features = ["all-databases"] }
```

**Note**:
- The `pool` feature automatically enables `reinhardt-di` (for DI integration)
- PostgreSQL is enabled by default (most common)

---

### reinhardt-auth

**Purpose**: Authentication system

**Default**: None (all optional)

**Available features**:

#### Authentication Methods

| Feature | Description | Dependent Crates |
|---------|-------------|------------------|
| `jwt` | JWT authentication | jsonwebtoken |
| `session` | Session authentication | reinhardt-sessions |
| `oauth` | OAuth authentication | oauth2 |
| `token` | Token authentication | - |

#### Storage

| Feature | Description | Dependent Crates |
|---------|-------------|------------------|
| `database` | Database storage | sqlx, sea-query, sea-query-binder |
| `redis-sessions` | Redis sessions | redis, deadpool-redis |

**Usage example**:
```toml
[dependencies]
# JWT authentication only
reinhardt-auth = { version = "0.1.0-alpha.1", features = ["jwt"] }

# JWT + database storage
reinhardt-auth = { version = "0.1.0-alpha.1", features = ["jwt", "database"] }

# All authentication methods
reinhardt-auth = { version = "0.1.0-alpha.1", features = ["jwt", "session", "oauth", "token", "database"] }
```

---

### reinhardt-sessions

**Purpose**: Session management

**Default**: None (all optional)

**Available features**:

| Feature | Description | Dependent Crates |
|---------|-------------|------------------|
| `database` | Database backend | reinhardt-orm, reinhardt-db, sea-query, sea-query-binder |
| `file` | File backend | tokio, fs2 |
| `cookie` | Cookie-based sessions | base64, aes-gcm, rand, hmac, sha2 |
| `jwt` | JWT sessions | jsonwebtoken |
| `middleware` | HTTP middleware integration | reinhardt-http, reinhardt-types, reinhardt-exception, bytes |
| `messagepack` | MessagePack serialization | rmp-serde |

**Usage example**:
```toml
[dependencies]
# Database session + middleware
reinhardt-sessions = { version = "0.1.0-alpha.1", features = ["database", "middleware"] }

# Cookie-based sessions
reinhardt-sessions = { version = "0.1.0-alpha.1", features = ["cookie", "middleware"] }

# All backends
reinhardt-sessions = { version = "0.1.0-alpha.1", features = ["database", "file", "cookie", "jwt", "middleware"] }
```

**Backend selection**:
- `database`: Large apps, multi-server support
- `file`: Development environment, small apps
- `cookie`: Stateless, no server-side storage needed
- `jwt`: API-focused, token-based

---

### reinhardt-cache

**Purpose**: Cache system

**Default**: None (all optional)

**Available features**:

| Feature | Description | Dependent Crates |
|---------|-------------|------------------|
| `redis-backend` | Redis backend | redis, deadpool-redis |
| `redis-cluster` | Redis cluster | Same as above |
| `redis-sentinel` | Redis sentinel | Same as above |
| `memcached-backend` | Memcached backend | memcache-async, tokio-util |
| `all-backends` | All backends | All of the above |

**Usage example**:
```toml
[dependencies]
# Redis standalone
reinhardt-cache = { version = "0.1.0-alpha.1", features = ["redis-backend"] }

# Redis cluster support
reinhardt-cache = { version = "0.1.0-alpha.1", features = ["redis-backend", "redis-cluster"] }

# Redis and Memcached both
reinhardt-cache = { version = "0.1.0-alpha.1", features = ["redis-backend", "memcached-backend"] }
```

---

### reinhardt-middleware

**Purpose**: HTTP middleware

**Default**: None (all optional)

**Available features**:

| Feature | Description | Functionality |
|---------|-------------|---------------|
| `cors` | CORS middleware | Cross-origin request control |
| `compression` | Compression middleware | gzip/brotli compression |
| `security` | Security middleware | Security header configuration |
| `rate-limit` | Rate limiting | Request count limiting |
| `session` | Session middleware | Session management integration |
| `sqlx` | SQLx database support | Database connection management |

**Usage example**:
```toml
[dependencies]
# CORS + compression
reinhardt-middleware = { version = "0.1.0-alpha.1", features = ["cors", "compression"] }

# All middleware
reinhardt-middleware = { version = "0.1.0-alpha.1", features = ["cors", "compression", "security", "rate-limit", "session"] }
```

---

### reinhardt-serializers

**Purpose**: Data serialization

**Default**: `["json"]`

**Available features**:

| Feature | Format | Dependent Crates |
|---------|--------|------------------|
| `json` | JSON | serde_json |
| `xml` | XML | quick-xml, serde-xml-rs |
| `yaml` | YAML | serde_yaml |

**Usage example**:
```toml
[dependencies]
# JSON only (default)
reinhardt-serializers = "0.1.0-alpha.1"

# JSON + YAML
reinhardt-serializers = { version = "0.1.0-alpha.1", features = ["json", "yaml"] }

# All formats
reinhardt-serializers = { version = "0.1.0-alpha.1", features = ["json", "xml", "yaml"] }
```

---

### reinhardt-rest

**Purpose**: REST API core functionality

**Default**: `["serializers", "parsers", "renderers"]`

**Available features**:

| Feature | Description | Dependent Crates |
|---------|-------------|------------------|
| `serializers` | Serializers | reinhardt-orm |
| `parsers` | Parsers | reinhardt-parsers |
| `renderers` | Renderers | reinhardt-renderers |
| `jwt` | JWT support | rest-core/jwt |

**Usage example**:
```toml
[dependencies]
# Default configuration
reinhardt-rest = "0.1.0-alpha.1"

# With JWT
reinhardt-rest = { version = "0.1.0-alpha.1", features = ["serializers", "parsers", "renderers", "jwt"] }
```

**Note**: The `serializers` feature includes `reinhardt-orm` as a dependency.

---

### reinhardt-contrib

**Purpose**: Contrib apps aggregation

**Default**: None (all optional)

**Available features**:

| Feature | Description | Enabled Crates |
|---------|-------------|----------------|
| `auth` | Authentication | reinhardt-auth |
| `contenttypes` | Content types | reinhardt-contenttypes |
| `sessions` | Sessions | reinhardt-sessions |
| `messages` | Messages | reinhardt-messages |
| `static` | Static files | reinhardt-static |
| `mail` | Email | reinhardt-mail |
| `graphql` | GraphQL | reinhardt-graphql |
| `websockets` | WebSocket | reinhardt-websockets |
| `i18n` | Internationalization | reinhardt-i18n |
| `full` | All | All of the above |

**Usage example**:
```toml
[dependencies]
# Individual features
reinhardt-contrib = { version = "0.1.0-alpha.1", features = ["auth", "sessions"] }

# All features
reinhardt-contrib = { version = "0.1.0-alpha.1", features = ["full"] }
```

---

### reinhardt-di

**Purpose**: Dependency injection system

**Default**: None (all optional)

**Available features**:

| Feature | Description | Dependent Crates |
|---------|-------------|------------------|
| `params` | Parameter extraction | reinhardt-params |
| `dev-tools` | Development tools | indexmap |
| `generator` | Generator functionality | genawaiter |

**Usage example**:
```toml
[dependencies]
# With parameter extraction
reinhardt-di = { version = "0.1.0-alpha.1", features = ["params"] }

# All features
reinhardt-di = { version = "0.1.0-alpha.1", features = ["params", "dev-tools", "generator"] }
```

---

### reinhardt-test

**Purpose**: Test utilities

**Default**: None (all optional)

**Available features**:

| Feature | Description | Dependent Crates |
|---------|-------------|------------------|
| `testcontainers` | TestContainers integration | testcontainers, testcontainers-modules, sqlx, memcache-async, tokio-util |
| `static` | Static file testing | reinhardt-static |

**Usage example**:
```toml
[dev-dependencies]
# TestContainers integration (for database/cache testing)
reinhardt-test = { version = "0.1.0-alpha.1", features = ["testcontainers"] }

# All test utilities
reinhardt-test = { version = "0.1.0-alpha.1", features = ["testcontainers", "static"] }
```

**TestContainers use cases**:
- Testing with actual PostgreSQL/MySQL/SQLite containers
- Cache testing with Redis containers
- Cache testing with Memcached containers

---

## Feature Flag Dependency Map

### Bundle Feature Dependencies

```
default
└── standard
    ├── minimal
    │   ├── reinhardt-params
    │   └── reinhardt-di
    ├── reinhardt-orm
    ├── reinhardt-serializers
    ├── reinhardt-viewsets
    ├── reinhardt-auth
    ├── reinhardt-middleware
    ├── reinhardt-pagination
    ├── reinhardt-filters
    ├── reinhardt-throttling
    ├── reinhardt-signals
    ├── reinhardt-parsers
    ├── reinhardt-renderers
    ├── reinhardt-versioning
    ├── reinhardt-metadata
    ├── reinhardt-negotiation
    └── reinhardt-rest

full
├── standard (all of the above)
├── database
│   ├── reinhardt-orm
│   ├── reinhardt-migrations
│   ├── reinhardt-contenttypes
│   └── reinhardt-db
│       ├── backends
│       ├── pool (→ reinhardt-di)
│       ├── postgres
│       ├── orm
│       ├── migrations
│       ├── hybrid
│       └── associations
├── auth → reinhardt-auth
├── admin
│   ├── reinhardt-forms
│   └── reinhardt-template
├── graphql → reinhardt-graphql
├── websockets → reinhardt-websockets
├── cache → reinhardt-cache
├── i18n → reinhardt-i18n
├── mail → reinhardt-mail
├── sessions → reinhardt-sessions
├── static-files → reinhardt-static
├── storage → reinhardt-storage
└── contrib → reinhardt-contrib
```

### Database Feature Dependencies

```
database
├── reinhardt-orm
├── reinhardt-migrations
├── reinhardt-contenttypes
└── reinhardt-db (default features enabled)
    ├── backends
    ├── pool
    │   ├── reinhardt-backends-pool
    │   ├── reinhardt-pool
    │   └── reinhardt-di (auto-enabled)
    ├── postgres (default)
    ├── orm
    ├── migrations
    ├── hybrid
    └── associations

db-postgres
├── database
└── reinhardt-db/postgres

db-mysql
├── database
└── reinhardt-db/mysql

db-sqlite
├── database
└── reinhardt-db/sqlite

db-mongodb
├── database
└── reinhardt-db/mongodb-backend

db-cockroachdb
├── database
└── reinhardt-db/cockroachdb-backend
```

### Authentication Feature Dependencies

```
auth
└── reinhardt-auth

auth-jwt
├── auth
└── reinhardt-auth/jwt
    └── jsonwebtoken

auth-session
├── auth
├── reinhardt-auth/session
└── sessions (auto-enabled)
    └── reinhardt-sessions

auth-oauth
├── auth
└── reinhardt-auth/oauth
    └── oauth2

auth-token
├── auth
└── reinhardt-auth/token
```

### Cache Feature Dependencies

```
cache
└── reinhardt-cache

redis-backend
├── cache
└── reinhardt-cache/redis-backend
    ├── redis
    └── deadpool-redis

redis-cluster
├── redis-backend
└── reinhardt-cache/redis-cluster

redis-sentinel
├── redis-backend
└── reinhardt-cache/redis-sentinel

memcached-backend
├── cache
└── reinhardt-cache/memcached-backend
    ├── memcache-async
    └── tokio-util
```

### Middleware Feature Dependencies

```
middleware
├── reinhardt-middleware
└── sessions (auto-enabled)

middleware-cors
└── reinhardt-middleware/cors

middleware-compression
└── reinhardt-middleware/compression

middleware-security
└── reinhardt-middleware/security

middleware-rate-limit
└── reinhardt-middleware/rate-limit

middleware + session
└── reinhardt-middleware/session
    └── reinhardt-sessions
```

### Important Interdependency Notes

1. **pool → reinhardt-di**: Auto-enabled for connection pool DI integration
2. **middleware → sessions**: Auto-enabled as middleware uses session functionality
3. **auth-session → sessions**: Auto-enabled as session authentication uses session management
4. **serializers → reinhardt-orm**: Dependency as serializers handle ORM models

---

## Usage Examples and Best Practices

### Scenario 1: Simple Microservice API

**Requirements**:
- No database needed
- Lightweight and fast startup
- Basic routing and parameter extraction

**Recommended configuration**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal"] }
```

**Or**:
```toml
[dependencies]
reinhardt-micro = "0.1.0-alpha.1"  # Default: routing + params + di
```

**Binary size**: ~5-10 MB
**Compile time**: 1-2 minutes

---

### Scenario 2: REST API with PostgreSQL

**Requirements**:
- PostgreSQL database
- JSON API
- JWT authentication
- Pagination and filtering

**Recommended configuration**:
```toml
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  default-features = false,
  features = [
    "api-only",      # REST API basics
    "db-postgres",   # PostgreSQL support
    "auth-jwt",      # JWT authentication
  ]
}
```

**Binary size**: ~20-25 MB
**Compile time**: 3-5 minutes

---

### Scenario 3: GraphQL + WebSocket Server

**Requirements**:
- GraphQL API
- Real-time communication with WebSocket
- Redis cache
- PostgreSQL database

**Recommended configuration**:
```toml
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  default-features = false,
  features = [
    "minimal",
    "graphql",
    "websockets",
    "db-postgres",
    "cache",
    "redis-backend",
    "auth-jwt",
  ]
}
```

**Binary size**: ~30-35 MB
**Compile time**: 5-7 minutes

---

### Scenario 4: Full-Featured Web Application

**Requirements**:
- REST API + GraphQL
- WebSocket
- Admin panel
- Multi-language support
- Email sending
- Static file serving

**Recommended configuration**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }
```

**Binary size**: ~50+ MB
**Compile time**: 10-15 minutes

---

### Scenario 5: CLI Tool/Background Jobs

**Requirements**:
- Database migrations
- Email sending batch
- Task scheduling

**Recommended configuration**:
```toml
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  default-features = false,
  features = [
    "cli-tools",  # database, migrations, tasks, mail
  ]
}
```

**Binary size**: ~15-20 MB
**Compile time**: 3-4 minutes

---

### Best Practices

#### 1. Control default-features

**Start from minimal configuration**:
```toml
# ❌ Bad: Includes unnecessary features
[dependencies]
reinhardt = "0.1.0-alpha.1"  # Enables all of standard

# ✅ Good: Select only needed features
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  default-features = false,
  features = ["minimal", "database", "db-postgres"]
}
```

#### 2. Explicitly Specify Database Backend

**Explicitly declare which database to use**:
```toml
# ❌ Bad: Default PostgreSQL gets enabled
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["database"] }

# ✅ Good: Explicitly specify the database
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  features = ["database", "db-sqlite"]  # Explicitly SQLite
}
```

#### 3. Separate Development and Production Environments

**Switch features per environment**:
```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", default-features = false }

[features]
# Development: Include test utilities
dev = ["reinhardt/standard", "reinhardt/test-utils"]

# Production: Minimal configuration
prod = ["reinhardt/minimal", "reinhardt/database", "reinhardt/db-postgres"]
```

Build:
```bash
# Development
cargo build --features dev

# Production
cargo build --release --features prod
```

#### 4. Appropriate Cache Backend Selection

**Select backend based on use case**:
```toml
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  features = [
    "cache",
    # Development: Memcached (easy setup)
    "memcached-backend",

    # Production: Redis Cluster (high availability)
    # "redis-backend",
    # "redis-cluster",
  ]
}
```

#### 5. Test Configuration

**Add test utilities in dev-dependencies**:
```toml
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  default-features = false,
  features = ["minimal", "database"]
}

[dev-dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  features = ["test-utils"]
}
reinhardt-test = { version = "0.1.0-alpha.1", features = ["testcontainers"] }
```

---

## Build Time and Binary Size Comparison

### Comparison by Configuration

| Configuration | Features | Compile Time | Binary Size | Recommended Use |
|--------------|----------|--------------|-------------|-----------------|
| **Minimal** | `minimal` | 1-2 min | ~5-10 MB | Microservices, simple APIs |
| **Minimal + DB** | `minimal`, `database`, `db-postgres` | 2-3 min | ~15-20 MB | Small APIs with database |
| **API Only** | `api-only`, `db-postgres` | 3-4 min | ~20-25 MB | REST API only |
| **Standard** | `standard` (default) | 5-7 min | ~25-30 MB | General web apps |
| **Standard + Extra** | `standard`, `graphql`, `cache` | 7-9 min | ~35-40 MB | REST + GraphQL + cache |
| **Full** | `full` | 10-15 min | ~50+ MB | Full-featured web apps |

### Database Backend Impact

| Database | Additional Compile Time | Additional Binary Size |
|----------|------------------------|------------------------|
| PostgreSQL | +30 sec | +2-3 MB |
| MySQL | +30 sec | +2-3 MB |
| SQLite | +10 sec | +1 MB |
| MongoDB | +1 min | +4-5 MB |
| All databases | +2 min | +8-10 MB |

### Cache Backend Impact

| Cache | Additional Compile Time | Additional Binary Size |
|-------|------------------------|------------------------|
| Redis | +20 sec | +1-2 MB |
| Memcached | +15 sec | +1 MB |
| Redis Cluster | +30 sec | +2 MB |

### Measurement Environment

- **CPU**: Apple M1/M2 or Intel Core i5 or above
- **Memory**: 16GB or more
- **Rust version**: 1.70 or above
- **Build mode**: `--release`

**Note**: Actual compile times and binary sizes vary depending on hardware, Rust version, and dependency cache status.

---

## Troubleshooting

### Issue 1: Compile Error "feature not found"

**Example error message**:
```
error: feature `foo` is not available in package `reinhardt`
```

**Cause**: Specified non-existent feature name

**Solution**:
1. Check correct feature name in [Quick Reference](#quick-reference)
2. Check for typos (e.g., `databse` → `database`)
3. Verify version differences (may be unimplemented in older versions)

---

### Issue 2: Dependency Conflicts

**Example error message**:
```
error: multiple versions of `sqlx` found
```

**Cause**: Multiple features requesting different versions of the same crate

**Solution**:
```toml
[patch.crates-io]
sqlx = { git = "https://github.com/launchbadge/sqlx", branch = "main" }
```

Or delete Cargo.lock and rebuild:
```bash
rm Cargo.lock
cargo build
```

---

### Issue 3: Linker Errors

**Example error message**:
```
error: linking with `cc` failed
```

**Cause**: Database driver shared library not found

**Solution**:

**PostgreSQL**:
```bash
# macOS
brew install postgresql

# Ubuntu/Debian
sudo apt-get install libpq-dev

# Fedora/RHEL
sudo dnf install postgresql-devel
```

**MySQL**:
```bash
# macOS
brew install mysql

# Ubuntu/Debian
sudo apt-get install libmysqlclient-dev

# Fedora/RHEL
sudo dnf install mysql-devel
```

**SQLite**:
```bash
# macOS
brew install sqlite

# Ubuntu/Debian
sudo apt-get install libsqlite3-dev

# Fedora/RHEL
sudo dnf install sqlite-devel
```

---

### Issue 4: Binary Size Too Large

**Symptom**: Binary over 50MB even in release build

**Cause**: Unnecessary features are enabled

**Solution**:

1. **Disable unused features**:
```toml
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  default-features = false,  # This is important
  features = ["minimal", "database", "db-postgres"]
}
```

2. **Enable LTO (Link Time Optimization) in Cargo.toml**:
```toml
[profile.release]
lto = true
codegen-units = 1
opt-level = "z"  # Size optimization
strip = true     # Remove debug symbols
```

3. **Verify actually used features**:
```bash
cargo tree --features standard | grep reinhardt
```

---

### Issue 5: Compile Time Too Long

**Symptom**: Build takes over 10 minutes

**Cause**: Unnecessary features enabled, or cache not working

**Solution**:

1. **Enable parallel build**:
```bash
# ~/.cargo/config.toml
[build]
jobs = 8  # Adjust based on CPU cores
```

2. **Use sccache (build cache)**:
```bash
# Install
cargo install sccache

# Set environment variable
export RUSTC_WRAPPER=sccache
```

3. **Disable unnecessary features**:
```toml
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  default-features = false,
  features = ["minimal"]  # Bare minimum
}
```

---

### Issue 6: Runtime Error "feature not enabled"

**Example error message**:
```
thread 'main' panicked at 'Redis backend not enabled'
```

**Cause**: Feature for functionality used in code is not enabled

**Solution**:

1. **Identify required feature from error message**:
   - `Redis backend not enabled` → Need `redis-backend` feature
   - `JWT support not enabled` → Need `auth-jwt` feature

2. **Add corresponding feature to Cargo.toml**:
```toml
[dependencies]
reinhardt = {
  version = "0.1.0-alpha.1",
  features = ["cache", "redis-backend"]  # Add this
}
```

---

### Issue 7: TestContainers Not Working

**Symptom**: Docker containers don't start during test execution

**Cause**: `testcontainers` feature not enabled, or Docker not running

**Solution**:

1. **Verify Docker is running**:
```bash
docker ps
```

2. **Enable `testcontainers` feature in dev-dependencies**:
```toml
[dev-dependencies]
reinhardt-test = { version = "0.1.0-alpha.1", features = ["testcontainers"] }
```

---

### Debugging Tips

#### 1. Check Enabled Features

```bash
# Display dependency tree (with features)
cargo tree -e features

# Display only reinhardt crate features
cargo tree -e features | grep reinhardt
```

#### 2. Check Conditional Compilation

```rust
// Check feature enabled state in code
#[cfg(feature = "redis-backend")]
println!("Redis backend is enabled");

#[cfg(not(feature = "redis-backend"))]
println!("Redis backend is NOT enabled");
```

#### 3. Detailed Build Logs

```bash
# Display detailed build logs
cargo build -vv

# Display only specific crate build logs
cargo build -vv 2>&1 | grep reinhardt
```

---

## Quick Reference

### Complete Feature Flag List (Alphabetical)

| Feature | Category | Description | Default |
|---------|----------|-------------|---------|
| `admin` | Feature | Admin panel (forms, template) | ❌ |
| `api` | Feature | Basic API functionality (serializers, viewsets) | ❌ |
| `api-only` | Bundle | REST API-only configuration | ❌ |
| `auth` | Feature | Authentication foundation | ❌ |
| `auth-jwt` | Auth | JWT authentication | ❌ |
| `auth-oauth` | Auth | OAuth authentication | ❌ |
| `auth-session` | Auth | Session authentication | ❌ |
| `auth-token` | Auth | Token authentication | ❌ |
| `cache` | Feature | Cache system | ❌ |
| `cli-tools` | Bundle | CLI/background job configuration | ❌ |
| `conf` | Crate | Configuration management | ❌ |
| `contrib` | Feature | Contrib apps aggregation | ❌ |
| `core` | Crate | Core functionality | ❌ |
| `database` | Feature | General database | ❌ |
| `db-cockroachdb` | Database | CockroachDB support | ❌ |
| `db-mongodb` | Database | MongoDB support | ❌ |
| `db-mysql` | Database | MySQL support | ❌ |
| `db-postgres` | Database | PostgreSQL support | ❌ |
| `db-sqlite` | Database | SQLite support | ❌ |
| `default` | - | Default configuration (standard) | ✅ |
| `di` | Crate | Dependency injection | ❌ |
| `di-generator` | DI | DI generator | ❌ |
| `forms` | Feature | Form processing | ❌ |
| `full` | Bundle | All features enabled | ❌ |
| `graphql` | Feature | GraphQL support | ❌ |
| `graphql-server` | Bundle | GraphQL server configuration | ❌ |
| `i18n` | Feature | Internationalization | ❌ |
| `mail` | Feature | Email sending | ❌ |
| `memcached-backend` | Cache | Memcached backend | ❌ |
| `middleware` | Feature | Middleware foundation | ❌ |
| `middleware-compression` | Middleware | Compression middleware | ❌ |
| `middleware-cors` | Middleware | CORS middleware | ❌ |
| `middleware-rate-limit` | Middleware | Rate limiting middleware | ❌ |
| `middleware-security` | Middleware | Security middleware | ❌ |
| `minimal` | Bundle | Minimal configuration | ❌ |
| `redis-backend` | Cache | Redis backend | ❌ |
| `redis-cluster` | Cache | Redis cluster | ❌ |
| `redis-sentinel` | Cache | Redis sentinel | ❌ |
| `rest` | Crate | REST API core | ❌ |
| `serialize-json` | Serialization | JSON format | ✅ (serializers) |
| `serialize-xml` | Serialization | XML format | ❌ |
| `serialize-yaml` | Serialization | YAML format | ❌ |
| `server` | Feature | Server components | ❌ |
| `sessions` | Feature | Session management | ❌ |
| `shortcuts` | Feature | Django-style shortcuts | ❌ |
| `standard` | Bundle | Standard configuration (default) | ✅ |
| `static-files` | Feature | Static file serving | ❌ |
| `storage` | Feature | Storage system | ❌ |
| `tasks` | Feature | Tasks/background jobs | ❌ |
| `templates` | Feature | Template engine | ❌ |
| `test` | Crate | Test utilities | ❌ |
| `test-utils` | Bundle | Test environment configuration | ❌ |
| `websocket-server` | Bundle | WebSocket server configuration | ❌ |
| `websockets` | Feature | WebSocket support | ❌ |

### Category Index

#### Bundle Features
- `minimal`, `standard`, `full`
- `api-only`, `graphql-server`, `websocket-server`, `cli-tools`, `test-utils`

#### Database
- `database`, `db-postgres`, `db-mysql`, `db-sqlite`, `db-mongodb`, `db-cockroachdb`

#### Authentication
- `auth`, `auth-jwt`, `auth-session`, `auth-oauth`, `auth-token`

#### Cache
- `cache`, `redis-backend`, `redis-cluster`, `redis-sentinel`, `memcached-backend`

#### Middleware
- `middleware`, `middleware-cors`, `middleware-compression`, `middleware-security`, `middleware-rate-limit`

#### API
- `api`, `rest`, `graphql`, `serialize-json`, `serialize-xml`, `serialize-yaml`

#### Other
- `admin`, `forms`, `templates`, `websockets`, `i18n`, `mail`, `sessions`, `static-files`, `storage`, `tasks`, `shortcuts`, `server`

### Configuration Templates

#### Microservice
```toml
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal"] }
```

#### REST API
```toml
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["api-only", "db-postgres"] }
```

#### GraphQL Server
```toml
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["graphql-server"] }
```

#### Full Features
```toml
reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }
```

---

## Summary

Reinhardt's feature flag system provides **over 70 features** with **3 levels of granularity** (bundle, feature group, individual features).

### Key Characteristics

1. **Flexible Configuration**: Optimal configuration for any use case, from microservices to full-featured applications
2. **Automatic Dependency Resolution**: Enabling higher-level features automatically enables required lower-level features
3. **Performance**: Reduced build time and binary size by excluding unnecessary functionality
4. **Default Configuration**: `standard` is the default, a balanced configuration suitable for most projects

### Selection Guide

| Use Case | Recommended Configuration | Binary Size |
|----------|--------------------------|-------------|
| Simple API | `minimal` | ~5-10 MB |
| REST API | `api-only` + database | ~20-25 MB |
| General web app | `standard` | ~25-30 MB |
| Full-featured app | `full` | ~50+ MB |

For detailed information, refer to each section.

---

**Related Documentation**:
- [README.md](../README.md) - Project overview
- [GETTING_STARTED.md](GETTING_STARTED.md) - Getting started guide
- [CLAUDE.md](../CLAUDE.md) - Developer guidelines
