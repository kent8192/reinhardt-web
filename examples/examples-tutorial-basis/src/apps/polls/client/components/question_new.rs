//! Route-backed new-question component.

use reinhardt::pages::component;
use reinhardt::pages::component::Page;

use crate::client::components::nav::with_nav;

#[component("/polls/new/", "question_new")]
pub fn question_new() -> Page {
	with_nav(super::question_new())
}
