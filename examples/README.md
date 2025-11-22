# Reinhardt Examples Tests

This directory contains practical application examples using **reinhardt from crates.io or local workspace**.

## 🎯 Purpose

- **Dual Mode Support**:
  - **Production Mode**: Uses published version from crates.io
  - **Local Development Mode**: Uses local workspace crates via `REINHARDT_LOCAL_DEV=1`
- **Version validation**: Ensures each example works with specific versions
- **End-to-end testing**: Validates functionality in actual user environments
- **Infrastructure**: TestContainers for automatic database setup (no docker-compose required)

## 📋 Prerequisites

### Required
- **Rust**: 1.85+ (Rust 2024 Edition)
- **Docker**: Container runtime for TestContainers

### Optional
- **cargo-nextest**: For faster test execution (`cargo install cargo-nextest`)

### Installation Check

```bash
# Check Docker installation
docker --version
docker ps  # Verify Docker daemon is running
```

## 🚀 Quick Start

### Local Development Mode (Recommended for Development)

Use local workspace crates instead of crates.io:

```bash
cd examples

# Set environment variable for local development mode
export REINHARDT_LOCAL_DEV=1

# Run tests (TestContainers will automatically start PostgreSQL)
cargo test --workspace

# Or with nextest
cargo nextest run --workspace
```

### Production Mode (Testing Published Crates)

Test against published crates.io versions:

```bash
cd examples

# Run tests without REINHARDT_LOCAL_DEV
cargo test --workspace

# Or with nextest
cargo nextest run --workspace
```

**Note**: Production mode requires reinhardt to be published on crates.io. If not published, tests will be skipped.

## 📝 Version Specification (Cargo Compatible)

Each test can specify version requirements using `#[example_test(version = "...")]` attribute with **the same syntax as Cargo.toml**.

### Supported Version Specifiers

```rust
// 1. Exact version
#[example_test(version = "0.1.0")]
fn test_exact() { }

// 2. Caret requirement (^)
#[example_test(version = "^0.1")]
fn test_caret() { }  // 0.1.x only

// 3. Tilde requirement (~)
#[example_test(version = "~0.1.2")]
fn test_tilde() { }  // 0.1.2 <= version < 0.2.0

// 4. Range specification
#[example_test(version = ">=0.1.0, <0.2.0")]
fn test_range() { }

// 5. Wildcard
#[example_test(version = "*")]
fn test_latest() { }  // Latest version
```

## 📂 Examples List

| Example | Version Requirement | Database | Description | README |
|---------|---------------------|----------|-------------|--------|
| `hello-world` | `*` (latest) | Not required | Minimal application | - |
| `rest-api` | `^0.1` (0.1.x) | Not required | RESTful API with Django-style structure | [README](rest-api/README.md) |
| `database-integration` | `^0.1` (0.1.x) | Required | PostgreSQL integration with migrations | [README](database-integration/README.md) |

### Example Features

#### hello-world
- Minimal configuration
- Simple entry point
- Basic Reinhardt usage

#### rest-api ([Details](rest-api/README.md))
- **Django-style project structure**: config/, settings/, apps.rs
- **Environment-specific settings**: local, staging, production
- **manage CLI**: Django-style management commands via `cargo run --bin manage`
- **URL routing**: RESTful API endpoints

#### database-integration ([Details](database-integration/README.md))
- **Django-style project structure**: config/, settings/, apps.rs
- **Database configuration management**: Environment-specific DB connection settings
- **Migration system**: Schema version control
- **manage CLI**: makemigrations, migrate commands

## 🏗️ Workspace Structure

```
examples/                    # Independent workspace
├── Cargo.toml              # Workspace configuration
├── test-macros/            # Custom test macros
├── common/                 # Common utilities
│   └── src/
│       └── manage_cli.rs   # Shared manage CLI implementation
├── hello-world/            # Example 1 (minimal structure)
├── rest-api/               # Example 2 (full structure)
│   └── src/
│       ├── config/         # Django-style config
│       ├── bin/
│       │   └── manage.rs   # Management CLI
│       └── main.rs
└── database-integration/   # Example 3 (full structure)
    └── src/
        ├── config/         # Django-style config
        ├── bin/
        │   └── manage.rs   # Management CLI
        └── main.rs
```

Each example is a **workspace member**, managed in `examples/Cargo.toml`.

### Project Structure

Examples (`rest-api`, `database-integration`) use **Django-style project structure**:

```
src/
├── config/
│   ├── apps.rs              # Installed apps definition
│   ├── settings.rs          # Environment-based settings loader
│   ├── settings/
│   │   ├── base.rs          # Common settings for all environments
│   │   ├── local.rs         # Local development settings
│   │   ├── staging.rs       # Staging environment settings
│   │   └── production.rs    # Production environment settings
│   └── urls.rs              # URL routing configuration
├── apps.rs                  # App registry
├── config.rs                # config module declaration
├── main.rs                  # Application entry point
└── bin/
    └── manage.rs            # Management CLI tool (Django's manage.py)
```

### manage CLI

Django-style management command tool:

```bash
# Start development server
cargo run --bin manage runserver [address]

# Database migrations
cargo run --bin manage makemigrations [app_labels...]
cargo run --bin manage migrate [app_label] [migration_name]

# Interactive shell
cargo run --bin manage shell [-c command]

# Project check
cargo run --bin manage check [app_label]

# Collect static files
cargo run --bin manage collectstatic [options]

# Show URL list
cargo run --bin manage showurls [--names]
```

See each example's README for details.

## 🐳 Infrastructure with TestContainers

### Automatic Container Management

TestContainers automatically starts and stops containers for each test:

- **PostgreSQL**: Automatically started for database tests
- **Isolated**: Each test gets its own container instance
- **Cleanup**: Containers are automatically removed after tests

### How It Works

```rust
use testcontainers::{clients::Cli, GenericImage, RunnableImage};

#[test]
async fn test_with_database() {
    // Container automatically started
    let docker = Cli::default();
    let postgres = docker.run(
        GenericImage::new("postgres", "16-alpine")
            .with_env_var("POSTGRES_PASSWORD", "test")
    );

    let port = postgres.get_host_port_ipv4(5432);
    let url = format!("postgres://postgres:test@localhost:{}/testdb", port);

    // Test code here

    // Container automatically stopped and removed when dropped
}
```

### Benefits

- **No Manual Setup**: No need to start docker-compose before tests
- **Isolated**: Tests don't interfere with each other
- **Portable**: Works on any system with Docker installed
- **Fast**: Containers start only when needed

### Database Migrations

Examples using databases utilize **reinhardt-migrations** for schema management:

- **No SQL Scripts**: Database initialization handled through migrations
- **Automatic Application**: Migrations run on application startup
- **Version Control**: Migration history tracked in code

**Example Migration Structure:**
```
database-integration/
├── migrations/
│   ├── mod.rs
│   └── 0001_initial.rs
└── src/
    └── main.rs         # Applies migrations on startup
```

**Migration Example:**
```rust
use reinhardt_migrations::{Migration, Operation};

pub fn migration() -> Migration {
    Migration::new("0001_initial")
        .add_operation(Operation::CreateTable {
            name: "users".to_string(),
            columns: vec![
                ("id", "SERIAL PRIMARY KEY"),
                ("name", "VARCHAR(255) NOT NULL"),
                ("email", "VARCHAR(255) NOT NULL UNIQUE"),
            ],
        })
}
```

## 🔧 Development Workflow

### Adding a New Example

1. **Create directory**
   ```bash
   mkdir examples/my-example
   cd examples/my-example
   ```

2. **Create Cargo.toml**
   ```toml
   [package]
   name = "example-my-example"
   version = "0.1.0"
   edition = "2024"
   publish = false

   [dependencies]
   reinhardt = "^0.1.0-alpha.1"
   ```

3. **Add to workspace**
   ```toml
   # examples/Cargo.toml
   [workspace]
   members = [
       # ...
       "my-example",
   ]
   ```

4. **Create tests**
   ```rust
   // examples/my-example/tests/integration.rs
   use example_test_macros::example_test;

   #[example_test(version = "^0.1")]
   fn test_my_feature() {
       // Test code
   }
   ```

## ⚠️ Troubleshooting

### Database Connection Error

```bash
# Check health
cargo make status

# Check logs
cargo make logs-postgres

# Restart database
cargo make down
cargo make up
```

### Port Conflict

```bash
# Change port numbers in .env file
POSTGRES_PORT=5433
MYSQL_PORT=3307
REDIS_PORT=6380
```

### Tests Are Skipped (Production Mode)

```
⏭️  Skipping test: reinhardt not available from crates.io
```

**Cause**: reinhardt is not yet published to crates.io

**Solution**: Use local development mode:
```bash
export REINHARDT_LOCAL_DEV=1
cargo test --workspace
```

## 📚 Related Documentation

- [Reinhardt Main Tests](../tests/)
- [Project README](../README.md)
- [Contributing Guide](../CONTRIBUTING.md)
- [docker-compose Specification](https://docs.docker.com/compose/compose-file/)

---

## 💡 Implementation Notes

### Dual Mode Architecture

This examples workspace supports two modes:

**Production Mode** (Default):
- Uses published crates from crates.io
- Tests the actual user experience
- Validates version compatibility
- Skips tests if crates aren't published

**Local Development Mode** (`REINHARDT_LOCAL_DEV=1`):
- Uses local workspace crates via `[patch.crates-io]`
- Enables testing unreleased features
- Bypasses version checks
- Allows development before publication

### How It Works

1. **Build Script Check**: Each example's `build.rs` checks:
   - If `REINHARDT_LOCAL_DEV=1`: Use local workspace
   - Otherwise: Check if reinhardt is available on crates.io

2. **Cargo Patch**: `examples/Cargo.toml` includes:
   ```toml
   [patch.crates-io]
   reinhardt = { path = "../crates/reinhardt" }
   ```
   This overrides crates.io versions with local workspace when building.

3. **Conditional Compilation**: Tests are compiled only when reinhardt is available:
   ```rust
   #[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
   mod tests_with_reinhardt { }
   ```

### Why Version-Specific Tests?

Different versions may have different APIs or behaviors. Version-specific tests:

1. **Prevent Regressions**: Detect breaking changes
2. **Document Compatibility**: Show which features work with which versions
3. **Aid Migration**: Help users understand version differences

### TestContainers Integration

- **No docker-compose needed**: Containers are managed automatically
- **Isolated testing**: Each test gets its own container
- **Automatic cleanup**: Containers are removed after tests
- **Cross-platform**: Works on any system with Docker
