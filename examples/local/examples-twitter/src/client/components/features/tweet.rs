//! Tweet components using React-like hooks
//!
//! Provides tweet card, tweet form, and tweet list components with hooks-styled state management.

use crate::shared::types::{CreateTweetRequest, TweetInfo};
use reinhardt_pages::component::{ElementView, IntoView, View};
use reinhardt_pages::page;
use reinhardt_pages::reactive::hooks::use_state;
use uuid::Uuid;

#[cfg(target_arch = "wasm32")]
use {
	crate::server_fn::tweet::{create_tweet, delete_tweet, list_tweets},
	wasm_bindgen::JsCast,
	wasm_bindgen_futures::spawn_local,
	web_sys::HtmlTextAreaElement,
};

/// Tweet card component using hooks
///
/// Displays a single tweet with delete button if owned by current user.
/// Uses React-like hooks for state management.
pub fn tweet_card(tweet: &TweetInfo, show_delete: bool) -> View {
	let tweet_id = tweet.id;

	// Hook-styled state management
	let (deleted, set_deleted) = use_state(false);
	let (error, set_error) = use_state(None::<String>);

	// Extract signal values for page! macro
	let deleted_state = deleted.get();
	let error_msg = error.get().unwrap_or_default();
	let has_error = error.get().is_some();

	// Clone tweet data for use in page! macro
	let username = tweet.username.clone();
	let content = tweet.content.clone();
	let created_at = tweet.created_at.clone();

	page!(|deleted_state: bool, error_msg: String, has_error: bool, show_delete: bool, username: String, content: String, created_at: String| {
		div {
			class: if deleted_state { "d-none" } else { "card mb-3" },
			div {
				class: "card-body",
				div {
					class: "d-flex justify-content-between align-items-start",
					div {
						h6 {
							class: "card-subtitle mb-2 text-muted",
							{ format!("@{}", username) }
						}
						p {
							class: "card-text",
							{ content }
						}
						small {
							class: "text-muted",
							{ created_at }
						}
					}
					if show_delete {
						button {
							class: "btn btn-sm btn-danger",
							r#type: "button",
							@click: { let set_deleted = set_deleted.clone(); let set_error = set_error.clone(); move |_event| { # [cfg(target_arch = "wasm32")] { let set_deleted = set_deleted.clone(); let set_error = set_error.clone(); spawn_local(async move { match delete_tweet(tweet_id).await { Ok(()) => { set_deleted(true); } Err(e) => { set_error(Some(e.to_string())); } } }); } } },
							"Delete"
						}
					}
				}
				if has_error {
					div {
						class: "alert alert-danger mt-2",
						{ error_msg }
					}
				}
			}
		}
	})(
		deleted_state,
		error_msg,
		has_error,
		show_delete,
		username,
		content,
		created_at,
	)
}

/// Tweet form component using hooks
///
/// Provides form for creating a new tweet with 280 character limit.
/// Uses React-like hooks for state management.
/// CSRF protection is handled automatically by #[server_fn] macro via headers.
pub fn tweet_form() -> View {
	// Hook-styled state for form fields
	let (content, set_content) = use_state(String::new());
	let (error, set_error) = use_state(None::<String>);
	let (loading, set_loading) = use_state(false);
	let (char_count, set_char_count) = use_state(0usize);

	// Extract signal values for page! macro
	let content_value = content.get();
	let error_msg = error.get().unwrap_or_default();
	let has_error = error.get().is_some();
	let loading_state = loading.get();
	let char_count_value = char_count.get();

	page!(|content_value: String, error_msg: String, has_error: bool, loading_state: bool, char_count_value: usize| {
		div {
			class: "card mb-4",
			div {
				class: "card-body",
				h5 {
					class: "card-title",
					"What's happening?"
				}
				if has_error {
					div {
						class: "alert alert-danger",
						{ error_msg }
					}
				}
				form {
					@submit: { let set_error = set_error.clone(); let set_loading = set_loading.clone(); let content = content.clone(); let set_content = set_content.clone(); let set_char_count = set_char_count.clone(); move |event : web_sys::Event| { # [cfg(target_arch = "wasm32")] { event.prevent_default(); let set_error = set_error.clone(); let set_loading = set_loading.clone(); let content_value = content.get(); let set_content = set_content.clone(); let set_char_count = set_char_count.clone(); spawn_local(async move { set_loading(true); set_error(None); let request = CreateTweetRequest { content : content_value, }; match create_tweet(request).await { Ok(_) => { set_content(String::new()); set_char_count(0); set_loading(false); if let Some(window) = web_sys::window() { let _ = window.location().reload(); } } Err(e) => { set_error(Some(e.to_string())); set_loading(false); } } }); } } },
					div {
						class: "mb-3",
						textarea {
							class: "form-control",
							id: "content",
							name: "content",
							rows: 3,
							maxlength: 280,
							placeholder: "What's on your mind?",
							@input: { let set_content = set_content.clone(); let set_char_count = set_char_count.clone(); move |event : web_sys::Event| { # [cfg(target_arch = "wasm32")] { if let Some(target) = event.target() { if let Ok(textarea) = target.dyn_into ::<HtmlTextAreaElement>() { let value = textarea.value(); set_char_count(value.len()); set_content(value); } } } } },
							{ content_value.clone() }
						}
						div {
							class: "d-flex justify-content-between align-items-center mt-2",
							small {
								class: if char_count_value>280 { "text-danger" } else if char_count_value>250 { "text-warning" } else { "text-muted" },
								{ format!("{}/280", char_count_value) }
							}
							button {
								r#type: "submit",
								class: if loading_state { "btn btn-primary disabled" } else { "btn btn-primary" },
								disabled: if char_count_value == 0 || char_count_value>280 { "true" } else { "" },
								if loading_state {
									"Posting..."
								} else {
									"Post"
								}
							}
						}
					}
				}
			}
		}
	})(
		content_value,
		error_msg,
		has_error,
		loading_state,
		char_count_value,
	)
}

/// Tweet list component using hooks
///
/// Displays list of tweets with loading and error states.
/// Uses React-like hooks for state management.
pub fn tweet_list(user_id: Option<Uuid>) -> View {
	// Hook-styled state management
	let (tweets, set_tweets) = use_state(Vec::<TweetInfo>::new());
	let (loading, set_loading) = use_state(true);
	let (error, set_error) = use_state(None::<String>);

	#[cfg(target_arch = "wasm32")]
	{
		let set_tweets = set_tweets.clone();
		let set_loading = set_loading.clone();
		let set_error = set_error.clone();

		spawn_local(async move {
			set_loading(true);
			set_error(None);

			match list_tweets(user_id, 0).await {
				Ok(tweet_list) => {
					set_tweets(tweet_list);
					set_loading(false);
				}
				Err(e) => {
					set_error(Some(e.to_string()));
					set_loading(false);
				}
			}
		});
	}

	// Extract signal values
	let tweets_data = tweets.get();
	let loading_state = loading.get();
	let error_msg = error.get().unwrap_or_default();
	let has_error = error.get().is_some();
	let is_empty = tweets_data.is_empty();

	// Generate tweet cards outside page! macro since it returns a Vec<View>
	let tweet_views = if !loading_state && !has_error && !is_empty {
		tweets_data
			.iter()
			.map(|tweet| tweet_card(tweet, false))
			.collect::<Vec<_>>()
	} else {
		Vec::new()
	};

	// Use page! macro for the wrapper and ElementView for dynamic children
	if loading_state {
		page!(|| {
			div {
				class: "text-center my-4",
				div {
					class: "spinner-border text-primary",
					role: "status",
					span {
						class: "visually-hidden",
						"Loading..."
					}
				}
			}
		})()
	} else if has_error {
		let error_msg_clone = error_msg.clone();
		page!(|error_msg: String| {
			div {
				class: "alert alert-danger",
				role: "alert",
				{ error_msg }
			}
		})(error_msg_clone)
	} else if is_empty {
		page!(|| {
			div {
				class: "text-center py-5",
				p {
					class: "text-muted",
					"No tweets yet. Be the first to post!"
				}
			}
		})()
	} else {
		// Use ElementView for dynamic children list
		ElementView::new("div").children(tweet_views).into_view()
	}
}
