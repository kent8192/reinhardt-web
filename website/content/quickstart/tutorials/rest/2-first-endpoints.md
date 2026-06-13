+++
title = "Part 2: Your First Endpoints"
description = "Add temporary JSON endpoints with Reinhardt HTTP method macros and request extractors."
weight = 20
+++

# Part 2: Your First Endpoints

In Part 1, you generated the project shell and mounted the snippets app under `/api/`. Now let's make that route answer real HTTP requests.

This chapter deliberately uses static JSON responses. The code is temporary scaffolding: it lets you learn the route macros, extractors, and response shape before adding a database in Part 3 and dependency injection in Part 4.

## Write the View Module

Open `src/apps/snippets/views.rs` and replace the generated placeholder with this first version:

```rust
use json::json;
use reinhardt::core::serde::json;
use reinhardt::http::ViewResult;
use reinhardt::{Json, Path, Query, Response, StatusCode};
use reinhardt::{delete, get, post, put};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SnippetQuery {
	pub language: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SnippetPayload {
	pub title: String,
	pub code: String,
	pub language: String,
}

/// List snippets.
#[get("/snippets/", name = "snippets-list")]
pub async fn list(Query(params): Query<SnippetQuery>) -> ViewResult<Response> {
	let response_data = json!({
		"snippets": [
			{
				"id": 1,
				"title": "Hello Reinhardt",
				"code": "fn main() { println!(\"Hello, Reinhardt!\"); }",
				"language": params.language.unwrap_or_else(|| "rust".to_string())
			}
		]
	});
	let body = json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(body))
}

/// Create a snippet.
#[post("/snippets/", name = "snippets-create")]
pub async fn create(Json(payload): Json<SnippetPayload>) -> ViewResult<Response> {
	let response_data = json!({
		"message": "Snippet created",
		"snippet": {
			"id": 2,
			"title": payload.title,
			"code": payload.code,
			"language": payload.language
		}
	});
	let body = json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::CREATED)
		.with_header("Content-Type", "application/json")
		.with_body(body))
}

/// Retrieve a snippet.
#[get("/snippets/{id}/", name = "snippets-retrieve")]
pub async fn retrieve(Path(snippet_id): Path<i64>) -> ViewResult<Response> {
	let response_data = json!({
		"snippet": {
			"id": snippet_id,
			"title": "Hello Reinhardt",
			"code": "fn main() { println!(\"Hello, Reinhardt!\"); }",
			"language": "rust"
		}
	});
	let body = json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(body))
}

/// Update a snippet.
#[put("/snippets/{id}/", name = "snippets-update")]
pub async fn update(
	Path(snippet_id): Path<i64>,
	Json(payload): Json<SnippetPayload>,
) -> ViewResult<Response> {
	let response_data = json!({
		"message": "Snippet updated",
		"snippet": {
			"id": snippet_id,
			"title": payload.title,
			"code": payload.code,
			"language": payload.language
		}
	});
	let body = json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(body))
}

/// Delete a snippet.
#[delete("/snippets/{id}/", name = "snippets-delete")]
pub async fn delete(Path(_snippet_id): Path<i64>) -> ViewResult<Response> {
	Ok(Response::new(StatusCode::NO_CONTENT))
}
```

The route attributes are the important part:

- `#[get]`, `#[post]`, `#[put]`, and `#[delete]` bind one Rust function to one HTTP method and path.
- `name = "..."` gives the route a stable name. Later, route reversal and generated URLs use that name instead of hard-coded path strings.
- `Path<i64>` extracts `{id}` from the path and parses it as an integer.
- `Query<SnippetQuery>` extracts query-string fields such as `?language=rust`.
- `Json<SnippetPayload>` parses the request body as JSON.
- `ViewResult<Response>` lets you use `?` for framework-compatible errors while still returning an explicit HTTP response.

The `json::to_string(&response_data)?` line is why the function returns `ViewResult<Response>` rather than a bare `Response`: serialization can fail, and Reinhardt can convert that error into an HTTP response.

## Register App URLs

Open `src/apps/snippets/urls.rs` and make the app expose a single router:

```rust
use reinhardt::ServerRouter;

use super::views;

pub fn url_patterns() -> ServerRouter {
	ServerRouter::new()
		.endpoint(views::list)
		.endpoint(views::create)
		.endpoint(views::retrieve)
		.endpoint(views::update)
		.endpoint(views::delete)
}
```

`ServerRouter::new()` starts an app-level router. Each `.endpoint(...)` call registers one function marked with an HTTP method macro.

Order matters when a literal route and a dynamic route could both match the same request. We will use that in Part 4 when `/snippets/config/` must be registered before `/snippets/{id}/`.

## Confirm the Project Mount

Your project-level router from Part 1 should already mount the snippets app under `/api/`:

```rust
use reinhardt::prelude::*;
use reinhardt::routes;

#[routes]
pub fn routes() -> UnifiedRouter {
	UnifiedRouter::new().mount("/api/", crate::apps::snippets::urls::url_patterns())
}
```

This means the app route `/snippets/` becomes the public route `/api/snippets/`.

## Run It

Start the server:

```bash
cargo run --bin manage -- runserver
```

If you are working inside the reference example, use the make task:

```bash
cargo make runserver
```

Now request the list endpoint:

```bash
curl 'http://127.0.0.1:8000/api/snippets/?language=rust'
```

You should see a JSON response with one temporary snippet:

```json
{"snippets":[{"code":"fn main() { println!(\"Hello, Reinhardt!\"); }","id":1,"language":"rust","title":"Hello Reinhardt"}]}
```

Try a POST request:

```bash
curl -X POST http://127.0.0.1:8000/api/snippets/ \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Hello World",
    "code": "println!(\"Hello, world!\");",
    "language": "rust"
  }'
```

You should see `201 Created` with the submitted data echoed back:

```json
{"message":"Snippet created","snippet":{"code":"println!(\"Hello, world!\");","id":2,"language":"rust","title":"Hello World"}}
```

## What You Built

You now have a REST app that responds to the five basic CRUD paths:

```text
GET    /api/snippets/
POST   /api/snippets/
GET    /api/snippets/{id}/
PUT    /api/snippets/{id}/
DELETE /api/snippets/{id}/
```

Nothing is persisted yet. Refreshing the page, changing the ID, or sending a different request only changes the JSON that this temporary code constructs. That is exactly the limitation we want. In [Part 3: Models and the Database](../3-models-and-database/), you will define the `Snippet` model and create the table that can store these records.
