//! Target-neutral page entry points for the users application.

use reinhardt::pages::component::Page;

#[cfg(client)]
use crate::client::components::nav::with_nav;

/// Login page - Username + password form.
pub fn login_page() -> Page {
	#[cfg(client)]
	{
		with_nav(crate::apps::users::client::components::login_form())
	}
	#[cfg(not(client))]
	{
		Page::Empty
	}
}

/// Logout page - Single-button session termination.
pub fn logout_page() -> Page {
	#[cfg(client)]
	{
		with_nav(crate::apps::users::client::components::logout_form())
	}
	#[cfg(not(client))]
	{
		Page::Empty
	}
}

/// Sign-up page - Create a new account.
pub fn signup_page() -> Page {
	#[cfg(client)]
	{
		with_nav(crate::apps::users::client::components::signup_form())
	}
	#[cfg(not(client))]
	{
		Page::Empty
	}
}
