# Reinhardt Examples

Example applications demonstrating [Reinhardt](https://github.com/kent8192/reinhardt-web) web framework features.

## Quick Start

### One Command Run

```bash
git clone https://github.com/kent8192/reinhardt-web.git && cd reinhardt-web/examples/examples-tutorial-basis && cargo run
```

### Step by Step

```bash
# 1. Clone the repository
git clone https://github.com/kent8192/reinhardt-web.git
cd reinhardt-web/examples

# 2. Choose an example
cd examples-tutorial-basis

# 3. Run it
cargo run
```

### Examples with PostgreSQL

For examples that require PostgreSQL (`examples-twitter`):

```bash
cd reinhardt-web/examples/examples-twitter

# Start PostgreSQL
docker compose up -d

# Copy local settings template
cp settings/local.example.toml settings/local.toml

# Run the example
cargo run
```

## Examples

| Example | Description | Features |
|---------|-------------|----------|
| `examples-tutorial-basis` | Polling app tutorial (basis) | Pages, Forms, Database, WASM |
| `examples-tutorial-rest` | REST API tutorial (snippets) | REST, Viewsets, Database |
| `examples-twitter` | Full Twitter-like application | Full-stack, WebSockets, Auth, WASM |

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

## Testing

```bash
# Test a specific example
cd examples-tutorial-basis && cargo nextest run --all-features
```

## License

See the [main repository license](../LICENSE.md).
