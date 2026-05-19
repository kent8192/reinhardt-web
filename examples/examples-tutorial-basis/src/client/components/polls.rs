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
//! **Anti-pattern that root-caused Issue #4514:** returning a static
//! `Page` (e.g. `return page!(|| spinner)();`) from outside the
//! `watch{}` block strands the SPA on the spinner forever because the
//! reactive subscription is never established.

use crate::shared::types::{ChoiceInfo, QuestionInfo};
use reinhardt::pages::component::Page;
use reinhardt::pages::form;
use reinhardt::pages::page;
use reinhardt::pages::reactive::hooks::{Action, use_action, use_effect};

use crate::apps::polls::server_fn::{
	create_choice, create_question, delete_choice, delete_question, get_question_detail,
	get_question_results, get_questions, submit_vote, update_choice, update_question,
};
use crate::client::links;

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
						{
							Page::Fragment(
							        load_questions_signal
							            .result()
							            .unwrap_or_default()
							            .iter()
							            .map(|question| {
							                let href = links::poll_detail(question.id);
							                let question_text = question.question_text.clone();
							                let pub_date = question.pub_date.format("%Y-%m-%d %H:%M").to_string();
							                page!(
							                    | href : String, question_text : String, pub_date : String | { a {
							                    href : href, class :
							                    "block p-4 border border-border rounded-lg bg-surface-primary hover:bg-surface-secondary transition-colors",
							                    div { class : "flex w-full justify-between", h5 { class : "mb-1", {
							                    question_text } } small { { pub_date } } } } }
							                )(href, question_text, pub_date)
							            })
							            .collect::<Vec<_>>(),
							    )
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

	// Create the voting form using form! macro.
	//
	// - server_fn: submit_vote accepts (question_id, choice_id, csrf_token)
	// - method: Post enables CSRF hidden input rendering for non-WASM submits
	// - strip_arguments: explicitly routes the CSRF token to the trailing
	//   server_fn argument (reinhardt-web#3971), replacing the implicit
	//   auto-injection that broke when server_fn signatures evolved.
	// - state: loading/error/success signals for form submission feedback.
	//   `success` is required so that a sibling `use_effect` below can
	//   trigger a one-shot navigation when the form's success signal flips
	//   to `true` (reinhardt-web#4519).
	// - watch blocks for reactive UI updates (submit button + error display)
	//
	// The success-driven redirect is implemented with `use_effect` rather
	// than the form macro's `success_url:` property because `success_url:`
	// (a) has a parser bug in `reinhardt-manouche` that emits
	// `error: expected `:`` at every call site
	// (`crates/reinhardt-manouche/src/parser/form.rs:117-121` consumes a
	// redundant colon), and (b) embeds the user closure inside the
	// generated form struct's `submit()` method, which cannot capture
	// enclosing-scope locals like `qid` (issues #4414 / #4420 fixed this
	// for `watch:` and `initial:` only, by binding them in the outer
	// scope where the form is constructed). `use_effect` runs in that
	// same outer scope, so `qid` is captured cleanly and the redirect
	// fires exactly once per `success.set(true)` transition.
	//
	// Ideal implementation (once the upstream parser bug is fixed AND
	// `success_url:` is reworked to bind in the outer scope like `watch:`):
	//     state: { loading, error },
	//     success_url: |_form, _value| links::poll_results(qid),
	let voting_form = form! {
		name: VotingForm,
		server_fn: submit_vote,
		method: Post,
		state: { loading, error, success },

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
				let back_href = links::polls_index();
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

	// One-shot redirect to the results page on a successful vote (reinhardt-web#4519).
	//
	// The form macro flips `__success` from `false` to `true` exactly once in
	// the `on_success` codepath (`crates/reinhardt-pages/macros/src/form/codegen.rs`
	// `generate_on_success_callback` — `self.__success.set(true)` after the
	// `Ok(value)` branch). This `use_effect` reads `voting_form.success().get()`
	// so it re-runs on every transition of that signal; the `if did_succeed`
	// guard ensures the navigation fires only on the `true` edge and not on
	// the initial `false` value at mount time. That avoids the redirect-on-mount
	// bug from PR #4517 (the previous `watch{}` predicate
	// `if !is_loading && err.is_none()` was true on first render).
	{
		let voting_form_for_redirect = voting_form.clone();
		let dest = links::poll_results(qid);
		use_effect(move || {
			let did_succeed = voting_form_for_redirect.success().get();
			if did_succeed {
				#[cfg(all(target_family = "wasm", target_os = "unknown"))]
				{
					if let Some(window) = web_sys::window() {
						let _ = window.location().set_href(&dest);
					}
				}
			}
		});
	}

	// Dispatch the action to load question data
	load_detail.dispatch(qid);

	let load_detail_signal = load_detail.clone();

	// Render reactively in the canonical shape (see module-level docs):
	// outer `div` + single `watch{}` + `Action<..>` and route id flowing
	// into `page!` as typed parameters. The voting form is captured by
	// the watch closure's implicit `move`.
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
							href: links::poll_detail(question_id),
							class: "btn-secondary",
							"Try Again"
						}
						a {
							href: links::polls_index(),
							class: "btn-primary ml-2",
							"Back to Polls"
						}
					}
				} else if load_detail_signal.result().is_some() {
					div {
						class: "max-w-4xl mx-auto px-4 mt-12",
						div {
							class: "flex justify-between items-center mb-4",
							h1 {
								{
									load_detail_signal
											.result()
											.map(|(q, _)| q.question_text.clone())
											.unwrap_or_default()
								}
							}
							div {
								class: "flex gap-2",
								a {
									href: links::poll_results(question_id),
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
						{ voting_form.clone().into_page() }
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
							href: links::polls_index(),
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
							href: links::polls_index(),
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
										Page::Fragment(
										        load_results_signal
										            .result()
										            .map(|(_, choices, total)| {
										                choices
										                    .iter()
										                    .map(|choice| {
										                        let percentage = if total > 0 {
										                            (choice.votes as f64 / total as f64 * 100.0) as i32
										                        } else {
										                            0
										                        };
										                        let choice_text = choice.choice_text.clone();
										                        let votes = choice.votes;
										                        page!(
										                            | choice_text : String, votes : i32, percentage : i32 | { div
										                            { class : "py-4", div { class :
										                            "flex justify-between items-center mb-2", strong { {
										                            choice_text } } span { class :
										                            "inline-flex items-center bg-brand rounded-full px-2.5 py-0.5 text-xs font-medium text-white",
										                            { format!("{} votes", votes) } } } div { class :
										                            "w-full bg-surface-tertiary rounded-full h-2.5", div { class
										                            : "bg-brand h-2.5 rounded-full", role : "progressbar", style
										                            : format!("width: {}%", percentage), aria_valuenow :
										                            percentage.to_string(), aria_valuemin : "0", aria_valuemax :
										                            "100", { format!("{}%", percentage) } } } } }
										                        )(choice_text, votes, percentage)
										                    })
										                    .collect::<Vec<_>>()
										            })
										            .unwrap_or_default(),
										    )
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
								href: links::poll_detail(question_id),
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
								href: links::polls_index(),
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
							href: links::polls_index(),
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
					src: "/static/images/poll-icon.svg",
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
						{
							Page::Fragment(
							        load_questions_signal
							            .result()
							            .unwrap_or_default()
							            .iter()
							            .map(|question| {
							                let href = format!("/polls/{}/", question.id);
							                let question_text = question.question_text.clone();
							                let pub_date = question.pub_date.format("%Y-%m-%d %H:%M").to_string();
							                page!(
							                    | href : String, question_text : String, pub_date : String | { a {
							                    href : href, class :
							                    "block p-4 border border-border rounded-lg bg-surface-primary hover:bg-surface-secondary transition-colors",
							                    div { class : "flex w-full justify-between items-center", img { src :
							                    "/static/images/poll-icon.svg", alt : "Poll", class : "w-8 h-8 mr-3",
							                    } div { class : "flex-1", h5 { class : "mb-1", { question_text } } }
							                    small { { pub_date } } } } }
							                )(href, question_text, pub_date)
							            })
							            .collect::<Vec<_>>(),
							    )
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
	let cancel_href = links::polls_index();

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

	// Render reactively in the canonical shape (see module-level docs).
	// The `edit_form` is captured by the watch closure's implicit `move`.
	//
	// Workaround for #4515 (framework usability: nested `watch{}` cannot
	// share `!Copy` Signal captures). The `edit_form`'s own loading/error UI
	// is inlined inside this single outer `watch{}` instead of a second
	// nested `watch{}` block. The root cause is that `Page::reactive` requires
	// `F: Fn() -> Page + 'static`; when an inner `watch` is added inside an
	// outer one, both closures try to capture the same `!Copy` `Signal<T>`
	// from the surrounding scope, which rustc rejects with
	// E0507 "cannot move out of … a captured variable in an `Fn` closure"
	// (the inner closure's capture conflicts with the outer closure's
	// re-execution semantics). Until #4515 lands a framework-level fix,
	// flattening the inner reactive switch into the outer `watch{}` is the
	// least-invasive way to preserve reactivity — a single `watch` already
	// subscribes to every Signal accessed inside its body, so the reactive
	// contract is unchanged.
	//
	// Ideal implementation (without workaround, blocked on #4515):
	//   watch {
	//       if edit_form.error().get().is_some() {
	//           div { class: "alert-danger mb-3",
	//               { edit_form.error().get().unwrap_or_default() } }
	//       }
	//   }
	//   { edit_form.clone().into_page() }
	//   div {
	//       class: "mt-3",
	//       watch {
	//           if edit_form.loading().get() {
	//               button { /* "Saving..." */ }
	//           } else {
	//               button { /* "Save" */ }
	//           }
	//       }
	//   }
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
							href: links::polls_index(),
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
								href: links::poll_detail(question_id),
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
	let cancel_href = links::poll_detail(question_id);

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
	let back_href = links::poll_detail(qid);

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
	let cancel_href = links::poll_detail(question_id);

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
	let cancel_href = links::poll_detail(question_id);

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
