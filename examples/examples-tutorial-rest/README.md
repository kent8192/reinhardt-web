# Reinhardt REST Tutorial Example - Code Snippet Management API

This example demonstrates the concepts covered in the [Reinhardt REST Tutorial](../../website/content/quickstart/tutorials/rest/). It implements a complete RESTful API for managing code snippets.

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
cargo make runserver
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
├── Cargo.toml                      # Project configuration
├── build.rs                        # Build script
├── README.md                       # This file
├── src/
│   ├── lib.rs                      # Library entry point
│   ├── config.rs                   # Config aggregator
│   ├── apps.rs                     # Apps aggregator
│   ├── urls_demo.rs                # Typed `ResolvedUrls` accessor shims (Issue #4548)
│   ├── bin/
│   │   └── manage.rs               # Management command
│   ├── config/
│   │   ├── apps.rs                 # installed_apps! { snippets: "snippets" }
│   │   ├── settings.rs             # Settings composition
│   │   └── urls.rs                 # #[routes] entry point, mounts /api/
│   └── apps/
│       ├── snippets.rs             # snippets app entry (sibling of snippets/)
│       └── snippets/
│           ├── models.rs           # Snippet model (#[model])
│           ├── serializers.rs      # SnippetSerializer + SnippetResponse
│           ├── urls.rs             # aggregator: #[url_patterns(InstalledApp::snippets, mode = server)]
│           │                       # registers both function-based and ViewSet endpoints
│           └── views.rs            # HTTP method handlers + #[viewset]
└── tests/
    ├── integration.rs              # CRUD + edge-case integration tests
    └── urls_typed_accessors.rs     # Typed `ResolvedUrls` accessor end-to-end tests
```

## Learning Path

This example is designed to be studied alongside the REST tutorial:

1. **Start with the tutorial**: Read [Quickstart](../../website/content/quickstart/tutorials/rest/quickstart.md)
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

The snippets app exposes a single `url_patterns()` entry point in
`src/apps/snippets/urls.rs`. It carries the typed
`#[url_patterns(InstalledApp::snippets, mode = server)]` macro (rc.18+,
discussion #3770), which binds the router to its owning app at compile
time via the `AppLabel` trait and applies `.with_namespace("snippets")`
for URL reversal (e.g. `"snippets:snippets_list"`) without changing the
request path. Both the function-based endpoints (Tutorial 1-5) and the
ViewSet endpoints (Tutorial 6) are registered on the same router:

```rust
// src/apps/snippets/urls.rs
#[url_patterns(InstalledApp::snippets, mode = server)]
pub fn url_patterns() -> ServerRouter {
    ServerRouter::new()
        // Function-based endpoints (Tutorial 1-5)
        .endpoint(views::list)
        .endpoint(views::create)
        .endpoint(views::retrieve)
        .endpoint(views::update)
        .endpoint(views::delete)
        // ViewSet endpoints (Tutorial 6)
        .viewset("/snippets-viewset", views::viewset())
}
```

> **Why the routes are inlined here**: the framework currently supports at
> most one `#[url_patterns(InstalledApp::<app>, mode = server)]` per app
> (sibling occurrences emit duplicate `__for_each_url_resolver` macros and
> fail with `E0659: ambiguous name`), and `.mount("/", helper())` calls
> additionally require each mount target to have its own `url_resolvers`
> module. Splitting the function-based and ViewSet registrations into
> separate helper files is therefore not possible today. See `urls.rs`'s
> module-level comment for the full rationale and the framework path that
> would lift the constraint.

Mounted at the project root with an explicit literal prefix.
`#[routes(server_only)]` is used because this project is REST-only —
the `server_only` flag (Issue #4509) tells the macro to skip per-app
`client_url_resolvers` / `ws_url_resolvers` module lookups, so the
`snippets` app does not need empty `client_router.rs` / `ws_urls.rs`
stub modules and the `websockets` Cargo feature can stay off. The
macro still consumes `installed_apps!` and generates the typed
`ResolvedUrls::snippets()` accessor that `standalone` would have
suppressed:

```rust
// src/config/urls.rs
#[routes(server_only)]
pub fn routes() -> UnifiedRouter {
    UnifiedRouter::new().mount("/api/", crate::apps::snippets::urls::url_patterns())
}
```

The `/api/` prefix is a literal path (no `{...}` segments), satisfying the
rc.24 guard that panics when `ServerRouter::mount()` receives a parameterised
prefix.

### 5. URL Resolution: Typed `ResolvedUrls` Accessors

Once the `#[routes]` macro has registered the router globally, application
code can resolve any registered route through the **typed**
`ResolvedUrls::server().<app>().<route>()` accessor instead of formatting
URLs inline or reaching for the deprecated flat `urls.snippet_list()`
surface (deprecated since `0.1.0-rc.16`). The typed accessor was
introduced by [PR #4518](https://github.com/kent8192/reinhardt-web/pull/4518)
and is the recommended pattern going forward — see
[Issue #4548](https://github.com/kent8192/reinhardt-web/issues/4548) for
the migration milestone.

This example demonstrates the typed accessor pattern in
`src/urls_demo.rs` (thin shims) and pins the resolved URL strings in
`tests/urls_typed_accessors.rs` (end-to-end registration + assertions).

```rust
use examples_tutorial_rest::urls_demo;
use reinhardt::ResolvedUrls;

// Once per request after the server has booted.
let urls = ResolvedUrls::from_global();

// Function-based endpoints (Tutorial 1-5)
let list_url   = urls.server().snippets().snippets_list();      // "/api/snippets/"
let create_url = urls.server().snippets().snippets_create();    // "/api/snippets/"
let detail_url = urls.server().snippets().snippets_retrieve("42"); // "/api/snippets/42/"

// ViewSet endpoints (Tutorial 6) — the typed accessor is namespaced
// per app, so the viewset's `<basename>_list` and `<basename>_detail`
// live next to the function-based ones on the same gateway.
let vs_list   = urls.server().snippets().snippet_list();        // "/api/snippets-viewset/"
let vs_detail = urls.server().snippets().snippet_detail("42");  // "/api/snippets-viewset/42/"

// Equivalent calls through the `urls_demo` shim — useful when a caller
// already has an `id: i64` and does not want to stringify at every
// call site.
assert_eq!(urls_demo::snippets_list(&urls), list_url);
assert_eq!(urls_demo::snippets_retrieve(&urls, 42), detail_url);
```

#### Why typed accessors

| Concern | Typed accessor | Deprecated flat surface |
|---|---|---|
| Compile-time misspelling check | ✅ method name is a Rust identifier | ❌ panics at runtime |
| Namespace safety | ✅ auto-prefixes `"<app>:"` | ❌ relies on `UrlResolverUnprefixed` iteration |
| Refactor-safe across renamed routes | ✅ accessor follows route name | ❌ same surface for every route |
| Discoverable from IDE auto-complete | ✅ on `SnippetsUrls<'_>` | ❌ blanket trait, hidden in extensions |

#### Migration recipe

1. `let urls = ResolvedUrls::from_global();` (or `ResolvedUrls::from_router(...)`
   when you already have an `Arc<ServerRouter>` in hand — useful in tests).
2. Replace `urls.<route>()` → `urls.server().<app>().<route>()`.
3. If a route takes a path parameter, pass it as `&str` (use
   `&id.to_string()` for `i64` primary keys).

#### Deprecation removal

The flat accessors (`urls.snippet_list()`, `urls.snippet_detail("42")`)
remain functional but will be removed in `v0.2.0`. Run
`cargo build --message-format=short 2>&1 | grep deprecated` to discover
remaining call sites in your own code.

### 6. Validation

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

This example demonstrates both function-based views (Tutorial 1-5) and ViewSet-based views (Tutorial 6). **Both are mounted simultaneously** on the same running server — there is no toggle between them. The two endpoint sets coexist under separate URL prefixes:

```bash
cargo make runserver

# Function-based endpoints (Tutorial 1-5)
curl http://127.0.0.1:8000/api/snippets/

# ViewSet endpoints (Tutorial 6)
curl http://127.0.0.1:8000/api/snippets-viewset/
```

The Bruno collection under `bruno/` contains a `Snippets CRUD` folder for the function-based path and a `Snippets ViewSet` folder for the ViewSet path; both can be exercised back-to-back without restarting the server.

> **rc.23+ runtime behaviour**: Starting with reinhardt-web rc.23,
> `ModelViewSet` (and `ReadOnlyModelViewSet`) issue **real database
> queries** instead of returning skeleton `[]` / `{}` responses. To exercise
> the ViewSet endpoints you must therefore migrate the `snippets` table
> first:
>
> ```bash
> cargo make migrate
> cargo make runserver
> ```
>
> Until rows are inserted, the `/api/snippets-viewset/` endpoints will
> return an empty list. The function-based path (`/api/snippets/`) is
> unaffected — it falls back to in-memory sample snippets defined in
> `views.rs::get_sample_snippets`.

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

- [REST Tutorial](../../website/content/quickstart/tutorials/rest/) - Step-by-step guide
- [API Documentation](https://docs.rs/reinhardt-web) - Complete API reference

## License

This example is part of the Reinhardt project and is licensed under the BSD 3-Clause License.
