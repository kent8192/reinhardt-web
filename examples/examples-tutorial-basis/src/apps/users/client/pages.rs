//! Page wrappers for the users application.
//!
//! The shared site navigation is composed at this layer so authentication
//! components stay focused on form behavior.

use reinhardt::pages::component::Page;

use crate::client::components::nav::with_nav;

/// Login page - Username + password form.
pub fn login_page() -> Page {
	with_nav(super::components::login_form())
}

/// Logout page - Single-button session termination.
pub fn logout_page() -> Page {
	with_nav(super::components::logout_form())
}

/// Sign-up page - Create a new account.
pub fn signup_page() -> Page {
	with_nav(super::components::signup_form())
}
