//! Page components
//!
//! This module re-exports page-level components that are defined in the router module.
//! Each page function returns a View that can be rendered.
//!
//! ## Available Pages
//!
//! - `home_page` - Landing page with welcome message
//! - `login_page` - User login form
//! - `register_page` - User registration form
//! - `profile_page` - User profile view
//! - `profile_edit_page` - Profile editing form
//! - `timeline_page` - Tweet timeline with compose form
//! - `not_found_page` - 404 error page
//!
//! ## Usage
//!
//! Pages are typically accessed through the router, but can also be
//! rendered directly for testing or embedding:
//!
//! ```ignore
//! use crate::client::pages::timeline_page;
//!
//! let view = timeline_page();
//! ```
//!
//! ## Design Note
//!
//! Page implementations are kept in `router.rs` for co-location with
//! route definitions. This module provides a clean public API for
//! accessing page components.

use reinhardt::pages::component::View;

// Re-export page functions from router for convenience
// Note: The actual implementations are in router.rs to keep routes and views together

/// Home/landing page
///
/// Displays a welcome message with links to login and register.
pub fn home_page() -> View {
	crate::client::router::home_page_view()
}

/// Login page
///
/// Displays the login form using the auth component.
pub fn login_page() -> View {
	crate::client::router::login_page_view()
}

/// Register page
///
/// Displays the registration form using the auth component.
pub fn register_page() -> View {
	crate::client::router::register_page_view()
}

/// Profile page
///
/// Displays a user's profile information.
///
/// # Arguments
///
/// * `user_id` - The UUID of the user
pub fn profile_page(user_id: uuid::Uuid) -> View {
	crate::client::router::profile_page_view(user_id)
}

/// Profile edit page
///
/// Displays the profile editing form.
///
/// # Arguments
///
/// * `user_id` - The UUID of the user
pub fn profile_edit_page(user_id: uuid::Uuid) -> View {
	crate::client::router::profile_edit_page_view(user_id)
}

/// Timeline page
///
/// Displays the tweet timeline with a compose form.
pub fn timeline_page() -> View {
	crate::client::router::timeline_page_view()
}

/// DM chat page
///
/// Displays the DM chat interface for a specific room.
///
/// # Arguments
///
/// * `room_id` - The ID of the chat room
pub fn dm_chat_page(room_id: String) -> View {
	crate::client::router::dm_chat_page_view(room_id)
}

/// 404 Not Found page
///
/// Displays an error message for invalid routes.
pub fn not_found_page() -> View {
	crate::client::router::not_found_page_view()
}
