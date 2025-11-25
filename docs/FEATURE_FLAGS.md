# Feature Flags Guide

## Table of Contents

- [Overview](#overview)
- [Basic Usage](#basic-usage)
- [Bundle Features](#bundle-features)
- [Feature Categories](#feature-categories)
- [Major Crate Features](#major-crate-features)
- [Usage Scenarios](#usage-scenarios)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)
- [Migration Guide](#migration-guide) ⚠️ **Breaking Changes**
- [Summary](#summary)
- [Quick Reference](#quick-reference)

---

## Overview

Reinhardt employs a **highly granular feature flag system** with **70+ features** across **3 levels of granularity**:

1. **Bundle Features**: `minimal`, `standard`, `full`
2. **Feature Groups**: `database`, `auth`, `cache`, `middleware`
3. **Individual Features**: `jwt`, `redis-backend`, `cors`

### Benefits

- **Reduced Compile Time**: Exclude unnecessary features
- **Smaller Binary Size**: Only include used code
- **Minimized Dependencies**: Only required crates included
- **Flexible Configuration**: From microservices to full-stack apps

---

## Basic Usage

### Default (full) ⚠️ Changed in v0.1.0-alpha.2

```toml
[dependencies]
reinhardt = "0.1.0-alpha.1"  # Enables full bundle (all features)
```

**Note**: The default has changed from `standard` to `full`. See [Migration Guide](#migration-guide) for details.

### Standard Configuration

For a balanced setup without all features:

```toml
[dependencies]
reinhardt = {
	version = "0.1.0-alpha.1",
	default-features = false,
	features = ["standard"]
}
```

### Custom Configuration

```toml
[dependencies]
reinhardt = {
	version = "0.1.0-alpha.1",
	default-features = false,
	features = ["minimal", "database", "db-postgres", "auth-jwt"]
}
```

---

## Bundle Features

### minimal ⚠️ Changed in v0.1.0-alpha.2

**Lightweight bundle** with essential features for microservices and simple APIs.

**Includes**:
- Core (types, macros, HTTP)
- Dependency Injection
- HTTP Server
- URL routing (always included)

```toml
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["minimal"] }
```

**Binary**: ~5-10 MB | **Compile**: Very fast

**Equivalent to**: `reinhardt-micro` functionality (routing, DI, params, server)

---

### standard

Balanced configuration for most projects. ⚠️ PostgreSQL is now included by default.

**Includes**:
- `minimal` features
- Database (ORM, migrations, PostgreSQL backend)
- REST API (Serializers, ViewSets, Parsers, Renderers)
- Auth, Middleware, Sessions
- Pagination, Filtering, Throttling, Versioning
- Templates, Signals

```toml
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["standard"] }
```

**Binary**: ~20-30 MB | **Compile**: Medium

**Note**: `db-postgres` is now explicitly included. For other databases, use `db-mysql`, `db-sqlite`, or `db-mongodb`.

---

### full (default) ⚠️ Now the default

All features enabled (batteries-included).

**Includes**: `standard` + admin, graphql, websockets, cache, i18n, mail, sessions, static-files, storage

```toml
reinhardt = "0.1.0-alpha.1"  # default enables full
# Or explicitly:
reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }
```

**Binary**: ~50+ MB | **Compile**: Slow

---

### Preset Bundles

| Bundle | Purpose | Key Features |
|--------|---------|--------------|
| `api-only` | REST API only | Serializers, ViewSets, Auth, Pagination |
| `graphql-server` | GraphQL API | GraphQL, Auth, Database |
| `websocket-server` | Real-time | WebSockets, Auth, Cache |
| `cli-tools` | CLI/Background jobs | Database, Migrations, Tasks, Mail |
| `test-utils` | Testing | Test utilities, Database |

---

## Feature Categories

### Database

#### database

Enables general database functionality.

```toml
features = ["database"]  # Includes: ORM, migrations, contenttypes
```

#### Database-Specific

| Feature | Database | Notes |
|---------|----------|-------|
| `db-postgres` | PostgreSQL | Default |
| `db-mysql` | MySQL | - |
| `db-sqlite` | SQLite | Lightweight |
| `db-mongodb` | MongoDB | Empty (planned) |
| `db-cockroachdb` | CockroachDB | Uses Postgres protocol |

---

### Authentication

| Feature | Method | Auto-enables |
|---------|--------|--------------|
| `auth` | Foundation | - |
| `auth-jwt` | JWT | `auth` |
| `auth-session` | Session | `auth`, `sessions` |
| `auth-oauth` | OAuth | `auth` |
| `auth-token` | Token | `auth` |

---

### Cache

| Feature | Backend | Exposure |
|---------|---------|----------|
| `redis-backend` | Redis | Root-level |
| `redis-cluster` | Redis Cluster | Subcrate only* |
| `redis-sentinel` | Redis Sentinel | Subcrate only* |
| `memcached-backend` | Memcached | Subcrate only* |

**Workaround for subcrate-only features**:
```toml
reinhardt = { version = "0.1.0-alpha.1", features = ["cache"] }
reinhardt-cache = { version = "0.1.0-alpha.1", features = ["redis-cluster"] }
```

---

### API

| Feature | Format | Default |
|---------|--------|---------|
| `api` | Basic API | - |
| `serialize-json` | JSON | ✅ (via serializers) |
| `serialize-xml` | XML | - |
| `serialize-yaml` | YAML | - |

---

### Middleware

| Feature | Functionality |
|---------|---------------|
| `middleware` | Foundation (auto-enables `sessions`) |
| `middleware-cors` | CORS |
| `middleware-compression` | gzip/brotli |
| `middleware-security` | Security headers |
| `middleware-rate-limit` | Rate limiting |

---

### Other Features

| Feature | Description | Key Crates |
|---------|-------------|------------|
| `admin` | Admin panel | reinhardt-admin, reinhardt-forms, reinhardt-template |
| `graphql` | GraphQL API | reinhardt-graphql |
| `websockets` | Real-time | reinhardt-websockets |
| `i18n` | Internationalization | reinhardt-i18n |
| `mail` | Email sending | reinhardt-mail |
| `sessions` | Session mgmt | reinhardt-auth (includes sessions subcrate) |
| `static-files` | Static serving | reinhardt-utils/static |
| `storage` | Storage abstraction | reinhardt-utils/storage |
| `tasks` | Background jobs | reinhardt-tasks |
| `shortcuts` | Django-style helpers | reinhardt-shortcuts |

---

## Major Crate Features

| Crate | Default Features | Key Features |
|-------|------------------|--------------|
| `reinhardt-micro` | `routing`, `params`, `di` | `database`, middleware options |
| `reinhardt-db` | `backends`, `pool`, `postgres`, `orm`, `migrations` | `sqlite`, `mysql`, `contenttypes` |
| `reinhardt-auth` | None | `jwt`, `session`, `oauth`, `token`, `argon2-hasher` |
| `reinhardt-rest` | `serializers`, `parsers`, `renderers` | `pagination`, `filters`, `throttling`, `versioning` |
| `reinhardt-cache` | None | `redis-backend`, `redis-cluster`, `memcached-backend` |
| `reinhardt-middleware` | None | `cors`, `compression`, `security`, `rate-limit` |
| `reinhardt-sessions` | None | `database`, `file`, `cookie`, `jwt` |
| `reinhardt-test` | None | `testcontainers`, `static`, `websockets` |

**Note**: `pool` auto-enables `reinhardt-di`

---

## Usage Scenarios

| Use Case | Configuration | Binary |
|----------|---------------|--------|
| Microservice | `reinhardt-micro = "0.1.0-alpha.1"` | ~5-10 MB |
| REST API | `features = ["api-only", "db-postgres", "auth-jwt"]` | ~20-25 MB |
| GraphQL/WebSocket | `features = ["graphql", "websockets", "db-postgres"]` | ~30-35 MB |
| Full-Featured | `features = ["full"]` | ~50+ MB |
| CLI/Background | `features = ["cli-tools"]` | ~15-20 MB |

## Best Practices

**Disable default-features**: Use `default-features = false` for explicit control

**Explicit Database**: Specify database backend (e.g., `db-postgres`, `db-sqlite`)

**Environment-Specific**: Use feature profiles (`dev`, `prod`)

**Test Configuration**: Add `test-utils` in `[dev-dependencies]` only

---

## Troubleshooting

### Common Issues

**"feature not found"**: Check [Quick Reference](#quick-reference) for correct feature names

**Linker Errors**: Install database client libraries (e.g., `libpq-dev` for PostgreSQL)

**Runtime "feature not enabled"**: Add required feature to `Cargo.toml`

**Debugging**: Use `cargo tree -e features | grep reinhardt` to check enabled features

---

## Migration Guide

### Breaking Changes in v0.1.0-alpha.2

#### 1. Default Feature Changed: `standard` → `full`

**Before (v0.1.0-alpha.1):**
```toml
reinhardt = "0.1.0-alpha.1"  # Enabled: standard bundle
```

**Now (v0.1.0-alpha.2):**
```toml
reinhardt = "0.1.0-alpha.1"  # Enables: full bundle (all features)
```

**Impact:**
- ⚠️ **Longer compile time**: Full bundle includes all features (admin, graphql, websockets, etc.)
- ⚠️ **Larger binary size**: ~50+ MB (was ~20-30 MB with standard)
- ✅ **More features available**: All Reinhardt features are immediately usable

**To keep previous behavior:**
```toml
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["standard"] }
```

#### 2. `minimal` Feature Now Includes Core Functionality

**Before (v0.1.0-alpha.1):**
```toml
# minimal was empty - no features enabled
features = ["minimal"]
```

**Now (v0.1.0-alpha.2):**
```toml
# minimal includes: core, di, server (equivalent to reinhardt-micro)
features = ["minimal"]
```

**Impact:**
- ✅ **More useful**: `minimal` now provides a working microservice framework
- ✅ **Backward compatible**: Adding features is non-breaking

**Equivalent to:**
- `reinhardt-micro` functionality
- Routing, DI, params, server, core

#### 3. `standard` Now Includes PostgreSQL by Default

**Before (v0.1.0-alpha.1):**
```toml
features = ["standard"]  # Database support, but no specific backend
```

**Now (v0.1.0-alpha.2):**
```toml
features = ["standard"]  # Includes db-postgres explicitly
```

**Impact:**
- ✅ **Works out of the box**: Database features now work without additional configuration
- ⚠️ **PostgreSQL dependency**: `libpq-dev` (or equivalent) required at build time

**For other databases:**
```toml
# MySQL
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["standard", "db-mysql"] }

# SQLite
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["standard", "db-sqlite"] }

# MongoDB
reinhardt = { version = "0.1.0-alpha.1", default-features = false, features = ["standard", "db-mongodb"] }
```

#### 4. Removed Features

The following features have been removed (they had no effect):

- `serialize-json`
- `serialize-xml`
- `serialize-yaml`

**Reason**: All three features enabled the same dependencies (`reinhardt-rest/serializers`), providing no actual format-specific control.

**Impact:**
- ⚠️ **Build may fail** if you explicitly used these features
- ✅ **No functional impact**: Serialization still works via `reinhardt-rest/serializers`

**Migration:**
```toml
# Before
features = ["serialize-json", "serialize-xml"]

# After
features = ["reinhardt-rest", "reinhardt-rest/serializers"]
# Or simply use "rest" or "standard" bundle
```

### Migration Checklist

- [ ] Update `Cargo.toml` if you want to keep `standard` instead of `full`
- [ ] Install PostgreSQL development libraries if using `standard` or `full`
- [ ] Remove `serialize-*` features if explicitly specified
- [ ] Test build with new configuration
- [ ] Update documentation references

---

## Summary

Reinhardt provides **70+ features** with **3 granularity levels** (bundle, group, individual).

**Default**: `full` bundle (all features) ⚠️ Changed from `standard`

**Key bundles**: `minimal` (microservice), `standard` (balanced), `full` (all features, default), `api-only`, `graphql-server`, `cli-tools`

**Auto-enabled dependencies**: `pool` → `reinhardt-di`, `middleware` → `sessions`, `auth-session` → `sessions`

**Best Practice**: Use `default-features = false` for explicit control

---

## Related Documentation

- **Bundle Features**: See Bundle Features section above
- **Feature Categories**: See Feature Categories section above
- **Usage Scenarios**: See Usage Scenarios section above
- [README.md](../README.md) - Project overview
- [GETTING_STARTED.md](GETTING_STARTED.md) - Getting started guide
- [CLAUDE.md](../CLAUDE.md) - Developer guidelines
