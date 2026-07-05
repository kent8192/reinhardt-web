//! Route-backed polls index component.

use reinhardt::pages::component;
use reinhardt::pages::component::Page;

use crate::client::components::nav::with_nav;

#[component("/", name = "index")]
pub fn polls_index() -> Page {
	with_nav(super::polls_index())
}
