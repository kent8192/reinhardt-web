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
├── .gitignore
├── Cargo.toml
├── Dockerfile
├── Dockerfile.bruno
├── Makefile.toml
├── README.md
├── bruno/
│   ├── Snippets CRUD/
│   │   ├── Create Snippet.bru
│   │   ├── Delete Snippet.bru
│   │   ├── Get Snippet.bru
│   │   ├── List Snippets (Initial).bru
│   │   ├── Update Snippet.bru
│   │   ├── Verify Deletion (404).bru
│   │   └── folder.bru
│   ├── Snippets ViewSet/
│   │   ├── Create Snippet.bru
│   │   ├── Delete Snippet.bru
│   │   ├── Filter by Language.bru
│   │   ├── Get Snippet.bru
│   │   ├── List Snippets (Initial).bru
│   │   ├── Order by Created At Desc.bru
│   │   ├── Update Snippet.bru
│   │   └── folder.bru
│   ├── Validation Tests/
│   │   ├── Create Snippet - Empty Code (400).bru
│   │   ├── Create Snippet - Missing Title (400).bru
│   │   └── folder.bru
│   ├── bruno.json
│   ├── environments/
│   │   └── local.bru
│   └── reports/
│       └── .gitkeep
├── build.rs
├── docker-compose.api-tests.yml
├── migrations/
│   ├── auth/
│   │   └── 0001_initial.rs
│   ├── default/
│   │   └── 0001_initial.rs
│   └── snippets/
│       └── 0001_initial.rs
├── scripts/
│   ├── db_url.sh
│   ├── infra_down.sh
│   ├── infra_up.sh
│   └── parse_local_toml.py
├── settings/
│   ├── base.toml
│   └── ci.toml
├── src/
│   ├── apps/
│   │   ├── snippets/
│   │   │   ├── models.rs
│   │   │   ├── serializers.rs
│   │   │   ├── urls.rs
│   │   │   └── views.rs
│   │   └── snippets.rs
│   ├── apps.rs
│   ├── bin/
│   │   └── manage.rs
│   ├── config/
│   │   ├── apps.rs
│   │   ├── settings.rs
│   │   └── urls.rs
│   ├── config.rs
│   └── lib.rs
└── tests/
    └── integration.rs
```

## Learning Path

This example is designed to be studied alongside the REST tutorial:

1. **Start with the tutorial**: Read [Quickstart](../../website/content/quickstart/tutorials/rest/quickstart.md)
2. **Examine the code**: Look at how concepts are implemented in this example
3. **Run the tests**: `cargo test` to see the functionality in action
4. **Experiment**: Modify the code and see what happens

## Key Concepts Demonstrated

- `src/apps/snippets/models.rs` defines the `Snippet` model with `#[model(app_label = "snippets", table_name = "snippets")]`, typed fields, `created_at`, and the `highlighted()` helper.
- `src/apps/snippets/serializers.rs` defines `SnippetSerializer` with `Validate` length rules and `SnippetResponse::from_model()`.
- `src/apps/snippets/views.rs` exposes function-based CRUD handlers with `#[get]`, `#[post(pre_validate = true)]`, `#[put]`, and `#[delete]`.
- `src/apps/snippets/views.rs` also exposes a `#[reinhardt::viewset(basename = "snippet")]` `ModelViewSet` with pagination, filtering, and ordering.
- `src/apps/snippets/urls.rs` registers both function-based endpoints and ViewSet endpoints on one `ServerRouter`.
- `src/config/urls.rs` uses `#[routes]` and mounts the snippets router under the literal `/api/` prefix.
- `bruno/` contains executable API collections for CRUD, ViewSet, and validation flows.
- `tests/integration.rs` covers native CRUD, validation, ViewSet, and routing behavior.

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
