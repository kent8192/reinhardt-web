# reinhardt-apps

Django-inspired application configuration and registry system for Reinhardt framework.

## Overview

`reinhardt-apps` provides the core infrastructure for managing Django-style applications in a Reinhardt project. It implements both runtime string-based and compile-time type-safe application registry mechanisms, along with comprehensive re-exports of essential framework components.

This crate serves as a high-level integration point, bringing together HTTP handling, settings management, error handling, and server functionality in a unified API.

## Features

### Implemented ✓

#### Application Registry System

- **AppConfig**: Application configuration with validation
  - Name and label management
  - Verbose name support
  - Default auto field configuration
  - Path management
  - Label validation (Rust identifier rules)
- **Apps Registry**: Global application registry
  - Application registration and lookup
  - Installed apps tracking
  - Duplicate detection (labels and names)
  - Readiness state management (apps_ready, models_ready, ready)
  - Cache clearing for testing
- **Global Registry Functions**:
  - `get_apps()`: Access global singleton registry
  - `init_apps()`: Initialize with string list
  - `init_apps_checked()`: Initialize with compile-time validated list

#### Type-Safe Application Registry

- **AppLabel Trait**: Compile-time application identity
  - Const label definition
  - Type-safe application references
- **Type-Safe Methods**:
  - `get_app_config_typed::<A>()`: Type-safe configuration lookup
  - `is_installed_typed::<A>()`: Type-safe installation check
- **Benefits**: Compile-time verification of application names

#### HTTP Request/Response

- **Request**: Comprehensive HTTP request handling
  - Query parameter parsing and access
  - Path parameter storage
  - JSON deserialization
  - HTTP method support (GET, POST, PUT, DELETE, PATCH)
  - HTTP version tracking (HTTP/1.1, HTTP/2)
  - Header management
  - Body handling
  - Path extraction
- **Response**: Builder pattern for HTTP responses
  - Status code helpers (ok, created, no_content, bad_request, unauthorized, forbidden, not_found, internal_server_error)
  - JSON serialization (`with_json`)
  - Body setting (`with_body`)
  - Header management (`with_header`)
  - Method chaining support
- **Status Code Utilities**: Category checking functions
  - Informational (1xx)
  - Success (2xx)
  - Redirect (3xx)
  - Client Error (4xx)
  - Server Error (5xx)

#### Internationalization (i18n) Support

- **Accept-Language Header Parsing**:
  - Quality value (q-value) parsing
  - Multiple language support
  - Language code validation (BCP 47 compliant)
  - Quality-based sorting
  - Maximum length validation (255 characters)
  - Invalid code filtering (wildcards, malformed codes)
- **Language Cookie Handling**:
  - Cookie-based language extraction
  - Language code validation
  - Custom cookie name support
- **Helper Methods**:
  - `get_accepted_languages()`: Parse and sort Accept-Language header
  - `get_preferred_language()`: Get highest quality language
  - `get_language_from_cookie()`: Extract language from cookies

#### Settings Management

- **Settings Struct**: Complete Django-like configuration
  - Base directory and secret key
  - Debug mode
  - Allowed hosts
  - Installed apps list
  - Middleware configuration
  - Database configuration (multiple databases support)
  - Template configuration
  - Static files (URL and root)
  - Media files (URL and root)
  - Internationalization settings (language_code, time_zone, use_i18n, use_tz)
  - Default auto field
  - Root URLconf
- **Database Configurations**:
  - SQLite support (`DatabaseConfig::sqlite`)
  - PostgreSQL support (`DatabaseConfig::postgresql`)
  - MySQL support (`DatabaseConfig::mysql`)
  - Custom database engines
- **Template Configuration**:
  - Backend selection
  - Template directories
  - App directories support
  - Context processors
  - Options management
- **Middleware Configuration**:
  - Path-based middleware specification
  - Custom options per middleware
  - Default middleware stack
- **Builder Pattern**:
  - `with_validated_apps()`: Add apps with compile-time validation
  - `with_root_urlconf()`: Set URL configuration
  - `add_installed_app()`: Add single app
  - `add_middleware()`: Add single middleware

#### Error Handling

- **Error Types**:
  - Http (400)
  - Database (500)
  - Serialization (400)
  - Validation (400)
  - Authentication (401)
  - Authorization (403)
  - NotFound (404)
  - Internal (500)
  - Other (anyhow integration, 500)
- **Error Conversion**:
  - `From<anyhow::Error>` implementation
  - `Into<Response>` implementation with JSON body
- **Status Code Mapping**: Automatic HTTP status code assignment
- **Display Formatting**: User-friendly error messages
- **Result Type**: Framework-wide `Result<T>` alias

#### Re-exports

- **HTTP**: `Request`, `Response`, `StreamBody`, `StreamingResponse` (from reinhardt-http)
- **Settings**: `Settings`, `DatabaseConfig`, `MiddlewareConfig`, `TemplateConfig` (from reinhardt-settings)
- **Errors**: `Error`, `Result` (from reinhardt-exception)
- **Server**: `serve`, `HttpServer` (from reinhardt-server)
- **Types**: `Handler`, `Middleware`, `MiddlewareChain` (from reinhardt-types)
- **Apps**: `AppConfig`, `AppError`, `AppResult`, `Apps`, `get_apps`, `init_apps`, `init_apps_checked`
- **Builder**: `Application`, `ApplicationBuilder`, `ApplicationDatabaseConfig`, `BuildError`, `BuildResult`, `RouteConfig`

#### Application Builder System

- **ApplicationBuilder**: Fluent builder pattern for application configuration
  - Add applications with `add_app()` and `add_apps()`
  - Add middleware with `add_middleware()` and `add_middlewares()`
  - Add URL patterns with `add_url_pattern()` and `add_url_patterns()`
  - Database configuration support
  - Custom settings management
  - Configuration validation (duplicate checks)
  - Method chaining support
- **RouteConfig**: Route definition with metadata
  - Path and handler name configuration
  - Optional route naming
  - Optional namespace support
  - Full name generation (namespace:name)
- **ApplicationDatabaseConfig**: Database configuration
  - URL-based configuration
  - Connection pool size settings
  - Maximum overflow connections
  - Connection timeout settings
- **Application**: Built application with full configuration access
  - Access to registered apps, middleware, URL patterns
  - Database configuration retrieval
  - Custom settings access
  - Apps registry integration
  - Readiness state verification

#### Application Registry Enhancements

- Model discovery and registration
- Reverse relation building
- Ready hooks (AppConfig.ready())
- Signal integration for app lifecycle events
- Migration detection

#### Advanced Settings Features

- Environment variable integration
- Settings validation
- Settings inheritance (base settings + environment-specific)
- Secure settings (secret key generation, sensitive data handling)
- Settings freezing (immutable after initialization)

#### Enhanced Error Handling

- Error code system
- Localized error messages
- Error details and context
- Structured logging integration
- Error aggregation

#### Request Enhancements

- Form data parsing
- Multipart file upload support
- URL decoding for query parameters
- Request context management
- Session integration
- Authentication/user integration
- CSRF token handling

#### Response Enhancements

- Response compression (gzip, brotli)
- Streaming response helpers
- Redirect helpers (permanent, temporary)
- Content negotiation
- ETag support
- Cache control headers

#### Testing Utilities

- Test client
- Mock request/response builders
- Application registry fixtures
- Database test isolation

## Usage

### Application Registry

```rust
use reinhardt_apps::{AppConfig, Apps, get_apps, init_apps_checked};

// Define applications
let app1 = AppConfig::new("myapp", "myapp")
    .with_verbose_name("My Application")
    .with_default_auto_field("BigAutoField");

// Register applications
let apps = Apps::new(vec!["myapp".to_string()]);
apps.register(app1)?;

// Check installation
assert!(apps.is_installed("myapp"));

// Get configuration
let config = apps.get_app_config("myapp")?;

// Initialize global registry
init_apps_checked(|| vec!["myapp".to_string()])?;
let global_apps = get_apps();
```

### Type-Safe Application Registry

```rust
use reinhardt_apps::{Apps, AppLabel};

// Define type-safe application
struct AuthApp;
impl AppLabel for AuthApp {
    const LABEL: &'static str = "auth";
}

let apps = Apps::new(vec!["auth".to_string()]);

// Type-safe checks (compile-time verified)
assert!(apps.is_installed_typed::<AuthApp>());
let config = apps.get_app_config_typed::<AuthApp>()?;
```

### Request Handling

```rust
use reinhardt_apps::Request;
use hyper::{Method, Uri, HeaderMap, Version};
use bytes::Bytes;

// Create request
let request = Request::new(
    Method::GET,
    Uri::from_static("/api/users?page=1"),
    Version::HTTP_11,
    HeaderMap::new(),
    Bytes::new(),
);

// Access query parameters
assert_eq!(request.query_params.get("page"), Some(&"1".to_string()));

// Parse JSON body
#[derive(Deserialize)]
struct User { name: String }
let user: User = request.json()?;
```

### Response Building

```rust
use reinhardt_apps::Response;
use serde_json::json;

// Simple response
let response = Response::ok().with_body("Hello, world!");

// JSON response
let data = json!({"message": "Success"});
let response = Response::ok().with_json(&data)?;

// Custom headers
let response = Response::created()
    .with_json(&data)?
    .with_header(
        hyper::header::LOCATION,
        hyper::header::HeaderValue::from_static("/api/users/1")
    );
```

### Internationalization

```rust
use reinhardt_apps::Request;

// Parse Accept-Language header
let languages = request.get_accepted_languages();
for (lang, quality) in languages {
    println!("{}: {}", lang, quality);
}

// Get preferred language
if let Some(lang) = request.get_preferred_language() {
    println!("User prefers: {}", lang);
}

// Get language from cookie
if let Some(lang) = request.get_language_from_cookie("django_language") {
    println!("Cookie language: {}", lang);
}
```

### Settings Management

```rust
use reinhardt_apps::{Settings, DatabaseConfig, TemplateConfig};
use std::path::PathBuf;

// Create settings
let settings = Settings::new(
    PathBuf::from("/project"),
    "secret-key".to_string()
)
.with_validated_apps(|| vec!["myapp".to_string()])
.with_root_urlconf("config.urls");

// Database configuration
let db = DatabaseConfig::postgresql(
    "mydb",
    "user",
    "password",
    "localhost",
    5432
);

// Template configuration
let template = TemplateConfig::default()
    .add_dir("/templates")
    .add_dir("/other_templates");
```

### Error Handling

```rust
use reinhardt_apps::{Error, Result, Response};

fn handle_request() -> Result<Response> {
    // Return specific error types
    if !authenticated {
        return Err(Error::Authentication("Invalid token".into()));
    }

    if !authorized {
        return Err(Error::Authorization("Permission denied".into()));
    }

    // Automatic conversion to HTTP response
    Ok(Response::ok().with_body("Success"))
}

// Errors automatically convert to appropriate HTTP responses
let response: Response = handle_request()
    .unwrap_or_else(|err| err.into());
```

### Application Builder

```rust
use reinhardt_apps::{
    ApplicationBuilder, ApplicationDatabaseConfig, AppConfig, RouteConfig
};

// Build a complete application
let app = ApplicationBuilder::new()
    // Add applications
    .add_app(AppConfig::new("myapp", "myapp").with_verbose_name("My Application"))
    .add_app(AppConfig::new("auth", "auth"))

    // Add middleware stack
    .add_middleware("CorsMiddleware")
    .add_middleware("AuthMiddleware")

    // Configure routes
    .add_url_pattern(
        RouteConfig::new("/api/users/", "UserListHandler")
            .with_namespace("api")
            .with_name("user-list")
    )
    .add_url_pattern(
        RouteConfig::new("/api/posts/", "PostListHandler")
            .with_namespace("api")
            .with_name("post-list")
    )

    // Configure database
    .database(
        ApplicationDatabaseConfig::new("postgresql://localhost/mydb")
            .with_pool_size(10)
            .with_max_overflow(5)
            .with_timeout(30)
    )

    // Add custom settings
    .add_setting("DEBUG", "true")
    .add_setting("SECRET_KEY", "super-secret")

    // Build the application
    .build()
    .expect("Failed to build application");

// Access configuration
assert!(app.apps_registry().is_installed("myapp"));
assert_eq!(app.middleware().len(), 2);
assert_eq!(app.url_patterns().len(), 2);
assert!(app.database_config().is_some());
assert_eq!(app.settings().get("DEBUG"), Some(&"true".to_string()));
```

## Integration with Other Crates

This crate integrates the following Reinhardt components:

- `reinhardt-http`: HTTP request/response abstractions
- `reinhardt-settings`: Configuration management
- `reinhardt-exception`: Error types and handling
- `reinhardt-server`: HTTP server implementation
- `reinhardt-types`: Core traits and type definitions

## Testing

The crate includes comprehensive test coverage:

- Unit tests in `src/apps.rs` (application registry)
- Integration tests in `tests/` directory:
  - `installed_apps_integration.rs`: Registry integration
  - `test_settings.rs`: Settings configuration
  - `test_request.rs`: Request handling
  - `test_response.rs`: Response building and status codes
  - `test_error.rs`: Error handling and conversion
  - `i18n_http_tests.rs`: Internationalization features

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
