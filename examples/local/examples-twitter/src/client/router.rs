//! Client-side routing
//!
//! This module defines the client-side router for the Twitter clone application.

use crate::client::pages::{
	dm_chat_page, home_page, login_page, not_found_page, profile_edit_page, profile_page,
	register_page, timeline_page,
};
use reinhardt::pages::component::View;
use reinhardt::pages::page;
use reinhardt::pages::router::Router;
use std::cell::RefCell;
use uuid::Uuid;

/// Application routes
#[derive(Debug, Clone, PartialEq)]
pub enum AppRoute {
	/// Home/Timeline page
	Home,
	/// Login page
	Login,
	/// Register page
	Register,
	/// User profile page
	Profile { user_id: String },
	/// Timeline page
	Timeline,
	/// DM chat page
	DMChat { room_id: String },
	/// Not found page
	NotFound,
}

// Global Router instance
thread_local! {
	static ROUTER: RefCell<Option<Router>> = const { RefCell::new(None) };
}

/// Initialize the global router instance
///
/// This must be called once at application startup before any routing operations.
/// Also sets up the popstate event listener for browser back/forward navigation.
pub fn init_global_router() {
	ROUTER.with(|r| {
		let router = init_router();
		// Set up popstate listener for browser back/forward navigation
		router.setup_history_listener();
		*r.borrow_mut() = Some(router);
	});
}

/// Provides access to the global router instance
///
/// # Panics
///
/// Panics if the router has not been initialized via `init_global_router()`.
pub fn with_router<F, R>(f: F) -> R
where
	F: FnOnce(&Router) -> R,
{
	ROUTER.with(|r| {
		f(r.borrow()
			.as_ref()
			.expect("Router not initialized. Call init_global_router() first."))
	})
}

/// Initialize the router with all application routes
fn init_router() -> Router {
	Router::new()
		// Home route
		.route("/", || home_page())
		// Authentication routes
		.route("/login", || login_page())
		.route("/register", || register_page())
		// User profile routes (Django-style {param} syntax)
		.route("/profile/{user_id}/edit", || {
			with_router(|r| {
				let params = r.current_params().get();
				let user_id_str = params
					.get("user_id")
					.cloned()
					.unwrap_or_else(|| "00000000-0000-0000-0000-000000000000".to_string());
				let user_id = Uuid::parse_str(&user_id_str)
					.unwrap_or_else(|_| Uuid::nil());
				profile_edit_page(user_id)
			})
		})
		.route("/profile/{user_id}", || {
			with_router(|r| {
				let params = r.current_params().get();
				let user_id_str = params
					.get("user_id")
					.cloned()
					.unwrap_or_else(|| "00000000-0000-0000-0000-000000000000".to_string());
				let user_id = Uuid::parse_str(&user_id_str)
					.unwrap_or_else(|_| Uuid::nil());
				profile_page(user_id)
			})
		})
		// Timeline route
		.route("/timeline", || timeline_page())
		// DM chat route (Django-style {param} syntax)
		.route("/dm/{room_id}", || {
			with_router(|r| {
				let params = r.current_params().get();
				let room_id = params
					.get("room_id")
					.cloned()
					.unwrap_or_else(|| "unknown".to_string());
				dm_chat_page(room_id)
			})
		})
		// 404 not found (fallback)
		.not_found(|| not_found_page())
}

/// Home page view
pub fn home_page_view() -> View {
	page!(|| {
		div {
			class: "container mt-5",
			h1 {
				class: "mb-4",
				"Twitter Clone - Home"
			}
			p {
				class: "lead",
				"Welcome to the Twitter Clone!"
			}
			div {
				class: "mt-4",
				a {
					href: "/login",
					class: "btn btn-primary me-2",
					"Login"
				}
				a {
					href: "/register",
					class: "btn btn-secondary",
					"Register"
				}
			}
		}
	})()
}

/// Login page view
pub fn login_page_view() -> View {
	use crate::client::components::features::auth::login_form;
	login_form()
}

/// Register page view
pub fn register_page_view() -> View {
	use crate::client::components::features::auth::register_form;
	register_form()
}

/// Profile page view
pub fn profile_page_view(user_id: Uuid) -> View {
	use crate::client::components::features::profile::profile_view;

	profile_view(user_id)
}

/// Profile edit page view
pub fn profile_edit_page_view(user_id: Uuid) -> View {
	use crate::client::components::features::profile::profile_edit;

	profile_edit(user_id)
}

/// Timeline page view
pub fn timeline_page_view() -> View {
	use crate::client::components::features::tweet::{tweet_form, tweet_list};

	// Get component views
	let form_view = tweet_form();
	let list_view = tweet_list(None);

	page!(|form_view: View, list_view: View| {
		div {
			class: "container mt-5",
			h1 {
				class: "mb-4",
				"Timeline"
			}
			{ form_view }
			hr {
				class: "my-4",
			}
			{ list_view }
		}
	})(form_view, list_view)
}

/// DM chat page view
pub fn dm_chat_page_view(room_id: String) -> View {
	use crate::client::components::features::dm::dm_chat;

	dm_chat(room_id)
}

/// Not found page view
pub fn not_found_page_view() -> View {
	page!(|| {
		div {
			class: "container mt-5",
			h1 {
				class: "mb-4",
				"404 - Page Not Found"
			}
			p {
				"The page you are looking for does not exist."
			}
			a {
				href: "/",
				class: "btn btn-primary",
				"Go Home"
			}
		}
	})()
}
