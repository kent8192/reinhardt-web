# REST API Example

This example demonstrates how to build a RESTful API with the Reinhardt framework.

## Features

- **Django-style project structure**: Uses config/, settings/, apps.rs
- **Environment-specific configuration**: Separated settings for local, staging, production
- **manage CLI**: Django-style management commands (`cargo run --bin manage`)
- **URL routing**: Simple API endpoint definitions

## Project Structure

```
src/
├── config/
│   ├── apps.rs              # Installed apps definition
│   ├── settings.rs          # Environment-based settings loader
│   ├── settings/
│   │   ├── base.rs          # Common settings for all environments
│   │   ├── local.rs         # Local development settings
│   │   ├── staging.rs       # Staging environment settings
│   │   └── production.rs    # Production environment settings
│   └── urls.rs              # URL routing configuration
├── apps.rs                  # App registry
├── config.rs                # config module declaration
├── main.rs                  # Application entry point
└── bin/
    └── manage.rs            # Management CLI tool
```

## Setup

### Prerequisites

- Rust 2024 edition or later
- Cargo

### Build

```bash
# From project root
cargo build --package example-rest-api
```

**Note**: This example will be buildable after reinhardt is published to crates.io (version ^0.1).

## Usage

### Starting Development Server

```bash
# Default (127.0.0.1:8000)
cargo run --bin manage runserver

# Custom address
cargo run --bin manage runserver 0.0.0.0:3000
```

### Management Commands

```bash
# Create database migrations
cargo run --bin manage makemigrations

# Apply migrations
cargo run --bin manage migrate

# Launch interactive shell
cargo run --bin manage shell

# Check project
cargo run --bin manage check

# Collect static files
cargo run --bin manage collectstatic

# Show URL list
cargo run --bin manage showurls
```

## API Endpoints

### GET /api/users

Sample endpoint returning user list

**Response example:**
```json
[
  {
    "id": 1,
    "name": "Alice",
    "email": "alice@example.com"
  },
  {
    "id": 2,
    "name": "Bob",
    "email": "bob@example.com"
  }
]
```

## Environment Configuration

Switch settings using the `REINHARDT_ENV` environment variable:

```bash
# Local development (default)
export REINHARDT_ENV=local
cargo run --bin manage runserver

# Staging
export REINHARDT_ENV=staging
export SECRET_KEY=your_secret_key
export ALLOWED_HOSTS=stage.example.com
cargo run --bin manage runserver

# Production
export REINHARDT_ENV=production
export SECRET_KEY=your_secret_key
export ALLOWED_HOSTS=example.com,www.example.com
cargo run --bin manage runserver
```

## Customization

### Adding New Endpoints

1. Define handler function in `src/config/urls.rs`
2. Register in router via `url_patterns()`

```rust
async fn new_endpoint() -> Json<MyData> {
    Json(MyData { /* ... */ })
}

pub fn url_patterns() -> Arc<UnifiedRouter> {
    let router = UnifiedRouter::builder().build();
    router.add_function_route("/api/new", Method::GET, new_endpoint);
    Arc::new(router)
}
```

### Adding Apps

1. Add to `installed_apps!` macro in `src/config/apps.rs`
2. Add necessary configuration in `src/config/settings/base.rs`

## References

- [Reinhardt Documentation](https://docs.rs/reinhardt)
- [Django Settings Best Practices](https://docs.djangoproject.com/en/stable/topics/settings/)
- [12 Factor App](https://12factor.net/)

## License

This example is provided as part of the Reinhardt project under MIT/Apache-2.0 license.
