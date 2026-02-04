# Reinhardt Examples

This directory contains practical application examples demonstrating Reinhardt framework usage.

## 🎯 Directory Structure

```
examples/
├── local/               # Local development examples
│   ├── examples-hello-world/
│   ├── examples-rest-api/
│   ├── examples-database-integration/
│   └── Cargo.toml       # Workspace with [patch.crates-io]
│
├── remote/              # Published version examples
│   ├── common/          # Common utilities (shared by both local/remote)
│   ├── test-macros/     # Custom test macros (shared by both local/remote)
│   ├── examples-hello-world/
│   ├── examples-rest-api/
│   ├── examples-database-integration/
│   └── Cargo.toml       # Workspace without patches
│
└── README.md            # This file
```

## 📂 Local vs Remote Examples

### Local Examples (`examples/local/`)

**Purpose**: Development and testing of the latest Reinhardt implementation

**Characteristics**:
- Always uses **local workspace** reinhardt crates via `[patch.crates-io]`
- For developing new features and examples
- Tests run against unreleased code
- **Migration policy**: When a local example can no longer build due to breaking changes in Reinhardt:
  1. Fix the example to work with the new API, OR
  2. Move it to `remote/` with appropriate version specifier for the last compatible version

**Use cases**:
- Framework development
- Testing new features before release
- Creating examples for upcoming versions
- Rapid prototyping

**Configuration**:
```toml
# examples/local/Cargo.toml
[patch.crates-io]
reinhardt = { path = "../../..", package = "reinhardt-web", version = "0.1.0-alpha.1" }
```

### Remote Examples (`examples/remote/`)

**Purpose**: Testing and validating published crates.io versions

**Characteristics**:
- Uses **published versions** from crates.io (NOT local workspace)
- Version requirements specified in `Cargo.toml` (no `path` attribute)
- Tests the actual user experience
- **Archive**: Contains examples that worked with specific published versions
- **Will fail to build** if reinhardt is not yet published to crates.io (this is intentional)

**Use cases**:
- Validating crates.io publications
- Testing backward compatibility
- User onboarding examples
- Documentation examples

**Configuration**:
```toml
# examples/remote/examples-rest-api/Cargo.toml
[dependencies]
# Uses crates.io by default (will fail until reinhardt is published)
reinhardt = { version = "0.1.0-alpha.1", features = ["core", "conf", "database", "commands"] }
```

**Important**: Remote examples do NOT use `path = "../.."` to reference the local workspace. They exclusively use published versions from crates.io.

## 🚀 Quick Start

### Running Local Examples

```bash
cd examples/local

# Run tests
cargo test --workspace

# Or with nextest
cargo nextest run --workspace

# Build specific example
cargo build -p examples-hello-world
```

### Running Remote Examples

```bash
cd examples/remote

# Run tests (requires reinhardt to be published on crates.io)
cargo test --workspace

# Or with nextest
cargo nextest run --workspace
```

**Note**: Remote examples will skip tests if the required reinhardt version is not available on crates.io.

## 📋 Prerequisites

### Required
- **Rust**: 1.85+ (Rust 2024 Edition)
- **Docker**: Container runtime for TestContainers (for database examples)

### Optional
- **cargo-nextest**: For faster test execution (`cargo install cargo-nextest`)

### Installation Check

```bash
# Check Docker installation
docker --version
docker ps  # Verify Docker daemon is running
```

## 📝 Available Examples

### Local Examples

| Example | Features | Database | Description |
|---------|----------|----------|-------------|
| `examples-hello-world` | Basic setup | Not required | Minimal Reinhardt application |
| `examples-rest-api` | REST API, routing | Not required | RESTful API with Django-style structure |
| `examples-database-integration` | ORM, migrations | Required (PostgreSQL) | Database integration with migrations |

### Remote Examples

Same examples as local, but tested against published crates.io versions.

## 🔄 Development Workflow

### Creating a New Example

**Step 1**: Create in `local/` first

```bash
cd examples/local
mkdir examples-my-feature
cd examples-my-feature

# Create Cargo.toml
cat > Cargo.toml << 'EOF'
[package]
name = "examples-my-feature"
version = "0.1.0-alpha.1"
edition = "2024"
publish = false

[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["core", "conf"] }
example-common = { path = "../../remote/common" }

[dev-dependencies]
example-test-macros = { path = "../../remote/test-macros" }
EOF

# Add to workspace members in examples/local/Cargo.toml
```

**Step 2**: When example is stable and reinhardt is published

Copy the example to `remote/` with appropriate version constraints:

```bash
# After reinhardt 0.1.0-alpha.1 is published
cp -r examples/local/examples-my-feature examples/remote/
cd examples/remote/examples-my-feature

# Update Cargo.toml with appropriate version constraint
# version = "^0.1.0-alpha.1"  (for 0.1.x series)
# version = ">=0.1.0-alpha.1, <0.2.0"  (explicit range)

# Add to workspace members in examples/remote/Cargo.toml
```

### Handling Breaking Changes

When Reinhardt introduces breaking changes:

**Option 1: Update the example**
```bash
# Fix the example to work with new API in local/
cd examples/local/examples-my-feature
# Update code to use new API
cargo test
```

**Option 2: Archive to remote/**
```bash
# If example cannot be easily updated, archive it in remote/
# with version constraint for the last compatible version
cd examples/remote/examples-my-feature
# Update Cargo.toml: version = "=0.1.0-alpha.1"  (exact version)
```

## 🐳 Infrastructure with TestContainers

### Automatic Container Management

TestContainers automatically starts and stops containers for each test:

- **PostgreSQL**: Automatically started for database tests
- **Isolated**: Each test gets its own container instance
- **Cleanup**: Containers are automatically removed after tests

### How It Works

```rust
use testcontainers::{clients::Cli, GenericImage};

#[tokio::test]
async fn test_with_database() {
	// Container automatically started
	let docker = Cli::default();
	let postgres = docker.run(
		GenericImage::new("postgres", "16-alpine")
			.with_env_var("POSTGRES_PASSWORD", "test")
	);

	let port = postgres.get_host_port_ipv4(5432).await.unwrap();
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

## 🧪 Testing Infrastructure

### Standard Fixtures from reinhardt-test

All examples now use **standard fixtures** from the `reinhardt-test` crate for consistent, maintainable testing. These fixtures provide automatic resource management and cleanup.

#### Available Fixtures

**`test_server_guard`** - Automatic test server lifecycle management
- Starts test server on random available port
- Provides `base_url()` for HTTP requests
- Automatically cleans up resources after test
- Usage with rstest:
  ```rust
  use reinhardt_test::fixtures::test_server_guard;
  use rstest::*;

  #[rstest]
  #[tokio::test]
  async fn test_endpoint(
      #[future] test_server_guard: reinhardt_test::resource::TeardownGuard<
          reinhardt_test::fixtures::TestServerGuard
      >,
  ) {
      let server = test_server_guard.await;
      let base_url = server.base_url();

      let client = reqwest::Client::new();
      let response = client.get(&format!("{}/", base_url)).send().await?;
      assert_eq!(response.status(), reqwest::StatusCode::OK);
  }
  ```

**`postgres_container`** - PostgreSQL TestContainer fixture
- Automatically starts PostgreSQL container
- Provides connection URL
- Cleanup handled via RAII (container dropped automatically)

**`test_user` / `admin_user`** - Authentication test users
- Pre-configured test users with different permission levels
- Used for testing authentication and authorization

**`fixture_loader`** - Test data loading
- Loads test data from files
- Manages test data lifecycle

**`mock_database_backend`** - Mockall-based database mocking
- For unit tests without real database
- Controllable mock behavior

### Testing Best Practices

**Use Standard Fixtures for E2E Tests:**
```rust
// ✅ GOOD: Using standard fixture
#[rstest]
#[tokio::test]
async fn test_with_standard_fixture(
    #[future] test_server_guard: TeardownGuard<TestServerGuard>,
) {
    let server = test_server_guard.await;
    // Test code with automatic cleanup
}

// ❌ BAD: Manual server setup
#[tokio::test]
async fn test_with_manual_setup() {
    let server = start_server().await;
    // Test code
    server.stop().await; // Manual cleanup required
}
```

**Test Coverage Requirements:**
- ✅ **Normal cases**: Test expected behavior
- ✅ **Error cases**: Test 4xx/5xx responses
- ✅ **Edge cases**: Test boundary conditions
- ✅ **Cleanup**: All fixtures handle cleanup automatically

See [Testing Standards](../docs/TESTING_STANDARDS.md) for comprehensive guidelines.

## 📚 Example Features

### examples-hello-world
- Minimal configuration
- Simple entry point
- Basic Reinhardt usage
- **E2E tests with standard fixtures**
- **Test coverage**: Normal cases (GET /, GET /health) and error cases (404, 405)

### examples-rest-api
- **Django-style project structure**: config/, settings/, apps.rs
- **Environment-specific settings**: local, staging, production
- **manage CLI**: Django-style management commands
- **URL routing**: RESTful API endpoints

See [examples-rest-api README](remote/examples-rest-api/README.md) for details.

### examples-database-integration
- **Django-style project structure**: config/, settings/, apps.rs
- **Database configuration management**: Environment-specific DB connection settings
- **Migration system**: Schema version control
- **manage CLI**: makemigrations, migrate commands

See [examples-database-integration README](remote/examples-database-integration/README.md) for details.

## ⚠️ Troubleshooting

### Remote Examples: Build Fails with "package `reinhardt` not found"

```
error: no matching package named `reinhardt` found
```

**Cause**: Required reinhardt version is not yet published to crates.io

**This is intentional behavior** - remote examples are designed to exclusively use crates.io versions.

**Solution**: Use local examples instead for development:
```bash
cd examples/local
cargo test --workspace
```

**When reinhardt is published**: Remote examples will automatically work without any code changes.

### Database Connection Error

```bash
# Check Docker is running
docker ps

# Check container logs
docker logs <container_id>
```

### Port Conflict

TestContainers automatically assigns random ports, so port conflicts should not occur. If you encounter issues:

```bash
# Restart Docker daemon
docker restart
```

## 📚 Related Documentation

- [Reinhardt Main Tests](../tests/)
- [Project README](../README.md)
- [Contributing Guide](../CONTRIBUTING.md)

---

## 💡 Design Rationale

### Why Two Directories?

**Separation of Concerns**:
- `local/`: Active development, latest features, may be unstable
- `remote/`: Published versions, stable, documented compatibility

**Dependency Resolution Strategy**:
- `local/`: Uses workspace patch (`[patch.crates-io]`) to override with local code
- `remote/`: Uses only crates.io published versions (no `path` attributes, no patches)

**Version Management**:
- `local/`: Always uses local workspace via patch (no version constraints needed)
- `remote/`: Explicit version requirements in Cargo.toml (e.g., `version = "0.1.0-alpha.1"`)

**Migration Path**:
1. Develop new examples in `local/`
2. Publish reinhardt to crates.io
3. Copy stable examples to `remote/` with version constraints
4. `remote/` becomes the archive of working examples for each version

**Key Difference**:
- `local/`: `[patch.crates-io]` in workspace Cargo.toml
- `remote/`: No patches, pure crates.io resolution (will fail until published)

### Common Utilities Sharing

`common/` and `test-macros/` are located in `remote/` but shared by both:

**Why in remote/**:
- Primarily used by remote examples for availability checking
- Contains crates.io availability detection logic
- Version checking utilities

**How local/ uses them**:
```toml
# examples/local/examples-hello-world/Cargo.toml
[dev-dependencies]
example-common = { path = "../../remote/common" }
example-test-macros = { path = "../../remote/test-macros" }
```

This allows both directories to share common testing infrastructure while maintaining separate dependency resolution strategies.
