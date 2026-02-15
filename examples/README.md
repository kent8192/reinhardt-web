# Reinhardt Examples

Example applications demonstrating [Reinhardt](https://github.com/kent8192/reinhardt-web) web framework features.

## Quick Start

```bash
# Clone this repository
git clone https://github.com/kent8192/reinhardt-examples.git
cd reinhardt-examples

# Run an example
cd examples-hello-world
cargo run
```

## Examples

| Example | Description | Features |
|---------|-------------|----------|
| `examples-hello-world` | Basic Hello World application | Core HTTP, Routing |
| `examples-rest-api` | REST API with CRUD operations | REST, Validation, Settings |
| `examples-database-integration` | Database integration with migrations | Database, ORM, SQLite, PostgreSQL |
| `examples-tutorial-basis` | Polling app tutorial (basis) | Pages, Forms, Database, WASM |
| `examples-tutorial-rest` | REST API tutorial (snippets) | REST, Viewsets, Database |
| `examples-github-issues` | GitHub Issues clone | GraphQL, JWT Auth |
| `examples-twitter` | Full Twitter-like application | Full-stack, WebSockets, Auth, WASM |

## Shared Crates

| Crate | Description |
|-------|-------------|
| `common` | Shared test utilities and version checking |
| `test-macros` | Procedural macros for conditional testing |

## Dependency Management

By default, examples use **crates.io published versions** of Reinhardt.

### Local Development (for framework contributors)

To test examples against your local Reinhardt workspace:

```bash
# Copy the local development config template
cp .cargo/config.local.toml .cargo/config.toml

# Build/test with local reinhardt
cargo build --workspace
cargo nextest run --workspace

# Clean up when done
rm -f .cargo/config.toml
```

Or use the Makefile.toml task from the main repo:

```bash
# From reinhardt-web root:
cargo make local-examples-test
```

### How It Works

- `.cargo/config.local.toml`: Pre-configured template with `[patch.crates-io]` overrides
- When copied to `.cargo/config.toml`, Cargo uses local workspace paths instead of crates.io
- `.cargo/config.toml` is gitignored so it won't be committed

## Subtree Integration

This repository is integrated into the main [reinhardt-web](https://github.com/kent8192/reinhardt-web) repository as a git subtree at `examples/`.

See [SUBTREE_OPERATIONS.md](SUBTREE_OPERATIONS.md) for subtree management instructions.

## Testing

```bash
# Test all examples
./scripts/test-all.sh

# Test a specific example
cd examples-hello-world && cargo nextest run --all-features

# Test common crates only
cargo test -p example-common -p example-test-macros
```

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
