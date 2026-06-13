+++
title = "Quickstart"
weight = 5

[extra]
sidebar_weight = 10
+++

# Quickstart

Build the same code-snippet API used by
[`examples/examples-tutorial-rest`](https://github.com/kent8192/reinhardt-web/tree/main/examples/examples-tutorial-rest).
The example is a REST-only crate: it has a `snippets` app, function-based
CRUD endpoints, a `ModelViewSet`, Bruno collections, migrations, and no
WASM client.

## Project Setup

Install the global tool. The command below pins this tutorial to the
documented release for reproducibility; omit `--version` to let Cargo
choose the latest stable release.

<!-- reinhardt-version-sync -->
```bash
# Terminal: project root
cargo install reinhardt-admin-cli --version "0.2.0-rc.6"
```

Create a new Reinhardt REST project:

```bash
# Terminal: project root
reinhardt-admin startproject tutorial --template rest
cd tutorial
```

The reference example has this shape:

```text
# Project tree: quickstart
examples-tutorial-rest/
├── Cargo.toml
├── Makefile.toml
├── Dockerfile
├── Dockerfile.bruno
├── docker-compose.api-tests.yml
├── bruno/
├── migrations/
│   ├── auth/0001_initial.rs
│   ├── default/0001_initial.rs
│   └── snippets/0001_initial.rs
├── settings/
│   ├── base.toml
│   └── ci.toml
├── src/
│   ├── apps.rs
│   ├── config.rs
│   ├── lib.rs
│   ├── apps/
│   │   ├── snippets.rs
│   │   └── snippets/
│   │       ├── models.rs
│   │       ├── serializers.rs
│   │       ├── urls.rs
│   │       └── views.rs
│   ├── config/
│   │   ├── apps.rs
│   │   ├── settings.rs
│   │   └── urls.rs
│   └── bin/
│       └── manage.rs
└── tests/
    └── integration.rs
```

## Snippet Model

Create a `snippets` app and define the `Snippet` model in
`src/apps/snippets/models.rs`:

```rust
// File: src/apps/snippets/models.rs
use chrono::{DateTime, Utc};
use reinhardt::core::serde::{Deserialize, Serialize};
use reinhardt::prelude::*;

/// Snippet model representing a code snippet
#[model(app_label = "snippets", table_name = "snippets")]
#[derive(Serialize, Deserialize)]
pub struct Snippet {
	#[field(primary_key = true)]
	pub id: i64,

	#[field(max_length = 100)]
	pub title: String,

	#[field(max_length = 10000)]
	pub code: String,

	#[field(max_length = 50)]
	pub language: String,

	#[field(auto_now_add = true)]
	pub created_at: DateTime<Utc>,
}
```

The full example also adds `Snippet::highlighted()` using `syntect`; the
REST response serializer calls it to return highlighted HTML alongside
the raw code.

## Serializers

`src/apps/snippets/serializers.rs` defines one input serializer and one
response shape:

```rust
// File: src/apps/snippets/serializers.rs
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

/// Response serializer for snippets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetResponse {
	pub id: i64,
	pub title: String,
	pub code: String,
	pub language: String,
	pub highlighted: String,
}
```

## Function-Based Endpoints

`src/apps/snippets/views.rs` exposes five CRUD handlers:

```rust
// File: src/apps/snippets/serializers.rs
use chrono::Utc;
use json::json;
use reinhardt::Validate;
use reinhardt::core::serde::json;
use reinhardt::http::ViewResult;
use reinhardt::{Json, Path, Response, StatusCode};
use reinhardt::{delete, get, post, put};

use super::models::Snippet;
use super::serializers::{SnippetResponse, SnippetSerializer};
```

The create endpoint uses declarative validation:

```rust
// File: src/apps/snippets/views.rs
/// Create a new snippet
///
/// POST /snippets/
/// Request body: JSON with title, code, language fields
/// Success response: 201 Created with created snippet
#[post("/snippets/", name = "snippets-create", pre_validate = true)]
pub async fn create(Json(serializer): Json<SnippetSerializer>) -> ViewResult<Response> {
	let snippet = Snippet::build()
		.title(serializer.title.clone())
		.code(serializer.code.clone())
		.language(serializer.language.clone())
		.finish();

	let response_data = json!({
		"message": "Snippet created",
		"snippet": SnippetResponse::from_model(&snippet)
	});

	let json = json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::CREATED)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}
```

`list`, `retrieve`, `update`, and `delete` use the same explicit
`Response` style. `update` performs manual `serializer.validate()?`
because it combines `Path<i64>` and `Json<SnippetSerializer>`; enabling
`pre_validate = true` there would also try to validate the path
extractor.

## URL Configuration

Register the snippets endpoints in `src/apps/snippets/urls.rs`:

```rust
// File: src/apps/snippets/urls.rs
use reinhardt::ServerRouter;

use super::views;

/// Register every snippets-app URL on a single `ServerRouter`.
pub fn url_patterns() -> ServerRouter {
	ServerRouter::new()
		.endpoint(views::list)
		.endpoint(views::create)
		.endpoint(views::retrieve)
		.endpoint(views::update)
		.endpoint(views::delete)
		.viewset("/snippets-viewset", views::viewset())
}
```

Then mount the app under `/api/` from `src/config/urls.rs`:

```rust
// File: src/apps/snippets/urls.rs
use reinhardt::prelude::*;
use reinhardt::routes;

#[routes]
pub fn routes() -> UnifiedRouter {
	UnifiedRouter::new().mount("/api/", crate::apps::snippets::urls::url_patterns())
}
```

## Run It

Start the local stack and server:

```bash
# Terminal: project root
cargo make runserver
```

Try the function-based endpoints:

```bash
# Terminal: project root
curl http://127.0.0.1:8000/api/snippets/

curl -X POST http://127.0.0.1:8000/api/snippets/ \
  -H "Content-Type: application/json" \
  -d '{"title":"Hello","code":"fn main() {}","language":"rust"}'
```

Try the ViewSet endpoints, which are mounted in the same process:

```bash
# Terminal: project root
curl http://127.0.0.1:8000/api/snippets-viewset/
curl "http://127.0.0.1:8000/api/snippets-viewset/?language=rust&ordering=-created_at"
```

The Bruno collection under `bruno/` mirrors these flows in the `Snippets
CRUD`, `Snippets ViewSet`, and `Validation Tests` folders.

## Next Steps

Continue with [Tutorial 0: HTTP Macros](./0-http-macros/) for the macro
details, then [Tutorial 6: ViewSets and Routers](./6-viewsets-and-routers/)
for the `ModelViewSet` path.
