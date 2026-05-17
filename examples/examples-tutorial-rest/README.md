# Reinhardt REST Tutorial Example - Code Snippet Management API

This example demonstrates the concepts covered in the [Reinhardt REST Tutorial](../../../docs/tutorials/en/rest/). It implements a complete RESTful API for managing code snippets.

## What This Example Covers

This example corresponds to the REST tutorial Quickstart and Tutorial 1-6:

- **Quickstart** - Project setup, serializers, views, routing
- **Tutorial 1: Serialization** - serde, validation, JSON serialization
- **Tutorial 2: Requests and Responses** - Request object, Response builder, parameter extraction
- **Tutorial 3: Class-Based Views** - Generic views (ListAPIView, CreateAPIView, etc.)
- **Tutorial 4: Authentication & Permissions** - Authentication systems
- **Tutorial 5: Relationships and Hyperlinked APIs** - Relationships and hyperlinked APIs
- **Tutorial 6: ViewSets and Routers** - ViewSets, ModelViewSet, Router

## Features

### Models

- **Snippet**: Code snippet with title, code, and language

### Serializers

- **SnippetSerializer**: Input validation with built-in validation
- **SnippetResponse**: Output serialization

### API Endpoints

```
GET    /api/snippets/       - List all snippets
POST   /api/snippets/       - Create a new snippet
GET    /api/snippets/<id>/  - Retrieve a specific snippet
PUT    /api/snippets/<id>/  - Update a snippet
DELETE /api/snippets/<id>/  - Delete a snippet
```

## Setup

### Prerequisites

- Rust 1.75 or later
- PostgreSQL (optional, for database features)
- Docker (optional, for TestContainers in tests)

### Installation

```bash
# From the project root
cd examples/examples-tutorial-rest

# Build the project
cargo build

# Run tests
cargo test
```

## Usage

### Run the Development Server

```bash
cargo run --bin manage runserver
```

The server will start at `http://127.0.0.1:8000/`.

### API Examples

```bash
# List all snippets
curl http://127.0.0.1:8000/api/snippets/

# Create a new snippet
curl -X POST http://127.0.0.1:8000/api/snippets/ \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Hello World",
    "code": "println!(\"Hello, world!\");",
    "language": "rust"
  }'

# Get a specific snippet
curl http://127.0.0.1:8000/api/snippets/1/

# Update a snippet
curl -X PUT http://127.0.0.1:8000/api/snippets/1/ \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Hello Reinhardt",
    "code": "println!(\"Hello, Reinhardt!\");",
    "language": "rust"
  }'

# Delete a snippet
curl -X DELETE http://127.0.0.1:8000/api/snippets/1/
```

## Project Structure

```
examples-tutorial-rest/
├── Cargo.toml                 # Project configuration
├── build.rs                   # Build script
├── README.md                  # This file
├── src/
│   ├── lib.rs                # Library entry point
│   ├── config.rs             # Config module
│   ├── apps.rs               # Apps module
│   ├── bin/
│   │   └── manage.rs         # Management command
│   ├── config/
│   │   ├── settings.rs       # Settings configuration
│   │   ├── urls.rs           # URL routing
│   │   └── apps.rs           # Installed apps
│   └── apps/
│       └── snippets/
│           ├── lib.rs        # App module
│           ├── models.rs     # Snippet model
│           ├── serializers.rs  # Serializers
│           ├── views.rs      # View handlers
│           └── urls.rs       # URL patterns
└── tests/
    ├── integration.rs        # Integration tests
    └── availability.rs       # Availability tests
```

## Learning Path

This example is designed to be studied alongside the REST tutorial:

1. **Start with the tutorial**: Read [Quickstart](../../../docs/tutorials/en/rest/quickstart.md)
2. **Examine the code**: Look at how concepts are implemented in this example
3. **Run the tests**: `cargo test` to see the functionality in action
4. **Experiment**: Modify the code and see what happens

## Key Concepts Demonstrated

### 1. Models (models.rs)

```rust
pub struct Snippet {
    pub id: Option<i64>,
    pub title: String,
    pub code: String,
    pub language: String,
}
```

### 2. Serializers (serializers.rs)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SnippetSerializer {
    #[validate(length(min = 1, max = 100))]
    pub title: String,

    #[validate(length(min = 1))]
    pub code: String,

    #[validate(length(min = 1, max = 50))]
    pub language: String,
}
```

### 3. Views (views.rs)

```rust
// HTTP method decorator + `pre_validate = true` (declarative validation,
// available since rc.5). The macro extracts `Json<SnippetSerializer>`, calls
// `Validate::validate` on the dereferenced value, and returns HTTP 400 with
// JSON error details on failure — all before this function body runs.
#[get("/snippets/", name = "snippets_list")]
pub async fn list() -> ViewResult<Response> {
    // List all snippets — return JSON via `Response::new(StatusCode::OK).with_body(...)`.
}

#[post("/snippets/", name = "snippets_create", pre_validate = true)]
pub async fn create(Json(serializer): Json<SnippetSerializer>) -> ViewResult<Response> {
    // `serializer` is already validated. Just persist and return 201.
}
```

### 4. URL Routing (urls.rs)

```rust
// `#[url_patterns(InstalledApp::snippets, mode = server)]` (rc.18+) binds
// this router to the `snippets` app at compile time via the `AppLabel` trait.
// The macro also applies `.with_namespace("snippets")` for URL reversal
// (e.g. `"snippets:snippets_list"`) without changing the request path.
#[url_patterns(InstalledApp::snippets, mode = server)]
pub fn server_url_patterns() -> ServerRouter {
    ServerRouter::new()
        .endpoint(views::list)
        .endpoint(views::create)
        .endpoint(views::retrieve)
        .endpoint(views::update)
        .endpoint(views::delete)
}
```

Mounted at the project root with an explicit literal prefix:

```rust
// src/config/urls.rs
#[routes(standalone)]
pub fn routes() -> UnifiedRouter {
    UnifiedRouter::new().mount(
        "/api/",
        crate::apps::snippets::urls::server_url_patterns(),
    )
}
```

The `/api/` prefix is a literal path (no `{...}` segments), satisfying the
rc.24 guard that panics when `ServerRouter::mount()` receives a parameterised
prefix.

### 5. Validation

```rust
// Serializer side: declare validation rules with `#[validate(...)]`.
use reinhardt::Validate;

#[derive(serde::Deserialize, Validate)]
pub struct SnippetSerializer { /* ... */ }

// Handler side: enable `pre_validate = true` on the route macro.
// Manual `serializer.validate()?` is no longer needed inside the handler
// body — the macro generates the call for you and converts failures into
// a HTTP 400 JSON response.
#[post("/snippets/", name = "snippets_create", pre_validate = true)]
pub async fn create(Json(serializer): Json<SnippetSerializer>) -> ViewResult<Response> {
    /* serializer is already validated here */
}
```

## Testing

Run the test suite:

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_snippet_model

# Run with output
cargo test -- --nocapture
```

## ViewSets (Tutorial 6)

This example demonstrates both function-based views (Tutorial 1-5) and ViewSet-based views (Tutorial 6).

### Switching Between Approaches

You can switch between the two approaches using the `USE_VIEWSET` environment variable:

```bash
# Function-based views (default) - Tutorial 1-5 approach
cargo run --bin manage runserver
# Visit http://127.0.0.1:8000/api/snippets/

# ViewSet-based views - Tutorial 6 approach
USE_VIEWSET=1 cargo run --bin manage runserver
# Visit http://127.0.0.1:8000/api/snippets-viewset/
```

> **rc.23+ runtime behaviour**: Starting with reinhardt-web rc.23,
> `ModelViewSet` (and `ReadOnlyModelViewSet`) issue **real database
> queries** instead of returning skeleton `[]` / `{}` responses. To exercise
> the ViewSet branch you must therefore migrate the `snippets` table first:
>
> ```bash
> cargo run --bin manage -- migrate
> USE_VIEWSET=1 cargo run --bin manage runserver
> ```
>
> Until rows are inserted, the ViewSet endpoints will return an empty list.
> The function-based branch (default) is unaffected — it falls back to
> in-memory sample snippets defined in `views.rs::get_sample_snippets`.

### Comparison

| Approach | Code Lines | Features |
|----------|------------|----------|
| Function-based (Tutorial 1-5) | ~200 lines | Full control, explicit implementation |
| ViewSet-based (Tutorial 6) | ~15 lines | CRUD automation, pagination, filtering, ordering |

### ViewSet Features

The ViewSet implementation provides:

- **Automatic CRUD operations**: list, create, retrieve, update, delete
- **Pagination**: `?page=1&page_size=10` (10 items per page, max 100)
- **Filtering**: `?language=rust&title=hello` (filter by language and title fields)
- **Ordering**: `?ordering=created_at,-title` (order by created_at ascending, title descending)

### Testing ViewSet Features

```bash
# List with pagination
curl "http://127.0.0.1:8000/api/snippets-viewset/?page=1&page_size=10"

# Filter by language
curl "http://127.0.0.1:8000/api/snippets-viewset/?language=rust"

# Order by created_at (descending)
curl "http://127.0.0.1:8000/api/snippets-viewset/?ordering=-created_at"

# Combine: Filter + Order + Paginate
curl "http://127.0.0.1:8000/api/snippets-viewset/?language=rust&ordering=-title&page=1&page_size=5"
```

### When to Use Each Approach

**Function-based views (Tutorial 1-5)**:
- Simple endpoints with custom logic
- Non-standard RESTful patterns
- When you need fine-grained control
- Learning HTTP handling basics

**ViewSet-based views (Tutorial 6)**:
- Standard RESTful CRUD APIs
- When pagination, filtering, and ordering are needed
- Rapid API development
- When code conciseness is important

## Next Steps

After understanding this example:

1. **Compare both approaches**: Try switching between function-based and ViewSet-based
2. **Understand the trade-offs**: When to use each approach (see above)
3. **Add custom actions**: Extend ViewSets with `#[action]` decorator for non-CRUD endpoints
4. **Add database integration**: Implement actual database storage instead of in-memory sample data
5. **Add authentication**: Implement JWT/Token/Session auth for both approaches
6. **Add permissions**: Implement permission classes to control access

## Related Documentation

- [REST Tutorial](../../../docs/tutorials/en/rest/) - Step-by-step guide
- [API Documentation](https://docs.rs/reinhardt-web) - Complete API reference

## License

This example is part of the Reinhardt project and is licensed under the BSD 3-Clause License.
