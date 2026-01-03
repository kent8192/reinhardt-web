//! Relationship components (follow/block)
//!
//! Provides follow button and user list components for managing user relationships.

use crate::shared::types::UserInfo;
use reinhardt_pages::Signal;
use reinhardt_pages::component::View;
use reinhardt_pages::page;
use uuid::Uuid;

#[cfg(target_arch = "wasm32")]
use {
	crate::server_fn::relationship::{
		fetch_followers, fetch_following, follow_user, unfollow_user,
	},
	wasm_bindgen_futures::spawn_local,
};

/// Type of user list to display
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UserListType {
	/// List of followers
	Followers,
	/// List of users being followed
	Following,
}

/// Follow button component
///
/// Provides a button to follow/unfollow a user with state management.
pub fn follow_button(target_user_id: Uuid, is_following_initial: bool) -> View {
	let is_following = Signal::new(is_following_initial);
	let loading = Signal::new(false);
	let error = Signal::new(None::<String>);

	// Determine button class and text based on state
	let btn_class = if loading.get() {
		"btn btn-secondary disabled"
	} else if is_following.get() {
		"btn btn-outline-primary"
	} else {
		"btn btn-primary"
	}
	.to_string();

	let btn_text = if loading.get() {
		"Processing..."
	} else if is_following.get() {
		"Unfollow"
	} else {
		"Follow"
	}
	.to_string();

	let error_msg = error.get();
	let has_error = error_msg.is_some();
	let error_text = error_msg.unwrap_or_default();

	#[cfg(target_arch = "wasm32")]
	{
		let is_following_clone = is_following.clone();
		let loading_clone = loading.clone();
		let error_clone = error.clone();

		page!(|btn_class: String, btn_text: String, has_error: bool, error_text: String| {
			div {
				button {
					r#type: "button",
					class: btn_class,
					@click: { let is_following = is_following_clone.clone(); let loading = loading_clone.clone(); let error = error_clone.clone(); move |_event| { let is_following_inner = is_following.clone(); let loading_inner = loading.clone(); let error_inner = error.clone(); let currently_following = is_following.get(); spawn_local(async move { loading_inner.set(true); error_inner.set(None); let result = if currently_following { unfollow_user(target_user_id).await } else { follow_user(target_user_id).await }; match result { Ok(()) => { is_following_inner.set(! currently_following); loading_inner.set(false); } Err(e) => { error_inner.set(Some(e.to_string())); loading_inner.set(false); } } }); } },
					{ btn_text }
				}
				if has_error {
					div {
						class: "alert alert-danger mt-2",
						{ error_text }
					}
				}
			}
		})(btn_class, btn_text, has_error, error_text)
	}

	#[cfg(not(target_arch = "wasm32"))]
	{
		page!(|btn_class: String, btn_text: String, has_error: bool, error_text: String| {
			div {
				button {
					r#type: "button",
					class: { btn_class },
					{ btn_text }
				}
				if has_error {
					div {
						class: "alert alert-danger mt-2",
						{ error_text }
					}
				}
			}
		})(btn_class, btn_text, has_error, error_text)
	}
}

/// User card component
///
/// Displays a single user in a list.
fn user_card(user: &UserInfo) -> View {
	let username = format!("@{}", user.username);
	let email = user.email.clone();
	let profile_url = format!("/profile/{}", user.id);

	page!(|username: String, email: String, profile_url: String| {
		div {
			class: "card mb-2",
			div {
				class: "card-body",
				div {
					class: "d-flex justify-content-between align-items-center",
					div {
						h6 {
							class: "card-subtitle mb-1",
							{ username }
						}
						small {
							class: "text-muted",
							{ email }
						}
					}
					a {
						href: profile_url,
						class: "btn btn-sm btn-outline-primary",
						"View Profile"
					}
				}
			}
		}
	})(username, email, profile_url)
}

/// User list component
///
/// Displays a list of users (followers or following) with loading and error states.
pub fn user_list(user_id: Uuid, list_type: UserListType) -> View {
	let users = Signal::new(Vec::<UserInfo>::new());
	let loading = Signal::new(true);
	let error = Signal::new(None::<String>);

	#[cfg(target_arch = "wasm32")]
	{
		let users = users.clone();
		let loading = loading.clone();
		let error = error.clone();

		spawn_local(async move {
			loading.set(true);
			error.set(None);

			let result = match list_type {
				UserListType::Followers => fetch_followers(user_id).await,
				UserListType::Following => fetch_following(user_id).await,
			};

			match result {
				Ok(user_list) => {
					users.set(user_list);
					loading.set(false);
				}
				Err(e) => {
					error.set(Some(e.to_string()));
					loading.set(false);
				}
			}
		});
	}

	let title = match list_type {
		UserListType::Followers => "Followers",
		UserListType::Following => "Following",
	}
	.to_string();

	let empty_message = match list_type {
		UserListType::Followers => "No followers yet.",
		UserListType::Following => "Not following anyone yet.",
	}
	.to_string();

	let is_loading = loading.get();
	let error_msg = error.get();
	let has_error = error_msg.is_some();
	let error_text = error_msg.unwrap_or_default();
	let user_list_data = users.get();
	let is_empty = user_list_data.is_empty();

	// Generate user cards as View objects
	let user_cards: Vec<View> = user_list_data.iter().map(|u| user_card(u)).collect();
	let user_cards_view = View::fragment(user_cards);

	page!(|title: String, is_loading: bool, has_error: bool, error_text: String, is_empty: bool, empty_message: String, user_cards_view: View| {
		div {
			h3 {
				class: "mb-4",
				{ title }
			}
			if is_loading {
				div {
					class: "text-center py-5",
					div {
						class: "spinner-border",
						role: "status",
						span {
							class: "visually-hidden",
							"Loading..."
						}
					}
				}
			} else if has_error {
				div {
					class: "alert alert-danger",
					{ error_text }
				}
			} else if is_empty {
				div {
					class: "text-center py-5",
					p {
						class: "text-muted",
						{ empty_message }
					}
				}
			} else {
				{ user_cards_view }
			}
		}
	})(
		title,
		is_loading,
		has_error,
		error_text,
		is_empty,
		empty_message,
		user_cards_view,
	)
}
