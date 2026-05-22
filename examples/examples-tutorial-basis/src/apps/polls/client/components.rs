//! Polling application components
//!
//! Provides UI components for the polling application including
//! the index page, detail page with voting form, and results page.
//!
//! ## Reactive page shape (canonical template)
//!
//! Every async-loading view in this module (`polls_detail`,
//! `polls_results`, `question_edit`, `question_delete_confirm`) follows
//! the same reactive shape as `polls_results`:
//!
//! - An outer `div` wraps a single `watch{}` block so the function
//!   returns a top-level `Page::Element` (matching the
//!   `matches!(view, Page::Element(_))` assertion in
//!   `tests/wasm/polls_mock_test.rs`).
//! - Only the reactive `Signal` (`Action<..>`) and the route id flow
//!   into `page!` as typed parameters. Forms (whose types live inside
//!   the `form!` macro's block expression and are therefore not
//!   nameable as `page!` parameter types) and static hrefs are
//!   captured from the surrounding scope by the implicit `move` of
//!   the `watch` closure.
//!
//! **Anti-pattern**: returning a static `Page`
//! (e.g. `return page!(|| spinner)();`) from outside the `watch{}` block
//! strands the SPA on the spinner forever — the reactive subscription is
//! never established. Every loading branch MUST stay inside the single
//! outer `watch{}` block.

use crate::shared::types::{ChoiceInfo, QuestionInfo};
use reinhardt::pages::component::Page;
use reinhardt::pages::form;
use reinhardt::pages::page;
use reinhardt::pages::reactive::hooks::{Action, use_action, use_effect};
use reinhardt::pages::resolve_static;

// Typed URL helpers are now emitted by `#[url_patterns]` directly
// (issue #4656); we alias the macro-emitted `urls` module as `links` to
// keep call sites concise.
use crate::apps::polls::server_fn::{
	create_choice, create_question, delete_choice, delete_question, get_question_detail,
	get_question_results, get_questions, submit_vote, update_choice, update_question,
};
use crate::apps::polls::urls::client_router::urls as links;

/// Polls index page - List all polls
///
/// Displays a list of available polls with links to vote.
/// Uses watch blocks for reactive UI updates when async data loads.
pub fn polls_index() -> Page {
	let load_questions =
		use_action(|_: ()| async move { get_questions().await.map_err(|e| e.to_string()) });
	load_questions.dispatch(());

	let load_questions_error = load_questions.clone();
	let load_questions_signal = load_questions.clone();
	let new_question_href = links::question_new();

	page!(|load_questions_error: Action<Vec<QuestionInfo>, String>, load_questions_signal: Action<Vec<QuestionInfo>, String>, new_question_href: String| {
		div {
			class: "max-w-4xl mx-auto px-4 mt-12",
			div {
				class: "flex justify-between items-center mb-4",
				h1 {
					"Polls"
				}
				a {
					href: new_question_href,
					class: "btn-primary",
					"New Question"
				}
			}
			watch {
				if load_questions_error.error().is_some() {
					div {
						class: "alert-danger",
						{ load_questions_error.error().unwrap_or_default() }
					}
				}
			}
			watch {
				if load_questions_signal.is_pending() {
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
				} else if load_questions_signal.result().unwrap_or_default().is_empty() {
					p {
						class: "text-muted",
						"No polls are available."
					}
				} else {
					div {
						class: "space-y-2",
						for question in load_questions_signal.result().unwrap_or_default() {
							a {
								href: links::detail(question.id),
								class: "block p-4 border border-border rounded-lg bg-surface-primary hover:bg-surface-secondary transition-colors",
								div {
									class: "flex w-full justify-between",
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
		}
	})(
		load_questions_error,
		load_questions_signal,
		new_question_href,
	)
}

/// Poll detail page - Show question and voting form
///
/// Displays a question with its choices and allows the user to vote.
/// Uses form! macro with Dynamic ChoiceField for declarative form handling.
/// CSRF protection is automatically injected for POST method.
pub fn polls_detail(question_id: i64) -> Page {
	let qid = question_id;

	// Create action for loading question detail
	let load_detail =
		use_action(
			|qid: i64| async move { get_question_detail(qid).await.map_err(|e| e.to_string()) },
		);

	// Voting form via the `form!` macro.
	//
	// - `server_fn: submit_vote` binds the form to the server function whose
	//   typed signature is `(question_id, choice_id, csrf_token)`.
	// - `method: Post` enables CSRF hidden-input rendering for non-WASM submits.
	// - `strip_arguments: { csrf_token: ... }` routes the CSRF token to the
	//   trailing server_fn argument — the macro then strips it from the
	//   client-side argument list so the form only owns `question_id` and
	//   `choice_id`. CSRF verification still happens server-side in the CSRF
	//   middleware before this handler runs.
	// - `state: { loading, error }` exposes per-field signals to drive the
	//   submit button and error banner below.
	// - `success_url: |_form| ...` triggers an in-SPA navigation to the results
	//   page after a successful vote. The closure captures `qid` from the
	//   outer scope; the macro stores it on the generated form struct and the
	//   generated `submit()` method dispatches through `pages::navigate()` so
	//   the route table installed by `ClientLauncher::router_client` handles
	//   the transition without a full page load.
	let voting_form = form! {
		name: VotingForm,
		server_fn: submit_vote,
		method: Post,
		state: { loading, error },

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
		},

		success_url: |_form| links::results(qid),
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

	let load_detail_signal = load_detail.clone();

	// Render reactively in the canonical shape (see module-level docs):
	// outer `div` + single `watch{}` block + the `Action<..>` signal and
	// the route id flow into `page!` as typed parameters. The voting form
	// is captured by the watch closure's implicit `move`.
	//
	// The `load_detail_signal.result().is_some()` branch renders either the
	// voting form or an empty-state message, depending on whether the
	// question has any choices yet (reinhardt-web#4686). The previous
	// unconditional render of `voting_form` produced an empty
	// `RadioSelect` group for choiceless questions, so any submit emitted
	// `choice_id=""` and `submit_vote` rejected the request with the
	// runtime `Invalid choice_id` application error.
	page!(|load_detail_signal: Action<(QuestionInfo, Vec<ChoiceInfo>), String>, question_id: i64| {
		div {
			watch {
				if load_detail_signal.is_pending() {
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
				} else if load_detail_signal.error().is_some() {
					div {
						class: "max-w-4xl mx-auto px-4 mt-12",
						div {
							class: "alert-danger",
							{ load_detail_signal.error().unwrap_or_default() }
						}
						a {
							href: links::detail(question_id),
							class: "btn-secondary",
							"Try Again"
						}
						a {
							href: links::index(),
							class: "btn-primary ml-2",
							"Back to Polls"
						}
					}
				} else if let Some((ref q, ref choices)) = load_detail_signal.result() {
					// Bind the result once: `Action::result()` clones the
					// underlying `(QuestionInfo, Vec<ChoiceInfo>)` on every
					// call, so reusing `q`/`choices` here avoids redundant
					// allocations on each reactive render.
					div {
						class: "max-w-4xl mx-auto px-4 mt-12",
						div {
							class: "flex justify-between items-center mb-4",
							h1 {
								{ q.question_text.clone() }
							}
							div {
								class: "flex gap-2",
								a {
									href: links::results(question_id),
									class: "btn-secondary",
									"View results"
								}
								a {
									href: links::question_edit(question_id),
									class: "btn-secondary",
									"Edit"
								}
								a {
									href: links::question_delete(question_id),
									class: "btn-danger",
									"Delete"
								}
							}
						}
						if !choices.is_empty() {
							{ voting_form.clone().into_page() }
						} else {
							div {
								class: "alert-warning",
								"This question has no choices yet. Add one below to start voting."
							}
						}
						div {
							class: "mt-4",
							a {
								href: links::choice_new(question_id),
								class: "btn-secondary",
								"Add choice"
							}
						}
					}
				} else {
					div {
						class: "max-w-4xl mx-auto px-4 mt-12",
						div {
							class: "alert-warning",
							"Question not found"
						}
						a {
							href: links::index(),
							class: "btn-primary",
							"Back to Polls"
						}
					}
				}
			}
		}
	})(load_detail_signal, question_id)
}

/// Poll results page - Show voting results
///
/// Displays the question with vote counts for each choice.
/// Uses watch blocks for reactive UI updates when async data loads.
pub fn polls_results(question_id: i64) -> Page {
	let load_results =
		use_action(
			|qid: i64| async move { get_question_results(qid).await.map_err(|e| e.to_string()) },
		);
	load_results.dispatch(question_id);

	let load_results_signal = load_results.clone();

	page!(|load_results_signal: Action<(QuestionInfo, Vec<ChoiceInfo>, i32), String>, question_id: i64| {
		div {
			watch {
				if load_results_signal.is_pending() {
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
				} else if load_results_signal.error().is_some() {
					div {
						class: "max-w-4xl mx-auto px-4 mt-12",
						div {
							class: "alert-danger",
							{ load_results_signal.error().unwrap_or_default() }
						}
						a {
							href: links::index(),
							class: "btn-primary",
							"Back to Polls"
						}
					}
				} else if load_results_signal.result().is_some() {
					div {
						class: "max-w-4xl mx-auto px-4 mt-12",
						h1 {
							class: "mb-4",
							{
								load_results_signal
										.result()
										.map(|(q, _, _)| q.question_text.clone())
										.unwrap_or_default()
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
									{
										if let Some((_, choices, total)) = load_results_signal.result() {
												page!(|choices: Vec<ChoiceInfo>, total: i32| {
													for choice in choices {
														{
															{
																	let percentage = if total > 0 {
																		(choice.votes as f64 / total as f64 * 100.0) as i32
																	} else {
																		0
																	};
																	page!(|choice: ChoiceInfo, percentage: i32| {
																		div {
																			class: "py-4",
																			div {
																				class: "flex justify-between items-center mb-2",
																				strong {
																					{ choice.choice_text.clone() }
																				}
																				span {
																					class: "inline-flex items-center bg-brand rounded-full px-2.5 py-0.5 text-xs font-medium text-white",
																					{ format!("{} votes", choice.votes) }
																				}
																			}
																			div {
																				class: "w-full bg-surface-tertiary rounded-full h-2.5",
																				div {
																					class: "bg-brand h-2.5 rounded-full",
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
												})(choices, total)
											} else {
												Page::Empty
											}
									}
								}
								div {
									class: "mt-3",
									p {
										class: "text-muted",
										{
											format!(
													"Total votes: {}",
													load_results_signal
														.result()
														.map(|(_, _, total)| total)
														.unwrap_or(0)
												)
										}
									}
								}
							}
						}
						div {
							class: "mt-3 flex flex-wrap gap-2",
							a {
								href: links::detail(question_id),
								class: "btn-primary",
								"Vote Again"
							}
							a {
								href: links::question_edit(question_id),
								class: "btn-secondary",
								"Edit question"
							}
							a {
								href: links::question_delete(question_id),
								class: "btn-danger",
								"Delete question"
							}
							a {
								href: links::index(),
								class: "btn-secondary",
								"Back to Polls"
							}
						}
					}
				} else {
					div {
						class: "max-w-4xl mx-auto px-4 mt-12",
						div {
							class: "alert-warning",
							"Question not found"
						}
						a {
							href: links::index(),
							class: "btn-primary",
							"Back to Polls"
						}
					}
				}
			}
		}
	})(load_results_signal, question_id)
}

/// Example component demonstrating static URL resolution
///
/// This shows how to use resolve_static() for images in page! macros.
/// This function is identical to polls_index() but adds poll icons using
/// static URL resolution.
pub fn polls_index_with_logo() -> Page {
	let load_questions =
		use_action(|_: ()| async move { get_questions().await.map_err(|e| e.to_string()) });
	load_questions.dispatch(());

	let load_questions_error = load_questions.clone();
	let load_questions_signal = load_questions.clone();

	page!(|load_questions_error: Action<Vec<QuestionInfo>, String>, load_questions_signal: Action<Vec<QuestionInfo>, String>| {
		div {
			class: "max-w-4xl mx-auto px-4 mt-12",
			div {
				class: "text-center mb-6",
				img {
					src: resolve_static("images/poll-icon.svg"),
					alt: "Polls App",
					class: "mx-auto w-16 h-16",
				}
			}
			h1 {
				class: "mb-4 text-center",
				"Polls"
			}
			watch {
				if load_questions_error.error().is_some() {
					div {
						class: "alert-danger",
						{ load_questions_error.error().unwrap_or_default() }
					}
				}
			}
			watch {
				if load_questions_signal.is_pending() {
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
				} else if load_questions_signal.result().unwrap_or_default().is_empty() {
					p {
						class: "text-muted",
						"No polls are available."
					}
				} else {
					div {
						class: "space-y-2",
						for question in load_questions_signal.result().unwrap_or_default() {
							a {
								href: format!("/polls/{}/", question.id),
								class: "block p-4 border border-border rounded-lg bg-surface-primary hover:bg-surface-secondary transition-colors",
								div {
									class: "flex w-full justify-between items-center",
									img {
										src: resolve_static("images/poll-icon.svg"),
										alt: "Poll",
										class: "w-8 h-8 mr-3",
									}
									div {
										class: "flex-1",
										h5 {
											class: "mb-1",
											{ question.question_text.clone() }
										}
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
		}
	})(load_questions_error, load_questions_signal)
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
		state: { loading, error },
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
			h1 {
				class: "mb-4",
				"New Question"
			}
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

	let load_detail =
		use_action(
			|qid: i64| async move { get_question_detail(qid).await.map_err(|e| e.to_string()) },
		);
	load_detail.dispatch(qid);

	let edit_form = form! {
		name: EditQuestionForm,
		server_fn: update_question,
		method: Post,
		state: { loading, error },
		redirect_on_success: "/",

		fields: {
			question_id: HiddenField {
				initial: qid.to_string(),
			},
			question_text: CharField {
				label: "Question",
				placeholder: "Updated question text",
				max_length: 200,
				class: "form-control",
			},
		},

		strip_arguments: {
			csrf_token: ::reinhardt::reinhardt_pages::csrf::get_csrf_token()
				.unwrap_or_default(),
		},
	};

	// Prefill the question_text input once the load_detail action resolves.
	{
		let load_detail_for_effect = load_detail.clone();
		let edit_form_for_effect = edit_form.clone();
		use_effect(move || {
			if let Some((ref question, _)) = load_detail_for_effect.result() {
				edit_form_for_effect
					.question_text()
					.set(question.question_text.clone());
			}
		});
	}

	let load_detail_signal = load_detail.clone();

	// Render reactively. The previous shape used an outer `watch{}` on
	// `load_detail_signal` with *nested* `watch{}` blocks on
	// `edit_form.error()` / `edit_form.loading()`. Each `watch{}` lowers
	// to `Page::reactive(move || ...)` (`Fn() -> Page + 'static`); the
	// inner closures each tried to take the non-`Copy`
	// `EditQuestionForm` out of the outer closure a second time, which
	// rustc rejects with `E0507` (issue #4515,
	// `crates/reinhardt-pages/docs/watch_semantics.md` § "Fix 1").
	//
	// The DSL does not accept Rust `let` statements between `} else {`
	// and the next node, so the doc's "Fix 2" (clone the form into
	// outer-scope locals inside the `else` branch) is not currently
	// expressible. Apply "Fix 1" (preferred): a single `watch{}` already
	// subscribes to every signal it reads, so collapsing all three
	// `watch{}` blocks into the outer one removes the nested-capture
	// move and still re-renders on `load_detail_signal`,
	// `edit_form.error()`, and `edit_form.loading()` changes.
	page!(|load_detail_signal: Action<(QuestionInfo, Vec<ChoiceInfo>), String>, question_id: i64| {
		div {
			watch {
				if load_detail_signal.is_pending() {
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
				} else if load_detail_signal.error().is_some() {
					div {
						class: "max-w-4xl mx-auto px-4 mt-12",
						div {
							class: "alert-danger",
							{ load_detail_signal.error().unwrap_or_default() }
						}
						a {
							href: links::index(),
							class: "btn-primary",
							"Back to Polls"
						}
					}
				} else {
					div {
						class: "max-w-4xl mx-auto px-4 mt-12",
						h1 {
							class: "mb-4",
							"Edit Question"
						}
						if edit_form.error().get().is_some() {
							div {
								class: "alert-danger mb-3",
								{ edit_form.error().get().unwrap_or_default() }
							}
						}
						{ edit_form.clone().into_page() }
						div {
							class: "mt-3",
							if edit_form.loading().get() {
								button {
									type: "submit",
									class: "btn-primary opacity-50 cursor-not-allowed",
									disabled: true,
									form: "edit-question-form",
									"Saving..."
								}
							} else {
								button {
									type: "submit",
									class: "btn-primary",
									form: "edit-question-form",
									"Save"
								}
							}
							a {
								href: links::detail(question_id),
								class: "btn-secondary ml-2",
								"Cancel"
							}
						}
					}
				}
			}
		}
	})(load_detail_signal, question_id)
}

/// Delete confirmation page (`/polls/{question_id}/delete/`).
pub fn question_delete_confirm(question_id: i64) -> Page {
	let qid = question_id;

	let load_detail =
		use_action(
			|qid: i64| async move { get_question_detail(qid).await.map_err(|e| e.to_string()) },
		);
	load_detail.dispatch(qid);

	let delete_form = form! {
		name: DeleteQuestionForm,
		server_fn: delete_question,
		method: Post,
		state: { loading, error },
		redirect_on_success: "/",

		fields: {
			question_id: HiddenField {
				initial: qid.to_string(),
			},
		},

		strip_arguments: {
			csrf_token: ::reinhardt::reinhardt_pages::csrf::get_csrf_token()
				.unwrap_or_default(),
		},
	};

	let loading_signal = delete_form.loading().clone();
	let error_signal = delete_form.error().clone();
	let form_view = delete_form.into_page();
	let load_detail_signal = load_detail.clone();
	let cancel_href = links::detail(question_id);

	page!(|load_detail_signal: Action<(QuestionInfo, Vec<ChoiceInfo>), String>, loading_signal: reinhardt::pages::reactive::Signal<bool>, error_signal: reinhardt::pages::reactive::Signal<Option<String>>, form_view: Page, cancel_href: String| {
		div {
			class: "max-w-4xl mx-auto px-4 mt-12",
			h1 {
				class: "mb-4",
				"Delete Question?"
			}
			watch {
				if load_detail_signal.is_pending() {
					div {
						class: "text-center",
						"Loading..."
					}
				} else if let Some((ref q, _)) = load_detail_signal.result() {
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
								{ q.question_text.clone() }
							}
						}
					}
				} else if load_detail_signal.error().is_some() {
					div {
						class: "alert-danger",
						{ load_detail_signal.error().unwrap_or_default() }
					}
				}
			}
			watch {
				if error_signal.get().is_some() {
					div {
						class: "alert-danger mt-3",
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
							form: "delete-question-form",
							"Deleting..."
						}
					} else {
						button {
							type: "submit",
							class: "btn-danger",
							form: "delete-question-form",
							"Delete"
						}
					}
				}
				a {
					href: cancel_href,
					class: "btn-secondary ml-2",
					"Cancel"
				}
			}
		}
	})(
		load_detail_signal,
		loading_signal,
		error_signal,
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
	let qid_str = qid.to_string();

	let new_form = form! {
		name: NewChoiceForm,
		server_fn: create_choice,
		method: Post,
		state: { loading, error },

		fields: {
			question_id: HiddenField {
				initial: qid_str,
			},
			choice_text: CharField {
				label: "Choice text",
				placeholder: "An answer option",
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
	let back_href = links::detail(qid);

	page!(|loading_signal: reinhardt::pages::reactive::Signal<bool>, error_signal: reinhardt::pages::reactive::Signal<Option<String>>, form_view: Page, back_href: String| {
		div {
			class: "max-w-4xl mx-auto px-4 mt-12",
			h1 {
				class: "mb-4",
				"Add a Choice"
			}
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
							form: "new-choice-form",
							"Adding..."
						}
					} else {
						button {
							type: "submit",
							class: "btn-primary",
							form: "new-choice-form",
							"Add Choice"
						}
					}
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
	let cid_str = choice_id.to_string();
	let cancel_href = links::detail(question_id);

	let edit_form = form! {
		name: EditChoiceForm,
		server_fn: update_choice,
		method: Post,
		state: { loading, error },
		redirect_on_success: "/",

		fields: {
			choice_id: HiddenField {
				initial: cid_str,
			},
			choice_text: CharField {
				label: "Choice text",
				placeholder: "Updated answer option",
				max_length: 200,
				class: "form-control",
			},
		},

		strip_arguments: {
			csrf_token: ::reinhardt::reinhardt_pages::csrf::get_csrf_token()
				.unwrap_or_default(),
		},
	};

	let loading_signal = edit_form.loading().clone();
	let error_signal = edit_form.error().clone();
	let form_view = edit_form.into_page();

	page!(|loading_signal: reinhardt::pages::reactive::Signal<bool>, error_signal: reinhardt::pages::reactive::Signal<Option<String>>, form_view: Page, cancel_href: String| {
		div {
			class: "max-w-4xl mx-auto px-4 mt-12",
			h1 {
				class: "mb-4",
				"Edit Choice"
			}
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
							form: "edit-choice-form",
							"Saving..."
						}
					} else {
						button {
							type: "submit",
							class: "btn-primary",
							form: "edit-choice-form",
							"Save"
						}
					}
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
	let cid_str = choice_id.to_string();
	let cancel_href = links::detail(question_id);

	let delete_form = form! {
		name: DeleteChoiceForm,
		server_fn: delete_choice,
		method: Post,
		state: { loading, error },
		redirect_on_success: "/",

		fields: {
			choice_id: HiddenField {
				initial: cid_str,
			},
		},

		strip_arguments: {
			csrf_token: ::reinhardt::reinhardt_pages::csrf::get_csrf_token()
				.unwrap_or_default(),
		},
	};

	let loading_signal = delete_form.loading().clone();
	let error_signal = delete_form.error().clone();
	let form_view = delete_form.into_page();

	page!(|loading_signal: reinhardt::pages::reactive::Signal<bool>, error_signal: reinhardt::pages::reactive::Signal<Option<String>>, form_view: Page, cancel_href: String| {
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
			watch {
				if error_signal.get().is_some() {
					div {
						class: "alert-danger mt-3",
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
							form: "delete-choice-form",
							"Deleting..."
						}
					} else {
						button {
							type: "submit",
							class: "btn-danger",
							form: "delete-choice-form",
							"Delete"
						}
					}
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
