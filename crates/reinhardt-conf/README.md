# Reinhardt Configuration Framework

Django-inspired settings management for Rust with secrets, encryption, and audit logging.

## Overview

The `reinhardt-conf` crate provides a comprehensive configuration management framework for Reinhardt applications, inspired by Django's settings system with additional security features.

## Features

- **Multiple configuration sources**: Files, environment variables, command-line arguments
- **Type-safe settings**: Strong type validation with custom validators
- **Secrets management**: Integration with HashiCorp Vault, AWS Secrets Manager, Azure Key Vault
- **Encryption**: Built-in encryption for sensitive settings
- **Dynamic backends**: Redis and database-backed dynamic settings
- **Secret rotation**: Automatic secret rotation support
- **Audit logging**: Track all setting changes

## Sub-crates

This crate is organized as a parent crate containing the following sub-crates:

- **`settings`** (`reinhardt-settings`): Core settings management functionality
- **`settings-cli`** (`reinhardt-settings-cli`): CLI tool for managing settings

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
reinhardt-conf = "0.1.0"
```

### Optional Features

Enable specific features based on your needs:

```toml
[dependencies]
reinhardt-conf = { version = "0.1.0", features = ["async", "encryption"] }
```

Available features:

- `settings` (default): Core settings functionality
- `async`: Asynchronous settings operations
- `dynamic-redis`: Redis-backed dynamic settings
- `dynamic-database`: Database-backed dynamic settings
- `vault`: HashiCorp Vault integration
- `aws-secrets`: AWS Secrets Manager integration
- `azure-keyvault`: Azure Key Vault integration
- `secret-rotation`: Automatic secret rotation
- `encryption`: Built-in encryption for sensitive settings

## Usage

```rust
use reinhardt_conf::SettingsBuilder;

// Basic usage
let settings = SettingsBuilder::new()
    .add_source(ConfigSource::File("config.toml"))
    .add_source(ConfigSource::Environment)
    .build()?;

// Access settings
let database_url = settings.get::<String>("DATABASE_URL")?;
```

## CLI Tool

The `settings-cli` sub-crate provides a command-line tool for managing settings:

```bash
# Install the CLI tool
cargo install --path crates/settings-cli

# Use the tool
reinhardt-settings --help
```

## Architecture

This parent crate re-exports functionality from its sub-crates:

```
reinhardt-conf/
├── Cargo.toml          # Parent crate definition
├── src/
│   └── lib.rs          # Re-exports from sub-crates
└── crates/
    ├── settings/       # Core settings functionality
    └── settings-cli/   # CLI tool
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../../LICENSE-MIT))

at your option.

## Contributing

Contributions are welcome! Please see the main [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.
