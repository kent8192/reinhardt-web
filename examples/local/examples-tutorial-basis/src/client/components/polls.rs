//! Polling application components
//!
//! Provides UI components for the polling application including
//! the index page, detail page with voting form, and results page.

use crate::shared::types::{ChoiceInfo, QuestionInfo};
use reinhardt::pages::Signal;
use reinhardt::pages::component::{ElementView, IntoView, View};
use reinhardt::pages::form;
use reinhardt::pages::page;
use reinhardt::pages::reactive::hooks::use_state;

#[cfg(target_arch = "wasm32")]
use {
	crate::server_fn::polls::{
		get_question_detail, get_question_results, get_questions, submit_vote,
	},
	wasm_bindgen_futures::spawn_local,
};

#[cfg(not(target_arch = "wasm32"))]
use crate::server_fn::polls::submit_vote;

/// Polls index page - List all polls
///
/// Displays a list of available polls with links to vote.
/// Uses watch blocks for reactive UI updates when async data loads.
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

	// Clone signals for passing to page! macro (NOT extracting values)
	let questions_signal = questions.clone();
	let loading_signal = loading.clone();
	let error_signal = error.clone();

	page!(|questions_signal: Signal<Vec<QuestionInfo>>, loading_signal: Signal<bool>, error_signal: Signal<Option<String>>| {
		div {
			class: "container mt-5",
			h1 {
				class: "mb-4",
				"Polls"
			}
			watch {
				if error_signal.get().is_some() {
					div {
						class: "alert alert-danger",
						{ error_signal.get().unwrap_or_default() }
					}
				}
			}
			watch {
				if loading_signal.get() {
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
				} else if questions_signal.get().is_empty() {
					p {
						class: "text-muted",
						"No polls are available."
					}
				} else {
					div {
						class: "list-group",
						{ View::fragment(questions_signal.get().iter().map(|question| { let href = format!("/polls/{}/", question.id); let question_text = question.question_text.clone(); let pub_date = question.pub_date.format("%Y-%m-%d %H:%M").to_string(); page!(|href : String, question_text : String, pub_date : String| { a { href : href, class : "list-group-item list-group-item-action", div { class : "d-flex w-100 justify-content-between", h5 { class : "mb-1", { question_text } } small { { pub_date } } } } }) (href, question_text, pub_date) }).collect()) }
					}
				}
			}
		}
	})(questions_signal, loading_signal, error_signal)
}

/// Poll detail page - Show question and voting form
///
/// Displays a question with its choices and allows the user to vote.
/// Uses form! macro with Dynamic ChoiceField for declarative form handling.
/// CSRF protection is automatically injected for POST method.
pub fn polls_detail(question_id: i64) -> View {
	// State for question data and loading status
	let (question, set_question) = use_state(None::<QuestionInfo>);
	let (data_loading, set_data_loading) = use_state(true);
	let (data_error, set_data_error) = use_state(None::<String>);

	// Convert question_id to String for form field
	let question_id_str = question_id.to_string();

	// Create the voting form using form! macro
	// - server_fn: submit_vote accepts (question_id: String, choice_id: String)
	// - method: Post enables automatic CSRF token injection
	// - state: loading/error signals for form submission feedback
	// - watch blocks for reactive UI updates
	let voting_form = form! {
		name: VotingForm,
		server_fn: submit_vote,
		method: Post,
		state: { loading, error },

		fields: {
			question_id: HiddenField {
				initial: question_id_str.clone(),
			},
			choice_id: ChoiceField {
				widget: RadioSelect,
				required,
				label: "Select your choice",
				class: "form-check poll-choice p-3 mb-2 border rounded",
				choices_from: "choices",
				choice_value: "id",
				choice_label: "choice_text",
			},
		},

		watch: {
			submit_button: |form| {
				let is_loading = form.loading().get();
				page!(|is_loading: bool| {
					div {
						class: "mt-3",
						button {
							r#type: "submit",
							class: if is_loading { "btn btn-primary disabled" } else { "btn btn-primary" },
							disabled: is_loading,
							{ if is_loading { "Voting..." } else { "Vote" } }
						}
						a {
							href: "/",
							class: "btn btn-secondary ms-2",
							"Back to Polls"
						}
					}
				})(is_loading)
			},
			error_display: |form| {
				let err = form.error().get();
				page!(|err: Option<String>| {
					watch {
						if let Some(e) = err.clone() {
							div {
								class: "alert alert-danger mt-3",
								{ e }
							}
						}
					}
				})(err)
			},
		},

		on_success: |_result| {
			#[cfg(target_arch = "wasm32")]
			{
				// Navigate to results page after successful vote
				if let Some(window) = web_sys::window() {
					let results_url = format!("/polls/{}/results/", question_id);
					let _ = window.location().set_href(&results_url);
				}
			}
		},
	};

	// Load question data and populate choice options
	#[cfg(target_arch = "wasm32")]
	{
		let set_question = set_question.clone();
		let set_data_loading = set_data_loading.clone();
		let set_data_error = set_data_error.clone();
		let voting_form_clone = voting_form.clone();

		spawn_local(async move {
			match get_question_detail(question_id).await {
				Ok((q, choices)) => {
					set_question(Some(q));

					// Populate choice options in the form
					// choice_id_choices Signal accepts Vec<(String, String)> as (value, label)
					let choice_options: Vec<(String, String)> = choices
						.iter()
						.map(|c| (c.id.to_string(), c.choice_text.clone()))
						.collect();
					voting_form_clone.choice_id_choices().set(choice_options);

					set_data_loading(false);
				}
				Err(e) => {
					set_data_error(Some(e.to_string()));
					set_data_loading(false);
				}
			}
		});
	}

	// Clone signals for page! macro
	let question_signal = question.clone();
	let loading_signal = data_loading.clone();
	let error_signal = data_error.clone();

	// Loading state
	if loading_signal.get() {
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

	// Error state
	if let Some(err) = error_signal.get() {
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

	// Question found - render voting form
	if let Some(q) = question_signal.get() {
		let question_text = q.question_text.clone();
		let form_view = voting_form.into_view();

		page!(|question_text: String, form_view: View| {
			div {
				class: "container mt-5",
				h1 {
					class: "mb-4",
					{ question_text }
				}
				{ form_view }
			}
		})(question_text, form_view)
	} else {
		// Question not found
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
///
/// Displays the question with vote counts for each choice.
/// Uses watch blocks for reactive UI updates when async data loads.
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

	// Clone signals for passing to page! macro (NOT extracting values)
	let question_signal = question.clone();
	let choices_signal = choices.clone();
	let total_signal = total_votes.clone();
	let loading_signal = loading.clone();
	let error_signal = error.clone();

	page!(|question_signal: Signal<Option<QuestionInfo>>, choices_signal: Signal<Vec<ChoiceInfo>>, total_signal: Signal<i32>, loading_signal: Signal<bool>, error_signal: Signal<Option<String>>, question_id: i64| {
		div {
			watch {
				if loading_signal.get() {
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
				} else if error_signal.get().is_some() {
					div {
						class: "container mt-5",
						div {
							class: "alert alert-danger",
							{ error_signal.get().unwrap_or_default() }
						}
						a {
							href: "/",
							class: "btn btn-primary",
							"Back to Polls"
						}
					}
				} else if question_signal.get().is_some() {
					div {
						class: "container mt-5",
						h1 {
							class: "mb-4",
							{ question_signal.get().map(|q| q.question_text.clone()).unwrap_or_default() }
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
									{ View::fragment(choices_signal.get().iter().map(|choice| { let total = total_signal.get(); let percentage = if total>0 { (choice.votes as f64 / total as f64 * 100.0) as i32 } else { 0 }; let choice_text = choice.choice_text.clone(); let votes = choice.votes; page!(|choice_text : String, votes : i32, percentage : i32| { div { class : "list-group-item", div { class : "d-flex justify-content-between align-items-center mb-2", strong { { choice_text } } span { class : "badge bg-primary rounded-pill", { format!("{} votes", votes) } } } div { class : "progress", div { class : "progress-bar", role : "progressbar", style : format!("width: {}%", percentage), aria_valuenow : percentage.to_string(), aria_valuemin : "0", aria_valuemax : "100", { format!("{}%", percentage) } } } } }) (choice_text, votes, percentage) }).collect()) }
								}
								div {
									class: "mt-3",
									p {
										class: "text-muted",
										{ format!("Total votes: {}", total_signal.get()) }
									}
								}
							}
						}
						div {
							class: "mt-3",
							a {
								href: format!("/polls/{}/", question_id),
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
				} else {
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
				}
			}
		}
	})(
		question_signal,
		choices_signal,
		total_signal,
		loading_signal,
		error_signal,
		question_id,
	)
}
