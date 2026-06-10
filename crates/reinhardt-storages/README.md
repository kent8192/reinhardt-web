# reinhardt-storages

Cloud storage backend abstraction for the Reinhardt framework, inspired by
[django-storages](https://django-storages.readthedocs.io/).

## Features

- **Unified API**: single `StorageBackend` trait for all providers
- **Settings-first configuration**: `StorageSettings` composes with the
  Reinhardt `#[settings]` macro
- **Async I/O**: all storage operations are asynchronous
- **Feature flags**: enable only the providers your application uses
- **Temporary URLs**: presigned URLs for S3, V4 signed URLs for GCS, and SAS
  URLs for Azure Blob Storage
- **Backends**:
  - Amazon S3
  - Google Cloud Storage
  - Azure Blob Storage
  - Local file system

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
reinhardt-storages = "0.2.0"
```

### Feature Flags

By default, `reinhardt-storages` enables the `s3` and `local` backends.

```toml
[dependencies]
# Only local storage
reinhardt-storages = { version = "0.2.0", default-features = false, features = ["local"] }

# S3 only
reinhardt-storages = { version = "0.2.0", default-features = false, features = ["s3"] }

# All available backends
reinhardt-storages = { version = "0.2.0", features = ["all"] }
```

Available features:

- `default`: `["s3", "local"]`
- `s3`: Amazon S3 support
- `gcs`: Google Cloud Storage support
- `azure`: Azure Blob Storage support
- `local`: local file system support
- `all`: all backends

## Usage

### Settings-First Example

```rust
use reinhardt_storages::{StorageSettings, create_storage_from_settings};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings: StorageSettings = toml::from_str(r#"
backend = "local"

[local]
base_path = "media"
"#)?;

    let storage = create_storage_from_settings(&settings).await?;

    storage.save("example.txt", b"Hello, world!").await?;
    let content = storage.open("example.txt").await?;
    println!("File content: {}", String::from_utf8_lossy(&content));

    if storage.exists("example.txt").await? {
        let size = storage.size("example.txt").await?;
        println!("File size: {size} bytes");

        let url = storage.url("example.txt", 3600).await?;
        println!("Temporary URL: {url}");

        storage.delete("example.txt").await?;
    }

    Ok(())
}
```

### Composed Application Settings

`StorageSettings` is a fragment for the `[storage]` section. Applications can
compose it with other settings fragments through `#[settings]`.

```rust
use reinhardt_core::macros::settings;
use reinhardt_storages::StorageSettings;

#[settings(storage: StorageSettings)]
pub struct AppSettings {}
```

Example TOML for Google Cloud Storage:

```toml
[storage]
backend = "gcs"

[storage.gcs]
bucket = "my-bucket"
prefix = "uploads/"
service_account_json = { secret = "{\"client_email\":\"storage@example.com\"}" }
```

Example TOML for Azure Blob Storage:

```toml
[storage]
backend = "azure"

[storage.azure]
account = "myaccount"
container = "media"
prefix = "uploads/"
access_key = { secret = "base64-account-key" }
```

Example TOML for local storage:

```toml
[storage]
backend = "local"

[storage.local]
base_path = "media"
```

## Backend Settings

### S3

```toml
[storage]
backend = "s3"

[storage.s3]
bucket = "my-bucket"
region = "us-east-1"
endpoint = "http://localhost:4566"
prefix = "uploads/"
```

`endpoint` and `prefix` are optional. Without `endpoint`, the AWS SDK default
endpoint and credential chain are used.

### Google Cloud Storage

```toml
[storage]
backend = "gcs"

[storage.gcs]
bucket = "my-bucket"
prefix = "uploads/"
service_account_json = { secret = "{\"type\":\"service_account\"}" }
```

`endpoint` is available for emulators such as fake-gcs-server. Without
`endpoint`, the Google Cloud SDK client is used. `service_account_json` is
optional and can provide explicit credentials and local signing material for
V4 signed URLs; otherwise Application Default Credentials are used.

### Azure Blob Storage

```toml
[storage]
backend = "azure"

[storage.azure]
account = "myaccount"
container = "media"
prefix = "uploads/"
access_key = { secret = "base64-account-key" }
```

`endpoint` is available for Azurite or custom endpoints. Temporary URLs use a
configured `sas_token` when present, otherwise `access_key` or
`connection_string` is used to generate a service SAS URL.

### Local

```toml
[storage]
backend = "local"

[storage.local]
base_path = "/var/storage"
```

## Compatibility API

`StorageConfig`, `S3Config`, `GcsConfig`, `AzureConfig`, `LocalConfig`, and
`StorageConfig::from_env()` are deprecated in favor of `StorageSettings`.
They remain available during the compatibility window so existing applications
can migrate incrementally.

```rust
use reinhardt_storages::{StorageSettings, create_storage_from_settings};

async fn build_storage(settings: &StorageSettings) -> reinhardt_storages::Result<()> {
    let storage = create_storage_from_settings(settings).await?;
    storage.save("example.txt", b"content").await?;
    Ok(())
}
```

## API Reference

All storage backends implement `StorageBackend`:

```rust
#[async_trait::async_trait]
pub trait StorageBackend: Send + Sync {
    async fn save(&self, name: &str, content: &[u8]) -> Result<String>;
    async fn open(&self, name: &str) -> Result<Vec<u8>>;
    async fn delete(&self, name: &str) -> Result<()>;
    async fn exists(&self, name: &str) -> Result<bool>;
    async fn url(&self, name: &str, expiry_secs: u64) -> Result<String>;
    async fn size(&self, name: &str) -> Result<u64>;
    async fn get_modified_time(&self, name: &str) -> Result<DateTime<Utc>>;
}
```

## Testing

Run tests with:

```bash
# All storage tests
cargo test -p reinhardt-storages --all-features

# GCS emulator tests with fake-gcs-server
cargo test -p reinhardt-storages --features gcs,local --test gcs_tests -- --nocapture

# Azure emulator tests with Azurite
cargo test -p reinhardt-storages --features azure,local --test azure_tests -- --nocapture
```

GCS and Azure emulator tests use TestContainers and require Docker.

## License

MIT OR Apache-2.0
