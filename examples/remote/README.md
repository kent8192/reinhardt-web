# Remote Examples (Published Versions)

This directory contains examples that test **published crates.io versions** of Reinhardt.

## 🎯 Purpose

- Testing and validating published crates.io versions
- User onboarding and documentation examples
- Backward compatibility testing
- Archive of working examples for each version

## 🔧 How It Works

### Dependency Resolution

Examples in this directory use **published versions** from crates.io:

```toml
# examples/remote/Cargo.toml
# No [patch.crates-io] section - uses crates.io directly

[workspace]
members = [
	"common",
	"test-macros",
	"examples-hello-world",
	"examples-rest-api",
	"examples-database-integration",
]
```

Each example specifies version requirements in its `Cargo.toml`:

```toml
# examples/remote/examples-rest-api/Cargo.toml
[dependencies]
reinhardt = { version = "^0.1.0-alpha.1", features = ["core", "conf", "database", "commands"] }
```

### Build Configuration

Each example's `build.rs` is simplified:

```rust
// examples/remote/examples-hello-world/build.rs
fn main() {
	// Remote mode: enable with-reinhardt feature
	// If reinhardt is not available on crates.io, dependency resolution will fail
	println!("cargo:rustc-cfg=feature=\"with-reinhardt\"");
	println!("cargo:warning=Using reinhardt from crates.io (examples/remote)");
	println!("cargo:rerun-if-changed=build.rs");
}
```

## 🚀 Quick Start

### Running Tests

```bash
cd examples/remote

# Run all tests
cargo test --workspace

# Run tests with nextest
cargo nextest run --workspace

# Run specific example tests
cargo test -p examples-hello-world
```

**Note**: Tests will be skipped if the required reinhardt version is not available on crates.io.

### Building Examples

```bash
# Build all examples
cargo build --workspace

# Build specific example
cargo build -p examples-rest-api

# Run example
cargo run -p examples-database-integration --bin manage
```

## 📝 Available Examples

| Example | Version Requirement | Features | Database |
|---------|---------------------|----------|----------|
| `examples-hello-world` | `*` (latest) | Basic setup | Not required |
| `examples-rest-api` | `^0.1` (0.1.x) | REST API, routing | Not required |
| `examples-database-integration` | `^0.1` (0.1.x) | ORM, migrations | Required (PostgreSQL) |

See [main examples README](../README.md) for detailed feature descriptions.

## 🧪 Testing Strategy

Remote examples use the **same testing infrastructure** as local examples to ensure consistency between development and published versions.

### Standard Fixtures from reinhardt-test

All examples use **standard fixtures** from the `reinhardt-test` crate for automatic resource management and cleanup.

#### Available Fixtures

**`test_server_guard`** - Automatic test server lifecycle management
- Starts test server on random available port
- Provides `base_url()` for HTTP requests
- Automatically cleans up resources after test

**`postgres_container`** - PostgreSQL TestContainer fixture
- Automatically starts PostgreSQL container
- Provides connection URL
- Cleanup handled via RAII

**`test_user` / `admin_user`** - Authentication test users
- Pre-configured test users with different permission levels
- Used for testing authentication and authorization

**`fixture_loader`** - Test data loading
- Loads test data from files
- Manages test data lifecycle

**`mock_database_backend`** - Mockall-based database mocking
- For unit tests without real database
- Controllable mock behavior

### Example Test Pattern

```rust
use reinhardt_test::fixtures::test_server_guard;
use rstest::*;

#[rstest]
#[example_test("*")]
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

### Version-Specific Testing

Tests can be version-specific using `#[example_test]` macro:

```rust
// Test runs only on reinhardt 0.1.x
#[example_test("^0.1")]
fn test_feature() {
    // Test code
}

// Test runs on any version
#[example_test("*")]
fn test_always() {
    // Test code
}
```

### Testing Best Practices

**Use Standard Fixtures for E2E Tests:**
```rust
// ✅ GOOD: Using standard fixture
#[rstest]
#[example_test("*")]
async fn test_with_standard_fixture(
    #[future] test_server_guard: TeardownGuard<TestServerGuard>,
) {
    let server = test_server_guard.await;
    // Test code with automatic cleanup
}

// ❌ BAD: Manual server setup
#[example_test("*")]
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

See [Testing Standards](../../docs/TESTING_STANDARDS.md) for comprehensive guidelines.

## 🔄 Maintenance Workflow

### Adding Examples from Local

When a local example is stable and reinhardt is published:

1. **Copy from local/**
   ```bash
   cp -r ../local/examples-my-feature .
   cd examples-my-feature
   ```

2. **Update version constraint**
   ```toml
   # Cargo.toml
   [dependencies]
   reinhardt = { version = "^0.1.0-alpha.1", features = ["core", "conf"] }
   # Use appropriate version constraint:
   # "^0.1.0-alpha.1"  - Caret (0.1.x series)
   # "~0.1.2"          - Tilde (0.1.2 <= version < 0.2.0)
   # ">=0.1, <0.2"     - Range
   # "=0.1.0-alpha.1"  - Exact (for archived examples)
   ```

3. **Update paths to common utilities**
   ```toml
   # Cargo.toml
   [dependencies]
   example-common = { path = "../common" }  # Note: ../ not ../../

   [dev-dependencies]
   example-test-macros = { path = "../test-macros" }
   ```

4. **Add to workspace**
   ```toml
   # examples/remote/Cargo.toml
   [workspace]
   members = [
       "common",
       "test-macros",
       "examples-hello-world",
       "examples-rest-api",
       "examples-database-integration",
       "examples-my-feature",  # Add here
   ]
   ```

5. **Test**
   ```bash
   cargo test -p examples-my-feature
   ```

### Archiving Examples for Specific Versions

When a breaking change makes an example incompatible with newer versions:

1. **Update version to exact constraint**
   ```toml
   # Cargo.toml
   [dependencies]
   reinhardt = { version = "=0.1.0-alpha.1", features = [...] }  # Exact version
   ```

2. **Add comment explaining compatibility**
   ```toml
   # This example is archived for reinhardt 0.1.0-alpha.1
   # For newer versions, see examples-my-feature-v2
   ```

3. **Optionally rename**
   ```bash
   mv examples-my-feature examples-my-feature-v0.1
   ```

## 📦 Common Utilities

This directory contains shared utilities:

### `common/`

Utilities for availability and version checking:

```rust
use example_common::availability;
use example_common::version;

// Check if reinhardt is available from crates.io
if availability::is_reinhardt_available() {
	// Run tests
}

// Check version requirement
if version::check_version("^0.1") {
	// Version matches
}
```

### `test-macros/`

Custom test macros for version-specific testing:

```rust
use example_test_macros::example_test;

// Test runs only on reinhardt 0.1.x
#[example_test(version = "^0.1")]
fn test_feature() {
	// Test code
}

// Test runs only on exact version
#[example_test(version = "=0.1.0-alpha.1")]
fn test_specific_version() {
	// Test code
}
```

These utilities are shared by both `remote/` and `local/` examples.

## ⚠️ Troubleshooting

### Tests Are Skipped

```
⏭️  Skipping test: reinhardt not available from crates.io
```

**Cause**: Required reinhardt version is not published to crates.io

**Solution**:
- Wait for publication, OR
- Use `examples/local/` for development

### Version Mismatch

```
⏭️  Skipping test: version mismatch
   Required: ^0.1, Actual: 0.2.0
```

**Cause**: Published version doesn't match requirement

**Solution**:
- Update version constraint in `Cargo.toml`, OR
- Create new example for newer version

### Dependency Resolution Failed

```
error: failed to select a version for `reinhardt`
```

**Cause**: Required version not available on crates.io

**Solution**: Use `examples/local/` or wait for publication

## 📚 Related Documentation

- [Main Examples README](../README.md)
- [Local Examples README](../local/README.md)
- [Project README](../../README.md)
