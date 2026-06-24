//! Route-backed logout component.

use reinhardt::pages::component;
use reinhardt::pages::component::Page;

use crate::client::components::nav::with_nav;

#[component("/logout/", "logout")]
pub fn logout_page() -> Page {
	with_nav(super::logout_form())
}
