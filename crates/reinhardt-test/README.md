# reinhardt-test

Testing utilities and test client for the Reinhardt framework

## Overview

Comprehensive testing utilities inspired by Django REST Framework's test utilities. This crate provides reusable testing tools including APIClient for making test requests, test case base classes, database fixtures, mock utilities, and TestContainers integration for infrastructure testing.

Supports both unit testing and integration testing with real or test databases.

## Async Stub Routes

`stub::StubRouter` provides native async closures for fake upstreams and webhook
receivers. When depending directly on `reinhardt-test`, use its own exports:

```rust
use reinhardt_test::{http::Response, stub::StubRouter};

let router = StubRouter::new()
    .post("/webhook", |request| async move {
        Ok(Response::ok().with_body(request.body().clone()))
    })
    .get("/health", |_request| async { Ok(Response::ok()) })
    .into_server_router();
```

With the top-level `reinhardt` crate's `test` feature, the same example uses:

```rust
use reinhardt::test::{http::Response, stub::StubRouter};
```

In an async test, start the server with
`reinhardt_test::fixtures::test_server_guard(router).await`, or
`reinhardt::test::fixtures::test_server_guard(router).await` through the
top-level facade. Keep the returned guard alive while sending requests;
dropping it shuts down the server. GET, POST, PUT, PATCH, and DELETE have
helpers; use `.route(path, method, handler)` for other methods. Method
mismatches return 405, unknown paths return 404, and duplicate `(path, method)`
registrations panic immediately.

`SimpleHandler` is deprecated starting in `0.4.0-alpha.21` and remains available
throughout `0.4.x`, with removal planned for `0.5.0`. Replace
`SimpleHandler::new(|request| { ... })` and manual
method checks with `StubRouter::new().post(path, |request| async move { ... })`
(or the matching HTTP method), then convert it into a `ServerRouter`. The
`reinhardt_test::stub` and `reinhardt::test::stub` paths work without adding
`reinhardt-testkit` as a direct dependency. For application endpoints with
extractors or named routes, use HTTP method macros and `ServerRouter::endpoint`.

## Test Databases

`reinhardt-test` re-exports `reinhardt-testkit`'s model-derived database
fixture:

```rust
use reinhardt_test::fixtures::{TestDatabase, test_database};
```

Use `test_database!(ModelA, ModelB)` for schemas derived from `Model`
metadata, or `TestDatabase::builder().migrations::<AppMigrations>()` when a
test should run against application migrations.

Derived schemas include model field defaults and generated columns,
relationships, constraints, and indexes.

## Installation

Add `reinhardt` to your `Cargo.toml`:

<!-- reinhardt-version-sync:3 -->
```toml
[dependencies]
reinhardt = { version = "0.4.0-alpha.20", features = ["test"] }

# Or use a preset:
# reinhardt = { version = "0.4.0-alpha.20", features = ["standard"] }  # Recommended
# reinhardt = { version = "0.4.0-alpha.20", features = ["full"] }      # All features
```

Then import testing features:

```rust
use reinhardt::test::{APIClient, APITestCase, TestResponse};
use reinhardt::test::{FixtureLoader, Factory, MockFunction, load_model_fixture_file};
```

**Note:** Testing features are included in the `standard` and `full` feature presets.

## Features

### Implemented ✓

#### API Testing Client

- **APIClient**: HTTP test client with authentication support
  - HTTP methods: GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS
  - Authentication: Force authentication, Basic auth, login/logout
  - Request customization: Headers, cookies, base URL configuration
  - Flexible serialization: JSON and form-encoded data support
- **APIRequestFactory**: Factory for creating test requests
  - Request builders for all HTTP methods
  - JSON and form data serialization
  - Header and query parameter management
  - Force authentication support

#### Response Testing

- **TestResponse**: Response wrapper with assertion helpers
  - Status code assertions: `assert_ok()`, `assert_created()`, `assert_not_found()`, etc.
  - Status range checks: `assert_success()`, `assert_client_error()`, `assert_server_error()`
  - Body parsing: JSON deserialization, text extraction
  - Header access and content type checking

#### Test Case Base Classes

- **APITestCase**: Base test case with common setup/teardown
  - Pre-configured APIClient instance
  - Setup and teardown lifecycle hooks
  - Optional TestContainers database integration
- **Test macros**: Convenience macros for defining test cases
  - `test_case!`: Standard test case definition
  - `authenticated_test_case!`: Pre-authenticated test cases
  - `test_case_with_db!`: Database-backed test cases (requires `testcontainers` feature)

#### Fixtures and Factories

- **FixtureLoader**: JSON-based test data loader
  - Load fixtures from JSON strings
  - Type-safe deserialization
  - Fixture existence checking and listing
- **load_model_fixture_file**: Load Django-compatible model fixture files
  into the active test database
- **Factory trait**: Test data generation
  - `Factory<T>` trait for creating test objects
  - `FactoryBuilder`: Simple factory implementation
  - Batch data generation support

#### Mock and Spy Utilities

- **MockFunction**: Function call tracking with configurable return values
  - Return value queuing and default values
  - Call count and argument tracking
  - Conditional assertions: `was_called()`, `was_called_with()`
- **Spy**: Method call tracking with optional wrapped objects
  - Call recording with timestamps
  - Argument verification
  - Reset and inspection capabilities

#### MSW-Style Request Mocking

- **WASM runtime**: `MockServiceWorker` overrides `window.fetch()` for browser
  tests.
- **Native runtime**: `MockServiceWorker` starts a loopback HTTP mock server
  and exposes `worker.url()` for explicit endpoint injection.
- **Shared API**: `rest::get(...)`, `MockResponse`, request recording, and
  `calls_to(...)` work across both runtimes.

Native MSW does not intercept arbitrary `reqwest`, `hyper`, AWS SDK, or
OS-level traffic. The code under test must use the mock server URL.

#### Message Testing (Django-style)

- **Message assertions**: Test message framework integration
  - `assert_message_count()`: Verify message count
  - `assert_message_exists()`: Check for specific messages
  - `assert_message_level()`: Verify message levels
  - `assert_message_tags()`: Check message tags
  - `assert_messages()`: Ordered and unordered message verification
- **MessagesTestMixin**: Test mixin for message testing utilities
  - Stack trace filtering for cleaner test output
  - Tag-based message assertions

#### JSON Assertions

- **JSON field assertions**:
  - `assert_json_field_eq()`: Field value equality
  - `assert_json_has_field()`: Field presence
  - `assert_json_missing_field()`: Field absence
- **JSON array assertions**:
  - `assert_json_array_len()`: Array length verification
  - `assert_json_array_empty()` / `assert_json_array_not_empty()`: Empty state checks
  - `assert_json_array_contains()`: Element presence
- **JSON pattern matching**:
  - `assert_json_matches()`: Subset matching for complex structures

#### HTTP Assertions

- **Status code assertions**:
  - `assert_status_eq()`: Exact status code matching
  - `assert_status_success()`: 2xx range verification
  - `assert_status_client_error()`: 4xx range verification
  - `assert_status_server_error()`: 5xx range verification
  - `assert_status_redirect()`: 3xx range verification
  - `assert_status_error()`: 4xx or 5xx verification
- **Content assertions**:
  - `assert_contains()`: Text substring presence
  - `assert_not_contains()`: Text substring absence

#### Debug Toolbar

- **DebugToolbar**: Request/response debugging utilities
  - Timing information tracking (total time, SQL time, cache hits/misses)
  - SQL query recording with duration and stack traces
  - Custom debug panels with various entry types (key-value, table, code, text)
  - HTML rendering for debug output
  - Enable/disable debugging support

#### TestContainers Integration (optional, requires `testcontainers` feature)

- **Database containers**:
  - `PostgresContainer`: PostgreSQL test container with custom credentials
  - `MySqlContainer`: MySQL test container with custom credentials
  - `RedisContainer`: Redis test container
- **TestDatabase trait**: Common interface for database containers
  - Connection URL generation
  - Database type identification
  - Readiness checking
- **Helper functions**:
  - `with_postgres()`: Run tests with PostgreSQL container
  - `with_mysql()`: Run tests with MySQL container
  - `with_redis()`: Run tests with Redis container
