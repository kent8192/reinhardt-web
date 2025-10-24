# reinhardt-macros

Procedural macros for the framework

## Overview

Procedural macros for reducing boilerplate code. Includes derive macros for models, serializers, and forms, as well as attribute macros for endpoints and middleware.

Provides compile-time code generation for common patterns.

## Features

### Implemented ✓

#### Function-based API Views

- **`#[api_view]`** - Convert function to API view
  - Supports multiple HTTP methods via `methods` parameter
  - Validates HTTP methods at compile time (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS)
  - Defaults to GET if no methods specified
  - Example: `#[api_view(methods = "GET,POST")]`

#### ViewSet Custom Actions

- **`#[action]`** - Define custom ViewSet actions
  - Supports HTTP method specification via `methods` parameter
  - Supports detail/list action via `detail` parameter (required)
  - Optional `url_path` and `url_name` parameters
  - Validates HTTP methods at compile time
  - Example: `#[action(methods = "POST", detail = true)]`

#### HTTP Method Decorators

- **`#[get]`** - GET method decorator with path validation
- **`#[post]`** - POST method decorator with path validation
- **`#[put]`** - PUT method decorator with path validation
- **`#[patch]`** - PATCH method decorator with path validation
- **`#[delete]`** - DELETE method decorator with path validation
- All support compile-time URL pattern validation
- Example: `#[get("/users/{id}")]`

#### Permission System

- **`#[permission_required]`** - Permission decorator
  - Validates permission strings at compile time
  - Supports Django-style permission format: `"app.permission"`
  - Uses nom parser for validation
  - Example: `#[permission_required("users.view_user")]`

#### Dependency Injection (FastAPI-style)

- **`#[use_injection]`** / **`#[endpoint]`** - Automatic dependency injection
  - FastAPI-style parameter attributes with `#[inject]`
  - Automatic resolution from `InjectionContext`
  - Cache control with `#[inject(cache = false)]`
  - Works with any function, not just endpoints
  - Example: `#[use_injection] async fn handler(#[inject] db: Database)`

#### Configuration Macros

- **`installed_apps!`** - Define installed applications
  - Compile-time validation of application paths
  - Type-safe enum generation for all installed apps
  - Validates `reinhardt.contrib.*` modules exist
  - Generates `Display` and `FromStr` implementations
  - Example: `installed_apps! { auth: "reinhardt.contrib.auth", }`

#### URL Pattern Validation

- **`path!`** - Validate URL patterns at compile time
  - Uses nom parser for pattern validation
  - Supports simple parameters: `{id}`
  - Supports Django-style typed parameters: `{<int:id>}`
  - Validates parameter names and type specifiers
  - Supported types: `int`, `str`, `uuid`, `slug`, `path`
  - Detailed error messages with position information
  - Example: `path!("users/{<int:user_id>}/posts/{post_id}/")`

#### Signal System

- **`#[receiver]`** - Connect receiver function to signal
  - Django-style `@receiver` decorator functionality
  - Supports signal and sender parameters
  - Marker macro for signal registration
  - Example: `#[receiver(signal = post_save::<User>())]`

#### Type-safe Query Fields

- **`#[derive(QueryFields)]`** - Generate field accessor methods
  - Automatic field accessor generation for models
  - Compile-time validated field lookups
  - Type-specific lookup methods based on field type
  - String fields: `lower()`, `upper()`, `trim()`, `contains()`
  - Numeric fields: `abs()`, `ceil()`, `floor()`, `round()`
  - DateTime fields: `year()`, `month()`, `day()`, `hour()`
  - All fields: `eq()`, `ne()`, `gt()`, `gte()`, `lt()`, `lte()`
  - Example: `QuerySet::<User>::new().filter(User::email().lower().contains("example.com"))`

### Planned

Currently all planned features are implemented.
