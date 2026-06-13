+++
title = "Part 6: Bonus: ViewSets and Routers"
description = "Expose the snippets API through ModelViewSet and mount it beside the function-based handlers."
weight = 60

[extra]
sidebar_weight = 60
+++

# Part 6: Bonus: ViewSets and Routers

You have already built the snippets API the explicit way. The function-based handlers show every moving part: extractors, dependency injection, ORM calls, validation, response construction, and status codes.

Now let's look at the high-level alternative. `ModelViewSet` can generate the same CRUD surface with far less code. Convention over configuration is a right, not an obligation: use ViewSets when their defaults match the API you want, and keep function-based handlers when you need exact control.

## Add the ViewSet Function

Open `src/apps/snippets/views.rs` and add this after the function-based handlers:

```rust
#[reinhardt::viewset(basename = "snippet")]
pub fn viewset() -> reinhardt::ModelViewSet<Snippet, SnippetSerializer> {
	use reinhardt::ModelViewSet;
	use reinhardt::views::viewsets::{FilterConfig, OrderingConfig, PaginationConfig};

	ModelViewSet::new("snippet")
		.with_pagination(PaginationConfig::page_number(10, Some(100)))
		.with_filters(
			FilterConfig::new()
				.with_filterable_fields(vec!["language".to_string(), "title".to_string()]),
		)
		.with_ordering(
			OrderingConfig::new()
				.with_ordering_fields(vec!["created_at".to_string(), "title".to_string()]),
		)
}
```

That is the whole ViewSet definition for this tutorial.

`ModelViewSet<Snippet, SnippetSerializer>` says which model and serializer drive the generated handlers. The `#[reinhardt::viewset(basename = "snippet")]` attribute gives the ViewSet a stable basename for generated route metadata. `ModelViewSet::new("snippet")` uses the same resource name inside the ViewSet builder.

The chained calls enable list behavior:

- `PaginationConfig::page_number(10, Some(100))` enables page-number pagination with a default page size of 10 and a max page size of 100.
- `FilterConfig::new().with_filterable_fields(...)` allows clients to filter by `language` and `title`.
- `OrderingConfig::new().with_ordering_fields(...)` allows clients to order by `created_at` and `title`.

The function-based code above it is roughly 200 lines. This ViewSet is roughly 15 lines. That reduction is useful when your API follows the standard CRUD shape.

## Mount It Beside the Function-Based Routes

Open `src/apps/snippets/urls.rs` and add the ViewSet to the same app router:

```rust
use reinhardt::ServerRouter;

use super::views;

pub fn url_patterns() -> ServerRouter {
	ServerRouter::new()
		.endpoint(views::list)
		.endpoint(views::create)
		.endpoint(views::config)
		.endpoint(views::retrieve)
		.endpoint(views::update)
		.endpoint(views::delete)
		.viewset("/snippets-viewset", views::viewset())
}
```

There is no toggle. Both endpoint sets are live in the same server process:

```text
Function-based:
GET    /api/snippets/
POST   /api/snippets/
GET    /api/snippets/{id}/
PUT    /api/snippets/{id}/
DELETE /api/snippets/{id}/

ViewSet-based:
GET    /api/snippets-viewset/
POST   /api/snippets-viewset/
GET    /api/snippets-viewset/{id}/
PUT    /api/snippets-viewset/{id}/
PATCH  /api/snippets-viewset/{id}/
DELETE /api/snippets-viewset/{id}/
```

The project-level router still mounts the whole snippets app under `/api/`, so `.viewset("/snippets-viewset", ...)` becomes `/api/snippets-viewset/`.

## Run It

Start from a migrated database:

```bash
cargo make migrate
cargo make runserver
```

List through the function-based route:

```bash
curl http://127.0.0.1:8000/api/snippets/
```

On a clean database:

```json
{"snippets":[]}
```

Now list through the ViewSet route:

```bash
curl http://127.0.0.1:8000/api/snippets-viewset/
```

On the same clean database, you should see an empty JSON list:

```json
[]
```

Both routes query the same `snippets` table. Insert a row through either path and both paths can see it:

```bash
curl -X POST http://127.0.0.1:8000/api/snippets-viewset/ \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Hello ViewSet",
    "code": "println!(\"Hello from a ViewSet\");",
    "language": "rust"
  }'
```

Then filter and order the ViewSet list:

```bash
curl "http://127.0.0.1:8000/api/snippets-viewset/?language=rust"
curl "http://127.0.0.1:8000/api/snippets-viewset/?ordering=-created_at"
curl "http://127.0.0.1:8000/api/snippets-viewset/?language=rust&ordering=-title&page=1&page_size=5"
```

Run the compile check:

```bash
cargo check --all-features
```

## When to Use Which Style

Use function-based handlers when:

- You need custom status-code decisions.
- The request does more than standard CRUD.
- You want to teach or audit every dependency and ORM call.
- You need one endpoint to orchestrate several models or services.

Use `ModelViewSet` when:

- The endpoint is ordinary model CRUD.
- Pagination, filtering, and ordering should follow framework conventions.
- You want fewer lines of application code.
- The generated route surface matches your API contract.

The two styles can coexist. The reference example keeps both mounted so you can compare them against the same model, serializer, migration, and database.

## What You Built

You now have the complete REST tutorial project:

- A generated REST project and `snippets` app
- Function-based CRUD handlers using HTTP method macros
- A real `Snippet` model, migration, and database table
- Dependency-injected database access
- Typed serializers and validation
- A `ModelViewSet` mounted beside the function-based API

From here, read the finished reference crate at `examples/examples-tutorial-rest`, run its integration tests with `cargo make test`, and use the Bruno collection if you want a repeatable API client workflow.
