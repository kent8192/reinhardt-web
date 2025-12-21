# {{ project_name }}

A Reinhardt Pages project with WASM frontend and server-side rendering.

## Prerequisites

- Rust 1.91.1 or later (2024 Edition)
- Trunk (WASM build tool): `cargo install trunk`
- PostgreSQL (optional, for database features)

## Getting Started

### Development Server

```bash
# Build and serve WASM client with hot reload
trunk serve
```

Visit `http://127.0.0.1:8080/` in your browser.

### Build for Production

```bash
# Build WASM client
trunk build --release

# Build server
cargo build --release
```

## Project Structure

```
{{ project_name }}/
├── src/
│   ├── client/       # WASM UI (runs in browser)
│   │   ├── lib.rs     # WASM entry point
│   │   ├── router.rs  # Client-side routing
│   │   └── state.rs   # Global state management
│   ├── server/       # Server functions (runs on server)
│   │   └── server_fn.rs
│   ├── shared/       # Shared types (used by both)
│   │   ├── types.rs
│   │   └── errors.rs
│   └── config/       # Server configuration
├── index.html        # WASM entry HTML
├── Trunk.toml        # WASM build configuration
└── Cargo.toml
```

## Management Commands

```bash
# Create a new app
cargo run --bin {{ project_name }} startapp myapp --with-pages

# Database migrations (when using database features)
cargo run --bin {{ project_name }} makemigrations
cargo run --bin {{ project_name }} migrate

# Check project for issues
cargo run --bin {{ project_name }} check
```

## Learn More

- [Reinhardt Documentation](https://github.com/kent8192/reinhardt-rs)
- [Reinhardt Pages Guide](https://github.com/kent8192/reinhardt-rs/tree/main/docs)
- [Trunk Documentation](https://trunkrs.dev/)
