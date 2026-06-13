+++
title = "Part 4: Dependency Injection"
description = "Inject a DatabaseConnection, query the real ORM, and register a fallible DI factory."
weight = 40
+++

# Part 4: Dependency Injection

Part 2 gave you static JSON. Part 3 gave you a database table. Now let's connect them.

This is the central chapter of the REST tutorial. Reinhardt handlers are plain async functions, but they can ask the framework for dependencies with `#[inject]`. We will inject a `DatabaseConnection`, use the ORM manager, and add a small configuration endpoint that shows how factory return types are keyed in the DI registry.

## Add the Support Serializer Module

The real handlers use `SnippetSerializer` for input and `SnippetResponse` for output. Create `src/apps/snippets/serializers.rs` now; Part 5 explains the validation rules in detail.

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

This is supporting code for the DI work below. The key point for now is that handlers can receive a parsed `Json<SnippetSerializer>` and can turn database models into JSON-friendly `SnippetResponse` values.

## Register a DI Module

Open `src/apps/snippets.rs` and make sure it declares the new module:

```rust
use reinhardt::app_config;

pub mod di;
pub mod models;
pub mod serializers;
pub mod urls;
pub mod views;

#[app_config(name = "snippets", label = "snippets")]
pub struct SnippetsConfig;
```

Now create `src/apps/snippets/di.rs`:

```rust
use reinhardt::di::{Depends, injectable, injectable_factory};

/// Snippet listing configuration resolved through DI.
#[injectable(scope = "singleton")]
pub struct SnippetListConfig {
	#[no_inject]
	pub max_page_size: usize,
}

impl Default for SnippetListConfig {
	fn default() -> Self {
		Self { max_page_size: 50 }
	}
}

/// Error type local to `checked_list_config`.
#[derive(Debug)]
pub struct ConfigError(pub String);

/// Fallible variant of `SnippetListConfig`, registered under the
/// `Result<SnippetListConfig, ConfigError>` key.
#[injectable_factory(scope = "singleton")]
async fn checked_list_config(
	#[inject] base: Depends<SnippetListConfig>,
) -> Result<SnippetListConfig, ConfigError> {
	if base.max_page_size == 0 {
		return Err(ConfigError("max_page_size must be positive".into()));
	}
	Ok(SnippetListConfig {
		max_page_size: base.max_page_size,
	})
}
```

`#[injectable]` is for types you own and can annotate directly. `SnippetListConfig` has no injected fields, so the macro builds it from `Default`; `#[no_inject]` tells the macro not to try to resolve `usize` from the container.

`#[injectable_factory]` is for async construction, validation, or types you cannot annotate directly. Factories can depend on other dependencies through `#[inject]` parameters. Here, `checked_list_config` asks for the plain config and returns a checked `Result`.

## Understand Scopes and Caching

Both DI macros accept a scope:

```rust
#[injectable(scope = "singleton")]
#[injectable(scope = "request")]
#[injectable(scope = "transient")]

#[injectable_factory(scope = "singleton")]
#[injectable_factory(scope = "request")]
#[injectable_factory(scope = "transient")]
```

Use them this way:

- `singleton`: build once and reuse. Good for immutable configuration.
- `request`: build once per request. Good for request-local context.
- `transient`: build every time it is resolved. Good for short-lived values that need unique mutable ownership.

Within one resolution path, dependencies are cached by default according to their scope. If you need a fresh value even when a cached value exists, write the injection as:

```rust
#[inject(cache = false)] fresh: Depends<MyDependency>
```

Most handlers should not need `cache = false`. It is an escape hatch, not the normal style.

## Know What the Registry Key Is

This is the part that saves you from confusing DI bugs.

Reinhardt keys each factory by the `TypeId` of its literal return type. A factory returning `SnippetListConfig` and a factory returning `SnippetListConfig` are competing for the same key. One will shadow or collide with the other.

When you need a second flavor of the same success value, use one of these patterns:

- Introduce a dedicated newtype, such as `CheckedSnippetListConfig(SnippetListConfig)`.
- Return `Result<T, FactoryLocalError>` where the error type is local to that factory.

The example uses the second pattern:

```rust
pub struct ConfigError(pub String);

#[injectable_factory(scope = "singleton")]
async fn checked_list_config(
	#[inject] base: Depends<SnippetListConfig>,
) -> Result<SnippetListConfig, ConfigError> {
	/* ... */
}
```

The plain config is registered as `TypeId::of::<SnippetListConfig>()`. The checked config is registered as `TypeId::of::<Result<SnippetListConfig, ConfigError>>()`. Those are different keys even though the success type is the same.

`DependsResult<T, E>` is a sugar alias for `Depends<Result<T, E>>`. It works in `#[injectable]` fields and `#[injectable_factory]` parameters. In route handlers, spell out the literal `Depends<Result<T, E>>` form. The route macro currently recognizes only the single-generic-argument shape `Depends<T>`.

## Respect the Pseudo Orphan Rule

Reinhardt protects framework-owned dependency types from project-local overrides. Types whose Rust path starts with framework prefixes such as `reinhardt::` or `reinhardt_*::` are treated as framework-managed.

That is why generated project and app names cannot start with `reinhardt_` or `reinhardt-`. Use names such as `tutorial`, `snippets`, or your product name. Do not create project-local crates or modules that pretend to be framework namespaces.

## Rewrite the Handlers

Replace `src/apps/snippets/views.rs` with the real database-backed version.

Start with the imports:

```rust
use json::json;
use reinhardt::Validate;
use reinhardt::core::serde::json;
use reinhardt::db::DatabaseConnection;
use reinhardt::db::orm::Manager;
use reinhardt::di::Depends;
use reinhardt::http::ViewResult;
use reinhardt::{Json, Path, Response, StatusCode};
use reinhardt::{delete, get, post, put};

use super::di::{ConfigError, SnippetListConfig};
use super::models::Snippet;
use super::serializers::{SnippetResponse, SnippetSerializer};
```

The new import is `Depends`, plus the database connection and ORM manager. The handlers will not create connections by hand. They ask for one:

```rust
#[get("/snippets/", name = "snippets-list")]
pub async fn list(#[inject] db: Depends<DatabaseConnection>) -> ViewResult<Response> {
	let snippets = Manager::<Snippet>::new().all().all_with_db(&db).await?;
	let snippet_responses: Vec<SnippetResponse> =
		snippets.iter().map(SnippetResponse::from_model).collect();

	let response_data = json!({ "snippets": snippet_responses });
	let json = json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}
```

`Depends<T>` implements `Deref<Target = T>`. When `all_with_db` wants `&DatabaseConnection`, passing `&db` works by deref coercion from `&Depends<DatabaseConnection>` to `&DatabaseConnection`. If you want to be explicit, write `&*db`: `*db` reaches the inner connection and `&*db` borrows it again.

Now replace `create` with a real insert:

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

The handler receives the JSON body and the database connection in the same function signature. The route macro extracts the body; the DI runtime resolves the connection.

Add retrieve:

```rust
#[get("/snippets/{id}/", name = "snippets-retrieve")]
pub async fn retrieve(
	Path(snippet_id): Path<i64>,
	#[inject] db: Depends<DatabaseConnection>,
) -> ViewResult<Response> {
	let snippets = Manager::<Snippet>::new()
		.get(snippet_id)
		.all_with_db(&db)
		.await?;

	let snippet = match snippets.first() {
		Some(snippet) => snippet,
		None => {
			let error = json::to_string(&json!({"error": "Snippet not found"}))?;
			return Ok(Response::new(StatusCode::NOT_FOUND)
				.with_header("Content-Type", "application/json")
				.with_body(error));
		}
	};

	let response_data = json!({
		"snippet": SnippetResponse::from_model(snippet)
	});

	let json = json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}
```

Add update:

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

We validate manually here because this handler mixes `Path<i64>` and `Json<SnippetSerializer>`. Part 5 explains the `pre_validate = true` limitation that makes this necessary.

Add delete:

```rust
#[delete("/snippets/{id}/", name = "snippets-delete")]
pub async fn delete(
	Path(snippet_id): Path<i64>,
	#[inject] db: Depends<DatabaseConnection>,
) -> ViewResult<Response> {
	let manager = Manager::<Snippet>::new();
	let existing = manager.get(snippet_id).all_with_db(&db).await?;

	if existing.is_empty() {
		let error = json::to_string(&json!({"error": "Snippet not found"}))?;
		return Ok(Response::new(StatusCode::NOT_FOUND)
			.with_header("Content-Type", "application/json")
			.with_body(error));
	}

	manager.delete_with_conn(&db, snippet_id).await?;

	Ok(Response::new(StatusCode::NO_CONTENT))
}
```

## Add the Config Endpoint

Now add the endpoint that exercises the fallible factory:

```rust
#[get("/snippets/config/", name = "snippets-config")]
pub async fn config(
	#[inject] cfg: Depends<Result<SnippetListConfig, ConfigError>>,
) -> ViewResult<Response> {
	match (*cfg).as_ref() {
		Ok(cfg) => {
			let body = json::to_string(&json!({ "max_page_size": cfg.max_page_size }))?;
			Ok(Response::new(StatusCode::OK)
				.with_header("Content-Type", "application/json")
				.with_body(body))
		}
		Err(ConfigError(msg)) => {
			let body = json::to_string(&json!({ "error": msg }))?;
			Ok(Response::new(StatusCode::SERVICE_UNAVAILABLE)
				.with_header("Content-Type", "application/json")
				.with_body(body))
		}
	}
}
```

The signature is intentionally `Depends<Result<SnippetListConfig, ConfigError>>`, not `DependsResult<SnippetListConfig, ConfigError>`. The alias names the idea, but route handlers need the literal form today.

The match uses `(*cfg).as_ref()`. `*cfg` dereferences `Depends<_>` into the inner `Result<_, _>`, and `.as_ref()` lets the handler inspect `Ok(&SnippetListConfig)` or `Err(&ConfigError)` without consuming the dependency wrapper.

## Register the Routes

Update `src/apps/snippets/urls.rs`:

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
}
```

`config` is registered before `retrieve`. That matters because `/snippets/config/` is a literal path, while `/snippets/{id}/` is dynamic. The literal route should get the first chance to match.

## Run It

Apply the migration and start the server:

```bash
cargo make migrate
cargo make runserver
```

In another terminal, list snippets:

```bash
curl http://127.0.0.1:8000/api/snippets/
```

On a clean database, you should see:

```json
{"snippets":[]}
```

Create one:

```bash
curl -X POST http://127.0.0.1:8000/api/snippets/ \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Hello World",
    "code": "println!(\"Hello, world!\");",
    "language": "rust"
  }'
```

You should see `201 Created` with the created snippet. Now check the DI demonstration endpoint:

```bash
curl http://127.0.0.1:8000/api/snippets/config/
```

Expected response:

```json
{"max_page_size":50}
```

Finally, run a compile check:

```bash
cargo check --all-features
```

## What You Built

You replaced the temporary Part 2 handlers with real database-backed CRUD:

- Handlers receive `#[inject] db: Depends<DatabaseConnection>`.
- `Depends<T>` dereferences to `T`, so `&db`, `&*db`, and `(*cfg).as_ref()` are the tools you need.
- `Manager::<Snippet>` queries, inserts, updates, and deletes rows through the injected connection.
- `di.rs` registers a plain singleton and a fallible factory.
- `Result<T, FactoryLocalError>` gives the checked factory a distinct registry key.
- Route handlers spell fallible dependencies as `Depends<Result<T, E>>`.

In [Part 5: Serializers and Validation](../5-serializers-and-validation/), you will focus on the request and response structs we used here and make the validation behavior explicit.
