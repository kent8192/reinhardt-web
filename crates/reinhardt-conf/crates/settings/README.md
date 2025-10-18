# reinhardt-settings

Django-inspired settings management for Rust with advanced features like secrets management, encryption, audit logging, and dynamic configuration.

[![Crates.io](https://img.shields.io/crates/v/reinhardt-settings.svg)](https://crates.io/crates/reinhardt-settings)
[![Documentation](https://docs.rs/reinhardt-settings/badge.svg)](https://docs.rs/reinhardt-settings)
[![License](https://img.shields.io/crates/l/reinhardt-settings.svg)](LICENSE)

## Features Status

### Core Features (Implemented ✓)

- **📁 Hierarchical Configuration**: TOML, JSON, and .env file support with environment-specific overrides
  - ✓ TOML file source (`TomlFileSource`)
  - ✓ JSON file source (`JsonFileSource`)
  - ✓ .env file loader with interpolation support (`EnvLoader`, `DotEnvSource`)
  - ✓ Environment variable source (`EnvSource`)
  - ✓ Default values source (`DefaultSource`)
  - ✓ Auto-detection of configuration format by file extension
  - ✓ Priority-based configuration merging

- **🌍 Environment Profiles**: Built-in development, staging, and production profiles
  - ✓ Profile enum (Development, Staging, Production, Custom)
  - ✓ Environment detection from `REINHARDT_ENV`, `ENVIRONMENT`, `REINHARDT_SETTINGS_MODULE`
  - ✓ Profile-specific .env file loading (`.env.development`, `.env.production`, etc.)
  - ✓ Profile-aware default settings

- **✅ Validation**: Profile-specific security validation
  - ✓ Required field validation (`RequiredValidator`)
  - ✓ Security validation for production environments (`SecurityValidator`)
  - ✓ Range validation for numeric values (`RangeValidator`)
  - ✓ Pattern validation with regex (`PatternValidator`)
  - ✓ Choice validation for enum-like values (`ChoiceValidator`)
  - ✓ Integration with `reinhardt-validators` crate

- **🎯 Type-Safe**: Full Rust type safety with serde integration
  - ✓ Django-compatible `Settings` structure
  - ✓ Database configuration (SQLite, PostgreSQL, MySQL)
  - ✓ Template engine configuration
  - ✓ Middleware configuration
  - ✓ Serde serialization/deserialization support

### Advanced Features (Implemented ✓)

- **🔐 Secret Management**: Integrated support for HashiCorp Vault, AWS Secrets Manager, and Azure Key Vault
  - ✓ Secret types with automatic redaction (`SecretString`, `SecretValue`)
  - ✓ Constant-time equality for timing attack prevention
  - ✓ Zeroization on drop for memory security
  - ✓ Secret provider trait (`SecretProvider`)
  - ✓ Environment variable provider (`env::EnvSecretProvider`)
  - ✓ Memory provider for testing (`memory::MemorySecretProvider`)
  - ✓ HashiCorp Vault provider (feature: `vault`)
  - ✓ AWS Secrets Manager provider (feature: `aws-secrets`)
  - ✓ Azure Key Vault provider (feature: `azure-keyvault`)
  - ✓ Secret rotation support (feature: `secret-rotation`)
  - ✓ Audit logging for secret access

- **🔒 Encryption**: AES-256-GCM file encryption for sensitive configuration
  - ✓ Configuration encryptor (`ConfigEncryptor`)
  - ✓ Encrypted configuration structure (`EncryptedConfig`)
  - ✓ Key-based encryption/decryption (feature: `encryption`)

- **📝 Audit Logging**: Track all configuration changes for compliance
  - ✓ Audit event types (Read, Write, Delete, etc.)
  - ✓ Audit backend trait (`AuditBackend`)
  - ✓ File-based audit backend (`FileAuditBackend`)
  - ✓ Database audit backend (`DatabaseAuditBackend`)
  - ✓ Memory audit backend for testing (`MemoryAuditBackend`)
  - ✓ Separate audit logging for secrets

### Dynamic Features (Partial Implementation)

- **⚡ Dynamic Settings**: Runtime configuration changes with Redis and SQL backends
  - ✓ Backend trait definition (`DynamicBackend`)
  - ✓ Memory backend for testing (`MemoryBackend`)
  - ✓ Redis backend (feature: `dynamic-redis`)
  - ✓ Database backend (feature: `dynamic-database`)
  - ⚠️ Note: Core dynamic settings functionality is placeholder-based

- **🔄 Hot Reload**: Dynamic configuration updates without restarts (with async backends)
  - ⚠️ Planned for future implementation with async backend support

### Planned Features

- **🛠️ CLI Tools**: Command-line utilities for configuration management
  - Planned: Settings validation CLI
  - Planned: Configuration migration tools
  - Planned: Secret management CLI
  - Planned: Encryption key generation utilities

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]reinhardt-settings = "0.1.0"

# With all features
reinhardt-settings = { version = "0.1.0", features = ["full"] }

# With specific features
reinhardt-settings = { version = "0.1.0", features = ["async", "encryption", "vault"] }
```

## Feature Flags

### Core Features
- `async` - Async support (required for dynamic backends and secret management)

### Dynamic Settings Backends
- `dynamic-redis` - Redis backend for runtime configuration changes (requires `async`)
- `dynamic-database` - SQL backend with sqlx for dynamic settings (requires `async`)

### Secret Management Providers
- `vault` - HashiCorp Vault integration for secret storage (requires `async`)
- `aws-secrets` - AWS Secrets Manager integration (requires `async`)
- `azure-keyvault` - Azure Key Vault integration (requires `async`)
- `secret-rotation` - Automatic secret rotation capabilities (requires `async`)

### Security Features
- `encryption` - AES-256-GCM file encryption with PBKDF2 key derivation

### Example Combinations

```toml
# Full async features with all secret providers
reinhardt-settings = { version = "0.1.0", features = ["async", "vault", "aws-secrets", "azure-keyvault", "encryption"] }

# Dynamic settings with Redis
reinhardt-settings = { version = "0.1.0", features = ["dynamic-redis", "encryption"] }

# Minimal with encryption only
reinhardt-settings = { version = "0.1.0", features = ["encryption"] }
```

## Quick Start

### Basic Configuration

```rust
use reinhardt_settings::Settings;
use std::path::PathBuf;

fn main() {
    // Create basic settings
    let settings = Settings::new(
        PathBuf::from("/app"),
        "your-secret-key-here".to_string()
    )
    .with_root_urlconf("myapp.urls");

    println!("Debug mode: {}", settings.debug);
    println!("Database: {}", settings.databases.get("default").unwrap().name);
}
```

### Using Configuration Sources

```rust
use reinhardt_settings::sources::{TomlFileSource, EnvSource, ConfigSource};
use reinhardt_settings::profile::Profile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load from TOML file
    let toml_source = TomlFileSource::new("settings.toml");
    let toml_config = toml_source.load()?;

    // Load from environment variables with prefix
    let env_source = EnvSource::new().with_prefix("APP_");
    let env_config = env_source.load()?;

    // Configuration sources are merged by priority
    // EnvSource (priority 100) > TomlFileSource (priority 50)

    Ok(())
}
```

### Environment Profiles

```rust
use reinhardt_settings::profile::Profile;
use reinhardt_settings::sources::DotEnvSource;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Detect profile from environment
    let profile = Profile::from_env().unwrap_or(Profile::Development);

    // Load profile-specific .env file
    let env_source = DotEnvSource::new()
        .with_profile(profile)
        .with_interpolation(true);

    env_source.load()?;

    println!("Running in {} mode", profile);
    println!("Debug enabled: {}", profile.default_debug());

    Ok(())
}
```

### Validation

```rust
use reinhardt_settings::validation::{SecurityValidator, SettingsValidator};
use reinhardt_settings::profile::Profile;
use std::collections::HashMap;
use serde_json::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut settings = HashMap::new();
    settings.insert("debug".to_string(), Value::Bool(false));
    settings.insert("secret_key".to_string(), Value::String("a-very-long-and-secure-secret-key-here".to_string()));
    settings.insert("allowed_hosts".to_string(), Value::Array(vec![
        Value::String("example.com".to_string())
    ]));

    // Validate for production
    let validator = SecurityValidator::new(Profile::Production);
    validator.validate_settings(&settings)?;

    println!("Settings validated successfully!");

    Ok(())
}
```

## Advanced Usage

### Secret Management

With the `async` feature enabled, you can use secret providers:

```rust
use reinhardt_settings::secrets::{SecretString, SecretProvider};
use reinhardt_settings::secrets::providers::memory::MemorySecretProvider;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = MemorySecretProvider::new();

    // Store a secret
    let secret = SecretString::new("my-database-password");
    provider.set_secret("db_password", secret).await?;

    // Retrieve the secret
    let retrieved = provider.get_secret("db_password").await?;

    // Secret is automatically redacted in logs
    println!("Secret: {}", retrieved); // Prints: [REDACTED]

    // Access the actual value when needed
    println!("Actual: {}", retrieved.expose_secret());

    Ok(())
}
```

### Configuration Encryption

With the `encryption` feature:

```rust
use reinhardt_settings::encryption::ConfigEncryptor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = vec![0u8; 32]; // Use a secure key in production
    let encryptor = ConfigEncryptor::new(key)?;

    // Encrypt configuration data
    let data = b"secret configuration";
    let encrypted = encryptor.encrypt(data)?;

    // Decrypt when needed
    let decrypted = encryptor.decrypt(&encrypted)?;

    assert_eq!(data, decrypted.as_slice());

    Ok(())
}
```

### Audit Logging

Track configuration changes for compliance:

```rust
use reinhardt_settings::audit::backends::memory::MemoryAuditBackend;
use reinhardt_settings::audit::{AuditEvent, AuditBackend};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = MemoryAuditBackend::new();

    // Log a configuration change
    let event = AuditEvent::write("database.host", "localhost", "db.example.com");
    backend.log(event).await?;

    // Query audit logs
    let logs = backend.query_all().await?;

    for log in logs {
        println!("Event: {:?}", log);
    }

    Ok(())
}
```

## Documentation

For complete documentation, examples, and API reference, visit:

- [API Documentation](https://docs.rs/reinhardt-settings)
- [Examples Directory](examples/)
  - [Basic Usage](examples/settings_basic_usage.rs)
  - [Environment Profiles](examples/environment_profiles.rs)
  - [Secret Management](examples/secret_management.rs)
  - [Encryption](examples/encryption.rs)
  - [Audit Logging](examples/audit_logging.rs)
  - [Dynamic Settings with Redis](examples/dynamic_settings_redis.rs)
  - [Dynamic Settings with Database](examples/dynamic_settings_database.rs)

## Module Structure

The crate is organized into the following modules:

- **Core Modules**
  - `config` - Configuration trait definitions
  - `env` - Environment variable utilities
  - `env_loader` - .env file loading with interpolation
  - `env_parser` - Environment variable parsing
  - `profile` - Environment profile management (Development, Staging, Production)
  - `sources` - Configuration source implementations (TOML, JSON, .env, environment)
  - `validation` - Configuration validation framework
  - `builder` - Fluent builder for settings construction
  - `prelude` - Common imports for convenience
  - `testing` - Testing utilities

- **Advanced Modules** (feature-gated)
  - `advanced` - Advanced settings structures (cache, CORS, email, logging, media, session, static)
  - `encryption` - AES-256-GCM configuration encryption (feature: `encryption`)
  - `audit` - Audit logging for configuration changes (feature: `async`)
    - `backends` - File, database, and memory audit backends
  - `secrets` - Secret management system (feature: `async`)
    - `types` - SecretString, SecretValue with automatic redaction
    - `providers` - HashiCorp Vault, AWS Secrets Manager, Azure Key Vault, environment, memory
    - `rotation` - Automatic secret rotation (feature: `secret-rotation`)
    - `audit` - Audit logging for secret access
  - `backends` - Dynamic settings backends (feature: `async`)
    - `memory` - In-memory backend for testing
    - `redis_backend` - Redis backend (feature: `dynamic-redis`)
    - `database` - SQL backend (feature: `dynamic-database`)
  - `dynamic` - Runtime configuration changes (feature: `async`)

## Testing

Run tests:

```bash
# Unit tests
cargo test --package reinhardt-settings

# With specific features
cargo test --package reinhardt-settings --features encryption
cargo test --package reinhardt-settings --features async

# With all features
cargo test --package reinhardt-settings --all-features

# Integration tests (requires Docker for Redis/Database backends)
cargo test --package reinhardt-settings --test integration_test --features encryption
cargo test --package reinhardt-settings --test integration_test --all-features
```

### Test Coverage

- **Core functionality**: 40+ unit tests
- **Secret management**: 20+ tests including constant-time equality and zeroization
- **Validation**: 10+ tests for security, range, pattern, and choice validators
- **Configuration sources**: 15+ tests for TOML, JSON, .env, and environment sources
- **Profile management**: 10+ tests for environment detection and profile behavior
- **Integration tests**: Encryption, Redis backend, Database backend

## Architecture Highlights

### Security-First Design

- **Secret Protection**: `SecretString` and `SecretValue` types prevent accidental exposure in logs
- **Constant-Time Comparison**: Timing attack prevention for secret equality checks
- **Memory Zeroization**: Automatic cleanup of sensitive data using `zeroize` crate
- **Production Validation**: Automatic security checks for production environments

### Flexible Configuration

- **Priority-Based Merging**: Environment variables (100) > .env files (90) > config files (50) > defaults (0)
- **Multiple Sources**: TOML, JSON, .env files, and environment variables
- **Profile-Aware**: Development, staging, and production environments with different defaults
- **Type-Safe**: Full Rust type safety with serde integration

### Extensible Backend System

- **Pluggable Providers**: Easy to add new secret providers or configuration backends
- **Async-Ready**: Full async/await support for I/O operations
- **Audit Trail**: Complete logging of configuration and secret access
- **Testing Support**: Memory-based backends for easy testing

## Performance Considerations

- **Zero-Cost Abstractions**: No runtime overhead for type safety
- **Lazy Loading**: Configuration sources are only loaded when needed
- **Efficient Merging**: IndexMap-based merging maintains insertion order
- **Minimal Allocations**: Careful use of string allocation and cloning

## Contributing

We welcome contributions! Areas that could use help:

- CLI tools for settings management
- Additional secret provider implementations
- Hot reload implementation for dynamic settings
- Performance optimizations
- Documentation improvements

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

Inspired by:
- [Django Settings](https://docs.djangoproject.com/en/stable/ref/settings/)
- [django-environ](https://django-environ.readthedocs.io/)
- [config-rs](https://github.com/mehcode/config-rs)
