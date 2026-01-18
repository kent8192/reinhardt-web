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

## Modules

This crate provides the following modules:

- **`` `settings` ``**: Core settings management functionality
- **`` `settings-cli` ``**: CLI tool for managing settings

## Installation

Add `reinhardt` to your `Cargo.toml`:

```toml
[dependencies]
reinhardt = { version = "0.1.0-alpha.1", features = ["conf"] }

# Or use a preset:
# reinhardt = { version = "0.1.0-alpha.1", features = ["standard"] }  # Recommended
# reinhardt = { version = "0.1.0-alpha.1", features = ["full"] }      # All features
```

Then import configuration features:

```rust
use reinhardt::conf::settings::{SettingsBuilder, Settings};
use reinhardt::conf::settings::sources::ConfigSource;
```

**Note:** Configuration features are included in the `standard` and `full` feature presets.

### Optional Features

Enable specific features based on your needs:

```toml
# With async support
reinhardt = { version = "0.1.0-alpha.1", features = ["conf", "conf-settings-async"] }

# With encryption
reinhardt = { version = "0.1.0-alpha.1", features = ["conf", "conf-settings-encryption"] }

# With Vault integration
reinhardt = { version = "0.1.0-alpha.1", features = ["conf", "conf-settings-vault"] }
```

Available features:

- `conf-settings` (default): Core settings functionality
- `conf-settings-async`: Asynchronous settings operations
- `conf-settings-dynamic-redis`: Redis-backed dynamic settings
- `conf-settings-dynamic-database`: Database-backed dynamic settings
- `conf-settings-vault`: HashiCorp Vault integration
- `conf-settings-aws-secrets`: AWS Secrets Manager integration
- `conf-settings-azure-keyvault`: Azure Key Vault integration
- `conf-settings-secret-rotation`: Automatic secret rotation
- `conf-settings-encryption`: Built-in encryption for sensitive settings

## Usage

```rust
use reinhardt::conf::settings::SettingsBuilder;
use reinhardt::conf::settings::sources::ConfigSource;

// Basic usage
let settings = SettingsBuilder::new()
    .add_source(ConfigSource::File("config.toml"))
    .add_source(ConfigSource::Environment)
    .build()?;

// Access settings
let database_url = settings.get::<String>("DATABASE_URL")?;
```

## CLI Tool

The `` `settings-cli` `` module provides a command-line tool for managing settings:

```bash
# Install the CLI tool
cargo install --path crates/settings-cli

# Use the tool
reinhardt-settings --help
```

## Module Organization

`` `reinhardt-conf` `` is organized into the following modules:

- `` `settings` `` - Core settings management (builder, validation, encryption)
- `` `settings-cli` `` - CLI tool for settings operations

### Using Modules

```rust
use reinhardt::conf::settings::{SettingsBuilder, SettingsConfig};
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../../LICENSE-MIT))

at your option.

## Contributing

Contributions are welcome! Please see the main [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.
