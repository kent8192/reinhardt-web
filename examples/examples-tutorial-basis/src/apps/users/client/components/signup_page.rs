//! Route-backed sign-up component.

use reinhardt::pages::component;
use reinhardt::pages::component::Page;

use crate::client::components::nav::with_nav;

#[component("/signup/", "signup")]
pub fn signup_page() -> Page {
	with_nav(super::signup_form())
}
