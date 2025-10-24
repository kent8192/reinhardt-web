# reinhardt-shortcuts

Django-style shortcut functions for the Reinhardt framework.

## Overview

Convenient shortcut functions for common HTTP operations, inspired by Django's `django.shortcuts` module. These functions provide simple, intuitive APIs for creating responses, redirects, and handling database queries with automatic 404 error handling.

## Implemented ✓

### Redirect Shortcuts

#### `redirect()`

Create a temporary redirect (HTTP 302) response.

**Example:**

```rust
use reinhardt_shortcuts::redirect;

let response = redirect("/new-location");
// Returns HTTP 302 with Location: /new-location
```

#### `redirect_permanent()`

Create a permanent redirect (HTTP 301) response.

**Example:**

```rust
use reinhardt_shortcuts::redirect_permanent;

let response = redirect_permanent("/permanent-location");
// Returns HTTP 301 with Location: /permanent-location
```

### Response Rendering Shortcuts

#### `render_json()`

Render data as JSON and return an HTTP 200 response.

**Example:**

```rust
use reinhardt_shortcuts::render_json;
use serde_json::json;

let data = json!({
    "status": "success",
    "message": "Hello, world!"
});

let response = render_json(&data);
// Returns HTTP 200 with Content-Type: application/json
```

#### `render_json_pretty()`

Render data as pretty-printed JSON with indentation.

**Example:**

```rust
use reinhardt_shortcuts::render_json_pretty;
use serde_json::json;

let data = json!({"key": "value"});
let response = render_json_pretty(&data);
// Returns formatted JSON with newlines and indentation
```

#### `render_html()`

Render HTML content and return an HTTP 200 response.

**Example:**

```rust
use reinhardt_shortcuts::render_html;

let html = "<h1>Hello, World!</h1>";
let response = render_html(html);
// Returns HTTP 200 with Content-Type: text/html; charset=utf-8
```

#### `render_text()`

Render plain text content and return an HTTP 200 response.

**Example:**

```rust
use reinhardt_shortcuts::render_text;

let text = "Plain text content";
let response = render_text(text);
// Returns HTTP 200 with Content-Type: text/plain; charset=utf-8
```

### Database Shortcuts (404 Error Handling)

#### `get_or_404_response()`

Get a single object or return a 404 response if not found.

**Example:**

```rust
use reinhardt_shortcuts::get_or_404_response;

// Simulating database query result
let result = Ok(Some(user));

match get_or_404_response(result) {
    Ok(user) => {
        // User found, continue processing
    }
    Err(response) => {
        // Returns HTTP 404 response
        return response;
    }
}
```

#### `get_list_or_404_response()`

Get a list of objects or return a 404 response if the list is empty.

**Example:**

```rust
use reinhardt_shortcuts::get_list_or_404_response;

let result = Ok(vec![user1, user2]);

match get_list_or_404_response(result) {
    Ok(users) => {
        // Users found, continue processing
    }
    Err(response) => {
        // Returns HTTP 404 if list is empty
        return response;
    }
}
```

#### `exists_or_404_response()`

Check if a record exists or return a 404 response.

**Example:**

```rust
use reinhardt_shortcuts::exists_or_404_response;

let result = Ok(true);

match exists_or_404_response(result) {
    Ok(_) => {
        // Record exists, continue
    }
    Err(response) => {
        // Returns HTTP 404 if not found
        return response;
    }
}
```

### Error Types

#### `GetError`

Error type for database query operations.

**Variants:**

- `NotFound` - Object not found in database
- `MultipleObjectsReturned` - Query returned multiple objects when one was expected
- `DatabaseError(String)` - Database operation error

## Implemented with Feature Gates ✓

### ORM Integration (requires `database` feature)

#### `get_object_or_404<M>(pk: M::PrimaryKey) -> Result<M, Response>`

Direct database integration for single object retrieval. Queries the database using
the ORM and returns HTTP 404 if the object is not found.

**Example:**

```rust
use reinhardt_shortcuts::get_object_or_404;

async fn user_detail(user_id: i64) -> Result<Response, Response> {
    let user = get_object_or_404::<User>(user_id).await?;
    render_json(&user)
}
```

#### `get_list_or_404<M>(queryset: QuerySet<M>) -> Result<Vec<M>, Response>`

Direct database integration for list retrieval. Executes a QuerySet and returns HTTP 404
if the result list is empty.

**Example:**

```rust
use reinhardt_shortcuts::get_list_or_404;

async fn user_list(status: &str) -> Result<Response, Response> {
    let queryset = User::objects()
        .filter("status", FilterOperator::Eq, FilterValue::String(status.to_string()));

    let users = get_list_or_404(queryset).await?;
    render_json(&users)
}
```

### Template Integration (requires `templates` feature)

Templates are loaded from the file system using `FileSystemTemplateLoader`. The template
directory is specified by the `REINHARDT_TEMPLATE_DIR` environment variable (defaults to `./templates`).

**Current Implementation**: Basic variable substitution is supported using `{{ variable }}` syntax.
Templates are rendered dynamically at runtime with simple placeholder replacement.

**Planned Enhancements**:

- Full Askama template syntax: control structures (`{% if %}`, `{% for %}`)
- Template inheritance (`{% extends %}`, `{% block %}`)
- Custom filters and tags

#### `render_template(request: &Request, template_name: &str, context: HashMap) -> Result<Response, Response>`

Template rendering with context. Loads a template file from the filesystem and returns
an HTTP response with the template content. Context variables are displayed in debug mode
as HTML comments.

**Example:**

```rust
use reinhardt_shortcuts::render_template;
use std::collections::HashMap;

async fn index_view(request: Request) -> Result<Response, Response> {
    let mut context = HashMap::new();
    context.insert("title", "Welcome");
    context.insert("user", request.user().name());

    render_template(&request, "index.html", context)
}
```

#### `render_to_response(request: &Request, template_name: &str, context: HashMap) -> Result<Response, Response>`

Advanced template rendering with custom response configuration. Similar to `render_template`
but returns a mutable Response that can be further customized.

**Example:**

```rust
use reinhardt_shortcuts::render_to_response;
use std::collections::HashMap;

async fn custom_view(request: Request) -> Result<Response, Response> {
    let mut context = HashMap::new();
    context.insert("message", "Custom response");

    let mut response = render_to_response(&request, "custom.html", context)?;

    // Customize the response
    response.status = hyper::StatusCode::CREATED;
    response.headers.insert(
        hyper::header::CACHE_CONTROL,
        hyper::header::HeaderValue::from_static("no-cache"),
    );

    Ok(response)
}
```

## Planned Features

### Error Handling (Not Yet Implemented)

- Custom error pages (404, 500, etc.)
- Error page templates
- Debug error pages for development

### Template Engine Integration

- Full Askama template engine integration for variable substitution
- Template inheritance and includes
- Custom template filters and tags

## Usage Patterns

### Combining Shortcuts

```rust
use reinhardt_shortcuts::{render_json, get_or_404_response};

async fn get_user_handler(user_id: i64) -> Response {
    let result = database::find_user(user_id).await;

    match get_or_404_response(result) {
        Ok(user) => render_json(&user),
        Err(not_found_response) => not_found_response,
    }
}
```

### Redirect After Action

```rust
use reinhardt_shortcuts::{redirect, redirect_permanent};

async fn create_user_handler(user_data: UserData) -> Response {
    database::create_user(user_data).await;
    redirect("/users/")
}

async fn old_url_handler() -> Response {
    redirect_permanent("/new-url/")
}
```

## Dependencies

- `reinhardt-http` - HTTP types (Request, Response)
- `serde` - Serialization support
- `serde_json` - JSON rendering
- `bytes` - Efficient byte buffer handling
- `thiserror` - Error type definitions

## Related Crates

- `reinhardt-http` - HTTP primitives
- `reinhardt-views` - View layer
- `reinhardt-orm` - Database ORM (for future direct integration)

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
