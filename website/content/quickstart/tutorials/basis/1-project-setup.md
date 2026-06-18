+++
title = "Part 1: Project Setup and SPA Shell"
description = "Generate a Reinhardt pages project, inspect the native/WASM layout, and run the empty SPA shell."
weight = 10

[extra]
sidebar_weight = 10
+++

# Part 1: Project Setup and SPA Shell

In this part you will create a Reinhardt pages project and run the browser shell. The polling features arrive in later parts; here the goal is to understand the project shape that lets one crate build both a native server and a WASM client.

The finished reference for this tutorial is `examples/examples-tutorial-basis`. Use it as the answer key when your local project differs from the snippets below.

## Install the Tools

Install the Reinhardt project generator:

<!-- reinhardt-version-sync -->
```bash
cargo install reinhardt-admin-cli --version "0.3.0-rc.1"
```

The installed binary is `reinhardt-admin`.

The pages template also needs the WASM target, `cargo-make`, and `wasm-pack`:

```bash
rustup target add wasm32-unknown-unknown
cargo install cargo-make wasm-pack
```

## Create a Pages Project

Create a new project from the pages template:

```bash
reinhardt-admin startproject tutorial --template pages
cd tutorial
```

The completed tutorial will eventually add `polls` and `users` apps. Do not create them yet. The first milestone is a project that can compile the browser entry point and serve the SPA shell.

The generated tree starts with these landmarks:

```text
tutorial/
+-- Cargo.toml
+-- Makefile.toml
+-- build.rs
+-- index.html
+-- settings/
|   +-- base.toml
|   +-- local.toml
+-- src/
    +-- lib.rs
    +-- apps.rs
    +-- config.rs
    +-- shared.rs
    +-- client.rs
    +-- bin/
    |   +-- manage.rs
    +-- config/
    |   +-- settings.rs
    |   +-- urls.rs
    +-- client/
        +-- lib.rs
        +-- pages.rs
```

The reference example has more files because it is the completed project. You will add those files as each slice needs them.

## Read the Crate Targets

Open `Cargo.toml`. The pages example builds an `rlib` for the native server and a `cdylib` for the WASM client:

```toml
[lib]
crate-type = ["cdylib", "rlib"]  # cdylib for WASM, rlib for server
```

The management command is native-only, so the binary is gated behind the `with-reinhardt` feature:

```toml
[[bin]]
name = "manage"
path = "src/bin/manage.rs"
required-features = ["with-reinhardt"]
```

The dependency split is the important design. WASM gets pages and client routing; the server gets the full framework, database backends, forms, commands, and session auth:

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
reinhardt = { workspace = true, features = ["pages", "client-router"] }
wasm-bindgen = "0.2.106"

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
reinhardt = { workspace = true, features = [
    "full",
    "pages",
    "conf",
    "commands",
    "db-postgres",
    "db-sqlite",
    "forms",
    "client-router",
    "auth-session",
] }
tokio = { version = "1.48.0", features = ["full"] }
```

In an example project, import Reinhardt APIs through the `reinhardt` facade. Do not depend on internal `reinhardt-*` crates directly.

## Understand the Target Aliases

Open `build.rs`. The example declares readable cfg aliases that the rest of the project uses:

```rust
cfg_aliases! {
    client: { target_arch = "wasm32" },
    server: { not(target_arch = "wasm32") },
    wasm: { target_arch = "wasm32" },
    native: { not(target_arch = "wasm32") },
}
```

That gives you `#[cfg(client)]` for browser code and `#[cfg(server)]` for server-only code. The top-level modules follow that split:

```rust
pub mod apps;
pub mod config;

#[cfg(client)]
pub mod client;

pub mod shared;
```

`apps` and `shared` compile on both targets. Server-only internals are gated inside those modules when they need database, forms, or admin APIs.

## Inspect Settings

Open `src/config/settings.rs`. The reference example composes the core settings with the contacts fragment:

```rust
#[settings(core: CoreSettings | contacts: ContactSettings)]
pub struct ProjectSettings;
```

`get_settings()` loads defaults, low-priority environment variables, `settings/base.toml`, and the active profile file:

```rust
SettingsBuilder::new()
    .profile(Profile::parse(&profile_str))
    .add_source(DefaultSource::new().with_value(
        "core.base_dir",
        json::Value::String(base_dir.to_string_lossy().to_string()),
    ))
    .add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
    .add_source(TomlFileSource::new(settings_dir.join("base.toml")))
    .add_source(TomlFileSource::new(
        settings_dir.join(format!("{}.toml", profile_str)),
    ))
    .build_composed()
    .expect("Failed to build settings")
```

The matching `settings/base.toml` must include `[contacts]` because `ProjectSettings` includes `ContactSettings`:

```toml
[contacts]
admins = []
managers = []
```

The example's database settings target the disposable PostgreSQL container started by the `cargo make` tasks:

```toml
[core.databases.default]
engine = "postgresql"
host = "localhost"
port = 5432
name = "examples_tutorial_basis"
user = "reinhardt"
password = "reinhardt"
```

Use your own database name if you generated a new project. Keep the schema shape the same.

## See the Browser Mount Point

Open `index.html`. The pages template gives the WASM client a stable DOM mount point:

```html
<div id="root">
    <div class="flex items-center justify-center min-h-screen">
        <div class="text-center">
            <div class="spinner w-12 h-12 mx-auto mb-4"></div>
            <p class="text-muted">Loading...</p>
        </div>
    </div>
</div>
```

The WASM entry point in `src/client/lib.rs` mounts the client router there:

```rust
#[cfg_attr(not(feature = "msw"), wasm_bindgen(start))]
pub fn main() -> Result<(), JsValue> {
    ClientLauncher::new("#root")
        .register_routes_from_inventory()
        .launch()
}
```

Later parts will register routes from the `polls` and `users` apps. For now, confirm that the browser can load the client bundle and that the server is serving the pages application.

## Run the Development Workflow

Start the dev workflow:

```bash
cargo make dev
```

In the reference example, `dev` runs the WASM build, applies migrations, and starts the pages server. The underlying `runserver` command passes `--with-pages`:

```bash
cargo run --bin manage -- runserver --with-pages
```

Open `http://127.0.0.1:8000/`. At this point the application is only the shell. If the page loads without a missing-WASM error and the server logs show the pages runtime starting, the setup slice is complete.

## Checkpoint

Before continuing:

- `cargo make dev` starts the server.
- The browser reaches `http://127.0.0.1:8000/`.
- `settings/base.toml` contains `[core]`, `[core.databases.default]`, and `[contacts]`.
- Your project imports framework APIs from `reinhardt`, not internal sub-crates.

Next, you will add the first real feature: the poll index.
