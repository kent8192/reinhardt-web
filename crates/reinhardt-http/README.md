# reinhardt-http

HTTP request and response handling for the Reinhardt framework

## Overview

Core HTTP abstractions for the Reinhardt framework. Provides comprehensive request and response types, header handling, cookie management, content negotiation, and streaming support with a Django/DRF-inspired API design.

## Features

### Implemented ✓

#### Request Type

- **Complete HTTP request representation** with all standard components
  - HTTP method, URI, version, headers, body
  - Path parameters (`path_params`) and lazy query string parsing (`query_params`)
  - HTTPS detection (`is_secure`)
  - Remote address tracking (`remote_addr`)
  - Type-safe extensions system (`Extensions`)
- **Builder pattern** for fluent request construction
  - `Request::builder()` - Start building
  - `.method()` - Set HTTP method
  - `.uri()` - Set URI (with automatic query parsing)
  - `.version()` - Set HTTP version (defaults to HTTP/1.1)
  - `.headers()` - Set headers
  - `.header()` - Set single header
  - `.body()` - Set request body
  - `.secure()` - Set HTTPS flag
  - `.remote_addr()` - Set remote address
  - `.build()` - Finalize construction
- **Request parsing** (with `parsers` feature)
  - JSON body parsing
  - Form data parsing
  - Multipart form data
  - Lazy parsing (parse on first access)

#### Response Type

- **Flexible HTTP response creation** with status code helpers
  - `Response::ok()` - 200 OK
  - `Response::created()` - 201 Created
  - `Response::no_content()` - 204 No Content
  - `Response::bad_request()` - 400 Bad Request
  - `Response::unauthorized()` - 401 Unauthorized
  - `Response::forbidden()` - 403 Forbidden
  - `Response::not_found()` - 404 Not Found
  - `Response::gone()` - 410 Gone
  - `Response::internal_server_error()` - 500 Internal Server Error
- **Redirect responses**
  - `Response::permanent_redirect(url)` - 301 Moved Permanently
  - `Response::temporary_redirect(url)` - 302 Found
  - `Response::temporary_redirect_preserve_method(url)` - 307 Temporary Redirect
- **Builder pattern methods**
  - `.with_body(data)` - Set response body (bytes or string)
  - `.with_static_body(data)` - Set a static byte body without copying
  - `.with_header(name, value)` - Add single header
  - `.with_typed_header(header)` - Add typed header
  - `.with_json(data)` - Serialize data to JSON and set Content-Type
  - `.with_location(url)` - Set Location header (for redirects)
  - `.with_stop_chain(bool)` - Control middleware chain execution
- **JSON serialization support** with automatic Content-Type
- **Middleware chain control** via `stop_chain` flag

#### StreamingResponse

- **Streaming response support** for large data or real-time content
  - Custom media type configuration
  - Header support
  - Stream-based body (any type implementing `Stream`)

#### Extensions System

- **Type-safe request extensions** for storing arbitrary typed data
  - `request.extensions.insert::<T>(value)` - Store typed data
  - `request.extensions.get::<T>()` - Retrieve typed data
  - Thread-safe with lazily initialized `Arc<Mutex<TypeMap>>`
  - Common use cases: authentication context, request ID, user data

#### Error Integration

- Re-exports `reinhardt_core::exception::Error` and `Result` for consistent error handling

#### Handler Traits

- `Handler` - Async request handler trait for routes that await I/O
- `SyncHandler` - Synchronous fast path for routes that only inspect the request and build a response
- `SyncHandlerAdapter` - Compatibility adapter used when synchronous handlers pass through async middleware APIs

#### Exception Handling

- `ExceptionHandler` converts dispatch errors into application-defined responses and can be installed on a router, server, or `MiddlewareChain` with `with_exception_handler`
- The hook receives the request context and the framework error, covering handler, middleware, and routing failures that reach the configured chain
- A custom handler owns the response body and all security headers; the default conversion is safer because it omits internal error details and returns a JSON `SafeErrorResponse` with `Content-Type: application/json` (with safe detail only for applicable 4xx errors)

Do not interpolate an error's `Display` output into a public response unless the details have been reviewed for disclosure of internal paths, credentials, or other sensitive data.

## Installation

Add `reinhardt` to your `Cargo.toml`:

<!-- reinhardt-version-sync:3 -->
```toml
[dependencies]
reinhardt = "0.4.0-alpha.20"

# Or use a preset with parsers support:
# reinhardt = { version = "0.4.0-alpha.20", features = ["standard"] }  # Recommended
# reinhardt = { version = "0.4.0-alpha.20", features = ["full"] }      # All features
```

**Note:** HTTP types are available through the main `reinhardt` crate, which provides a unified interface to all framework components.

## Usage Examples

### Basic Request Construction

```rust
use reinhardt::http::Request;
use hyper::Method;
use bytes::Bytes;

// Using builder pattern
let request = Request::builder()
	.method(Method::POST)
	.uri("/api/users?page=1")
	.body(Bytes::from(r#"{"name": "Alice"}"#))
	.build()
	.unwrap();

assert_eq!(request.method, Method::POST);
assert_eq!(request.path(), "/api/users");
assert_eq!(request.query_params.get("page"), Some("1"));
```

### Path and Query Parameters

```rust
use reinhardt::http::Request;
use hyper::Method;

let mut request = Request::builder()
	.method(Method::GET)
	.uri("/api/users/123?sort=name&order=asc")
	.build()
	.unwrap();

// Access query parameters
assert_eq!(request.query_params.get("sort"), Some("name"));
assert_eq!(request.query_params.get("order"), Some("asc"));

// Add path parameters (typically done by router)
request.path_params.insert("id", "123");
assert_eq!(request.path_params.get("id"), Some("123"));
```

### Request Extensions

```rust
use reinhardt::http::Request;
use hyper::Method;

#[derive(Clone)]
struct UserId(i64);

let mut request = Request::builder()
	.method(Method::GET)
	.uri("/api/profile")
	.build()
	.unwrap();

// Store typed data in extensions
request.extensions.insert(UserId(42));

// Retrieve typed data
let user_id = request.extensions.get::<UserId>().unwrap();
assert_eq!(user_id.0, 42);
```

### Synchronous Handlers

```rust
use reinhardt::http::{Request, Response, Result, SyncHandler};

struct HealthHandler;

impl SyncHandler for HealthHandler {
    fn handle_sync(&self, _request: Request) -> Result<Response> {
        Ok(Response::ok().with_static_body(b"ok"))
    }
}
```

### Response Helpers

```rust
use reinhardt::http::Response;

// Success responses
let response = Response::ok()
    .with_body("Success");
assert_eq!(response.status, hyper::StatusCode::OK);

let response = Response::created()
    .with_json(&serde_json::json!({
        "id": 123,
        "name": "Alice"
    }))
    .unwrap();
assert_eq!(response.status, hyper::StatusCode::CREATED);
assert_eq!(
    response.headers.get("content-type").unwrap(),
    "application/json"
);

// Error responses
let response = Response::bad_request()
    .with_body("Invalid input");
assert_eq!(response.status, hyper::StatusCode::BAD_REQUEST);

let response = Response::not_found()
    .with_body("Resource not found");
assert_eq!(response.status, hyper::StatusCode::NOT_FOUND);
```

### Redirect Responses

```rust
use reinhardt::http::Response;

// Permanent redirect (301)
let response = Response::permanent_redirect("/new-location");
assert_eq!(response.status, hyper::StatusCode::MOVED_PERMANENTLY);
assert_eq!(
	response.headers.get("location").unwrap().to_str().unwrap(),
	"/new-location"
);

// Temporary redirect (302)
let response = Response::temporary_redirect("/login");
assert_eq!(response.status, hyper::StatusCode::FOUND);

// Temporary redirect preserving method (307)
let response = Response::temporary_redirect_preserve_method("/users/123");
assert_eq!(response.status, hyper::StatusCode::TEMPORARY_REDIRECT);
```

### JSON Response

```rust
use reinhardt::http::Response;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct User {
    id: i64,
    name: String,
}

let user = User {
    id: 1,
    name: "Alice".to_string(),
};

let response = Response::ok()
    .with_json(&user)
    .unwrap();

// Automatically sets Content-Type: application/json
assert_eq!(
    response.headers.get("content-type").unwrap(),
    "application/json"
);
```

### Middleware Chain Control

```rust
use reinhardt::http::Response;

// Stop middleware chain (useful for authentication, rate limiting)
let response = Response::unauthorized()
    .with_body("Authentication required")
    .with_stop_chain(true);

// This response will stop further middleware execution
assert!(response.should_stop_chain());
```

### Streaming Response

HTTP macro endpoints can return `ViewResult<StreamingResponse<StreamBody>>`
directly through `ServerRouter`, middleware, and native `manage runserver`:

```rust
use bytes::Bytes;
use reinhardt::{get, ServerRouter};
use reinhardt::http::{StreamingResponse, StreamBody, ViewResult};

#[get("/events", name = "events")]
async fn events() -> ViewResult<StreamingResponse<StreamBody>> {
	let stream: StreamBody = Box::pin(futures_util::stream::iter([
		Ok(Bytes::from_static(b"data: hello\n\n")),
	]));
	Ok(StreamingResponse::new(stream).media_type("text/event-stream"))
}

let router = ServerRouter::new().endpoint(events);
```

Use the `api-only` facade feature with `bytes = "1"` and
`futures-util = "0.3"`. Replace the finite example stream with a channel receiver
or another asynchronous producer for an unbounded event feed. Manual `Handler`
implementations return `Ok(streaming_response.into())`; alternatively construct
`Response::ok().with_stream(stream)`.

HTTP/1 and HTTP/2 send available chunks before the producer completes. The
transport polls on demand without collecting the stream or spawning a producer
task. A body error terminates the transfer rather than becoming successful EOF.
Disconnecting drops the stream; any background producer owned by the application
must observe receiver closure or use its own cancellation guard.

The response's buffered `body` field is empty for streams. Middleware must check
`Response::is_streaming()` before inspecting or transforming those bytes. Built-in
buffered compression, caching, and automatic ETag generation skip streaming
bodies. Application-supplied validators remain usable. Body replacement through
`with_body`, `with_static_body`, `with_json`, or `with_file_body` releases the old
stream. Stream construction removes `Content-Length` and `Transfer-Encoding` so
the transport chooses framing for the unknown body length.

Cloning `Response` shares a single-use producer, not replayable data. The first
transport takes ownership; sending a second clone returns a body error. Retaining
another clone does not keep a producer alive after its transport disconnects.

## API Reference

### Request

**Fields:**
- `method: Method` - HTTP method (GET, POST, etc.)
- `uri: Uri` - Request URI
- `version: Version` - HTTP version
- `headers: HeaderMap` - HTTP headers
- `path_params: PathParams` - Path parameters from URL routing
- `query_params: QueryParams` - Lazily parsed query string parameters
- `is_secure: bool` - Whether request is over HTTPS
- `remote_addr: Option<SocketAddr>` - Client's remote address
- `extensions: Extensions` - Type-safe extension storage

**Methods:**
- `Request::builder()` - Create builder
- `.path()` - Get URI path without query
- `.body()` - Get request body as `Option<&Bytes>`
- `.read_body()` - Read the body with consumption tracking
- `.json::<T>()` - Parse body as JSON (requires `parsers` feature)
- `.post()` - Parse POST data (form/JSON, requires `parsers` feature)
- `.data()` - Get parsed data from body
- `.set_di_context::<T>()` - Set DI context for type T
- `.get_di_context::<T>()` - Get DI context for type T
- `.decoded_query_params()` - Get URL-decoded query parameters
- `.get_accepted_languages()` - Parse Accept-Language header
- `.get_preferred_language()` - Get user's preferred language
- `.is_secure()` - Check if request is over HTTPS
- `.scheme()` - Get URI scheme
- `.build_absolute_uri()` - Build absolute URI from request

### Response

**Fields:**
- `status: StatusCode` - HTTP status code
- `headers: HeaderMap` - HTTP headers
- `body: Bytes` - Response body

**Constructor Methods:**
- `Response::new(status)` - Create with status code
- `Response::ok()` - 200 OK
- `Response::created()` - 201 Created
- `Response::no_content()` - 204 No Content
- `Response::bad_request()` - 400 Bad Request
- `Response::unauthorized()` - 401 Unauthorized
- `Response::forbidden()` - 403 Forbidden
- `Response::not_found()` - 404 Not Found
- `Response::gone()` - 410 Gone
- `Response::internal_server_error()` - 500 Internal Server Error
- `Response::permanent_redirect(url)` - 301 Moved Permanently
- `Response::temporary_redirect(url)` - 302 Found
- `Response::temporary_redirect_preserve_method(url)` - 307 Temporary Redirect

**Builder Methods:**
- `.with_body(data)` - Set body (bytes or string)
- `.with_header(name, value)` - Add header
- `.with_typed_header(header)` - Add typed header
- `.with_json(data)` - Serialize to JSON
- `.with_location(url)` - Set Location header
- `.with_stop_chain(bool)` - Control middleware chain
- `.should_stop_chain()` - Check if chain should stop

### Extensions

**Methods:**
- `.insert::<T>(value)` - Store typed value
- `.get::<T>()` - Retrieve typed value (returns `Option<T>`)
- `.remove::<T>()` - Remove typed value

## Feature Flags

- `parsers` - Enable request body parsing (JSON, form data, multipart)
  - Adds `parse_json()`, `parse_form()` methods to Request
  - Requires `reinhardt-core` crate (parsers module)

## Dependencies

- `hyper` - HTTP types (Method, Uri, StatusCode, HeaderMap, Version)
- `bytes` - Efficient byte buffer handling
- `futures` - Stream support for streaming responses
- `serde` - Serialization support (with `serde_json` for JSON)
- `reinhardt-core` - Core types, error handling, and optional request body parsing (parsers module enabled via the `parsers` feature)

## Testing

The crate includes comprehensive unit tests and doctests covering:
- Request construction and builder pattern
- Response helpers and status codes
- Redirect responses
- JSON serialization
- Extensions system
- Query parameter parsing
- Middleware chain control

Run tests with:
```bash
cargo test
cargo test --features parsers  # With parsers support
```

## License

Licensed under the BSD 3-Clause License.
