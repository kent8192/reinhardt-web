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

## Examples

| Example | Description | Features |
|---------|-------------|----------|
| `examples-tutorial-basis` | Polling app tutorial (basis) | Pages, Forms, Database, WASM |
| `examples-tutorial-rest` | REST API tutorial (snippets) | REST, Viewsets, Database |

## Dependency Management

By default, examples use the parent Reinhardt checkout through the
`examples/Cargo.toml` workspace dependency. This lets example builds validate
the current workspace release line before and after publication to crates.io.

### Local Development

Run the examples directly from this repository:

```bash
cargo build --workspace
cargo nextest run --workspace
```

Or use the Makefile.toml task from the main repo:

```bash
# From reinhardt-web root:
cargo make local-examples-test
```

### How It Works

- `examples/Cargo.toml` declares `reinhardt` with `path = ".."` and the expected version
- Cargo uses the local checkout while still checking that the example version stays in sync
- `.cargo/config.local.toml` remains as a compatibility template for workflows that need an explicit `[patch.crates-io]` override

## Testing

```bash
# Test a specific example
cd examples-tutorial-basis && cargo nextest run --all-features
```

## License

See the [main repository license](../LICENSE.md).
