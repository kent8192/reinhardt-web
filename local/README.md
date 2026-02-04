# Local Development Examples

This directory contains examples that always use the **local workspace** version of Reinhardt.

## 🎯 Purpose

- Development and testing of the latest Reinhardt implementation
- Creating examples for unreleased features
- Rapid prototyping and experimentation
- Testing breaking changes before publication

## 🔧 How It Works

### Dependency Resolution

All examples in this directory use the local workspace version of Reinhardt via `[patch.crates-io]`:

```toml
# examples/local/Cargo.toml
[patch.crates-io]
reinhardt = { path = "../..", package = "reinhardt-web", version = "0.1.0-alpha.1" }
```

Even though each example's `Cargo.toml` specifies a version constraint:

```toml
# examples/local/examples-hello-world/Cargo.toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1" }
```

The workspace-level `[patch.crates-io]` overrides this to use the local workspace crates.

### Build Configuration

Each example's `build.rs` is simplified to always enable tests:

```rust
// examples/local/examples-hello-world/build.rs
fn main() {
	// Local development mode: always enable with-reinhardt feature
	println!("cargo:rustc-cfg=feature=\"with-reinhardt\"");
	println!("cargo:warning=Using local reinhardt workspace (examples/local)");
	println!("cargo:rerun-if-changed=build.rs");
}
```

## 🚀 Quick Start

### Running Tests

```bash
cd examples/local

# Run all tests
cargo test --workspace

# Run tests with nextest
cargo nextest run --workspace

# Run specific example tests
cargo test -p examples-hello-world
```

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

| Example | Features | Database |
|---------|----------|----------|
| `examples-hello-world` | Basic setup | Not required |
| `examples-rest-api` | REST API, routing | Not required |
| `examples-database-integration` | ORM, migrations | Required (PostgreSQL) |

See [main examples README](../README.md) for detailed feature descriptions.

## 🔄 Development Workflow

### Creating a New Example

1. **Create example directory**
   ```bash
   mkdir examples-my-feature
   cd examples-my-feature
   ```

2. **Create `Cargo.toml`**
   ```toml
   [package]
   name = "examples-my-feature"
   version = "0.1.0-alpha.1"
   edition = "2024"
   publish = false

   [dependencies]
   reinhardt = { version = "0.1.0-alpha.1", features = ["core", "conf"] }

   # Common utilities from remote/
   example-common = { path = "../../remote/common" }

   [dev-dependencies]
   example-test-macros = { path = "../../remote/test-macros" }

   [build-dependencies]
   example-common = { path = "../../remote/common" }

   [features]
   default = []
   with-reinhardt = []

   [[test]]
   name = "integration"
   required-features = ["with-reinhardt"]
   ```

3. **Create `build.rs`**
   ```rust
   //! Build script for my-feature example (local development mode)
   //!
   //! This example always uses local reinhardt workspace via [patch.crates-io].
   //! Tests are always enabled in local development mode.

   fn main() {
   	// Local development mode: always enable with-reinhardt feature
   	println!("cargo:rustc-cfg=feature=\"with-reinhardt\"");
   	println!("cargo:warning=Using local reinhardt workspace (examples/local)");
   	println!("cargo:rerun-if-changed=build.rs");
   }
   ```

4. **Add to workspace**
   ```toml
   # examples/local/Cargo.toml
   [workspace]
   members = [
       "examples-hello-world",
       "examples-rest-api",
       "examples-database-integration",
       "examples-my-feature",  # Add here
   ]
   ```

5. **Implement and test**
   ```bash
   cargo test -p examples-my-feature
   ```

### Testing Against Breaking Changes

When Reinhardt introduces breaking changes:

1. **Update the example to use new API**
   ```bash
   # Modify code to match new API
   vim src/main.rs

   # Test
   cargo test -p examples-my-feature
   ```

2. **If update is not feasible**, move to `remote/` with version constraint:
   ```bash
   # Copy to remote with last compatible version
   cp -r examples/local/examples-my-feature ../remote/
   cd ../remote/examples-my-feature

   # Update Cargo.toml with exact version
   # version = "=0.1.0-alpha.1"

   # Remove from local, add to remote workspace
   ```

## ⚠️ Important Notes

### Always Uses Local Workspace

- Examples in this directory **CANNOT** test published crates.io versions
- To test published versions, use `examples/remote/`
- Version constraints in `Cargo.toml` are informational only (overridden by `[patch.crates-io]`)

### Shared Utilities

`common/` and `test-macros/` are located in `../remote/` but are shared:

```toml
[dependencies]
example-common = { path = "../../remote/common" }

[dev-dependencies]
example-test-macros = { path = "../../remote/test-macros" }
```

This allows both local and remote examples to share testing infrastructure.

## 📚 Related Documentation

- [Main Examples README](../README.md)
- [Remote Examples README](../remote/README.md)
- [Project README](../../README.md)
