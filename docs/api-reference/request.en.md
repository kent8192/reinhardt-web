# Request API Reference

Comprehensive API reference for representing and manipulating HTTP requests.

## Table of Contents

- [Request](#request)
  - [Fields](#fields)
  - [Constructors](#constructors)
  - [Header Operations](#header-operations)
  - [Parameter Extraction](#parameter-extraction)
  - [Validation](#validation)
  - [DI Context](#di-context)
- [RequestBuilder](#requestbuilder)

---

## Request

Struct representing an HTTP request.

```rust
use reinhardt_http::Request;

pub struct Request {
    pub method: Method,
    pub uri: Uri,
    pub version: Version,
    pub headers: HeaderMap,
    pub path_params: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub is_secure: bool,
    pub remote_addr: Option<SocketAddr>,
    pub extensions: Extensions,
    // Internal fields omitted
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `` `method` `` | `` `Method` `` | HTTP method (GET, POST, etc.) |
| `` `uri` `` | `` `Uri` `` | Request URI |
| `` `version` `` | `` `Version` `` | HTTP version |
| `` `headers` `` | `` `HeaderMap` `` | Request headers |
| `` `path_params` `` | `` `HashMap<String, String>` `` | Path parameters (e.g., `` `id` `` in `` `/users/{id}` ``) |
| `` `query_params` `` | `` `HashMap<String, String>` `` | Query parameters (e.g., `` `page` `` in `` `?page=1` ``) |
| `` `is_secure` `` | `` `bool` `` | Whether the connection is HTTPS |
| `` `remote_addr` `` | `` `Option<SocketAddr>` `` | Client socket address |
| `` `extensions` `` | `` `Extensions` `` | For storing arbitrary typed data |

### Constructors

#### `Request::builder()`

Creates a new `` `RequestBuilder` ``.

```rust
use reinhardt_http::Request;
use hyper::Method;

let request = Request::builder()
    .method(Method::GET)
    .uri("/api/users")
    .build()
    .unwrap();

assert_eq!(request.method, Method::GET);
```

### Header Operations

#### `.get_header(name)`

Gets the value of a header by name.

Returns `` `None` `` if the header doesn't exist or cannot be converted to a string.

```rust
use reinhardt_http::Request;
use hyper::header;

let request = Request::builder()
    .header(header::USER_AGENT, "Mozilla/5.0")
    .build()
    .unwrap();

let user_agent = request.get_header("user-agent");
assert_eq!(user_agent, Some("Mozilla/5.0".to_string()));
```

#### `.extract_bearer_token()`

Extracts Bearer token from Authorization header.

Useful for extracting JWT or other bearer tokens.

```rust
use reinhardt_http::Request;
use hyper::header;

let request = Request::builder()
    .header(header::AUTHORIZATION, "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9")
    .build()
    .unwrap();

let token = request.extract_bearer_token();
assert_eq!(token, Some("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9".to_string()));
```

### Parameter Extraction

#### `.get_client_ip()`

Extracts client IP address from request headers or `` `remote_addr` ``.

**Search order**:
1. `` `X-Forwarded-For` `` header (for proxy environments)
2. `` `X-Real-IP` `` header
3. `` `remote_addr` `` field

```rust
use reinhardt_http::Request;

let request = Request::builder()
    .header("X-Forwarded-For", "203.0.113.1, 198.51.100.1")
    .build()
    .unwrap();

let ip = request.get_client_ip();
assert_eq!(ip, Some("203.0.113.1".parse().unwrap()));
```

#### `.query_as<T>()`

Deserializes query parameters into a typed struct.

Returns an error if types don't match.

```rust
use reinhardt_http::Request;
use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq)]
struct Pagination {
    page: u32,
    limit: u32,
}

let request = Request::builder()
    .uri("/api/users?page=2&limit=10")
    .build()
    .unwrap();

let params: Pagination = request.query_as().unwrap();
assert_eq!(params, Pagination { page: 2, limit: 10 });
```

### Validation

#### `.validate_content_type(expected)`

Validates that Content-Type header matches the expected value.

Returns an error if the header is missing or doesn't match.

```rust
use reinhardt_http::Request;
use hyper::header;

let request = Request::builder()
    .header(header::CONTENT_TYPE, "application/json")
    .build()
    .unwrap();

assert!(request.validate_content_type("application/json").is_ok());
assert!(request.validate_content_type("text/html").is_err());
```

### DI Context

#### `.set_di_context(ctx)`

Sets the DI context for this request.

Allows handlers to access dependencies when using DI with routers.

```rust
use reinhardt_http::Request;

# struct DummyDiContext;
let mut request = Request::builder()
    .uri("/")
    .build()
    .unwrap();

let di_ctx = DummyDiContext;
request.set_di_context(di_ctx);
```

#### `.get_di_context<T>()`

Gets the DI context from this request.

Returns `` `None` `` if no DI context was set.

```rust
use reinhardt_http::Request;

# struct DummyDiContext;
let mut request = Request::builder()
    .uri("/")
    .build()
    .unwrap();

let di_ctx = DummyDiContext;
request.set_di_context(di_ctx);

let ctx = request.get_di_context::<DummyDiContext>();
assert!(ctx.is_some());
```

---

## RequestBuilder

Provides a builder pattern for constructing `` `Request` `` instances.

### Builder Methods

#### `.method(method)`

Sets the HTTP method.

```rust
use reinhardt_http::Request;
use hyper::Method;

let request = Request::builder()
    .method(Method::POST)
    .uri("/api/users")
    .build()
    .unwrap();

assert_eq!(request.method, Method::POST);
```

#### `.uri(uri)`

Sets the request URI.

Query parameters are automatically parsed.

```rust
use reinhardt_http::Request;

let request = Request::builder()
    .uri("/api/users?page=1&limit=10")
    .build()
    .unwrap();

assert_eq!(request.path(), "/api/users");
assert_eq!(request.query_params.get("page"), Some(&"1".to_string()));
```

#### `.version(version)`

Sets the HTTP version.

Defaults to HTTP/1.1 if not specified.

```rust
use reinhardt_http::Request;
use hyper::Version;

let request = Request::builder()
    .version(Version::HTTP_2)
    .uri("/")
    .build()
    .unwrap();

assert_eq!(request.version, Version::HTTP_2);
```

#### `.headers(headers)`

Sets the request headers.

Replaces all existing headers.

```rust
use reinhardt_http::Request;
use hyper::{HeaderMap, header};

let mut headers = HeaderMap::new();
headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());

let request = Request::builder()
    .headers(headers)
    .build()
    .unwrap();

assert_eq!(
    request.headers.get(header::CONTENT_TYPE).unwrap(),
    "application/json"
);
```

#### `.header(key, value)`

Adds a single header to the request.

```rust
use reinhardt_http::Request;
use hyper::header;

let request = Request::builder()
    .header(header::AUTHORIZATION, "Bearer token123")
    .build()
    .unwrap();

assert_eq!(
    request.headers.get(header::AUTHORIZATION).unwrap(),
    "Bearer token123"
);
```

#### `.body(bytes)`

Sets the request body.

```rust
use reinhardt_http::Request;
use bytes::Bytes;

let body = Bytes::from(r#"{"name":"Alice"}"#);
let request = Request::builder()
    .body(body.clone())
    .build()
    .unwrap();

assert_eq!(request.body(), &body);
```

#### `.secure(is_secure)`

Sets whether the request is secure (HTTPS).

Defaults to `` `false` `` if not specified.

```rust
use reinhardt_http::Request;

let request = Request::builder()
    .secure(true)
    .build()
    .unwrap();

assert!(request.is_secure());
assert_eq!(request.scheme(), "https");
```

#### `.remote_addr(addr)`

Sets the remote address of the client.

```rust
use reinhardt_http::Request;
use std::net::{SocketAddr, IpAddr, Ipv4Addr};

let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
let request = Request::builder()
    .remote_addr(addr)
    .build()
    .unwrap();

assert_eq!(request.remote_addr, Some(addr));
```

#### `.path_params(params)`

Sets path parameters.

Primarily useful in test environments when testing views without a router.

```rust
use reinhardt_http::Request;
use std::collections::HashMap;

let mut params = HashMap::new();
params.insert("id".to_string(), "42".to_string());

let request = Request::builder()
    .path_params(params)
    .build()
    .unwrap();

assert_eq!(request.path_params.get("id"), Some(&"42".to_string()));
```

#### `.build()`

Builds the final `` `Request` `` instance.

Returns an error if URI is missing.

```rust
use reinhardt_http::Request;

let request = Request::builder()
    .uri("/api/users")
    .build()
    .unwrap();

assert_eq!(request.path(), "/api/users");
```

---

## Helper Methods

### Path Access

#### `.path()`

Gets the path portion of the request (without query string).

```rust
use reinhardt_http::Request;

let request = Request::builder()
    .uri("/api/users?page=1")
    .build()
    .unwrap();

assert_eq!(request.path(), "/api/users");
```

### Scheme Access

#### `.scheme()`

Gets the request scheme (http or https).

```rust
use reinhardt_http::Request;

let request = Request::builder()
    .secure(true)
    .build()
    .unwrap();

assert_eq!(request.scheme(), "https");
```

---

## See Also

- [Response API](./response.en.md)
- [Router API](./router.en.md)
- [Request Body Parsing](../cookbook/request-body-parsing.en.md)
