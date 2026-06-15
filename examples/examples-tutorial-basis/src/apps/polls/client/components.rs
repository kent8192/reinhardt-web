//! Polling application components
//!
//! Provides UI components for the polling application including
//! the index page, detail page with voting form, and results page.
//!
//! ## Reactive page shape (canonical template)
//!
//! Every async-loading view in this module (`polls_index`, `polls_detail`,
//! `polls_results`, `question_edit`, `question_delete_confirm`) follows the
//! same page! v2 shape:
//!
//! - Data is loaded with `use_resource(fetcher, ())` and flows into `page!`
//!   as a `Resource<T, String>` parameter. The view branches on it inside a
//!   single `{ match resource.get() { Loading => .., Error(e) => .., Success(v)
//!   => .. } }` block. Reading the resource exactly once matters: `page!`
//!   auto-wraps each `{ .. }` / `if` / `for` in `Page::reactive(move || ..)`
//!   (`Fn() -> Page`), and a non-`Copy` handle consumed by two sibling blocks
//!   would be moved out of that `Fn` closure twice (`E0507`).
//! - An outer `div` wraps that block so the function returns a top-level
//!   `Page::Element` (matching the `matches!(view, Page::Element(_))` assertion
//!   in `tests/wasm/polls_mock_test.rs`).
//! - page! v2 forbids implicit captures, so every value used in the body is a
//!   declared parameter and free functions are called through a multi-segment
//!   path (`self::format_server_error`, `polls_routes::reverse`). Form sub-views
//!   that depend on the form's `error` / `loading` signals are rendered inline as
//!   `{ .. }` blocks that read each signal exactly once.

use crate::shared::types::{ChoiceInfo, QuestionInfo, UserInfo};
use reinhardt::pages::component::Page;
use reinhardt::pages::form;
use reinhardt::pages::page;
use reinhardt::pages::reactive::hooks::use_effect;
use reinhardt::pages::reactive::{Resource, ResourceState, Signal, use_resource};

use crate::apps::polls::server_fn::{
	create_choice, create_question, delete_choice, delete_question, get_question_detail,
	get_question_results, get_questions, submit_vote, update_choice, update_question,
};
use crate::apps::polls::urls::client_router as polls_routes;
// Used by `polls_detail` to gate owner-only controls (Edit / Delete / Add
// choice) on the viewer being the question's author (issue #4703). Server-
// side `require_question_author` checks remain in place as defense in depth.
use crate::apps::users::server_fn::current_user;

// =========================================================================
// Error display helpers
// =========================================================================

/// Extract the human-readable message from a `ServerFnError`-shaped JSON
/// payload so the alert banner shows prose, not raw JSON (issue #4702).
///
/// `ServerFnError` is serialized with serde's externally-tagged format —
/// e.g. `{"Application":"Invalid choice_id"}` for `ServerFnError::Application`
/// or `{"Server":{"status":403,"message":"..."}}` for `ServerFnError::Server`.
/// This helper unwraps the variant tag for display purposes only; the
/// wire format the server sends is intentionally unchanged.
fn format_server_error(raw: &str) -> String {
	if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw)
		&& let Some(obj) = value.as_object()
		&& let Some((_, payload)) = obj.iter().next()
	{
		if let Some(s) = payload.as_str() {
			return s.to_string();
		}
		if let Some(msg) = payload.get("message").and_then(|v| v.as_str()) {
			return msg.to_string();
		}
	}
	raw.to_string()
}

/// Polls index page - List all polls
///
/// Displays a list of available polls with links to vote.
/// Uses watch blocks for reactive UI updates when async data loads.
pub fn polls_index() -> Page {
	let load_questions = use_resource(
		|| async move { get_questions().await.map_err(|e| e.to_string()) },
		(),
	);
	let new_question_href = polls_routes::reverse("question_new", &[]);

	page!(|load_questions: Resource<Vec<QuestionInfo>, String>, new_question_href: String| {
		div {
			class: "max-w-4xl mx-auto px-4 mt-12",
			div {
				class: "flex justify-between items-center mb-4",
				h1 { "Polls" }
				a {
					href: new_question_href,
					class: "btn-primary",
					"Create new poll"
				}
			}
			{
				match load_questions.get() {
					ResourceState::Loading => page!(|| {
						div {
							class: "text-center",
							div {
								class: "spinner w-8 h-8",
								role: "status",
								span {
									class: "sr-only",
									"Loading..."
								}
							}
						}
					})(),
					ResourceState::Error(error) => page!(|error: String| {
						div {
							class: "alert-danger",
							{
								self::format_server_error(&error)
							}
						}
					})(error),
					ResourceState::Success(questions)if questions.is_empty() => page!(|| {
						p {
							class: "text-muted",
							"No polls are available."
						}
					})(),
					ResourceState::Success(questions) => page!(|questions: Vec<QuestionInfo>| {
						div {
							class: "space-y-2",
							for question in questions {
								a {
									href: polls_routes::reverse("detail", &[("question_id", question.id.to_string().as_str())]),
									class: "block p-4 border border-border rounded-lg bg-surface-primary hover:bg-surface-secondary transition-colors",
									div {
										class: "flex w-full justify-between",
										h5 {
											class: "mb-1",
											{
												question.question_text.clone()
											}
										}
										small { {
											question.pub_date.format("%Y-%m-%d %H:%M").to_string()
										} }
									}
								}
							}
						}
					})(questions),
				}
			}
		}
	})(load_questions, new_question_href)
}

/// Poll detail page - Show question and voting form
///
/// Displays a question with its choices and allows the user to vote.
/// Uses form! macro with Dynamic ChoiceField for declarative form handling.
/// CSRF protection is automatically injected for POST method.
pub fn polls_detail(question_id: i64) -> Page {
	let qid = question_id;

	// Load the question detail once on mount.
	let load_detail = use_resource(
		move || async move { get_question_detail(qid).await.map_err(|e| e.to_string()) },
		(),
	);

	// Resolve the viewer so the render branch can hide owner-only controls
	// (Edit / Delete / Add choice) for non-authors and unauthenticated
	// viewers (issue #4703). `current_user` returns `Ok(None)` when the
	// session has no authenticated user, so any non-`Some(Some(u))` shape
	// disables the controls. Server-side `require_question_author` still
	// rejects unauthorized mutations as defense in depth.
	let load_current_user = use_resource(
		|| async move { current_user().await.map_err(|e| e.to_string()) },
		(),
	);

	// Voting form via the `form!` macro. Keep this instance stable for the
	// lifetime of the route component; recreating it inside the reactive render
	// path resets the selected radio value immediately after a change event
	// (reinhardt-web#5169).
	//
	// - `server_fn: submit_vote` binds the form to the server function whose
	//   typed signature is `(question_id, choice_id)`.
	// - `method: Post` enables CSRF hidden-input rendering for non-WASM submits.
	// - The `#[server_fn]` client stub attaches `X-CSRFToken` for WASM submits,
	//   so CSRF stays transport-level rather than becoming a business argument.
	// - `success_url: |_form| ...` triggers an in-SPA navigation to the results
	//   page after a successful vote. The closure captures `qid` from the
	//   outer scope; the macro stores it on the generated form struct and the
	//   generated `submit()` method dispatches through `pages::navigate()` so
	//   the route table installed from `#[routes]` inventory handles the
	//   transition without a full page load.
	let voting_form = form! {
		name: VotingForm,
		server_fn: submit_vote,
		method: Post,
		success_url: |_form| polls_routes::reverse("results", &[("question_id", qid.to_string().as_str())]),
		fields: {
			question_id: HiddenField<i64> {
				initial: qid,
			}
			choice_id: ChoiceField<i64> {
				widget: RadioSelect,
				required,
				label: "Select your choice",
				class: "poll-choice-input",
				wrapper_class: "poll-choice-field",
				label_class: "poll-choice-label",
				choices_from: "choices",
				choice_value: "id",
				choice_label: "choice_text",
			}
		}
		watch: {
			submit_button: |form| {
				let is_loading = form.loading().get();
				let back_href = polls_routes::reverse("index", &[]);
				page!(|is_loading: bool, back_href: String| {
					div {
						class: "mt-3",
						button {
							type: "submit",
							class: if is_loading { "btn-primary opacity-50 cursor-not-allowed" } else { "btn-primary" },
							disabled: is_loading,
							{
								if is_loading { "Voting..." } else { "Vote" }
							}
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
				page!(|err: Option<String>| { {
					err.clone().map(|e| page!(|e: String| {
						div {
							class: "alert-danger mt-3",
							{
								self::format_server_error(&e)
							}
						}
					})(e)).unwrap_or(Page::Empty)
				} })(err)
			},
		}
	};
	let choice_options_signal = voting_form.choice_id_choices().clone();
	let voting_form_page = voting_form.into_page();

	// Render reactively in the canonical shape (see module-level docs):
	// outer `div` + auto-wrapped body expression + the route resources,
	// voting form state/view, and route id flow into `page!` as typed parameters.
	//
	// The `load_detail_signal.result().is_some()` branch renders either the
	// voting form or an empty-state message, depending on whether the
	// question has any choices yet (reinhardt-web#4686). The previous
	// unconditional render of `voting_form` produced an empty
	// `RadioSelect` group for choiceless questions, so any submit emitted
	// `choice_id=""` and `submit_vote` rejected the request with the
	// runtime `Invalid choice_id` application error.
	page!(|load_detail: Resource<(QuestionInfo, Vec<ChoiceInfo>), String>, load_current_user: Resource<Option<UserInfo>, String>, choice_options_signal: Signal<Vec<(i64, String) >>, voting_form_page: Page, question_id: i64| {
		div { {
			match load_detail.get() {
				ResourceState::Loading => page!(|| {
					div {
						class: "max-w-4xl mx-auto px-4 mt-12 text-center",
						div {
							class: "spinner w-8 h-8",
							role: "status",
							span {
								class: "sr-only",
								"Loading..."
							}
						}
					}
				})(),
				ResourceState::Error(error) => page!(|error: String, question_id: i64| {
					div {
						class: "max-w-4xl mx-auto px-4 mt-12",
						div {
							class: "alert-danger",
							{
								self::format_server_error(&error)
							}
						}
						a {
							href: polls_routes::reverse("detail", &[("question_id", question_id.to_string().as_str())]),
							class: "btn-secondary",
							"Try Again"
						}
						a {
							href: polls_routes::reverse("index", &[]),
							class: "btn-primary ml-2",
							"Back to Polls"
						}
					}
				})(error, question_id),
				ResourceState::Success((q, choices)) => {
					// Owner-only controls (Edit / Delete / Add choice) are hidden for
					// non-authors and unauthenticated viewers (issue #4703). Any
					// non-`Success(Some(u))` shape leaves `is_author` as `false`.
					let is_author = matches!(load_current_user.get(), ResourceState::Success(Some(ref u))if u.id == q.author.id);
					// Render the voting form only when the question has choices;
					// otherwise show an empty-state prompt (reinhardt-web#4686).
					let choices_view = if choices.is_empty() {
						page!(|| {
							div {
								class: "alert-warning",
								"This question has no choices yet. Add one below to start voting."
							}
						})()
					} else {
						let choice_options: Vec<(i64, String) > = choices.iter().map(|c|(c.id, c.choice_text.clone())).collect();
						if choice_options_signal.get_untracked() != choice_options {
							choice_options_signal.set(choice_options);
						}
						voting_form_page.clone()
					};
					page!(|q: QuestionInfo, is_author: bool, choices_view: Page, question_id: i64| {
						div {
							class: "max-w-4xl mx-auto px-4 mt-12",
							div {
								class: "flex justify-between items-center mb-4",
								h1 { {
									q.question_text.clone()
								} }
								div {
									class: "flex gap-2",
									a {
										href: polls_routes::reverse("results", &[("question_id", question_id.to_string().as_str())]),
										class: "btn-secondary",
										"View results"
									}
									if is_author {
										a {
											href: polls_routes::reverse("question_edit", &[("question_id", question_id.to_string().as_str())]),
											class: "btn-secondary",
											"Edit"
										}
										a {
											href: polls_routes::reverse("question_delete", &[("question_id", question_id.to_string().as_str())]),
											class: "btn-danger",
											"Delete"
										}
									}
								}
							}
							{ choices_view }
							if is_author {
								div {
									class: "mt-4",
									a {
										href: polls_routes::reverse("choice_new", &[("question_id", question_id.to_string().as_str())]),
										class: "btn-secondary",
										"Add choice"
									}
								}
							}
						}
					})(q, is_author, choices_view, question_id)
				}
			}
		} }
	})(
		load_detail,
		load_current_user,
		choice_options_signal,
		voting_form_page,
		question_id,
	)
}

/// Poll results page - Show voting results
///
/// Displays the question with vote counts for each choice.
/// Uses watch blocks for reactive UI updates when async data loads.
///
/// Owner-only controls (Edit / Delete) mirror [`polls_detail`]: the viewer
/// is resolved via `current_user`, and the links only render when the
/// viewer is the question's author (issue #4703). Server-side
/// `require_question_author` checks remain in place as defense in depth.
pub fn polls_results(question_id: i64) -> Page {
	let load_results = use_resource(
		move || async move {
			get_question_results(question_id)
				.await
				.map_err(|e| e.to_string())
		},
		(),
	);
	let load_current_user = use_resource(
		|| async move { current_user().await.map_err(|e| e.to_string()) },
		(),
	);

	page!(|load_results: Resource<(QuestionInfo, Vec<ChoiceInfo>, i32), String>, load_current_user: Resource<Option<UserInfo>, String>, question_id: i64| {
		div { {
			match load_results.get() {
				ResourceState::Loading => page!(|| {
					div {
						class: "max-w-4xl mx-auto px-4 mt-12 text-center",
						div {
							class: "spinner w-8 h-8",
							role: "status",
							span {
								class: "sr-only",
								"Loading..."
							}
						}
					}
				})(),
				ResourceState::Error(error) => page!(|error: String| {
					div {
						class: "max-w-4xl mx-auto px-4 mt-12",
						div {
							class: "alert-danger",
							{
								self::format_server_error(&error)
							}
						}
						a {
							href: polls_routes::reverse("index", &[]),
							class: "btn-primary",
							"Back to Polls"
						}
					}
				})(error),
				ResourceState::Success((q, choices, total)) => {
					// Owner-only controls (Edit / Delete) are hidden for non-authors
					// and unauthenticated viewers (issue #4703).
					let is_author = matches!(load_current_user.get(), ResourceState::Success(Some(ref u))if u.id == q.author.id);
					page!(|q: QuestionInfo, choices: Vec<ChoiceInfo>, total: i32, is_author: bool, question_id: i64| {
						div {
							class: "max-w-4xl mx-auto px-4 mt-12",
							h1 {
								class: "mb-4",
								{
									q.question_text.clone()
								}
							}
							div {
								class: "card",
								div {
									class: "card-body",
									h5 {
										class: "text-xl font-bold",
										"Results"
									}
									div {
										class: "divide-y divide-border",
										for choice in choices { {
											let percentage = if total > 0 {
												(choice.votes as f64 / total as f64 * 100.0) as i32
											} else { 0 };
											let choice_text = choice.choice_text.clone();
											let votes = choice.votes;
											let progress_label = format!("{} received {} percent of votes", choice_text, percentage);
											page!(|choice_text: String, votes: i32, percentage: i32, progress_label: String| {
												div {
													class: "py-4",
													div {
														class: "flex justify-between items-center mb-2",
														strong { { choice_text } }
														span {
															class: "inline-flex items-center bg-brand rounded-full px-2.5 py-0.5 text-xs font-medium text-white",
															{
																format!("{} votes", votes)
															}
														}
													}
													div {
														class: "poll-result-meter-row",
														div {
															class: "poll-result-meter",
															role: "progressbar",
															aria_label: progress_label,
															style: format!("width: {}%", percentage),
															aria_valuenow: percentage.to_string(),
															aria_valuemin: "0",
															aria_valuemax: "100",
														}
														span {
															class: "poll-result-percent",
															{
																format!("{}%", percentage)
															}
														}
													}
												}
											})(choice_text, votes, percentage, progress_label)
										} }
									}
									div {
										class: "mt-3",
										p {
											class: "text-muted",
											{
												format!("Total votes: {}", total)
											}
										}
									}
								}
							}
							div {
								class: "mt-3 flex flex-wrap gap-2",
								a {
									href: polls_routes::reverse("detail", &[("question_id", question_id.to_string().as_str())]),
									class: "btn-primary",
									"Vote Again"
								}
								if is_author {
									a {
										href: polls_routes::reverse("question_edit", &[("question_id", question_id.to_string().as_str())]),
										class: "btn-secondary",
										"Edit question"
									}
									a {
										href: polls_routes::reverse("question_delete", &[("question_id", question_id.to_string().as_str())]),
										class: "btn-danger",
										"Delete question"
									}
								}
								a {
									href: polls_routes::reverse("index", &[]),
									class: "btn-secondary",
									"Back to Polls"
								}
							}
						}
					})(q, choices, total, is_author, question_id)
				}
			}
		} }
	})(load_results, load_current_user, question_id)
}

/// Example component demonstrating static URL resolution
///
/// This shows how to use resolve_static() for images in page! macros.
/// This function is identical to polls_index() but adds poll icons using
/// static URL resolution.
pub fn polls_index_with_logo() -> Page {
	let load_questions = use_resource(
		|| async move { get_questions().await.map_err(|e| e.to_string()) },
		(),
	);

	page!(|load_questions: Resource<Vec<QuestionInfo>, String>| {
		div {
			class: "max-w-4xl mx-auto px-4 mt-12",
			div {
				class: "text-center mb-6",
				img {
					src: reinhardt::pages::resolve_static("images/poll-icon.svg"),
					alt: "Polls App",
					class: "mx-auto w-16 h-16",
				}
			}
			h1 {
				class: "mb-4 text-center",
				"Polls"
			}
			{
				match load_questions.get() {
					ResourceState::Loading => page!(|| {
						div {
							class: "text-center",
							div {
								class: "spinner w-8 h-8",
								role: "status",
								span {
									class: "sr-only",
									"Loading..."
								}
							}
						}
					})(),
					ResourceState::Error(error) => page!(|error: String| {
						div {
							class: "alert-danger",
							{
								self::format_server_error(&error)
							}
						}
					})(error),
					ResourceState::Success(questions)if questions.is_empty() => page!(|| {
						p {
							class: "text-muted",
							"No polls are available."
						}
					})(),
					ResourceState::Success(questions) => page!(|questions: Vec<QuestionInfo>| {
						div {
							class: "space-y-2",
							for question in questions {
								a {
									href: format!("/polls/{}/", question.id),
									class: "block p-4 border border-border rounded-lg bg-surface-primary hover:bg-surface-secondary transition-colors",
									div {
										class: "flex w-full justify-between items-center",
										img {
											src: reinhardt::pages::resolve_static("images/poll-icon.svg"),
											alt: "Poll",
											class: "w-8 h-8 mr-3",
										}
										div {
											class: "flex-1",
											h5 {
												class: "mb-1",
												{
													question.question_text.clone()
												}
											}
										}
										small { {
											question.pub_date.format("%Y-%m-%d %H:%M").to_string()
										} }
									}
								}
							}
						}
					})(questions),
				}
			}
		}
	})(load_questions)
}

// =========================================================================
// Question CUD pages (Phase 2)
// =========================================================================
//
// All three pages share the same shape: a `form!` declaration backed by one
// of the CUD server functions in `crate::apps::polls::server_fn`. The server
// re-checks authentication and ownership, so these pages render
// unconditionally — unauthenticated visitors land on the form, submit it,
// and receive the 401 surfaced through the form's `error` signal.

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
			}
		}
	};

	let loading_signal = new_form.loading().clone();
	let error_signal = new_form.error().clone();
	let form_view = new_form.into_page();
	let cancel_href = polls_routes::reverse("index", &[]);

	page!(|loading_signal: Signal<bool>, error_signal: Signal<Option<String>>, form_view: Page, cancel_href: String| {
		div {
			class: "max-w-4xl mx-auto px-4 mt-12",
			h1 {
				class: "mb-4",
				"New Question"
			}
			{
				error_signal.get().map(|message| page!(|message: String| {
					div {
						class: "alert-danger mb-3",
						{
							self::format_server_error(&message)
						}
					}
				})(message)).unwrap_or(Page::Empty)
			}
			{ form_view }
			div {
				class: "mt-3",
				{
					let is_loading = loading_signal.get();
					page!(|is_loading: bool| {
						button {
							type: "submit",
							class: if is_loading { "btn-primary opacity-50 cursor-not-allowed" } else { "btn-primary" },
							disabled: is_loading,
							form: "new-question-form",
							{
								if is_loading { "Creating..." } else { "Create" }
							}
						}
					})(is_loading)
				}
				a {
					href: cancel_href,
					class: "btn-secondary ml-2",
					"Cancel"
				}
			}
		}
	})(loading_signal, error_signal, form_view, cancel_href)
}

/// Edit question page (`/polls/{question_id}/edit/`).
///
/// Loads the existing question via `get_question_detail`, then renders an
/// edit form pre-populated with the current text. The server enforces that
/// only the author can submit successfully.
pub fn question_edit(question_id: i64) -> Page {
	let qid = question_id;
	let load_detail = use_resource(
		move || async move { get_question_detail(qid).await.map_err(|e| e.to_string()) },
		(),
	);

	let edit_form = form! {
		name: EditQuestionForm,
		server_fn: update_question,
		method: Post,
		redirect_on_success: "/",
		fields: {
			question_id: HiddenField<i64> {
				initial: qid,
			}
			question_text: CharField {
				label: "Question",
				placeholder: "Updated question text",
				max_length: 200,
				class: "form-control",
			}
		}
	};

	// Prefill the question_text input once the load_detail resource resolves,
	// re-running whenever its state changes.
	{
		let load_detail_for_effect = load_detail.clone();
		let load_detail_for_deps = load_detail.clone();
		let edit_form_for_effect = edit_form.clone();
		use_effect(
			move || {
				if let ResourceState::Success((question, _)) = load_detail_for_effect.get() {
					edit_form_for_effect
						.question_text()
						.set(question.question_text.clone());
				}
				None::<fn()>
			},
			(load_detail_for_deps,),
		);
	}

	// Build the edit-form view once; its internal `{ ... }` blocks subscribe to
	// the form's error/loading signals so it stays reactive when embedded below.
	let edit_form_error = edit_form.error().clone();
	let edit_form_loading = edit_form.loading().clone();
	let edit_form_page = edit_form.into_page();
	let edit_form_view = page!(|edit_form_error: Signal<Option<String>>, edit_form_loading: Signal<bool>, edit_form_page: Page, question_id: i64| {
		div {
			class: "max-w-4xl mx-auto px-4 mt-12",
			h1 {
				class: "mb-4",
				"Edit Question"
			}
			{
				edit_form_error.get().map(|message| page!(|message: String| {
					div {
						class: "alert-danger mb-3",
						{
							self::format_server_error(&message)
						}
					}
				})(message)).unwrap_or(Page::Empty)
			}
			{ edit_form_page }
			div {
				class: "mt-3",
				{
					let is_loading = edit_form_loading.get();
					page!(|is_loading: bool, question_id: i64| {
						button {
							type: "submit",
							class: if is_loading { "btn-primary opacity-50 cursor-not-allowed" } else { "btn-primary" },
							disabled: is_loading,
							form: "edit-question-form",
							{
								if is_loading { "Saving..." } else { "Save" }
							}
						}
						a {
							href: polls_routes::reverse("detail", &[("question_id", question_id.to_string().as_str())]),
							class: "btn-secondary ml-2",
							"Cancel"
						}
					})(is_loading, question_id)
				}
			}
		}
	})(
		edit_form_error,
		edit_form_loading,
		edit_form_page,
		question_id,
	);

	page!(|load_detail: Resource<(QuestionInfo, Vec<ChoiceInfo>), String>, edit_form_view: Page, question_id: i64| {
		div { {
			match load_detail.get() {
				ResourceState::Loading => page!(|| {
					div {
						class: "max-w-4xl mx-auto px-4 mt-12 text-center",
						div {
							class: "spinner w-8 h-8",
							role: "status",
							span {
								class: "sr-only",
								"Loading..."
							}
						}
					}
				})(),
				ResourceState::Error(error) => page!(|error: String| {
					div {
						class: "max-w-4xl mx-auto px-4 mt-12",
						div {
							class: "alert-danger",
							{
								self::format_server_error(&error)
							}
						}
						a {
							href: polls_routes::reverse("index", &[]),
							class: "btn-primary",
							"Back to Polls"
						}
					}
				})(error),
				ResourceState::Success(_) => edit_form_view.clone(),
			}
		} }
	})(load_detail, edit_form_view, question_id)
}

/// Delete confirmation page (`/polls/{question_id}/delete/`).
pub fn question_delete_confirm(question_id: i64) -> Page {
	let qid = question_id;
	let load_detail = use_resource(
		move || async move { get_question_detail(qid).await.map_err(|e| e.to_string()) },
		(),
	);

	let delete_form = form! {
		name: DeleteQuestionForm,
		server_fn: delete_question,
		method: Post,
		redirect_on_success: "/",
		fields: {
			question_id: HiddenField<i64> {
				initial: qid,
			}
		}
	};

	let loading_signal = delete_form.loading().clone();
	let error_signal = delete_form.error().clone();
	let form_view = delete_form.into_page();
	let cancel_href = polls_routes::reverse(
		"detail",
		&[("question_id", question_id.to_string().as_str())],
	);

	page!(|load_detail: Resource<(QuestionInfo, Vec<ChoiceInfo>), String>, error_signal: Signal<Option<String>>, loading_signal: Signal<bool>, form_view: Page, cancel_href: String| {
		div {
			class: "max-w-4xl mx-auto px-4 mt-12",
			h1 {
				class: "mb-4",
				"Delete Question?"
			}
			{
				match load_detail.get() {
					ResourceState::Loading => page!(|| {
						div {
							class: "text-center",
							"Loading..."
						}
					})(),
					ResourceState::Success((q, _)) => page!(|q: QuestionInfo| {
						div {
							class: "card",
							div {
								class: "card-body",
								p {
									class: "text-muted",
									"You are about to delete the following question. This action cannot be undone."
								}
								blockquote {
									class: "border-l-4 border-border-secondary pl-4 italic my-3",
									{
										q.question_text.clone()
									}
								}
							}
						}
					})(q),
					ResourceState::Error(error) => page!(|error: String| {
						div {
							class: "alert-danger",
							{
								self::format_server_error(&error)
							}
						}
					})(error),
				}
			}
			{
				error_signal.get().map(|message| page!(|message: String| {
					div {
						class: "alert-danger mt-3",
						{
							self::format_server_error(&message)
						}
					}
				})(message)).unwrap_or(Page::Empty)
			}
			{ form_view }
			div {
				class: "mt-3",
				{
					let is_loading = loading_signal.get();
					page!(|is_loading: bool, cancel_href: String| {
						button {
							type: "submit",
							class: if is_loading { "btn-primary opacity-50 cursor-not-allowed" } else { "btn-danger" },
							disabled: is_loading,
							form: "delete-question-form",
							{
								if is_loading { "Deleting..." } else { "Delete" }
							}
						}
						a {
							href: cancel_href,
							class: "btn-secondary ml-2",
							"Cancel"
						}
					})(is_loading, cancel_href.clone())
				}
			}
		}
	})(
		load_detail,
		error_signal,
		loading_signal,
		form_view,
		cancel_href,
	)
}

// =========================================================================
// Choice CUD pages (Phase 3)
// =========================================================================
//
// Same shape as the Question CUD pages above. Ownership is enforced
// server-side via the parent question's author check; the client pages
// render unconditionally and surface 401/403 through the form error
// signal.

/// New choice page (`/polls/{question_id}/choices/new/`).
pub fn choice_new(question_id: i64) -> Page {
	let qid = question_id;

	// `redirect_on_success` (issue #4700): without it the form submits
	// successfully but the client stays on `/polls/{qid}/choices/new/`
	// with no visible feedback, so the user perceives the action as a
	// no-op. Returning to the parent detail page makes the newly added
	// choice immediately visible.
	//
	// `required` on `choice_text` (issue #4701, defense in depth): the
	// server's `create_choice` already rejects empty input, but anchoring
	// the rule in the browser at the field level prevents blank submits
	// from reaching the server at all.
	let new_form = form! {
		name: NewChoiceForm,
		server_fn: create_choice,
		method: Post,
		success_url: |_form| polls_routes::reverse("detail", &[("question_id", qid.to_string().as_str())]),
		fields: {
			question_id: HiddenField<i64> {
				initial: qid,
			},
			choice_text: CharField {
				label: "Choice text",
				placeholder: "An answer option",
				required,
				max_length: 200,
				class: "form-control",
			},
		},
	};

	let loading_signal = new_form.loading().clone();
	let error_signal = new_form.error().clone();
	let form_view = new_form.into_page();
	let back_href = polls_routes::reverse("detail", &[("question_id", qid.to_string().as_str())]);

	page!(|loading_signal: Signal<bool>, error_signal: Signal<Option<String>>, form_view: Page, back_href: String| {
		div {
			class: "max-w-4xl mx-auto px-4 mt-12",
			h1 {
				class: "mb-4",
				"Add a Choice"
			}
			{
				error_signal.get().map(|message| page!(|message: String| {
					div {
						class: "alert-danger mb-3",
						{
							self::format_server_error(&message)
						}
					}
				})(message)).unwrap_or(Page::Empty)
			}
			{ form_view }
			div {
				class: "mt-3",
				{
					let is_loading = loading_signal.get();
					page!(|is_loading: bool| {
						button {
							type: "submit",
							class: if is_loading { "btn-primary opacity-50 cursor-not-allowed" } else { "btn-primary" },
							disabled: is_loading,
							form: "new-choice-form",
							{
								if is_loading { "Adding..." } else { "Add Choice" }
							}
						}
					})(is_loading)
				}
				a {
					href: back_href,
					class: "btn-secondary ml-2",
					"Back to poll"
				}
			}
		}
	})(loading_signal, error_signal, form_view, back_href)
}

/// Edit choice page (`/polls/{question_id}/choices/{choice_id}/edit/`).
///
/// Both ids are carried in the route, so the "Cancel" link to the parent
/// poll is synchronous — no extra server roundtrip and no
/// pending-state fallback href.
pub fn choice_edit(question_id: i64, choice_id: i64) -> Page {
	let cancel_href = polls_routes::reverse(
		"detail",
		&[("question_id", question_id.to_string().as_str())],
	);

	let edit_form = form! {
		name: EditChoiceForm,
		server_fn: update_choice,
		method: Post,
		redirect_on_success: "/",
		fields: {
			choice_id: HiddenField<i64> {
				initial: choice_id,
			}
			choice_text: CharField {
				label: "Choice text",
				placeholder: "Updated answer option",
				max_length: 200,
				class: "form-control",
			}
		}
	};

	let loading_signal = edit_form.loading().clone();
	let error_signal = edit_form.error().clone();
	let form_view = edit_form.into_page();

	page!(|loading_signal: Signal<bool>, error_signal: Signal<Option<String>>, form_view: Page, cancel_href: String| {
		div {
			class: "max-w-4xl mx-auto px-4 mt-12",
			h1 {
				class: "mb-4",
				"Edit Choice"
			}
			{
				error_signal.get().map(|message| page!(|message: String| {
					div {
						class: "alert-danger mb-3",
						{
							self::format_server_error(&message)
						}
					}
				})(message)).unwrap_or(Page::Empty)
			}
			{ form_view }
			div {
				class: "mt-3",
				{
					let is_loading = loading_signal.get();
					page!(|is_loading: bool| {
						button {
							type: "submit",
							class: if is_loading { "btn-primary opacity-50 cursor-not-allowed" } else { "btn-primary" },
							disabled: is_loading,
							form: "edit-choice-form",
							{
								if is_loading { "Saving..." } else { "Save" }
							}
						}
					})(is_loading)
				}
				a {
					href: cancel_href,
					class: "btn-secondary ml-2",
					"Cancel"
				}
			}
		}
	})(loading_signal, error_signal, form_view, cancel_href)
}

/// Delete-choice confirmation page
/// (`/polls/{question_id}/choices/{choice_id}/delete/`).
///
/// Like [`choice_edit`], both ids are part of the route so "Cancel"
/// links back to the parent poll synchronously without an extra fetch.
pub fn choice_delete_confirm(question_id: i64, choice_id: i64) -> Page {
	let cancel_href = polls_routes::reverse(
		"detail",
		&[("question_id", question_id.to_string().as_str())],
	);

	let delete_form = form! {
		name: DeleteChoiceForm,
		server_fn: delete_choice,
		method: Post,
		redirect_on_success: "/",
		fields: {
			choice_id: HiddenField<i64> {
				initial: choice_id,
			}
		}
	};

	let loading_signal = delete_form.loading().clone();
	let error_signal = delete_form.error().clone();
	let form_view = delete_form.into_page();

	page!(|loading_signal: Signal<bool>, error_signal: Signal<Option<String>>, form_view: Page, cancel_href: String| {
		div {
			class: "max-w-4xl mx-auto px-4 mt-12",
			h1 {
				class: "mb-4",
				"Delete Choice?"
			}
			p {
				class: "mb-3",
				"This action cannot be undone."
			}
			{
				error_signal.get().map(|message| page!(|message: String| {
					div {
						class: "alert-danger mt-3",
						{
							self::format_server_error(&message)
						}
					}
				})(message)).unwrap_or(Page::Empty)
			}
			{ form_view }
			div {
				class: "mt-3",
				{
					let is_loading = loading_signal.get();
					page!(|is_loading: bool| {
						button {
							type: "submit",
							class: if is_loading { "btn-primary opacity-50 cursor-not-allowed" } else { "btn-danger" },
							disabled: is_loading,
							form: "delete-choice-form",
							{
								if is_loading { "Deleting..." } else { "Delete" }
							}
						}
					})(is_loading)
				}
				a {
					href: cancel_href,
					class: "btn-secondary ml-2",
					"Cancel"
				}
			}
		}
	})(loading_signal, error_signal, form_view, cancel_href)
}
