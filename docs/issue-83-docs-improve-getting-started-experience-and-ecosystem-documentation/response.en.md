# Response API Reference

Comprehensive API reference for building and manipulating HTTP responses.

## Table of Contents

- [Response](#response)
  - [Constructors](#constructors)
  - [Builder Methods](#builder-methods)
  - [Chain Control](#chain-control)
- [StreamingResponse](#streamingresponse)
- [Type Aliases](#type-aliases)

---

## Response

Struct representing an HTTP response.

```rust
use reinhardt_http::Response;

pub struct Response {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
    stop_chain: bool, // Internal use
}
```

### Constructors

#### `Response::new(status)`

Creates a new response with the specified status code.

```rust
use reinhardt_http::Response;
use hyper::StatusCode;

let response = Response::new(StatusCode::OK);
assert_eq!(response.status, StatusCode::OK);
assert!(response.body.is_empty());
```

#### `Response::ok()`

Creates a response with HTTP 200 OK status.

```rust
use reinhardt_http::Response;

let response = Response::ok();
assert_eq!(response.status, StatusCode::OK);
```

#### `Response::created()`

Creates a response with HTTP 201 Created status.

```rust
use reinhardt_http::Response;

let response = Response::created();
assert_eq!(response.status, StatusCode::CREATED);
```

#### `Response::no_content()`

Creates a response with HTTP 204 No Content status.

```rust
use reinhardt_http::Response;

let response = Response::no_content();
assert_eq!(response.status, StatusCode::NO_CONTENT);
```

#### `Response::bad_request()`

Creates a response with HTTP 400 Bad Request status.

```rust
use reinhardt_http::Response;

let response = Response::bad_request();
assert_eq!(response.status, StatusCode::BAD_REQUEST);
```

#### `Response::unauthorized()`

Creates a response with HTTP 401 Unauthorized status.

```rust
use reinhardt_http::Response;

let response = Response::unauthorized();
assert_eq!(response.status, StatusCode::UNAUTHORIZED);
```

#### `Response::forbidden()`

Creates a response with HTTP 403 Forbidden status.

```rust
use reinhardt_http::Response;

let response = Response::forbidden();
assert_eq!(response.status, StatusCode::FORBIDDEN);
```

#### `Response::not_found()`

Creates a response with HTTP 404 Not Found status.

```rust
use reinhardt_http::Response;

let response = Response::not_found();
assert_eq!(response.status, StatusCode::NOT_FOUND);
```

#### `Response::internal_server_error()`

Creates a response with HTTP 500 Internal Server Error status.

```rust
use reinhardt_http::Response;

let response = Response::internal_server_error();
assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
```

#### `Response::gone()`

Creates a response with HTTP 410 Gone status.

Used when a resource has been permanently removed.

```rust
use reinhardt_http::Response;

let response = Response::gone();
assert_eq!(response.status, StatusCode::GONE);
```

#### `Response::permanent_redirect(location)`

Creates a response with HTTP 301 Moved Permanently (permanent redirect).

```rust
use reinhardt_http::Response;

let response = Response::permanent_redirect("/new-location");
assert_eq!(response.status, StatusCode::MOVED_PERMANENTLY);
assert_eq!(
    response.headers.get("location").unwrap().to_str().unwrap(),
    "/new-location"
);
```

#### `Response::temporary_redirect(location)`

Creates a response with HTTP 302 Found (temporary redirect).

```rust
use reinhardt_http::Response;

let response = Response::temporary_redirect("/temp-location");
assert_eq!(response.status, StatusCode::FOUND);
assert_eq!(
    response.headers.get("location").unwrap().to_str().unwrap(),
    "/temp-location"
);
```

#### `Response::temporary_redirect_preserve_method(location)`

Creates a response with HTTP 307 Temporary Redirect (preserves HTTP method).

Unlike 302, this guarantees the request method is preserved during redirect.

```rust
use reinhardt_http::Response;

let response = Response::temporary_redirect_preserve_method("/temp-location");
assert_eq!(response.status, StatusCode::TEMPORARY_REDIRECT);
assert_eq!(
    response.headers.get("location").unwrap().to_str().unwrap(),
    "/temp-location"
);
```

### Builder Methods

#### `.with_body(body)`

Sets the response body.

```rust
use reinhardt_http::Response;
use bytes::Bytes;

let response = Response::ok().with_body("Hello, World!");
assert_eq!(response.body, Bytes::from("Hello, World!"));
```

#### `.with_header(name, value)`

Adds a custom header to the response.

**Panics** if the header name or value is invalid according to HTTP specifications.

```rust
use reinhardt_http::Response;

let response = Response::ok().with_header("X-Custom-Header", "custom-value");
assert_eq!(
    response.headers.get("X-Custom-Header").unwrap().to_str().unwrap(),
    "custom-value"
);
```

#### `.with_json(data)`

Sets the response body to JSON and adds Content-Type header.

```rust
use reinhardt_http::Response;
use serde_json::json;

let data = json!({"message": "Hello, World!"});
let response = Response::ok().with_json(&data).unwrap();

assert_eq!(
    response.headers.get("content-type").unwrap().to_str().unwrap(),
    "application/json"
);
```

#### `.with_location(location)`

Adds a Location header (typically used for redirects).

```rust
use reinhardt_http::Response;
use hyper::StatusCode;

let response = Response::new(StatusCode::FOUND).with_location("/redirect-target");
assert_eq!(
    response.headers.get("location").unwrap().to_str().unwrap(),
    "/redirect-target"
);
```

#### `.with_typed_header(name, value)`

Adds a custom header using typed `` `HeaderName` `` and `` `HeaderValue` ``.

```rust
use reinhardt_http::Response;
use hyper::header::{HeaderName, HeaderValue};

let header_name = HeaderName::from_static("x-custom-header");
let header_value = HeaderValue::from_static("custom-value");
let response = Response::ok().with_typed_header(header_name, header_value);

assert_eq!(
    response.headers.get("x-custom-header").unwrap().to_str().unwrap(),
    "custom-value"
);
```

### Chain Control

#### `.should_stop_chain()`

Checks if this response should stop the middleware chain.

```rust
use reinhardt_http::Response;

let response = Response::ok();
assert!(!response.should_stop_chain());

let stopping_response = Response::ok().with_stop_chain(true);
assert!(stopping_response.should_stop_chain());
```

#### `.with_stop_chain(stop)`

Sets whether this response should stop the middleware chain.

When set to `` `true` ``, the middleware chain will stop processing and return this response immediately, skipping any remaining middleware and handlers.

**Use cases**:
- Authentication failures (401 Unauthorized)
- CORS preflight responses (204 No Content)
- Rate limiting rejections (429 Too Many Requests)
- Cache hits (304 Not Modified)

```rust
use reinhardt_http::Response;
use hyper::StatusCode;

// Early return for authentication failure
let auth_failure = Response::unauthorized()
    .with_body("Authentication required")
    .with_stop_chain(true);
assert!(auth_failure.should_stop_chain());

// CORS preflight response
let preflight = Response::no_content()
    .with_header("Access-Control-Allow-Origin", "*")
    .with_stop_chain(true);
assert!(preflight.should_stop_chain());
```

---

## StreamingResponse

Struct representing a streaming HTTP response.

```rust
use reinhardt_http::StreamingResponse;

pub struct StreamingResponse<S> {
    pub status: StatusCode,
    pub headers: HeaderMap,
    stream: S,
}
```

### Constructors

#### `StreamingResponse::new(stream)`

Creates a new streaming response with OK status.

```rust
use reinhardt_http::StreamingResponse;
use futures::stream;
use bytes::Bytes;

let data = vec![Ok(Bytes::from("chunk1")), Ok(Bytes::from("chunk2"))];
let stream = stream::iter(data);
let response = StreamingResponse::new(stream);

assert_eq!(response.status, StatusCode::OK);
```

#### `StreamingResponse::with_status(stream, status)`

Creates a streaming response with a specific status code.

```rust
use reinhardt_http::StreamingResponse;
use hyper::StatusCode;
use futures::stream;
use bytes::Bytes;

let data = vec![Ok(Bytes::from("data"))];
let stream = stream::iter(data);
let response = StreamingResponse::with_status(stream, StatusCode::PARTIAL_CONTENT);

assert_eq!(response.status, StatusCode::PARTIAL_CONTENT);
```

### Builder Methods

#### `.status(status)`

Sets the status code.

```rust
use reinhardt_http::StreamingResponse;
use hyper::StatusCode;
use futures::stream;
use bytes::Bytes;

let data = vec![Ok(Bytes::from("data"))];
let stream = stream::iter(data);
let response = StreamingResponse::new(stream).status(StatusCode::ACCEPTED);

assert_eq!(response.status, StatusCode::ACCEPTED);
```

#### `.header(key, value)`

Adds a header to the streaming response.

```rust
use reinhardt_http::StreamingResponse;
use hyper::header::{CACHE_CONTROL, HeaderValue};
use futures::stream;
use bytes::Bytes;

let data = vec![Ok(Bytes::from("data"))];
let stream = stream::iter(data);
let response = StreamingResponse::new(stream)
    .header(CACHE_CONTROL, HeaderValue::from_static("no-cache"));

assert_eq!(
    response.headers.get(CACHE_CONTROL).unwrap().to_str().unwrap(),
    "no-cache"
);
```

#### `.media_type(type)`

Sets the Content-Type header (media type).

```rust
use reinhardt_http::StreamingResponse;
use hyper::header::CONTENT_TYPE;
use futures::stream;
use bytes::Bytes;

let data = vec![Ok(Bytes::from("data"))];
let stream = stream::iter(data);
let response = StreamingResponse::new(stream).media_type("video/mp4");

assert_eq!(
    response.headers.get(CONTENT_TYPE).unwrap().to_str().unwrap(),
    "video/mp4"
);
```

#### `.into_stream()`

Consumes the response and returns the underlying stream.

```rust
use reinhardt_http::StreamingResponse;
use futures::stream::{self, StreamExt};
use bytes::Bytes;

# futures::executor::block_on(async {
let data = vec![Ok(Bytes::from("chunk1")), Ok(Bytes::from("chunk2"))];
let stream = stream::iter(data);
let response = StreamingResponse::new(stream);

let mut extracted_stream = response.into_stream();
let first_chunk = extracted_stream.next().await.unwrap().unwrap();
assert_eq!(first_chunk, Bytes::from("chunk1"));
# });
```

---

## Type Aliases

#### `StreamBody`

Type alias for streaming body.

```rust
pub type StreamBody = Pin<Box<dyn Stream<Item = Result<Bytes, Box<dyn std::error::Error + Send + Sync>>> + Send>>;
```

---

## Conversion from Error

`` `Response` `` provides a `` `From<Error>` `` implementation:

```rust
impl From<crate::Error> for Response {
    fn from(error: crate::Error) -> Self {
        // Generates JSON response with error information
    }
}
```

This allows converting errors to responses:

```rust
use reinhardt_http::{Response, Error};

fn handle_error() -> Response {
    let error = Error::Http("Something went wrong".to_string());
    Response::from(error)
}
```

---

## See Also

- [Request API](./request.en.md)
- [Router API](./router.en.md)
- [Middleware Creation](../cookbook/middleware-creation.en.md)
- [Response Serialization](../cookbook/response-serialization.en.md)
