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

Use Rust 1.96.0 or newer. The `0.3.0-rc.4` generator and the generated Rust
2024 project require that toolchain level.

Install the Reinhardt project generator:

<!-- reinhardt-version-sync -->
```bash
cargo install reinhardt-admin-cli --version "0.2.3"
```

The installed binary is `reinhardt-admin`.

The pages template also needs the WASM target, `cargo-make`, and `wasm-pack`:

```bash
rustup target add wasm32-unknown-unknown
cargo install cargo-make wasm-pack
```

## Create a Pages Project

Create a new project from the pages template. `startproject` can prompt for
feature flags interactively, but this tutorial uses a deterministic one-liner so
your `Cargo.toml` matches the reference project:

```bash
reinhardt-admin startproject tutorial --template pages \
  --features minimal,pages,admin,conf,commands-server,commands-autoreload,db-sqlite,forms,auth-session,middleware,argon2-hasher,static-files \
  --default-features false \
  --no-interactive
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
    |   +-- wasm.rs
    +-- shared/
    |   +-- forms.rs
    |   +-- types.rs
    +-- client/
        +-- lib.rs
        +-- components.rs
```

The reference example has more files because it is the completed project. You will add those files as each slice needs them.

## Read the Crate Targets

Open `Cargo.toml`. The pages example builds an `rlib` for the native server and a `cdylib` for the WASM client:

```toml
[lib]
crate-type = ["cdylib", "rlib"]  # cdylib for WASM, rlib for server
```

In the reference example, the management command is native-only, so the binary
is gated behind the `with-reinhardt` feature:

```toml
[[bin]]
name = "manage"
path = "src/bin/manage.rs"
required-features = ["with-reinhardt"]
```

That gate is required for the example's explicit native test target: `wasm-pack
test` builds Cargo test targets for `wasm32-unknown-unknown`, and
`required-features = ["with-reinhardt"]` keeps the native-only integration test
and management binary out of that build when the WASM task uses
`--no-default-features`.

The dependency split is the important design. WASM gets pages and client
routing; the server gets only the framework features this tutorial uses:

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
reinhardt = { workspace = true, features = ["pages", "client-router"] }
wasm-bindgen = "=0.2.122"

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
reinhardt = { workspace = true, features = [
    "minimal",
    "pages",
    "conf",
    "commands-server",
    "commands-autoreload",
    "db-sqlite",
    "forms",
    "auth-session",
    "middleware",
    "argon2-hasher",
    "admin",
    "static-files",
] }
tokio = { version = "1.48.0", features = ["full"] }
```

The `full` feature is intentionally absent. `tokio` still uses its own `full`
runtime feature; that is unrelated to Reinhardt's `full` preset.

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

`get_settings()` loads defaults, `settings/base.toml`, the active profile file, and high-priority environment variables:

```rust
SettingsBuilder::new()
    .profile(profile)
    .add_source(DefaultSource::new())
    .add_source(TomlFileSource::new(settings_dir.join("base.toml")))
    .add_source(TomlFileSource::new(
        settings_dir.join(format!("{}.toml", profile_str)),
    ))
    .add_source(HighPriorityEnvSource::new().with_prefix("REINHARDT_"))
    .build_composed::<ProjectSettings>()
    .expect("Failed to build/compose settings")
```

The matching `settings/base.toml` must include `[contacts]` because `ProjectSettings` includes `ContactSettings`:

```toml
[core]
secret_key = "insecure-..."

[contacts]
admins = []
managers = []
```

The tutorial database is a local SQLite file:

```toml
[core.databases.default]
engine = "sqlite"
name = "db.sqlite3"
```

`cargo make migrate` and `cargo make dev` resolve that setting through
`scripts/db_url.sh` and create the file on demand. No PostgreSQL or Redis
container is required for this tutorial path.

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

`cargo make wasm-build-dev` runs `wasm-pack` against the library target and copies the generated browser bundle from `dist-wasm/` into `dist/`. It does not require a separate `wasm-bindgen-cli` install and does not bind the native `manage` binary.

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
