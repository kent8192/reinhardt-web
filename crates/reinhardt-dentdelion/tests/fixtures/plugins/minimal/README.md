# Minimal WASM Plugin for Dentdelion

This is a minimal test plugin that implements all required lifecycle functions with no additional capabilities. It serves as a basic fixture for integration tests.

## Structure

```
minimal/
├── Cargo.toml          # Plugin manifest with WASM component metadata
├── wit/
│   └── dentdelion.wit  # WIT interface definition (copy from crates/reinhardt-dentdelion/wit/)
└── src/
    └── lib.rs          # Plugin implementation
```

## Implementation

The plugin implements the `reinhardt:dentdelion/plugin` interface with minimal functionality:

- **Metadata**: Returns basic plugin information (name: "minimal", version: "0.1.0")
- **Capabilities**: Returns empty list (no capabilities)
- **Lifecycle**:
  - `on_load()`: Accepts any configuration, returns Ok
  - `on_enable()`: No-op, returns Ok
  - `on_disable()`: No-op, returns Ok
  - `on_unload()`: No-op, returns Ok

## Building

```bash
cd crates/reinhardt-dentdelion/tests/fixtures/plugins/minimal
cargo component build --release
```

The WASM component will be generated at:
```
target/wasm32-wasip1/release/minimal_plugin.wasm
```

Copy to test fixtures:
```bash
cp target/wasm32-wasip1/release/minimal_plugin.wasm ../../minimal_plugin.wasm
```

## Dependencies

- `wit-bindgen 0.44` - WIT bindings generator
- `wit-bindgen-rt 0.44` - Runtime support for bindings

## Requirements

- `cargo-component` tool for building WASM components
- Rust toolchain with `wasm32-wasip1` target

## Usage in Tests

```rust
use reinhardt_dentdelion::wasm::{WasmPluginInstance, WasmRuntime};

let plugin_wasm = std::fs::read("tests/fixtures/minimal_plugin.wasm")?;
let runtime = WasmRuntime::new(Default::default())?;
let instance = WasmPluginInstance::load("minimal", &plugin_wasm, runtime).await?;

// Test lifecycle
instance.on_load(&config).await?;
instance.on_enable(&ctx).await?;
instance.on_disable(&ctx).await?;
instance.on_unload(&ctx).await?;
```
