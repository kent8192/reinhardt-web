# reinhardt-dentdelion

Plugin system for the Reinhardt framework - easy to create, distribute, and install.

## Overview

**Dentdelion** (French for "lion's tooth" = dandelion) provides a comprehensive plugin architecture for extending Reinhardt applications. It supports both static plugins (Rust crates compiled into your application) and dynamic plugins (WebAssembly modules loaded at runtime).

## Key Features

- **Static Plugins**: Compile-time integration via Rust crates
- **Dynamic Plugins**: Runtime loading via WebAssembly (WASM) modules
- **Capability System**: Fine-grained access control for plugin features
- **Dependency Management**: Automatic dependency resolution with cycle detection
- **crates.io Integration**: Install plugins directly from crates.io
- **Lifecycle Hooks**: `on_load`, `on_enable`, `on_disable`, `on_unload`

## Installation

Add `reinhardt` to your `Cargo.toml`:

<!-- reinhardt-version-sync:3 -->
```toml
[dependencies]
reinhardt = { version = "0.4.0", features = ["dentdelion"] }

# Or use a preset:
# reinhardt = { version = "0.4.0", features = ["standard"] }  # Recommended
# reinhardt = { version = "0.4.0", features = ["full"] }      # All features
```

Then import dentdelion features:

```rust
use reinhardt::dentdelion::prelude::*;
```

**Note:** Dentdelion features are included in the `standard` and `full` feature presets.

### Feature Flags

| Feature | Description |
|---------|-------------|
| `default` | Core plugin system only |
| `wasm` | WebAssembly plugin support (requires wasmtime 36.x) |
| `ts` | TypeScript/JavaScript SSR support via boa_engine (pure-Rust ECMAScript engine); also enables `wasm` |
| `cli` | CLI support for crates.io integration |
| `full` | wasm + cli (note: `ts` is NOT included in `full`) |

## Quick Start

### Creating a Static Plugin

<!-- reinhardt-version-sync -->
```rust
use reinhardt::dentdelion::prelude::*;

pub struct MyPlugin {
    metadata: PluginMetadata,
}

impl MyPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata::builder("my-plugin", "0.4.0")
                .description("My custom plugin")
                .author("Your Name")
                .build()
                .unwrap(),
        }
    }
}

impl Plugin for MyPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> &[Capability] {
        &[
            Capability::Core(PluginCapability::Middleware),
            Capability::Core(PluginCapability::Commands),
        ]
    }
}
```

### Registering Plugins

```rust
use reinhardt::dentdelion::prelude::*;
use std::sync::Arc;

let mut registry = PluginRegistry::new();

// Register a plugin
let plugin = Arc::new(MyPlugin::new());
registry.register(plugin)?;

// Enable the plugin
registry.enable("my-plugin")?;

// Query plugins by capability
let middleware_plugins = registry.plugins_with_capability(&Capability::Core(PluginCapability::Middleware));
```

### Implementing Lifecycle Hooks

```rust
use reinhardt::dentdelion::prelude::*;

#[async_trait]
impl PluginLifecycle for MyPlugin {
    async fn on_load(&self, ctx: &PluginContext) -> PluginResult<()> {
        // Initialize resources
        Ok(())
    }

    async fn on_enable(&self, ctx: &PluginContext) -> PluginResult<()> {
        // Activate plugin features
        Ok(())
    }

    async fn on_disable(&self, ctx: &PluginContext) -> PluginResult<()> {
        // Deactivate plugin features
        Ok(())
    }

    async fn on_unload(&self, ctx: &PluginContext) -> PluginResult<()> {
        // Cleanup resources
        Ok(())
    }
}
```

## Core Types

### Plugin Trait

The fundamental interface all plugins must implement:

```rust
pub trait Plugin: Send + Sync {
    fn metadata(&self) -> &PluginMetadata;
    fn capabilities(&self) -> &[Capability];
    fn has_capability(&self, capability: &Capability) -> bool;
    fn is_dynamic(&self) -> bool;
    fn name(&self) -> &str;
    fn version(&self) -> &semver::Version;
}
```

### PluginCapability

Standard capabilities that plugins can provide:

| Capability | Description | WASM Support |
|------------|-------------|--------------|
| `Middleware` | HTTP middleware components | ✅ |
| `Models` | Database models and migrations | ❌ (Static only) |
| `Commands` | CLI management commands | ✅ |
| `ViewSets` | REST API ViewSets | ✅ |
| `Signals` | Custom signals | ✅ |
| `Services` | DI services | ✅ |
| `Auth` | Authentication backends | ✅ |
| `Templates` | Template engines or filters | ✅ |
| `StaticFiles` | Static file handling | ✅ |
| `Routing` | URL routing | ✅ |
| `SignalReceivers` | Signal receivers | ✅ |
| `Handlers` | HTTP handlers/views | ✅ |
| `NetworkAccess` | HTTP client for external requests | ✅ |
| `DatabaseAccess` | SQL query execution | ✅ |
| `StaticSiteGeneration` | Build static HTML from routes and components | ❌ (Static only) |
| `FrontendSsr` | Server-side rendering for frontend frameworks | ❌ |
| `FrontendHydration` | Client-side hydration and state restoration | ❌ |
| `TypeScriptRuntime` | JavaScript execution through the Boa ECMAScript engine | ❌ |
| `BuildToolIntegration` | Integration with frontend build tools | ❌ (WASM configuration maps it as a custom capability, not the standard capability) |
| `HotModuleReplacement` | Development-time dynamic module reload | ❌ |

### PluginRegistry

Central plugin management:

- Register and unregister plugins
- Enable and disable plugins
- Query plugins by capability
- Manage dependencies with topological sorting
- Detect circular dependencies

### PluginMetadata

Plugin identification information:

```rust
let metadata = PluginMetadata::builder("my-plugin", "1.0.0")
    .description("My plugin description")
    .author("Author Name")
    .license("MIT")
    .repository("https://github.com/example/my-plugin")
    .add_dependency("other-plugin", ">=1.0.0")
    .build()?;
```

### ProjectManifest (dentdelion.toml)

Plugin configuration file:

```toml
[plugins]
my-plugin = { version = "1.0.0", enabled = true }
other-plugin = { version = "2.0.0", enabled = false }

[wasm]
plugins_dir = "plugins/"
memory_limit = 134217728  # 128MB
timeout_seconds = 30
```

## WASM Plugin Development

### Requirements

- Enable the `wasm` feature
- WASM plugins must be compiled with the Component Model
- Plugins must implement the `dentdelion-plugin` WIT world
- Use `wit-bindgen` or `cargo-component` for code generation

### WIT Interface Definition

Dentdelion uses the WebAssembly Interface Types (WIT) standard for plugin interfaces. The complete interface is defined in `wit/dentdelion.wit`:

<!-- reinhardt-version-sync -->
```wit
package reinhardt:dentdelion@0.4.0;

// Host functions available to plugins
interface host {
    get-config: func(key: string) -> option<list<u8>>;
    set-config: func(key: string, value: list<u8>) -> result<_, plugin-error>;

    log-info: func(message: string);
    log-warn: func(message: string);
    log-error: func(message: string);

    http-get: func(url: string, headers: list<tuple<string, string>>) -> result<http-response, plugin-error>;
    db-query: func(sql: string, params: list<u8>) -> result<list<u8>, plugin-error>;
}

// Plugin functions that must be implemented
interface plugin {
    get-metadata: func() -> plugin-metadata;
    get-capabilities: func() -> list<capability>;
    on-load: func(config: list<u8>) -> result<_, plugin-error>;
    on-enable: func() -> result<_, plugin-error>;
    on-disable: func() -> result<_, plugin-error>;
    on-unload: func() -> result<_, plugin-error>;
}
```

### WasmRuntime Configuration

```rust
use reinhardt::dentdelion::wasm::{WasmRuntime, WasmRuntimeConfigBuilder};
use std::time::Duration;

let config = WasmRuntimeConfigBuilder::new()
    .memory_limit_mb(128)
    .timeout(Duration::from_secs(30))
    .fuel_metering(true)
    .initial_fuel(100_000_000)
    .build();

let runtime = WasmRuntime::new(config)?;
```

### Host API

WASM plugins can access the following host functions:

| Category | Functions | Capability Required |
|----------|-----------|---------------------|
| Config | `get_config`, `set_config` | None |
| Logging | `log_debug`, `log_info`, `log_warn`, `log_error` | None |
| Services | `register_service`, `get_service`, `unregister_service` | `Services` |
| HTTP | `http_get`, `http_post` | `NetworkAccess` |
| Database | `db_query`, `db_execute` | `DatabaseAccess` |

**Data Serialization:**
- Configuration values, query parameters, and results are serialized using MessagePack
- This provides compact binary encoding for efficient WASM boundary crossing
- Use `rmp-serde` crate in Rust plugins for MessagePack serialization

### Capability-Based Security

WASM plugins must declare required capabilities in their manifest:

```toml
[wasm.my-plugin]
capabilities = ["NetworkAccess", "DatabaseAccess"]
```

Host functions check capabilities at runtime:
- `http_get`/`http_post` require `NetworkAccess` capability
- `db_query`/`db_execute` require `DatabaseAccess` capability
- Unauthorized calls return `PluginError` with code 403

### HTTP Resource Limits

`http_get`/`http_post` buffer responses in host memory, outside the guest's
memory and fuel limits, so each `HostState` bounds them with
`HttpResourceLimits`:

| Limit | Default | Error code when exceeded |
|-------|---------|--------------------------|
| `max_response_body_bytes` | 10 MiB | 502 |
| `max_request_body_bytes` | 10 MiB | 413 |
| `request_timeout` (connect, send, and full body) | 30 s | 504 |
| `max_concurrent_requests` per plugin | 4 | 429 |

```rust
use reinhardt::dentdelion::wasm::{HostStateBuilder, HttpResourceLimits};
use std::time::Duration;

let mut limits = HttpResourceLimits::default();
limits.max_response_body_bytes = 1024 * 1024;
limits.request_timeout = Duration::from_secs(5);

let host_state = HostStateBuilder::new("my-plugin")
    .http_limits(limits)
    .build();
```

### Loading WASM Plugins

```rust
use reinhardt::dentdelion::wasm::{WasmPluginLoader, WasmRuntime};
use std::sync::Arc;

let runtime = Arc::new(WasmRuntime::new(Default::default())?);
let loader = WasmPluginLoader::new("plugins/", runtime);

// Discover .wasm files
let discovered = loader.discover().await?;

// Load and instantiate
for path in discovered {
    let instance = loader.load(&path).await?;

    // Initialize plugin
    let ctx = PluginContext::default();
    instance.on_load(&ctx).await?;
    instance.on_enable(&ctx).await?;
}
```

### Limitations

- **Models capability is NOT supported** for WASM plugins (requires compile-time integration)
- Aggregate linear-memory limit: 128MB per store by default (configurable via
  `WasmRuntimeConfig`). Initial allocations and growth across memories/instances
  share the budget; unsupported shared memories are rejected.
- Execution timeout: 30 seconds by default (configurable). A wall-clock bound
  covers each complete lifecycle invocation, including asynchronous host imports;
  Wasmtime epoch interruption also stops guest CPU execution.
- Fuel-based CPU metering: 100M instructions default (configurable)
- Plugin HTTP requests resolve DNS asynchronously, reject internal/reserved
  addresses, pin every validated DNS result, and never follow redirects or
  environment proxies. IPv6 special assignments exclude non-global ranges and
  ORCHID/DET overlay identifiers while preserving routable anycast, AMT and AS112
  locators (RFC 7343 and RFC 9374 distinguish identifiers from IPv6 locators).
- JavaScript evaluation and result conversion share one elapsed-time timeout,
  with a separate
  `TsResourceLimits::loop_iteration_limit` (default: 1,000,000 iterations per
  loop). Native builtins can synchronously reenter JavaScript, so the interpreter
  guard remains necessary even with cooperative deadline checks. Interpreter
  limits report `TsError::RuntimeLimitExceeded`, rather than `ExecutionTimeout`.
- Custom plugin HTTP clients use `HostStateBuilder::http_client_factory` so CA
  roots, client certificates, and default headers survive construction. The host
  applies validated DNS pinning and no redirects/proxies after the factory, and
  every call stays bounded by `HttpResourceLimits` (30-second `request_timeout`
  by default). Opaque clients passed to `http_client` return a migration error
  because their settings cannot be safely extracted.
- Config values must be MessagePack-serializable

## CLI Integration

When using the `cli` feature, the following commands are available:

```bash
# List installed plugins
reinhardt plugin list

# Show plugin details
reinhardt plugin info <name>

# Install from crates.io
reinhardt plugin install <name>

# Remove a plugin
reinhardt plugin remove <name>

# Enable/disable plugins
reinhardt plugin enable <name>
reinhardt plugin disable <name>

# Search crates.io
reinhardt plugin search <query>

# Update a plugin
reinhardt plugin update <name>
```

## Architecture

```mermaid
flowchart TB
    subgraph Registry["PluginRegistry"]
        plugins["plugins<br/>HashMap"]
        capability["capability<br/>map"]
        dependency["dependency_graph<br/>(topological)"]
    end

    Registry --> Static["Static Plugins<br/>(Rust crate)"]
    Registry --> WASM["WASM Plugins<br/>(.wasm)"]
    Registry --> Remote["Remote<br/>(crates.io)"]
```

## Related Crates

- `reinhardt-commands`: CLI command integration
- `reinhardt-di`: Dependency injection for services
- `reinhardt-middleware`: Middleware integration
- `reinhardt-core`: Signal system (signals module)

## License

Licensed under the MIT license.
