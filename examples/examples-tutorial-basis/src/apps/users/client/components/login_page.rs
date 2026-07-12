//! Route-backed login component.

use reinhardt::pages::component;
use reinhardt::pages::component::Page;

use crate::client::components::nav::with_nav;

#[component("/login/", "login")]
pub fn login_page() -> Page {
	with_nav(super::login_form())
}
