# reinhardt-params

FastAPI-inspired parameter extraction system for Reinhardt.

## Features

### Implemented ✓

#### Core Extraction System

- **`FromRequest` trait**: Core abstraction for asynchronous parameter extraction
- **`ParamContext`**: Management of path parameters and header/cookie names
- **Type-safe parameter extraction**: Extraction from requests with compile-time type checking
- **Error handling**: Detailed error messages via `ParamError`

#### Path Parameters (`path.rs`)

- **`Path<T>`**: Extract single value from URL path
  - Support for all primitive types: `i8`, `i16`, `i32`, `i64`, `i128`, `u8`, `u16`, `u32`, `u64`, `u128`, `f32`, `f64`, `bool`, `String`
  - Transparent access via `Deref`: `*path` or `path.0`
  - Value extraction via `into_inner()` method
- **`PathStruct<T>`**: Extract multiple path parameters into struct
  - Supports any struct implementing `DeserializeOwned`
  - Automatic type conversion using URL-encoded format (`"42"` → `42`)

#### Query Parameters (`query.rs`)

- **`Query<T>`**: Extract parameters from URL query string
  - Flexible deserialization using `serde`
  - Support for optional fields (`Option<T>`)
- **Multi-value query parameters** (`multi-value-arrays` feature):
  - `?q=5&q=6` → `Vec<i32>`
  - Automatic type conversion: string → numeric, boolean, etc.
  - JSON value-based deserialization

#### Headers (`header.rs`, `header_named.rs`)

- **`Header<T>`**: Extract value from request headers
  - Support for `String` and `Option<String>`
  - Runtime header name specification via `ParamContext`
- **`HeaderStruct<T>`**: Extract multiple headers into struct
  - Header name lowercase normalization
  - Automatic type conversion using URL-encoded
- **`HeaderNamed<N, T>`**: Compile-time header name specification
  - Type-safe header names via marker types: `Authorization`, `ContentType`
  - Support for `String` and `Option<String>`
  - Custom header name definition via `HeaderName` trait

#### Cookies (`cookie.rs`, `cookie_named.rs`)

- **`Cookie<T>`**: Extract value from cookies
  - Support for `String` and `Option<String>`
  - Runtime cookie name specification via `ParamContext`
- **`CookieStruct<T>`**: Extract multiple cookies into struct
  - RFC 6265-compliant cookie parsing
  - URL-decoding support
- **`CookieNamed<N, T>`**: Compile-time cookie name specification
  - Type-safe cookie names via marker types: `SessionId`, `CsrfToken`
  - Support for `String` and `Option<String>`
  - Custom cookie name definition via `CookieName` trait

#### Body Extraction (`body.rs`, `json.rs`, `form.rs`)

- **`Body`**: Extract raw request body as bytes
- **`Json<T>`**: JSON body deserialization
  - Type-safe deserialization using `serde_json`
  - Access via `Deref` and `into_inner()`
- **`Form<T>`**: Extract application/x-www-form-urlencoded form data
  - Content-Type validation
  - Deserialization using `serde_urlencoded`

#### Multipart Support (`multipart.rs`, requires `multipart` feature)

- **`Multipart`**: Multipart/form-data support
  - Streaming parsing using `multer` crate
  - File upload support
  - Iteration via `next_field()`

#### Validation Support (`validation.rs`, requires `validation` feature)

- **`Validated<T, V>`**: Validated parameter wrapper
- **`WithValidation` trait**: Fluent API for validation constraints
  - **Length constraints**: `min_length()`, `max_length()`
  - **Numeric ranges**: `min_value()`, `max_value()`
  - **Pattern matching**: `regex()`
  - **Format validation**: `email()`, `url()`
- **`ValidationConstraints<T>`**: Chainable validation builder
  - `validate_string()`: String value validation
  - `validate_number()`: Numeric validation
  - Support for combining multiple constraints
- **Type aliases**: `ValidatedPath<T>`, `ValidatedQuery<T>`, `ValidatedForm<T>`
- **Integration with `reinhardt-validators`**

## Quick Start

```rust
use reinhardt_params::{Path, Query, Json};
use serde::Deserialize;

#[derive(Deserialize)]
struct UserQuery {
    page: Option<i32>,
    per_page: Option<i32>,
}

#[endpoint(GET "/users/{id}")]
async fn get_user(
    id: Path<i64>,
    query: Query<UserQuery>,
    body: Json<UpdateUser>,
) -> Result<User> {
    // id.0 is the extracted i64
    // query.page is Option<i32>
    // body.0 is the deserialized UpdateUser
    Ok(User { id: id.0, ..body.0 })
}
```

## Parameter Types

## Path Parameters

```rust
// Single value
#[endpoint(GET "/users/{id}")]
async fn get_user(id: Path<i64>) -> String {
    format!("User ID: {}", id.0)
}

// Multiple values with struct
#[derive(Deserialize)]
struct UserPath {
    org: String,
    user_id: i64,
}

#[endpoint(GET "/orgs/{org}/users/{user_id}")]
async fn get_org_user(path: PathStruct<UserPath>) -> String {
    format!("Org: {}, User: {}", path.org, path.user_id)
}
```

## Query Parameters

```rust
#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    page: Option<i32>,
    tags: Vec<String>,  // ?tags=rust&tags=web → vec!["rust", "web"]
}
```

## Headers & Cookies

```rust
#[derive(Deserialize)]
struct CustomHeaders {
    #[serde(rename = "x-request-id")]
    x_request_id: String,

    #[serde(rename = "x-count")]
    count: i64,  // Auto type conversion: "123" → 123
}

#[endpoint(GET "/info")]
async fn info(headers: HeaderStruct<CustomHeaders>) -> String {
    format!("Request: {}", headers.x_request_id)
}
```

## Form & File Upload

```rust
// Form data
#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

#[endpoint(POST "/login")]
async fn login(form: Form<LoginForm>) -> String { /* ... */ }

// File upload (requires "multipart" feature)
#[endpoint(POST "/upload")]
async fn upload(mut multipart: Multipart) -> Result<()> {
    while let Some(field) = multipart.next_field().await? {
        let data = field.bytes().await?;
        // Process file...
    }
    Ok(())
}
```

## Feature Flags

```toml
[dependencies]
reinhardt-params = { version = "0.1", features = ["multipart", "validation"] }
```

- `multi-value-arrays` (default): Multi-value query parameters
- `multipart`: File upload support via multer
- `validation`: Integration with reinhardt-validators

## Testing Status

✅ **183 tests passing**

- Path parameters: 41 tests
- Query parameters: 51 tests (including multi-value)
- Headers: 29 tests (with type conversion)
- Cookies: 29 tests
- Form data: 29 tests
- JSON body: 26 tests
- Unit tests: 7 tests

Integration tests in `tests/`:

- OpenAPI schema generation (3 tests)
- Validation constraints (10 tests)

## Documentation

See [crate documentation](https://docs.rs/reinhardt-params) for detailed API reference and examples.

## License

Dual-licensed under MIT and Apache-2.0.
