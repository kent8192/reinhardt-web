+++
title = "Part 4: Client-Side Forms and Component Patterns"
weight = 40

[extra]
sidebar_weight = 40
+++

# Part 4: Client-Side Forms and Component Patterns

In this chapter we add the interactive layer of the polling app: the voting form, the question CUD pages, and the choice CUD pages. The work splits across three files of the reference implementation:

- [`src/shared/types.rs`](https://github.com/kent8192/reinhardt-web/tree/main/examples/examples-tutorial-basis/src/shared/types.rs) — DTOs that cross the WASM/native boundary, plus the `#[derive(Validate)]` rules that run *only* on the server.
- [`src/shared/forms.rs`](https://github.com/kent8192/reinhardt-web/tree/main/examples/examples-tutorial-basis/src/shared/forms.rs) — server-only `Form` definitions that the unit test in this chapter pins against. The actual `FormMetadata` (incl. CSRF hidden input) is emitted on the client by the `form!` macro at expansion time.
- [`src/apps/polls/client/components.rs`](https://github.com/kent8192/reinhardt-web/tree/main/examples/examples-tutorial-basis/src/apps/polls/client/components.rs) — the `form!` macro pages backed by `#[server_fn]` mutations in [`src/apps/polls/server_fn.rs`](https://github.com/kent8192/reinhardt-web/tree/main/examples/examples-tutorial-basis/src/apps/polls/server_fn.rs).

If you are coming from Django, this is roughly the chapter where "forms + ModelForm + class-based generic views" would appear. The pages template solves the same problem with a different cast: typed DTO validators, a server-side `Form` purely for metadata, and the **`form!`** macro on the client that renders the UI and dispatches to a `#[server_fn]`.

There is no `ListView` or `DetailView` to import. The closest equivalent is the page factory functions you wrote in Part 3 (`polls_index`, `polls_detail`, …) composed with the reactive `page!` / `watch` / `use_action` primitives. We will not introduce any new "generic view" concept — the parts you already have are enough, and we will lean on them harder.

## The Two Flavors of Validation in This Tutorial

Reinhardt offers two complementary validation paths and the tutorial uses both. Knowing which goes where keeps the WASM bundle small and the server checks honest:

| Flavor | Where it lives | What it validates | What enforces it |
|---|---|---|---|
| **DTO field validation** | `src/shared/types.rs` | The shape of a single request payload (lengths, non-empty, etc.) | The server, by calling `request.validate()` inside a `#[server_fn]` |
| **Form metadata + CSRF** | `src/shared/forms.rs` (server-only) | The HTML form schema and per-request CSRF token | The CSRF middleware before the handler runs |

Notice what *neither* does: client-side mirror validation. We deliberately do not derive `Validate` on the WASM side — the server is the source of truth, and shaving the validator crate off the browser bundle is worth the round trip for a server error message.

### Flavor 1: DTO field validation in `shared/types.rs`

The `LoginRequest` and `RegisterRequest` DTOs both live in `src/shared/types.rs`. They are normal `serde` payloads, decorated with the **`#[dto]`** attribute macro — `#[dto]` is the convention-driven entry point that wraps `Validate` (and an OpenAPI `Schema`) `derive` behind `cfg(native)` for you, so the per-field `#[validate(...)]` attributes can be written plainly without any `#[cfg_attr(...)]` noise:

```rust
// src/shared/types.rs

use chrono::{DateTime, Utc};
use reinhardt::dto;
use serde::{Deserialize, Serialize};

/// Login request (DTO)
///
/// Sent from the WASM client to the server when submitting the login form.
///
/// The `#[dto]` macro emits `Validate` (and an OpenAPI `Schema`)
/// derive behind `cfg(native)` so the WASM client does not pull in the
/// validator-crate machinery — the server is the only side that runs
/// `request.validate()` before hitting the database.
#[dto]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
	#[validate(length(
		min = 1,
		max = 150,
		message = "Username must be between 1 and 150 characters"
	))]
	pub username: String,

	#[validate(length(min = 1, message = "Password must not be empty"))]
	pub password: String,
}

/// Register request (DTO)
///
/// Sent from the WASM client to the server when submitting the sign-up form.
/// `password_confirmation` is matched against `password` server-side; both
/// fields travel in the clear over HTTPS just like the login form and are
/// never persisted — only the Argon2 hash of `password` is stored.
///
/// Validation gating is handled by `#[dto]` (same rationale as on
/// [`LoginRequest`]). Field-level rules (length / non-empty) run through
/// `request.validate()`; the password-confirmation equality check is
/// expressed as a dedicated [`RegisterRequest::validate_passwords_match`]
/// helper because the validator crate's `must_match` is brittle across
/// versions (mirroring the pattern used by the tutorial examples).
#[dto]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
	#[validate(length(
		min = 1,
		max = 150,
		message = "Username must be between 1 and 150 characters"
	))]
	pub username: String,

	#[validate(length(min = 8, message = "Password must be at least 8 characters"))]
	pub password: String,

	#[validate(length(
		min = 8,
		message = "Password confirmation must be at least 8 characters"
	))]
	pub password_confirmation: String,
}
```

Three details are load-bearing:

1. **`#[dto]`** — this single attribute is the convention. It emits `#[cfg_attr(native, derive(Validate, Schema))]` for you so the validator-crate `derive` (and the OpenAPI `Schema` derive) are server-only. On WASM the struct still serialises and deserialises, but it has no `validate()` method and pulls in neither dependency.
2. **Plain `#[validate(...)]`** on every rule — no `#[cfg_attr(...)]` wrapping needed. `#[dto]` propagates the native-only gating to these attributes too, so the validator crate is not pulled into the browser bundle at all.
3. **No `must_match` for password confirmation.** Cross-field equality lives in a hand-written helper rather than the derive macro:

```rust
// src/shared/types.rs (continued)

#[cfg(native)]
impl RegisterRequest {
	/// Confirm that `password` and `password_confirmation` match.
	///
	/// Kept out of the derived `Validate` because the validator crate's
	/// `must_match` argument is positional (string field name), brittle
	/// across versions, and produces an awkward error message at the
	/// struct level rather than against the confirmation field. The
	/// server function calls this immediately after `request.validate()`
	/// so the two checks surface as the same kind of `ServerFnError`.
	pub fn validate_passwords_match(&self) -> Result<(), &'static str> {
		if self.password == self.password_confirmation {
			Ok(())
		} else {
			Err("Passwords do not match")
		}
	}
}
```

A server function that consumes `RegisterRequest` first runs `request.validate()?` (the derived field-level checks), then `request.validate_passwords_match()?` (the manual cross-field check). Both produce the same `ServerFnError::server(400, …)` shape so the client treats them identically.

### Flavor 2: Form metadata + CSRF in `shared/forms.rs`

The other piece of validation we need is *not* about a DTO payload — it is about HTML forms: which fields exist, what widgets they render with, and what CSRF token to attach. That lives in `src/shared/forms.rs`, which is gated `#[cfg(native)] pub mod forms;` from `src/shared.rs`:

```rust
// src/shared.rs

//! Shared types and utilities
//!
//! This module contains types and utilities shared between client and server.

#[cfg(native)]
pub mod forms;
pub mod types;
```

```rust
// src/shared/forms.rs

//! Form definitions for examples-tutorial-basis
//!
//! These forms are used server-side to generate FormMetadata
//! that is sent to the WASM client for CSRF token retrieval.

use reinhardt::forms::field::Widget;
use reinhardt::forms::{CharField, Form};

/// Create vote form definition
///
/// This form is primarily used to generate CSRF tokens for the voting form.
/// The actual choice selection uses dynamic radio buttons.
///
/// Fields:
/// - choice: The selected choice ID (hidden field for form metadata purposes)
pub fn create_vote_form() -> Form {
	let mut form = Form::new();

	form.add_field(Box::new(
		CharField::new("choice".to_string())
			.with_label("Choice")
			.with_widget(Widget::HiddenInput)
			.required(),
	));

	form
}
```

That is the entire file. It does three things and nothing else:

1. Builds a `reinhardt::forms::Form` with one `CharField` named `"choice"`.
2. Marks the field as `Widget::HiddenInput` and `required()`.
3. Returns the form so a server function can call `Form::to_metadata()` on it.

The `Form` itself never runs in the browser — it cannot, because the `forms` module is `#[cfg(native)]`. Its job is to be turned into a serialisable `FormMetadata` that the WASM client can request over the wire.

A small unit test in the same file shows the metadata shape:

```rust
// src/shared/forms.rs (continued)

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt::forms::wasm_compat::FormExt;
	use rstest::rstest;

	#[rstest]
	fn test_vote_form_metadata() {
		let form = create_vote_form();
		let metadata = form.to_metadata();

		assert_eq!(metadata.fields.len(), 1);
		assert_eq!(metadata.fields[0].name, "choice");
		assert!(metadata.fields[0].required);
	}
}
```

`FormExt::to_metadata()` is the bridge from a native `Form` to a `FormMetadata` that survives the WASM boundary. We will use exactly that bridge in the next section.

## Exposing the Form to the WASM Client

The WASM client cannot call `create_vote_form()` directly — that function exists only when `#[cfg(native)]` is set. But it does not have to: the `form!` macro that drives the voting page (covered in the next section) handles the metadata plumbing internally. When a `form!` block declares `server_fn: submit_vote` + `method: Post`, the macro emits the matching `FormMetadata` (including a CSRF hidden input) at expansion time on the client side, and the `strip_arguments: { csrf_token: ::reinhardt::reinhardt_pages::csrf::get_csrf_token().unwrap_or_default() }` clause pulls the per-request token from the page-level CSRF helper. The server-side `Form` definition in `shared/forms.rs` is no longer round-tripped through a dedicated `get_*_metadata` `#[server_fn]`; the test from the previous section is exactly enough to prove that the `Form` shape matches what the `form!` macro will emit on the client.

This is the convention the reference example settled on. Earlier iterations of the tutorial exposed a `get_vote_form_metadata` server function for this purpose, and that pattern is still viable for one-off bespoke forms — but the typed `form!` macro removes the need from the canonical voting case, so the project no longer ships that handler.

## The `form!` Macro on the Client

Now the interesting part: `form!`. This is the single recommended path for forms in this tutorial — and in nearly every production reinhardt-pages component. It is declarative, it integrates with `#[server_fn]`, and it lets you trade a few lines of macro syntax for what would otherwise be dozens of lines of imperative `use_state` plumbing.

We will walk through the voting form from `src/apps/polls/client/components.rs`. The shape is dense; we will quote it first and then break it down.

### The voting form, in full

```rust
// src/apps/polls/client/components.rs (extract)

use crate::shared::types::{ChoiceInfo, QuestionInfo};
use reinhardt::pages::component::Page;
use reinhardt::pages::form;
use reinhardt::pages::page;
use reinhardt::pages::reactive::hooks::{Action, use_action, use_effect};

use crate::apps::polls::server_fn::{
	create_choice, create_question, delete_choice, delete_question, get_question_detail,
	get_question_results, get_questions, submit_vote, update_choice, update_question,
};
// Typed URL helpers live next to the app's client router
// (issue #4656); we alias the macro-emitted `urls` module as `links` to
// keep call sites concise.
use crate::apps::polls::urls::client_router::urls as links;

/// Poll detail page - Show question and voting form
pub fn polls_detail(question_id: i64) -> Page {
	let qid = question_id;

	// Create action for loading question detail
	let load_detail =
		use_action(
			|qid: i64| async move { get_question_detail(qid).await.map_err(|e| e.to_string()) },
		);

	// Create the voting form using form! macro
	// - server_fn: submit_vote accepts (question_id, choice_id, csrf_token)
	// - method: Post enables CSRF hidden input rendering for non-WASM submits
	// - strip_arguments: explicitly routes the CSRF token to the trailing
	//   server_fn argument (reinhardt-web#3971), replacing the implicit
	//   auto-injection that broke when server_fn signatures evolved.
	// - watch blocks for reactive UI updates
	let voting_form = form! {
		name: VotingForm,
		server_fn: submit_vote,
		method: Post,

		fields: {
			question_id: HiddenField {
				initial: qid.to_string(),
			},
			choice_id: ChoiceField {
				widget: RadioSelect,
				required,
				label: "Select your choice",
				class: "form-check",
				choices_from: "choices",
				choice_value: "id",
				choice_label: "choice_text",
			},
		},

		strip_arguments: {
			csrf_token: ::reinhardt::reinhardt_pages::csrf::get_csrf_token()
				.unwrap_or_default(),
		},

		watch: {
			submit_button: |form| {
				let is_loading = form.loading().get();
				let back_href = links::index();
				page!(|is_loading: bool, back_href: String| {
					div {
						class: "mt-3",
						button {
							type: "submit",
							class: if is_loading { "btn-primary opacity-50 cursor-not-allowed" } else { "btn-primary" },
							disabled: is_loading,
							{ if is_loading { "Voting..." } else { "Vote" } }
						}
						a {
							href: back_href,
							class: "btn-secondary ml-2",
							"Back to Polls"
						}
					}
				})(is_loading, back_href)
			},
			error_display: |form| {
				let err = form.error().get();
				page!(|err: Option<String>| {
					watch {
						if let Some(e) = err.clone() {
							div {
								class: "alert-danger mt-3",
								{ e }
							}
						}
					}
				})(err)
			},
			success_navigation: |form| {
				let is_loading = form.loading().get();
				let err = form.error().get();
				page!(|is_loading: bool, err: Option<String>| {
					watch {
						if ! is_loading &&err.is_none() {
							#[cfg(wasm)]
									{
										if let Some(window) = web_sys::window() {
											let pathname = window.location().pathname().ok();
											if let Some(path) = pathname {
												let parts: Vec<&str> = path.split('/').collect();
												if parts.len() >= 3 && parts[1] == "polls" {
													if let Ok(question_id) = parts[2].parse::<i64>() {
														let results_url = links::results(question_id);
														let _ = window.location().set_href(&results_url);
													}
												}
											}
										}
									}
						}
					}
				})(is_loading, err)
			},
		},
	};

	// Bridge load_detail results to form choices via use_effect
	{
		let load_detail_for_effect = load_detail.clone();
		let voting_form_for_effect = voting_form.clone();
		use_effect(move || {
			if let Some((_, ref choices)) = load_detail_for_effect.result() {
				let choice_options: Vec<(String, String)> = choices
					.iter()
					.map(|c| (c.id.to_string(), c.choice_text.clone()))
					.collect();
				voting_form_for_effect
					.choice_id_choices()
					.set(choice_options);
			}
		});
	}

	// Dispatch the action to load question data
	load_detail.dispatch(qid);
```

### Reading the macro top-to-bottom

The block above is doing six things; the cleanest way to internalise `form!` is to map each clause back to what it produces.

| Clause | What it means |
|---|---|
| `name: VotingForm` | Names the generated struct (`VotingForm`) and DOM id (`voting-form`). Used by `<button form="…">` references later. |
| `server_fn: submit_vote` | Picks the `#[server_fn]` this form submits to. The macro generates a client-side call to it on submit. |
| `method: Post` | Tells the macro this is a mutating form. That decision enables CSRF hidden-input rendering and the `_csrf_token` argument convention discussed below. |
| `fields: { … }` | Declares the form fields. `HiddenField`, `CharField`, `ChoiceField`, etc., correspond to widget builders the macro knows about. |
| `strip_arguments: { csrf_token: … }` | Explicitly tells `form!` how to supply the trailing `_csrf_token: String` argument of the server function (see below). |
| `watch: { … }` | Reactive view fragments — small `page!` blocks whose output is re-evaluated whenever the signals they capture change. |

Two behaviours are worth flagging because they are easy to miss:

1. **All fields submit as `String`.** This is tracked upstream as [reinhardt-web#4397](https://github.com/kent8192/reinhardt-web/issues/4397). Once that ships, the matching `#[server_fn]` will be able to accept typed parameters directly. Until then, every server function reachable from `form!` accepts `String` and parses inside the handler — we will see this in the next section.
2. **`form!` appends a `_csrf_token: String` argument for non-GET handlers.** The CSRF middleware verifies the token before the handler body runs; the parameter exists in the server function signature only so the macro-generated client stub stays positional with the server signature (tracked in [reinhardt-web#3971](https://github.com/kent8192/reinhardt-web/issues/3971)). The `strip_arguments` clause above tells `form!` to pull the token via `reinhardt::reinhardt_pages::csrf::get_csrf_token()` and append it to every call.

### What the generated `voting_form` value gives you

The macro returns a struct value (here, `voting_form: VotingForm`) with three useful surfaces:

- `voting_form.loading()` and `voting_form.error()` — compatibility submit signals generated by `form!`.
- `voting_form.choice_id_choices()` — a setter signal generated because the `choice_id` field carries `choices_from: "choices"`. We populate it dynamically below.
- `voting_form.into_page()` — converts the form into a `Page` you can drop inside an outer `page! { … }`.

This is the entirety of the macro's public surface — there is no hidden registry, no global state, no decorator stack to climb.

## Reactive UI Patterns: `page!`, `watch`, `use_action`

Three primitives appear over and over in the components. They are the entire reactive vocabulary the tutorial uses.

### `page!`

`page!(|deps: Type, …| { html-like body })(deps, …)` builds a `Page` whose body is recomputed whenever the captured dependencies change. The closure-then-arguments shape is what lets the macro track exactly which signals each fragment depends on. You can see it used both at the top level (returning the full page) and inside `watch` clauses (returning fragment trees).

### `watch { … }`

A `watch { … }` block is a *conditional fragment*. The block's body is re-evaluated whenever any signal it references changes value; if the condition is false the fragment disappears from the DOM. In the voting form above, three `watch` blocks live inside the `watch:` clause of `form!`:

- `submit_button` re-renders when `form.loading()` flips, swapping the button label between *Vote* and *Voting…* and toggling the `opacity-50 cursor-not-allowed` classes.
- `error_display` mounts an `alert-danger` div when `form.error()` becomes `Some(…)`, and unmounts it when it returns to `None`.
- `success_navigation` watches both `loading` and `error`; when loading completes with no error, it triggers a redirect to the results page via `web_sys::window().location().set_href(...)`. The whole inner block is gated `#[cfg(wasm)]` because `web_sys` only compiles for the browser target.

### `use_action`

`use_action(|arg| async move { … })` wraps an async function into a typed reactive action with `.dispatch(arg)`, `.is_pending()`, `.result()`, and `.error()`. In the detail page we have:

```rust
let load_detail =
	use_action(
		|qid: i64| async move { get_question_detail(qid).await.map_err(|e| e.to_string()) },
	);
// …
load_detail.dispatch(qid);
```

Calling `dispatch` kicks off the async call once; the action then exposes the result reactively to any `watch` block that observes `load_detail.result()` / `load_detail.is_pending()` / `load_detail.error()`. The full `polls_detail` function uses this pattern to render a spinner while the question loads, then an error card if it fails, then the question text and the voting form on success — all from the same component, no manual state machine.

## Connecting Form Metadata + Action: the Voting Lifecycle

The voting form's choices are not known at compile time — they come from the database. The pattern that wires loaded data into a `form!` is to (a) start an action that loads the data, and (b) use a `use_effect` to write the result into the generated choices signal:

```rust
// src/apps/polls/client/components.rs (continued)

// Bridge load_detail results to form choices via use_effect
{
	let load_detail_for_effect = load_detail.clone();
	let voting_form_for_effect = voting_form.clone();
	use_effect(move || {
		if let Some((_, ref choices)) = load_detail_for_effect.result() {
			let choice_options: Vec<(String, String)> = choices
				.iter()
				.map(|c| (c.id.to_string(), c.choice_text.clone()))
				.collect();
			voting_form_for_effect
				.choice_id_choices()
				.set(choice_options);
		}
	});
}
```

`use_effect` re-runs whenever the closure's captured signals change. The first time `load_detail.result()` becomes `Some(…)`, the effect converts the `Vec<ChoiceInfo>` into the `Vec<(String, String)>` shape that `choices_from: "choices"` expects — value first, label second — and pushes it through the generated `choice_id_choices()` setter. The DOM updates automatically.

When the user picks a choice and presses Vote, the complete round-trip looks like this:

```mermaid
sequenceDiagram
    participant U as User
    participant F as VotingForm (form!)
    participant SF as submit_vote (#[server_fn])
    participant DB as Database
    participant W as watch (error_display, success_navigation)

    U->>F: select choice, click Vote
    F->>F: collect fields as String, append _csrf_token
    F->>SF: submit_vote(question_id, choice_id, _csrf_token)
    SF->>SF: parse Strings, build VoteRequest
    SF->>DB: atomic SELECT choice, UPDATE votes by 1
    DB-->>SF: updated Choice
    SF-->>F: Result of ChoiceInfo or ServerFnError
    F->>W: form.loading() to false, form.error() to None or Some(...)
    W->>U: rerender (success redirect or error alert)
```

The CSRF check happens *before* `submit_vote` runs — it is a middleware concern, not a handler concern.

Here is the matching server function in full, including the `String`-typed workaround commented at the top of the CUD block:

```rust
// src/apps/polls/server_fn.rs

/// Submit vote via form! macro
///
/// Wrapper function that accepts individual field values from form! macro's submit.
/// Converts String field values to the required types and calls the underlying vote function.
///
/// The trailing `_csrf_token: String` argument is supplied by `form!`'s
/// `strip_arguments` block (reinhardt-web#3971). Actual CSRF verification is
/// performed by the server-side CSRF middleware before this handler runs;
/// receiving the value here keeps the WASM client stub's positional argument
/// list aligned with the server signature.
#[server_fn]
pub async fn submit_vote(
	question_id: String,
	choice_id: String,
	_csrf_token: String,
	#[inject] db: reinhardt::DatabaseConnection,
) -> std::result::Result<ChoiceInfo, ServerFnError> {
	let question_id: i64 = question_id
		.parse()
		.map_err(|_| ServerFnError::application("Invalid question_id"))?;
	let choice_id: i64 = choice_id
		.parse()
		.map_err(|_| ServerFnError::application("Invalid choice_id"))?;

	let request = VoteRequest {
		question_id,
		choice_id,
	};

	// Reuse the existing vote logic
	vote_internal(request, db).await
}
```

`vote_internal` is the reusable native helper (already covered in Part 3); it wraps the read-modify-write in `atomic(&db, …)` so two simultaneous voters cannot race past one another. Notice that the typed `vote` server function still exists alongside `submit_vote` — that one accepts a real `VoteRequest` and is the better entry point for code that calls server functions directly (e.g. tests, native code, future clients). `submit_vote` is the `form!` adapter.

## Question CUD via `form!`

The voting form is the headline use case, but the same pattern composes naturally for create / update / delete. The Question CUD handlers in `src/apps/polls/server_fn.rs` show what an authenticated mutation looks like when stitched together with the `String`-based ABI and the session-user DI factory (see Part 3 for the factory definition):

```rust
// src/apps/polls/server_fn.rs

// =========================================================================
// Question CUD (Phase 2)
// =========================================================================
//
// All three mutations below follow the same conventions:
//
// * Every form field is received as `String` because `form!` currently
//   serializes all fields as strings on submit. This is tracked upstream as
//   reinhardt-web#4397 — once that ships, the `String` + `.parse()` dance
//   below can be replaced with the typed signatures shown next to each
//   handler. The trailing `_csrf_token: String` parameter is appended by the
//   `form!` macro for non-GET forms; the CSRF middleware verifies it before
//   the handler runs.
// * Authentication is required: `Depends<Result<User, SessionError>>` is
//   resolved by the request-scoped factory in `apps::polls::di` and
//   exposes `.as_ref().map_err(ServerFnError::from)?` for the 401/403/500
//   surface.
// * For `update_question` and `delete_question`, ownership is enforced by
//   comparing `question.author_id()` with the current user's id; mismatched
//   ownership returns a 403.

/// Create a new question owned by the current user.
///
/// Ideal implementation (without the form! String workaround tracked in #4397):
///   pub async fn create_question(
///       question_text: String,
///       _csrf_token: String,
///       #[inject] _db: reinhardt::DatabaseConnection,
///       #[inject] session_user: Depends<Result<User, SessionError>>,
///   ) -> std::result::Result<QuestionInfo, ServerFnError> { ... }
#[server_fn]
pub async fn create_question(
	question_text: String,
	_csrf_token: String,
	#[inject] _db: reinhardt::DatabaseConnection,
	#[inject] session_user: Depends<Result<User, SessionError>>,
) -> std::result::Result<QuestionInfo, ServerFnError> {
	use crate::apps::polls::models::Question;

	let user = (*session_user).as_ref().map_err(ServerFnError::from)?;

	let trimmed = question_text.trim();
	if trimmed.is_empty() || trimmed.len() > 200 {
		return Err(ServerFnError::server(
			400,
			"Question text must be between 1 and 200 characters",
		));
	}

	let manager = Question::objects();
	let new_question = Question::build()
		.question_text(trimmed)
		.author(user.id())
		.finish();
	let saved = manager
		.create(&new_question)
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;

	Ok(QuestionInfo::from(saved))
}
```

`(*session_user).as_ref().map_err(ServerFnError::from)?` is the shared 401/403/500 gate, layered on the session-user DI factory in `apps::polls::di`. The factory does the "load user_id from session, fetch the row, classify Anonymous / active / Inactive / Unavailable" dance once per request and returns a `Result<User, SessionError>`; each authenticated handler just borrows the result and converts the error via `From<&SessionError>`:

```rust
// src/apps/polls/di.rs (extract)

/// Error variants for the session-based user lookup factory.
///
/// Three failure modes are distinguished so handlers can surface the
/// correct HTTP status *and* so that an operational outage (DB
/// connection drop, query timeout) does not get silently rewritten into
/// a fake 401:
///
/// - `Anonymous` — no `user_id` in the session, or the row has been
///   deleted between login and request. Surfaced as **401** via
///   `From<&SessionError> for ServerFnError`.
/// - `Inactive` — a row exists but `is_active = false`. Surfaced as **403**.
/// - `Unavailable` — the user-lookup query itself failed (DB down, pool
///   exhausted, schema mismatch, …). Surfaced as **500** so the client sees
///   an operational error instead of being pushed into a misleading re-auth
///   loop.
#[derive(Clone, Debug)]
pub enum SessionError {
	Anonymous,
	Inactive,
	Unavailable(String),
}

/// Convert a `SessionError` reference to a `ServerFnError` with the
/// appropriate HTTP status code.
///
/// Handlers use this via `user.as_ref().map_err(ServerFnError::from)?`
/// to surface a 401, 403, or 500 depending on the failure mode.
impl From<&SessionError> for ServerFnError {
	fn from(err: &SessionError) -> Self {
		match err {
			SessionError::Anonymous => ServerFnError::server(401, "Authentication required"),
			SessionError::Inactive => ServerFnError::server(403, "User account is inactive"),
			SessionError::Unavailable(_) => {
				ServerFnError::server(500, "User lookup temporarily unavailable")
			}
		}
	}
}

#[injectable_factory(scope = "request")]
async fn session_user_factory(
	#[inject] session: SessionData,
) -> Result<User, SessionError> { /* ... */ }
```

`update_question` and `delete_question` follow the same shape; the only difference is the ownership check after loading the row:

```rust
// src/apps/polls/server_fn.rs (continued)

/// Update a question's text. Only the author may update.
///
/// Ideal implementation (without the form! String workaround tracked in #4397):
///   pub async fn update_question(
///       question_id: i64,
///       question_text: String,
///       _csrf_token: String,
///       ...
///   ) -> std::result::Result<QuestionInfo, ServerFnError> { ... }
#[server_fn]
pub async fn update_question(
	question_id: String,
	question_text: String,
	_csrf_token: String,
	#[inject] _db: reinhardt::DatabaseConnection,
	#[inject] session_user: Depends<Result<User, SessionError>>,
) -> std::result::Result<QuestionInfo, ServerFnError> {
	use crate::apps::polls::models::Question;

	let user = (*session_user).as_ref().map_err(ServerFnError::from)?;

	let question_id: i64 = question_id
		.parse()
		.map_err(|_| ServerFnError::application("Invalid question_id"))?;

	let trimmed = question_text.trim();
	if trimmed.is_empty() || trimmed.len() > 200 {
		return Err(ServerFnError::server(
			400,
			"Question text must be between 1 and 200 characters",
		));
	}

	let manager = Question::objects();
	let mut question = manager
		.get(question_id)
		.first()
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?
		.ok_or_else(|| ServerFnError::server(404, "Question not found"))?;

	if *question.author_id() != user.id() {
		return Err(ServerFnError::server(
			403,
			"Only the question's author can edit it",
		));
	}

	question.question_text = trimmed.to_string();

	let updated = manager
		.update(&question)
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;

	Ok(QuestionInfo::from(updated))
}

/// Delete a question. Only the author may delete.
///
/// Ideal implementation (without the form! String workaround tracked in #4397):
///   pub async fn delete_question(
///       question_id: i64,
///       _csrf_token: String,
///       ...
///   ) -> std::result::Result<(), ServerFnError> { ... }
#[server_fn]
pub async fn delete_question(
	question_id: String,
	_csrf_token: String,
	#[inject] _db: reinhardt::DatabaseConnection,
	#[inject] session_user: Depends<Result<User, SessionError>>,
) -> std::result::Result<(), ServerFnError> {
	use crate::apps::polls::models::Question;

	let user = (*session_user).as_ref().map_err(ServerFnError::from)?;

	let question_id: i64 = question_id
		.parse()
		.map_err(|_| ServerFnError::application("Invalid question_id"))?;

	let manager = Question::objects();
	let question = manager
		.get(question_id)
		.first()
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?
		.ok_or_else(|| ServerFnError::server(404, "Question not found"))?;

	if *question.author_id() != user.id() {
		return Err(ServerFnError::server(
			403,
			"Only the question's author can delete it",
		));
	}

	manager
		.delete(question.id())
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;

	Ok(())
}
```

The "ideal implementation" comments in the source are not aspirational decoration — they are the literal signatures the handlers will collapse to once `form!` ships typed-field serialisation (#4397). The intent is that the only thing that needs to change in this file then is the parameter types and the deletion of the `.parse()` lines; the rest of the body, the session check, and the ownership check stay put.

### What the client side of CUD looks like

The matching client pages are short. Here is the "new question" page — it is the entire pattern in one block:

```rust
// src/apps/polls/client/components.rs (extract)

/// New question page (`/polls/new/`).
pub fn question_new() -> Page {
	let new_form = form! {
		name: NewQuestionForm,
		server_fn: create_question,
		method: Post,
		redirect_on_success: "/",

		fields: {
			question_text: CharField {
				label: "Question",
				placeholder: "What do you want to ask?",
				max_length: 200,
				class: "form-control",
			},
		},

		strip_arguments: {
			csrf_token: ::reinhardt::reinhardt_pages::csrf::get_csrf_token()
				.unwrap_or_default(),
		},
	};

	let loading_signal = new_form.loading().clone();
	let error_signal = new_form.error().clone();
	let form_view = new_form.into_page();
	let cancel_href = links::index();

	page!(|loading_signal: reinhardt::pages::reactive::Signal<bool>, error_signal: reinhardt::pages::reactive::Signal<Option<String>>, form_view: Page, cancel_href: String| {
		div {
			class: "max-w-4xl mx-auto px-4 mt-12",
			h1 { class: "mb-4", "New Question" }
			watch {
				if error_signal.get().is_some() {
					div {
						class: "alert-danger mb-3",
						{ error_signal.get().unwrap_or_default() }
					}
				}
			}
			{ form_view }
			div {
				class: "mt-3",
				watch {
					if loading_signal.get() {
						button {
							type: "submit",
							class: "btn-primary opacity-50 cursor-not-allowed",
							disabled: true,
							form: "new-question-form",
							"Creating..."
						}
					} else {
						button {
							type: "submit",
							class: "btn-primary",
							form: "new-question-form",
							"Create"
						}
					}
				}
				a { href: cancel_href, class: "btn-secondary ml-2", "Cancel" }
			}
		}
	})(loading_signal, error_signal, form_view, cancel_href)
}
```

Two things make this shorter than the voting form:

- **`redirect_on_success: "/"`** — `form!` knows how to navigate on its own; you do not have to write a `success_navigation` watch block by hand.
- **No `watch:` clause inside `form!`** — the page renders the button and error display *outside* `form!`. Both patterns are valid; the choice is purely aesthetic.

`question_edit` and `question_delete_confirm` follow the same shape, adding a `HiddenField` for `question_id` and (for edit) a `load_detail` action that pre-fills the form. The choice CUD pages (`choice_new`, `choice_edit`, `choice_delete_confirm`) are structurally identical — see `src/apps/polls/client/components.rs` for the full set.

## Choice CUD: Ownership Through the Parent

Choices have no author field of their own; ownership is derived from the parent question. The `create_choice` server function shows the composition pattern with the shared `require_question_author` helper:

```rust
// src/apps/polls/server_fn.rs

/// Internal helper: load a Question by id and ensure the given user is its
/// author. Returns 401/403/404 as appropriate.
#[cfg(native)]
async fn require_question_author(
	question_id: i64,
	user: &User,
) -> std::result::Result<crate::apps::polls::models::Question, ServerFnError> {
	use crate::apps::polls::models::Question;

	let question = Question::objects()
		.get(question_id)
		.first()
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?
		.ok_or_else(|| ServerFnError::server(404, "Question not found"))?;

	if *question.author_id() != user.id() {
		return Err(ServerFnError::server(
			403,
			"Only the question's author can manage its choices",
		));
	}

	Ok(question)
}

/// Create a new Choice on a Question. Only the question's author may add
/// choices.
#[server_fn]
pub async fn create_choice(
	question_id: String,
	choice_text: String,
	_csrf_token: String,
	#[inject] _db: reinhardt::DatabaseConnection,
	#[inject] session_user: Depends<Result<User, SessionError>>,
) -> std::result::Result<ChoiceInfo, ServerFnError> {
	use crate::apps::polls::models::Choice;

	let user = (*session_user).as_ref().map_err(ServerFnError::from)?;
	let question_id: i64 = question_id
		.parse()
		.map_err(|_| ServerFnError::application("Invalid question_id"))?;
	let question = require_question_author(question_id, user).await?;

	let trimmed = choice_text.trim();
	if trimmed.is_empty() || trimmed.len() > 200 {
		return Err(ServerFnError::server(
			400,
			"Choice text must be between 1 and 200 characters",
		));
	}

	let manager = Choice::objects();
	let new_choice = Choice::build()
		.choice_text(trimmed)
		.votes(0)
		.question(question.id())
		.finish();
	let saved = manager
		.create(&new_choice)
		.await
		.map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;

	Ok(ChoiceInfo::from(saved))
}
```

Read this top-to-bottom and the layering becomes obvious:

1. `(*session_user).as_ref().map_err(ServerFnError::from)?` — authentication, resolved via the `Depends<Result<User, SessionError>>` DI factory.
2. `question_id.parse()?` — workaround for the `String`-only ABI.
3. `require_question_author(question_id, &user).await?` — authorization, *through the parent row*.
4. Local content validation (length).
5. `Choice::build() … .finish()` — typed model construction (from Part 2).
6. `Choice::objects().create(...).await?` — the actual mutation.

The pattern repeats for `update_choice` (load choice → look up parent question → check author) and `delete_choice`. Each tiered check returns its own `ServerFnError::server(status, message)`, which surfaces directly on the client through the form's `error` signal. There is no shared exception class to design or middleware to register — the server function simply returns the error, and the `form!` macro plumbs it to `form.error()`.

## What This Chapter Does NOT Teach

If you are coming from Django or another classic server-rendered framework, you may be wondering where the generic views went. In short: the pages template does not have them, and does not need them.

- **`ListView` / `DetailView`** are replaced by **page factory functions** — `polls_index`, `polls_detail`, `polls_results`, `question_new`, `question_edit`, `choice_new`, … each defined in `src/apps/polls/client/components.rs` and wrapped from `src/client/pages.rs`. We wrote them in Part 3.
- **The reusability story** is **component composition with `page!` + `watch` + `use_action`**, not subclassing. The voting page composes a `page!` outer shell, a `form!` block, two `watch`-driven action states, and a `use_effect` bridge — six small pieces, each independently reasonable.
- **Form rendering** is **the `form!` macro**, not a templating language with form tags. The HTML is in your component.

There is also no client-side validator block. The tutorial does *not* mirror DTO validation into the WASM bundle: server-side `request.validate()` plus the `form.error()` signal closes the loop with a smaller bundle and one canonical source of truth. (Historically a `client_validators` block existed; it is deprecated and not used in this tutorial — see [reinhardt-web#3769](https://github.com/kent8192/reinhardt-web/issues/3769).)

If you absolutely need a lower-level form-handling path — multi-step wizards with branching that `form!` cannot express, drag-and-drop form builders with runtime-defined fields, or integration with a third-party state management library — you can drop down to `use_state` and assemble the form imperatively. That escape hatch exists, but it is not part of the basis tutorial, and it should not be reached for unless `form!` truly cannot express what you need.

## Recap

You now have everything Part 4 set out to deliver:

- DTO field-level validation lives in `src/shared/types.rs`, with `#[dto]` emitting `derive(Validate)` (and OpenAPI `Schema`) behind `cfg(native)` so the WASM bundle stays small.
- The voting form's metadata + CSRF token are emitted by the `form!` macro itself at expansion time on the client side; the server-only `create_vote_form()` in `src/shared/forms.rs` exists so the same shape can be unit-tested (`test_vote_form_metadata`) without compiling the macro, and the `strip_arguments: { csrf_token: ::reinhardt::reinhardt_pages::csrf::get_csrf_token() … }` clause pulls the per-request CSRF token from the page-level helper.
- The `form!` macro in `src/apps/polls/client/components.rs` declares the UI, dispatches to `submit_vote`, serialises every field as `String`, appends the CSRF token through `strip_arguments`, and surfaces success/error reactively through the generated `loading()` / `error()` signals and matching `watch` blocks.
- Question and Choice CUD reuse the same `form!` + `#[server_fn]` shape, composing `(*session_user).as_ref().map_err(ServerFnError::from)?` (authentication, via the `Depends<Result<User, SessionError>>` DI factory) and `require_question_author` (authorization) on top of typed model builders.
- "Generic views" are not a separate concept in the pages template — they are the page factory functions you already have, glued together with the reactive primitives above.

In the next chapter we put this layer under test: native integration tests with `rstest` + `reinhardt-test` + `sqlx` + `tempfile`, plus a WASM-only target that mocks the server function HTTP calls with MSW.

Continue to [Part 5: Testing](../5-testing/).
