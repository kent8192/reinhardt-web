# {{ project_name }}

A Reinhardt Pages project with WASM frontend and server-side rendering.

## Prerequisites

- Rust 1.94.1 or later (2024 Edition)
- cargo-make: `cargo install cargo-make`
- wasm-pack: `cargo install wasm-pack`

## Getting Started

### Install Tools

```bash
# Install all development tools (includes WASM build tools)
cargo make install-tools
```

### Development Server

```bash
# Build WASM and start development server
cargo make dev

# Or step by step:
cargo make wasm-build-dev    # Build WASM
cargo make runserver         # Start server
```

Visit `http://127.0.0.1:8000/` in your browser.

### Build for Production

```bash
# Build WASM (release + optimized)
cargo make wasm-build-release

# Build server
cargo build --release
```

## Project Structure

```
{{ project_name }}/
├── src/
│   ├── client/       # WASM shell (entry point and shared layout)
│   │   ├── lib.rs
│   │   └── components.rs
│   ├── apps/         # App-local models, server functions, routes, and pages
│   ├── shared/       # Shared types and server form metadata
│   │   ├── types.rs
│   │   └── forms.rs
│   └── config/       # Server configuration
├── dist/             # WASM build output
├── dist-wasm/        # wasm-pack output before collectstatic
├── index.html        # WASM entry HTML
└── Cargo.toml
```

## Management Commands

```bash
# Create a new app
cargo run --bin manage startapp myapp --with-pages

# Database migrations (when using database features)
cargo run --bin manage makemigrations
cargo run --bin manage migrate

# Check project for issues
cargo run --bin manage check
```

## WASM Build Commands

```bash
cargo make wasm-build-dev      # Build WASM (debug)
cargo make wasm-build-release  # Build WASM (release + optimize)
cargo make wasm-watch          # Watch and rebuild on changes
cargo make wasm-clean          # Clean WASM build artifacts
```

## Learn More

- [Reinhardt Documentation](https://github.com/kent8192/reinhardt-rs)
- [Reinhardt Pages Guide](https://github.com/kent8192/reinhardt-rs/tree/main/docs)
- [wasm-pack Documentation](https://rustwasm.github.io/wasm-pack/)
