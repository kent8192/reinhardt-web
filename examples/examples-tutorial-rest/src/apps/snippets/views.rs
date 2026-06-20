use json::json;
use reinhardt::Validate;
use reinhardt::core::serde::json;
use reinhardt::db::DatabaseConnection;
use reinhardt::db::orm::Manager;
use reinhardt::di::Depends;
use reinhardt::http::ViewResult;
use reinhardt::{Json, Path, Response, StatusCode};
use reinhardt::{delete, get, post, put};

// `pre_validate = true` is the preferred declarative validation form (rc.5+).
// It is applied below on `create` — which has a single `Json<SnippetSerializer>`
// extractor — and skipped on `update`, which mixes `Path<i64>` and
// `Json<SnippetSerializer>`.
//
// The current route macro validates *every* extractor on the handler when
// `pre_validate = true`, calling `Validate::validate(&*tmp)` on each
// dereferenced extractor (see `reinhardt-core/macros/src/routes.rs` around
// line 493). `Path<i64>` derefs to `i64`, and `i64` does not implement
// `Validate`, so enabling `pre_validate` on `update` would fail to compile
// (`the trait Validate is not implemented for i64`). Until the macro grows
// per-parameter opt-in (e.g. `#[validate]` on individual extractor params
// or a Validate blanket impl for primitives), `update` keeps the manual
// `serializer.validate()?` call below — hence the `use reinhardt::Validate`
// import above is intentional.

use super::di::{CheckedSnippetListConfigKey, ConfigError, SnippetListConfig};
use super::models::Snippet;
use super::serializers::{SnippetResponse, SnippetSerializer};

/// Snippet listing configuration endpoint.
///
/// Demonstrates keyed `Depends<K, T>` for a fallible provider. The
/// `checked_list_config` provider in `di.rs` returns
/// `FactoryOutput<CheckedSnippetListConfigKey, Result<SnippetListConfig,
/// ConfigError>>`, so the key type distinguishes this provider from any other
/// `SnippetListConfig` provider.
///
/// Registered before `retrieve` (`/snippets/{id}/`) in `urls.rs` so this
/// literal `/snippets/config/` path is matched first.
///
/// GET /snippets/config/
/// Success response: 200 OK with `{ "max_page_size": <usize> }`
/// Error response: 503 Service Unavailable with `{ "error": <message> }`
#[get("/snippets/config/", name = "snippets-config")]
pub async fn config(
	#[inject] cfg: Depends<CheckedSnippetListConfigKey, Result<SnippetListConfig, ConfigError>>,
) -> ViewResult<Response> {
	// `Depends<K, Result<T, E>>` derefs to `Result<T, E>`. `.as_ref()` matches
	// on `Result<&SnippetListConfig, &ConfigError>` without consuming `cfg`.
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

/// List all snippets
///
/// GET /snippets/
/// Success response: 200 OK with array of snippets
#[get("/snippets/", name = "snippets-list")]
pub async fn list(#[inject] db: DatabaseConnection) -> ViewResult<Response> {
	let snippets = Manager::<Snippet>::new().all().all_with_db(&db).await?;
	let snippet_responses: Vec<SnippetResponse> =
		snippets.iter().map(SnippetResponse::from_model).collect();

	let response_data = json!({ "snippets": snippet_responses });
	let json = json::to_string(&response_data)?;
	Ok(Response::new(StatusCode::OK)
		.with_header("Content-Type", "application/json")
		.with_body(json))
}

/// Create a new snippet
///
/// POST /snippets/
/// Request body: JSON with title, code, language fields
/// Success response: 201 Created with created snippet
/// Error responses:
/// - 400 Bad Request: Validation errors (emitted by the `pre_validate = true`
///   macro option below — the macro returns HTTP 400 with a JSON error body
///   before this function body runs)
#[post("/snippets/", name = "snippets-create", pre_validate = true)]
pub async fn create(
	Json(serializer): Json<SnippetSerializer>,
	#[inject] db: DatabaseConnection,
) -> ViewResult<Response> {
	// `pre_validate = true` on the route macro extracts `Json<SnippetSerializer>`
	// into a temporary, calls `Validate::validate(&__tmp)`, then re-destructures
	// into the original `Json(serializer)` binding. No manual `serializer.validate()?`
	// is needed (the previous explicit call was redundant once `pre_validate`
	// was introduced in rc.5).

	// `Snippet::build()` is the macro-generated typestate builder (see
	// `#[model]` on `Snippet`). `id` defaults to `0`, which `create_with_conn`
	// recognizes as "auto-increment" and omits from the INSERT statement.
	// `created_at` is stamped client-side with `Utc::now()` by the builder
	// because the field is declared `auto_now_add = true`, so it is sent as a
	// concrete value in the INSERT.
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

/// Retrieve a specific snippet
///
/// GET /snippets/{id}/
/// Success response: 200 OK with snippet data
/// Error responses:
/// - 404 Not Found: Snippet not found
#[get("/snippets/{id}/", name = "snippets-retrieve")]
pub async fn retrieve(
	Path(snippet_id): Path<i64>,
	#[inject] db: DatabaseConnection,
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

/// Update a snippet
///
/// PUT /snippets/{id}/
/// Request body: JSON with title, code, language fields
/// Success response: 200 OK with updated snippet
/// Error responses:
/// - 404 Not Found: Snippet not found
/// - 422 Unprocessable Entity: Validation errors (manual `serializer.validate()?`
///   below — `pre_validate = true` would force `Path<i64>` through `Validate`
///   as well, which `i64` does not implement; see the module-level comment
///   on the `Validate` import for details)
#[put("/snippets/{id}/", name = "snippets-update")]
pub async fn update(
	Path(snippet_id): Path<i64>,
	Json(serializer): Json<SnippetSerializer>,
	#[inject] db: DatabaseConnection,
) -> ViewResult<Response> {
	// Manual validation — see module-level comment on why `pre_validate = true`
	// is not used here.
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

/// Delete a snippet
///
/// DELETE /snippets/{id}/
/// Success response: 204 No Content
/// Error responses:
/// - 404 Not Found: Snippet not found
#[delete("/snippets/{id}/", name = "snippets-delete")]
pub async fn delete(
	Path(snippet_id): Path<i64>,
	#[inject] db: DatabaseConnection,
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

	// Return 204 No Content for successful deletion
	Ok(Response::new(StatusCode::NO_CONTENT))
}

// ============================================================================
// ViewSet Implementation (Tutorial 6)
// ============================================================================

/// ViewSet-based approach for managing snippets (Tutorial 6)
///
/// This demonstrates the ViewSet pattern from Tutorial 6, which provides:
/// - Automatic CRUD operations (list, create, retrieve, update, delete)
/// - Built-in pagination support
/// - Built-in filtering and ordering
/// - Significantly less code compared to function-based views above
///
/// Compare this implementation (~15 lines) with the function-based views above (~200 lines)
/// for the same functionality!
///
/// Features enabled:
/// - Pagination: 10 items per page (max 100)
/// - Filtering: by language and title fields
/// - Ordering: by created_at and title fields
///
/// # Runtime behavior (rc.23+)
///
/// Starting with reinhardt-web rc.23 (discussion-tracked under the
/// `Breaking Changes` category as the "ModelViewSet performs real CRUD" fix),
/// `ModelViewSet` and `ReadOnlyModelViewSet` no longer return skeleton
/// `[]` / `{}` responses. The generated handlers issue real database queries
/// through the `Snippet` model's manager (and honour `with_filters`,
/// `with_ordering`, and `with_pagination` end-to-end), so the ViewSet
/// endpoints registered under `/api/snippets-viewset/` require a working
/// database backend with the `snippets` schema migrated:
///
/// ```text
/// cargo run --bin manage -- migrate
/// cargo run --bin manage -- runserver
/// ```
///
/// Both the function-based endpoints (under `/api/snippets/`) and the
/// ViewSet endpoints (under `/api/snippets-viewset/`) are served by the
/// same process — `crate::apps::snippets::urls::url_patterns` registers
/// them on a single `ServerRouter` with no `USE_VIEWSET`-style toggle.
/// Like the function-based views above, the ViewSet path queries the real
/// database, so both observe an empty list until rows are inserted into the
/// `snippets` table.
#[reinhardt::viewset(basename = "snippet")]
pub fn viewset() -> reinhardt::ModelViewSet<Snippet, reinhardt::JsonSerializer<Snippet>> {
	use reinhardt::ModelViewSet;
	use reinhardt::views::viewsets::{FilterConfig, OrderingConfig, PaginationConfig};

	ModelViewSet::<Snippet, reinhardt::JsonSerializer<Snippet>>::new("snippet")
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
