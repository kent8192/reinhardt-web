# Part 3: Views and URLs

In this tutorial, we'll create a modern WASM-based frontend using reinhardt-pages with server-side rendering (SSR) support.

## Understanding reinhardt-pages Architecture

reinhardt-pages provides a reactive frontend framework with three layers:

- **`client/`**: WASM UI components that run in the browser
- **`server/`**: Server functions that run on the server
- **`shared/`**: Common types used by both client and server

This architecture enables:
- **Type-safe RPC**: Server functions are called from WASM like regular async functions
- **SSR support**: Components can be pre-rendered on the server
- **Reactive UI**: State management with `use_state()` hooks

## Project Setup

### 1. Update Cargo.toml

Add WASM support and reinhardt-pages dependency:

```toml
[lib]
crate-type = ["cdylib", "rlib"]  # cdylib for WASM, rlib for server

# WASM-specific dependencies
[target.'cfg(target_arch = "wasm32")'.dependencies]
reinhardt-pages = { workspace = true }
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = [
	"Window", "Document", "Element",
	"HtmlFormElement", "HtmlInputElement",
	"Event", "EventTarget",
] }
console_error_panic_hook = "0.1"

# Server-specific dependencies
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
reinhardt = { workspace = true, features = ["full", "pages"] }
tokio = { version = "1", features = ["full"] }
```

### 2. Create Build Configuration

Create `index.html`:

```html
<!DOCTYPE html>
<html lang="en">
<head>
	<meta charset="UTF-8">
	<meta name="viewport" content="width=device-width, initial-scale=1.0">
	<title>Polls App - Reinhardt Tutorial</title>
	<link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet">
</head>
<body>
	<div id="root">
		<div class="container mt-5 text-center">
			<div class="spinner-border text-primary" role="status">
				<span class="visually-hidden">Loading...</span>
			</div>
		</div>
	</div>
	<script type="module">
		// wasm-bindgen generated module
		import init from './polls_app.js';
		init();
	</script>
</body>
</html>
```

### 3. Create Directory Structure

```bash
mkdir -p src/client/components
mkdir -p src/server_fn
mkdir -p src/shared
```

Update `src/lib.rs`:

```rust
// Client-side modules (WASM only)
#[cfg(target_arch = "wasm32")]
pub mod client;

// Server function definitions (both WASM and server)
pub mod server_fn;

// Shared types (both WASM and server)
pub mod shared;

// Existing modules
pub mod apps;
pub mod config;
```

## Creating Shared Types

Create `src/shared.rs`:

```rust
pub mod types;
```

Create `src/shared/types.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionInfo {
	pub id: i64,
	pub question_text: String,
	pub pub_date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceInfo {
	pub id: i64,
	pub question_id: i64,
	pub choice_text: String,
	pub votes: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRequest {
	pub question_id: i64,
	pub choice_id: i64,
}

// Server-side conversions (not available in WASM)
#[cfg(not(target_arch = "wasm32"))]
impl From<crate::apps::polls::models::Question> for QuestionInfo {
	fn from(question: crate::apps::polls::models::Question) -> Self {
		QuestionInfo {
			id: question.id,
			question_text: question.question_text,
			pub_date: question.pub_date,
		}
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl From<crate::apps::polls::models::Choice> for ChoiceInfo {
	fn from(choice: crate::apps::polls::models::Choice) -> Self {
		ChoiceInfo {
			id: choice.id,
			question_id: choice.question_id,
			choice_text: choice.choice_text,
			votes: choice.votes,
		}
	}
}
```

## Implementing Server Functions

Create `src/server_fn.rs`:

```rust
pub mod polls;
```

Create `src/server_fn/polls.rs`:

```rust
use crate::shared::types::{ChoiceInfo, QuestionInfo, VoteRequest};

// Re-export server_fn types
#[cfg(not(target_arch = "wasm32"))]
use reinhardt::pages::server_fn::{server_fn, ServerFnError};
#[cfg(target_arch = "wasm32")]
use reinhardt_pages::server_fn::{server_fn, ServerFnError};

/// Get all questions (latest 5)
#[cfg(not(target_arch = "wasm32"))]
#[server_fn(use_inject = true)]
pub async fn get_questions(
	#[inject] _db: reinhardt::DatabaseConnection,
) -> std::result::Result<Vec<QuestionInfo>, ServerFnError> {
	use crate::apps::polls::models::Question;
	use reinhardt::Model;

	let manager = Question::objects();
	let questions = manager.all().all().await
		.map_err(|e| ServerFnError::ServerError(e.to_string()))?;

	let latest: Vec<QuestionInfo> = questions.into_iter().take(5)
		.map(QuestionInfo::from).collect();

	Ok(latest)
}

#[cfg(target_arch = "wasm32")]
#[server_fn]
pub async fn get_questions() -> std::result::Result<Vec<QuestionInfo>, ServerFnError> {
	unreachable!()
}

/// Get question detail with choices
#[cfg(not(target_arch = "wasm32"))]
#[server_fn(use_inject = true)]
pub async fn get_question_detail(
	question_id: i64,
	#[inject] _db: reinhardt::DatabaseConnection,
) -> std::result::Result<(QuestionInfo, Vec<ChoiceInfo>), ServerFnError> {
	use crate::apps::polls::models::{Choice, Question};
	use reinhardt::db::orm::{FilterOperator, FilterValue};
	use reinhardt::Model;

	let question = Question::objects().get(question_id).first().await
		.map_err(|e| ServerFnError::ServerError(e.to_string()))?
		.ok_or_else(|| ServerFnError::ServerError("Question not found".to_string()))?;

	let choices = Choice::objects()
		.filter(Choice::field_question_id(), FilterOperator::Eq, FilterValue::Int(question_id))
		.all().await
		.map_err(|e| ServerFnError::ServerError(e.to_string()))?;

	Ok((QuestionInfo::from(question), choices.into_iter().map(ChoiceInfo::from).collect()))
}

#[cfg(target_arch = "wasm32")]
#[server_fn]
pub async fn get_question_detail(
	_question_id: i64,
) -> std::result::Result<(QuestionInfo, Vec<ChoiceInfo>), ServerFnError> {
	unreachable!()
}

/// Vote for a choice
#[cfg(not(target_arch = "wasm32"))]
#[server_fn(use_inject = true)]
pub async fn vote(
	request: VoteRequest,
	#[inject] _db: reinhardt::DatabaseConnection,
) -> std::result::Result<ChoiceInfo, ServerFnError> {
	use crate::apps::polls::models::Choice;
	use reinhardt::Model;

	let mut choice = Choice::objects().get(request.choice_id).first().await
		.map_err(|e| ServerFnError::ServerError(e.to_string()))?
		.ok_or_else(|| ServerFnError::ServerError("Choice not found".to_string()))?;

	if choice.question_id != request.question_id {
		return Err(ServerFnError::ServerError(
			"Choice does not belong to this question".to_string(),
		));
	}

	choice.votes += 1;
	let updated_choice = Choice::objects().update(&choice).await
		.map_err(|e| ServerFnError::ServerError(e.to_string()))?;

	Ok(ChoiceInfo::from(updated_choice))
}

#[cfg(target_arch = "wasm32")]
#[server_fn]
pub async fn vote(_request: VoteRequest) -> std::result::Result<ChoiceInfo, ServerFnError> {
	unreachable!()
}
```

**Key points:**

- `#[server_fn(use_inject = true)]`: Enables dependency injection for database connections
- `#[inject]` attribute: Automatically injects dependencies like `DatabaseConnection`
- Conditional compilation: Server implementation vs WASM stub
- Type-safe RPC: Client calls server functions as regular async functions

## Creating Client Components

Create `src/client.rs`:

```rust
#[cfg(target_arch = "wasm32")]
pub mod lib;

#[cfg(target_arch = "wasm32")]
pub mod router;

#[cfg(target_arch = "wasm32")]
pub mod pages;

#[cfg(target_arch = "wasm32")]
pub mod components;
```

### Polls Index Component

Create `src/client/components.rs`:

```rust
pub mod polls;
```

Create `src/client/components/polls.rs`:

```rust
use crate::shared::types::{ChoiceInfo, QuestionInfo, VoteRequest};
use reinhardt_pages::component::{ElementView, IntoView, View};
use reinhardt_pages::page;
use reinhardt_pages::reactive::hooks::use_state;

#[cfg(target_arch = "wasm32")]
use {
	crate::server_fn::polls::{get_question_detail, get_question_results, get_questions, vote},
	wasm_bindgen::JsCast,
	wasm_bindgen_futures::spawn_local,
	web_sys::HtmlInputElement,
};

/// Polls index page - List all polls
pub fn polls_index() -> View {
	let (questions, set_questions) = use_state(Vec::<QuestionInfo>::new());
	let (loading, set_loading) = use_state(true);
	let (error, set_error) = use_state(None::<String>);

	#[cfg(target_arch = "wasm32")]
	{
		let set_questions = set_questions.clone();
		let set_loading = set_loading.clone();
		let set_error = set_error.clone();

		spawn_local(async move {
			match get_questions().await {
				Ok(qs) => {
					set_questions(qs);
					set_loading(false);
				}
				Err(e) => {
					set_error(Some(e.to_string()));
					set_loading(false);
				}
			}
		});
	}

	let questions_list = questions.get();
	let loading_state = loading.get();
	let error_state = error.get();

	page!(|questions_list: Vec<QuestionInfo>, loading_state: bool, error_state: Option<String>| {
		div {
			class: "container mt-5",
			h1 {
				class: "mb-4",
				"Polls"
			}

			if let Some(err) = error_state {
				div {
					class: "alert alert-danger",
					{ err }
				}
			}

			if loading_state {
				div {
					class: "text-center",
					div {
						class: "spinner-border text-primary",
						role: "status",
						span {
							class: "visually-hidden",
							"Loading..."
						}
					}
				}
			} else if questions_list.is_empty() {
				p {
					class: "text-muted",
					"No polls are available."
				}
			} else {
				div {
					class: "list-group",
					for question in questions_list {
						a {
							href: format!("/polls/{}/", question.id),
							class: "list-group-item list-group-item-action",
							div {
								class: "d-flex w-100 justify-content-between",
								h5 {
									class: "mb-1",
									{ question.question_text.clone() }
								}
								small {
									{ question.pub_date.format("%Y-%m-%d %H:%M").to_string() }
								}
							}
						}
					}
				}
			}
		}
	})(questions_list, loading_state, error_state)
}

/// Poll detail page - Show question and voting form
pub fn polls_detail(question_id: i64) -> View {
	let (question, set_question) = use_state(None::<QuestionInfo>);
	let (choices, set_choices) = use_state(Vec::<ChoiceInfo>::new());
	let (loading, set_loading) = use_state(true);
	let (error, set_error) = use_state(None::<String>);
	let (selected_choice, set_selected_choice) = use_state(None::<i64>);
	let (submitting, set_submitting) = use_state(false);

	#[cfg(target_arch = "wasm32")]
	{
		let set_question = set_question.clone();
		let set_choices = set_choices.clone();
		let set_loading = set_loading.clone();
		let set_error = set_error.clone();

		spawn_local(async move {
			match get_question_detail(question_id).await {
				Ok((q, cs)) => {
					set_question(Some(q));
					set_choices(cs);
					set_loading(false);
				}
				Err(e) => {
					set_error(Some(e.to_string()));
					set_loading(false);
				}
			}
		});
	}

	#[cfg(target_arch = "wasm32")]
	let on_submit = {
		let set_error = set_error.clone();
		let set_submitting = set_submitting.clone();
		let selected_choice = selected_choice.clone();

		move |event: web_sys::Event| {
			event.prevent_default();

			if let Some(choice_id) = selected_choice.get() {
				let set_error = set_error.clone();
				let set_submitting = set_submitting.clone();

				spawn_local(async move {
					set_submitting(true);
					set_error(None);

					let request = VoteRequest { question_id, choice_id };

					match vote(request).await {
						Ok(_) => {
							if let Some(window) = web_sys::window() {
								let _ = window.location()
									.set_href(&format!("/polls/{}/results/", question_id));
							}
						}
						Err(e) => {
							set_error(Some(e.to_string()));
							set_submitting(false);
						}
					}
				});
			} else {
				set_error(Some("Please select a choice".to_string()));
			}
		}
	};

	#[cfg(not(target_arch = "wasm32"))]
	let on_submit = |_event: web_sys::Event| {};

	let question_opt = question.get();
	let choices_list = choices.get();
	let loading_state = loading.get();
	let error_state = error.get();
	let submitting_state = submitting.get();

	if loading_state {
		return page!(|| {
			div {
				class: "container mt-5 text-center",
				div {
					class: "spinner-border text-primary",
					role: "status",
					span {
						class: "visually-hidden",
						"Loading..."
					}
				}
			}
		})();
	}

	if let Some(err) = error_state.clone() {
		return page!(|err: String, question_id: i64| {
			div {
				class: "container mt-5",
				div {
					class: "alert alert-danger",
					{ err }
				}
				a {
					href: format!("/polls/{}/", question_id),
					class: "btn btn-secondary",
					"Try Again"
				}
				a {
					href: "/",
					class: "btn btn-primary ms-2",
					"Back to Polls"
				}
			}
		})(err, question_id);
	}

	if let Some(q) = question_opt {
		// Build choice radio buttons using ElementView
		let choice_radios: Vec<View> = choices_list.iter().map(|choice| {
			let choice_id = choice.id;
			let choice_text = choice.choice_text.clone();

			#[cfg(target_arch = "wasm32")]
			let on_change = {
				let set_selected_choice = set_selected_choice.clone();
				move |_event: web_sys::Event| {
					set_selected_choice(Some(choice_id));
				}
			};

			#[cfg(not(target_arch = "wasm32"))]
			let on_change = |_event: web_sys::Event| {};

			ElementView::new("div")
				.attr("class", "form-check poll-choice p-3 mb-2 border rounded")
				.child(
					ElementView::new("input")
						.attr("type", "radio")
						.attr("class", "form-check-input")
						.attr("id", &format!("choice{}", choice_id))
						.attr("name", "choice")
						.listener("change", on_change),
				)
				.child(
					ElementView::new("label")
						.attr("class", "form-check-label")
						.attr("for", &format!("choice{}", choice_id))
						.child(choice_text),
				)
				.into_view()
		}).collect();

		ElementView::new("div")
			.attr("class", "container mt-5")
			.child(
				ElementView::new("h1")
					.attr("class", "mb-4")
					.child(&q.question_text),
			)
			.child(
				ElementView::new("form")
					.listener("submit", on_submit)
					.child({
						let mut form_content = ElementView::new("div");

						for choice_radio in choice_radios {
							form_content = form_content.child(choice_radio);
						}

						form_content = form_content.child(
							ElementView::new("div")
								.attr("class", "mt-3")
								.child(
									ElementView::new("button")
										.attr("type", "submit")
										.attr("class", if submitting_state {
											"btn btn-primary disabled"
										} else {
											"btn btn-primary"
										})
										.child(if submitting_state { "Voting..." } else { "Vote" }),
								)
								.child(
									ElementView::new("a")
										.attr("href", "/")
										.attr("class", "btn btn-secondary ms-2")
										.child("Back to Polls"),
								),
						);

						form_content
					}),
			)
			.into_view()
	} else {
		page!(|| {
			div {
				class: "container mt-5",
				div {
					class: "alert alert-warning",
					"Question not found"
				}
				a {
					href: "/",
					class: "btn btn-primary",
					"Back to Polls"
				}
			}
		})()
	}
}

/// Poll results page - Show voting results
pub fn polls_results(question_id: i64) -> View {
	let (question, set_question) = use_state(None::<QuestionInfo>);
	let (choices, set_choices) = use_state(Vec::<ChoiceInfo>::new());
	let (total_votes, set_total_votes) = use_state(0);
	let (loading, set_loading) = use_state(true);
	let (error, set_error) = use_state(None::<String>);

	#[cfg(target_arch = "wasm32")]
	{
		let set_question = set_question.clone();
		let set_choices = set_choices.clone();
		let set_total_votes = set_total_votes.clone();
		let set_loading = set_loading.clone();
		let set_error = set_error.clone();

		spawn_local(async move {
			match get_question_results(question_id).await {
				Ok((q, cs, total)) => {
					set_question(Some(q));
					set_choices(cs);
					set_total_votes(total);
					set_loading(false);
				}
				Err(e) => {
					set_error(Some(e.to_string()));
					set_loading(false);
				}
			}
		});
	}

	let question_opt = question.get();
	let choices_list = choices.get();
	let total = total_votes.get();
	let loading_state = loading.get();
	let error_state = error.get();

	if loading_state {
		return page!(|| {
			div {
				class: "container mt-5 text-center",
				div {
					class: "spinner-border text-primary",
					role: "status",
					span {
						class: "visually-hidden",
						"Loading..."
					}
				}
			}
		})();
	}

	if let Some(err) = error_state {
		return page!(|err: String| {
			div {
				class: "container mt-5",
				div {
					class: "alert alert-danger",
					{ err }
				}
				a {
					href: "/",
					class: "btn btn-primary",
					"Back to Polls"
				}
			}
		})(err);
	}

	if let Some(q) = question_opt {
		page!(|q: QuestionInfo, choices_list: Vec<ChoiceInfo>, total: i32| {
			div {
				class: "container mt-5",
				h1 {
					class: "mb-4",
					{ q.question_text.clone() }
				}
				div {
					class: "card",
					div {
						class: "card-body",
						h5 {
							class: "card-title",
							"Results"
						}
						div {
							class: "list-group list-group-flush",
							for choice in choices_list {
								{
									let percentage = if total > 0 {
										(choice.votes as f64 / total as f64 * 100.0) as i32
									} else {
										0
									};

									page!(|choice: ChoiceInfo, percentage: i32| {
										div {
											class: "list-group-item",
											div {
												class: "d-flex justify-content-between align-items-center mb-2",
												strong { { choice.choice_text.clone() } }
												span {
													class: "badge bg-primary rounded-pill",
													{ format!("{} votes", choice.votes) }
												}
											}
											div {
												class: "progress",
												div {
													class: "progress-bar",
													role: "progressbar",
													style: format!("width: {}%", percentage),
													aria_valuenow: percentage.to_string(),
													aria_valuemin: "0",
													aria_valuemax: "100",
													{ format!("{}%", percentage) }
												}
											}
										}
									})(choice, percentage)
								}
							}
						}
						div {
							class: "mt-3",
							p {
								class: "text-muted",
								{ format!("Total votes: {}", total) }
							}
						}
					}
				}
				div {
					class: "mt-3",
					a {
						href: format!("/polls/{}/", q.id),
						class: "btn btn-primary",
						"Vote Again"
					}
					a {
						href: "/",
						class: "btn btn-secondary ms-2",
						"Back to Polls"
					}
				}
			}
		})(q, choices_list, total)
	} else {
		page!(|| {
			div {
				class: "container mt-5",
				div {
					class: "alert alert-warning",
					"Question not found"
				}
				a {
					href: "/",
					class: "btn btn-primary",
					"Back to Polls"
				}
			}
		})()
	}
}
```

**Component patterns:**

- **`page!` macro**: JSX-like syntax for simple HTML structures
- **`ElementView`**: Builder pattern for complex dynamic elements
- **`use_state()` hooks**: Reactive local state management
- **`spawn_local`**: Async operations in WASM
- **Conditional rendering**: `if let`, `for` loops in JSX-like syntax

### Client-Side Routing

Create `src/client/router.rs`:

```rust
use crate::client::pages::{index_page, polls_detail_page, polls_results_page};
use reinhardt_pages::component::View;
use reinhardt_pages::page;
use reinhardt_pages::router::Router;
use std::cell::RefCell;

thread_local! {
	static ROUTER: RefCell<Option<Router>> = const { RefCell::new(None) };
}

pub fn init_global_router() {
	ROUTER.with(|r| {
		*r.borrow_mut() = Some(init_router());
	});
}

pub fn with_router<F, R>(f: F) -> R
where
	F: FnOnce(&Router) -> R,
{
	ROUTER.with(|r| {
		f(r.borrow().as_ref()
			.expect("Router not initialized. Call init_global_router() first."))
	})
}

fn init_router() -> Router {
	Router::new()
		.route("/", || index_page())
		.route("/polls/{question_id}/", || {
			with_router(|r| {
				let params = r.current_params().get();
				let question_id_str = params.get("question_id")
					.cloned().unwrap_or_else(|| "0".to_string());

				match question_id_str.parse::<i64>() {
					Ok(question_id) => polls_detail_page(question_id),
					Err(_) => error_page("Invalid question ID"),
				}
			})
		})
		.route("/polls/{question_id}/results/", || {
			with_router(|r| {
				let params = r.current_params().get();
				let question_id_str = params.get("question_id")
					.cloned().unwrap_or_else(|| "0".to_string());

				match question_id_str.parse::<i64>() {
					Ok(question_id) => polls_results_page(question_id),
					Err(_) => error_page("Invalid question ID"),
				}
			})
		})
		.not_found(|| error_page("Page not found"))
}

fn error_page(message: &str) -> View {
	let message = message.to_string();
	page!(|message: String| {
		div {
			class: "container mt-5",
			div {
				class: "alert alert-danger",
				{ message }
			}
			a {
				href: "/",
				class: "btn btn-primary",
				"Back to Home"
			}
		}
	})(message)
}
```

Create `src/client/pages.rs`:

```rust
use reinhardt_pages::component::View;

pub fn index_page() -> View {
	crate::client::components::polls::polls_index()
}

pub fn polls_detail_page(question_id: i64) -> View {
	crate::client::components::polls::polls_detail(question_id)
}

pub fn polls_results_page(question_id: i64) -> View {
	crate::client::components::polls::polls_results(question_id)
}
```

### WASM Entry Point

Create `src/client/lib.rs`:

```rust
//! WASM entry point

use reinhardt_pages::dom::Element;
use wasm_bindgen::prelude::*;

use super::router;

pub use router::{init_global_router, with_router};

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
	// Set panic hook for better error messages
	console_error_panic_hook::set_once();

	// Initialize router
	router::init_global_router();

	// Get root element and mount app
	let window = web_sys::window().expect("no global `window` exists");
	let document = window.document().expect("should have a document on window");
	let root = document.get_element_by_id("root")
		.expect("should have #root element");

	// Clear loading spinner
	root.set_inner_html("");

	// Mount router's current view
	router::with_router(|router| {
		let view = router.render_current();
		let root_element = Element::new(root.clone());
		let _ = view.mount(&root_element);
	});

	Ok(())
}
```

## Running the Application

### Install WASM Build Tools (First Time Only)

```bash
cargo make install-wasm-tools
```

This installs:
- `wasm32-unknown-unknown` target for Rust
- `wasm-bindgen-cli` for JavaScript bindings generation
- `wasm-opt` for optimization (via binaryen)

### Development Server

```bash
cargo make dev
```

Visit `http://127.0.0.1:8000/` in your browser.

**Features:**
- WASM automatically built before server starts
- Static files served from same server as API
- SPA mode with index.html fallback for client-side routing

### Watch Mode (Auto-Rebuild)

```bash
cargo make dev-watch
```

This watches for file changes and automatically rebuilds WASM.

### Production Build

```bash
cargo make wasm-build-release
```

Output files in `dist/` directory with optimized WASM.

## Advanced Routing Patterns

The reinhardt-pages pattern shown in this tutorial is suitable for modern full-stack Rust applications. However, Reinhardt also supports other routing approaches.

### GraphQL APIs

For GraphQL APIs, use the `async-graphql` library:

```rust
use async_graphql::{Schema, EmptySubscription};
use reinhardt::contrib::graphql::GraphQLRouter;

let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription).finish();
let router = GraphQLRouter::new(schema).with_playground("/graphql");
```

**When to use:**
- Building GraphQL APIs instead of REST
- Complex nested queries
- Strongly-typed API schema

**Example:** See [examples/examples-github-issues](../../../../examples/local/examples-github-issues)

### Server Functions with reinhardt-pages

The server functions pattern demonstrated in this tutorial provides:

- **Type-safe RPC**: Server functions called from WASM like regular async functions
- **Automatic serialization**: serde handles request/response encoding
- **Dependency injection**: `#[inject]` attribute for database connections
- **SSR support**: Components can be pre-rendered on the server

**When to use:**
- Building full-stack Rust applications (WASM + SSR)
- Need seamless client-server integration
- Want reactive UI with server-side data

**Example:** See [examples/examples-twitter](../../../../examples/local/examples-twitter) for a complete implementation.

### Recommendation

**For different project types:**

- **WASM + SSR Apps** → reinhardt-pages (this tutorial)
- **REST APIs** → UnifiedRouter with HTTP method decorators
- **GraphQL APIs** → async-graphql integration

The examples mentioned above demonstrate production-ready patterns for each approach.

## Summary

In this tutorial, you learned:

- How to set up a reinhardt-pages project with WASM support
- How to create shared types for client-server communication
- How to implement server functions with dependency injection
- How to build reactive UI components with `page!` macro and `ElementView`
- How to use `use_state()` hooks for reactive state management
- How to set up client-side routing with dynamic parameters
- How to run development server with `cargo make dev`

## What's Next?

In the next tutorial, we'll explore form processing and validation in reinhardt-pages applications.

Continue to [Part 4: Forms and Generic Views](4-forms-and-generic-views.md).
