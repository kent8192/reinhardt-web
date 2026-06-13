+++
title = "Part 5: Serializers and Validation"
description = "Validate request bodies, serialize responses, and choose status codes for API errors."
weight = 50

[extra]
sidebar_weight = 50
+++

# Part 5: Serializers and Validation

Part 4 moved the handlers onto the real database. It also introduced `SnippetSerializer` and `SnippetResponse` so the handlers had typed request and response shapes.

Now let's slow down and look at those types. Serializers are where your API boundary becomes explicit: what fields a client may send, what fields a client receives, and what happens when the payload is invalid.

## Read the Input Serializer

Open `src/apps/snippets/serializers.rs`:

```rust
use reinhardt::Validate;
use serde::{Deserialize, Serialize};

/// Serializer for creating/updating snippets
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SnippetSerializer {
	#[validate(length(
		min = 1,
		max = 100,
		message = "Title must be between 1 and 100 characters"
	))]
	pub title: String,

	#[validate(length(
		min = 1,
		max = 10000,
		message = "Code must be between 1 and 10000 characters"
	))]
	pub code: String,

	#[validate(length(
		min = 1,
		max = 50,
		message = "Language must be between 1 and 50 characters"
	))]
	pub language: String,
}
```

`Deserialize` lets `Json<SnippetSerializer>` parse a request body into this type. `Serialize` is useful in tests and error reporting. `Validate` is the important derive for this chapter: it turns the `#[validate(...)]` field attributes into a `validate()` method.

The serializer is deliberately smaller than the model. Clients may send `title`, `code`, and `language`. They do not send `id`, `created_at`, or `highlighted`; those belong to the server.

## Read the Response Serializer

The response type goes in the same file:

```rust
/// Response serializer for snippets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetResponse {
	pub id: i64,
	pub title: String,
	pub code: String,
	pub language: String,
	pub highlighted: String,
}

impl SnippetResponse {
	pub fn from_model(snippet: &super::models::Snippet) -> Self {
		Self {
			id: snippet.id,
			title: snippet.title.clone(),
			code: snippet.code.clone(),
			language: snippet.language.clone(),
			highlighted: snippet.highlighted(),
		}
	}
}
```

This is the public shape of a snippet returned by the API. The `from_model` method keeps that transformation in one place: handlers do not need to repeat field-by-field JSON construction.

## Use Declarative Validation on Create

Look back at `create` in `src/apps/snippets/views.rs`:

```rust
#[post("/snippets/", name = "snippets-create", pre_validate = true)]
pub async fn create(
	Json(serializer): Json<SnippetSerializer>,
	#[inject] db: Depends<DatabaseConnection>,
) -> ViewResult<Response> {
	let snippet = Snippet::build()
		.title(serializer.title.clone())
		.code(serializer.code.clone())
		.language(serializer.language.clone())
		.finish();

	let created = Manager::<Snippet>::new()
		.create_with_conn(&db, &snippet)
		.await?;

	let response_data = json!({
		"message": "Snippet created",
		"snippet": SnippetResponse::from_model(&created)
	});

	let json = json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::CREATED)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}
```

`pre_validate = true` tells the route macro to validate the extracted request data before your function body runs. For this handler, there is only one extractor that needs validation: `Json<SnippetSerializer>`. That is the clean case.

When validation fails here, the macro returns `400 Bad Request` with a JSON error body. Your handler does not insert anything and does not run any of the code after the signature.

## Validate Manually on Update

The `update` handler looks similar, but it cannot use `pre_validate = true` today:

```rust
#[put("/snippets/{id}/", name = "snippets-update")]
pub async fn update(
	Path(snippet_id): Path<i64>,
	Json(serializer): Json<SnippetSerializer>,
	#[inject] db: Depends<DatabaseConnection>,
) -> ViewResult<Response> {
	serializer.validate()?;

	let manager = Manager::<Snippet>::new();
	let existing = manager.get(snippet_id).all_with_db(&db).await?;

	let mut snippet = match existing.into_iter().next() {
		Some(snippet) => snippet,
		None => {
			let error = json::to_string(&json!({"error": "Snippet not found"}))?;
			return Ok(Response::new(StatusCode::NOT_FOUND)
				.with_header("Content-Type", "application/json")
				.with_body(error));
		}
	};

	snippet.title = serializer.title.clone();
	snippet.code = serializer.code.clone();
	snippet.language = serializer.language.clone();

	let updated = manager.update_with_conn(&db, &snippet).await?;

	let response_data = json!({
		"message": "Snippet updated",
		"snippet": SnippetResponse::from_model(&updated)
	});

	let json = json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}
```

The reason is in the module comment in the reference example:

```rust
// `pre_validate = true` is the preferred declarative validation form.
// It is applied on `create`, which has a single `Json<SnippetSerializer>`
// extractor, and skipped on `update`, which mixes `Path<i64>` and
// `Json<SnippetSerializer>`.
//
// The current route macro validates every extractor on the handler when
// `pre_validate = true`. `Path<i64>` derefs to `i64`, and `i64` does not
// implement `Validate`, so enabling `pre_validate` on `update` would fail
// to compile. Until the macro grows per-parameter validation, `update`
// keeps the manual `serializer.validate()?` call.
```

That is the practical rule: use `pre_validate = true` when the handler's extractors are all validatable in the way the macro expects. When a handler mixes a path parameter with a body serializer, validate the body manually.

Manual validation errors from `serializer.validate()?` are surfaced as `422 Unprocessable Entity`.

## Return 404 for Missing Rows

Validation answers the question, "is this request shape acceptable?" It does not answer, "does this row exist?"

For retrieve, update, and delete, keep the not-found branch explicit:

```rust
let snippet = match snippets.first() {
	Some(snippet) => snippet,
	None => {
		let error = json::to_string(&json!({"error": "Snippet not found"}))?;
		return Ok(Response::new(StatusCode::NOT_FOUND)
			.with_header("Content-Type", "application/json")
			.with_body(error));
	}
};
```

That produces `404 Not Found` with a stable JSON body:

```json
{"error":"Snippet not found"}
```

Do not blur validation errors and missing rows together. They mean different things to API clients.

## Status Code Summary

The function-based endpoints now use these status codes:

| Situation | Status |
|---|---|
| List snippets | `200 OK` |
| Create snippet | `201 Created` |
| Retrieve snippet | `200 OK` |
| Update snippet | `200 OK` |
| Delete snippet | `204 No Content` |
| Invalid JSON shape or `pre_validate` failure on create | `400 Bad Request` |
| Manual serializer validation failure on update | `422 Unprocessable Entity` |
| Missing snippet ID | `404 Not Found` |

This is not decoration. Clear status choices let clients distinguish "fix your payload" from "the resource is not here" from "the operation succeeded and there is no body."

## Try the Validation Paths

Start from a migrated database and running server:

```bash
cargo make migrate
cargo make runserver
```

Send an invalid create request:

```bash
curl -i -X POST http://127.0.0.1:8000/api/snippets/ \
  -H "Content-Type: application/json" \
  -d '{
    "title": "",
    "code": "println!(\"Hello\");",
    "language": "rust"
  }'
```

You should see `400 Bad Request`.

Now send an invalid update request:

```bash
curl -i -X PUT http://127.0.0.1:8000/api/snippets/1/ \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Hello",
    "code": "",
    "language": "rust"
  }'
```

You should see `422 Unprocessable Entity` because the handler validates the body before it looks up the row.

Finally, request an ID that is not present:

```bash
curl -i http://127.0.0.1:8000/api/snippets/999999/
```

You should see `404 Not Found` and:

```json
{"error":"Snippet not found"}
```

Run the compile check:

```bash
cargo check --all-features
```

## What You Built

You now have:

- A request serializer that validates client-supplied `title`, `code`, and `language`
- A response serializer that exposes `id`, `title`, `code`, `language`, and `highlighted`
- Declarative validation on `create` with `pre_validate = true`
- Manual validation on `update` because `Path<i64>` cannot be validated by the current macro
- Explicit status-code behavior for success, validation errors, and missing rows

The function-based API is complete. In [Part 6: Bonus: ViewSets and Routers](../6-viewsets-and-routers/), you will see how much of this CRUD surface `ModelViewSet` can generate for you.
