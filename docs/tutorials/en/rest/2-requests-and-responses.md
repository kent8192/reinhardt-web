# Tutorial 2: Requests and Responses

Learn how to handle HTTP requests and responses in Reinhardt.

## Request Object

Reinhardt's `Request` object provides access to HTTP request data:

```rust
use reinhardt::prelude::*;
use reinhardt_macros::endpoint;
use hyper::{Method, StatusCode};

#[endpoint]
async fn my_view(request: Request) -> Result<Response> {
    // Access HTTP method
    match request.method {
        Method::GET => println!("GET request"),
        Method::POST => println!("POST request"),
        _ => println!("Other method"),
    }

    // Access headers
    if let Some(content_type) = request.headers.get("content-type") {
        println!("Content-Type: {:?}", content_type);
    }

    // Access query parameters
    let query = request.query_string();

    // Access request body
    let body_bytes = &request.body;

    Response::ok()
        .with_body("Success")
}
```

## Response Object

Create responses using the builder pattern:

```rust
use reinhardt::prelude::*;
use serde_json::json;

// Simple text response
let response = Response::ok()
    .with_body("Hello, World!");

// JSON response
let data = json!({
    "message": "Success",
    "count": 42
});
let response = Response::ok()
    .with_json(&data)?;

// Custom status code (201 Created)
let response = Response::new(201)
    .with_body("Created");

// Response with custom headers
let response = Response::ok()
    .with_body("Data")
    .with_header("X-Custom-Header", "value");
```

## Status Codes

Reinhardt provides convenience methods for common status codes:

```rust
// 200 OK
Response::ok()
    .with_json(&data)?

// 201 Created
Response::new(201)
    .with_json(&data)?

// 204 No Content
Response::new(204)

// 400 Bad Request
Response::bad_request()
    .with_body("Invalid input")

// 404 Not Found
Response::not_found()
    .with_body("Resource not found")

// 500 Internal Server Error
Response::internal_server_error()
    .with_body("Error occurred")
```

## Parsing Request Data

Reinhardt automatically parses JSON request bodies. Simply use `serde_json::from_slice`:

```rust
use reinhardt::prelude::*;
use reinhardt_macros::endpoint;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct CreateSnippet {
    title: String,
    code: String,
    language: String,
}

#[endpoint]
async fn create_snippet(mut request: Request) -> Result<Response> {
    // Parse JSON from request body
    let body_bytes = std::mem::take(&mut request.body);
    let data: CreateSnippet = serde_json::from_slice(&body_bytes)?;

    println!("Title: {}", data.title);
    println!("Code: {}", data.code);

    Response::new(201)
        .with_json(&data)
}
```

**Note**: Reinhardt's `#[endpoint]` macro handles request parsing automatically. The framework takes care of:
- Content-Type header checking
- JSON deserialization
- Error handling for invalid JSON

## Content Negotiation

Reinhardt supports multiple content types:

```rust
use reinhardt::prelude::*;
use reinhardt_macros::endpoint;
use serde_json::Value;

#[endpoint]
async fn handle_request(mut request: Request) -> Result<Response> {
    let content_type = request.headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let body_bytes = std::mem::take(&mut request.body);

    match content_type {
        "application/json" => {
            let data: Value = serde_json::from_slice(&body_bytes)?;
            Response::ok()
                .with_json(&data)
        }
        "application/x-www-form-urlencoded" => {
            let form_data = request.parse_form().await?;
            Response::ok()
                .with_json(&form_data)
        }
        _ => {
            Response::bad_request()
                .with_body("Unsupported content type")
        }
    }
}
```

## Error Handling

Handle errors gracefully:

```rust
use reinhardt::prelude::*;
use reinhardt_macros::endpoint;

#[endpoint]
async fn safe_view(mut request: Request) -> Result<Response> {
    // Parse and validate data
    let body_bytes = std::mem::take(&mut request.body);
    let data: CreateSnippet = match serde_json::from_slice(&body_bytes) {
        Ok(d) => d,
        Err(e) => {
            return Response::bad_request()
                .with_body(&format!("Invalid JSON: {}", e));
        }
    };

    // Validate required fields
    if data.title.is_empty() {
        return Response::bad_request()
            .with_body("Title is required");
    }

    Response::new(201)
        .with_json(&data)
}
```

## Complete Example

Full request/response handling with modern Reinhardt patterns:

```rust
use reinhardt::prelude::*;
use reinhardt_macros::endpoint;
use serde::{Serialize, Deserialize};
use hyper::Method;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Snippet {
    id: Option<i64>,
    title: String,
    code: String,
    language: String,
}

fn validate_snippet(snippet: &Snippet) -> Result<(), String> {
    if snippet.title.is_empty() {
        return Err("Title is required".to_string());
    }

    if snippet.code.is_empty() {
        return Err("Code is required".to_string());
    }

    Ok(())
}

#[endpoint]
async fn snippet_list(mut request: Request) -> Result<Response> {
    match request.method {
        Method::GET => {
            // Return list of snippets
            let snippets = vec![
                Snippet {
                    id: Some(1),
                    title: "Hello".to_string(),
                    code: "print('hello')".to_string(),
                    language: "python".to_string(),
                }
            ];
            Response::ok()
                .with_json(&snippets)
        }
        Method::POST => {
            // Create new snippet
            let body_bytes = std::mem::take(&mut request.body);
            let mut snippet: Snippet = serde_json::from_slice(&body_bytes)?;

            // Validate
            if let Err(e) = validate_snippet(&snippet) {
                return Response::bad_request()
                    .with_body(&e);
            }

            // Assign ID and save
            snippet.id = Some(1);

            Response::new(201)
                .with_json(&snippet)
        }
        _ => {
            Response::new(405)
                .with_body("Method not allowed")
        }
    }
}
```

## Summary

In this tutorial, you learned:

1. Accessing request data (method, headers, body)
2. Creating responses with different status codes
3. Parsing JSON from request bodies
4. Content negotiation
5. Error handling
6. Complete request/response workflow

Next tutorial: [Tutorial 3: Class-Based Views](3-class-based-views.md)
