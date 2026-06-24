//! Route-backed new-choice component.

use reinhardt::pages::Path;
use reinhardt::pages::component;
use reinhardt::pages::component::Page;

use crate::client::components::nav::with_nav;

#[component("/polls/{question_id}/choices/new/", "choice_new")]
pub fn choice_new(Path(question_id): Path<i64>) -> Page {
	with_nav(super::choice_new(question_id))
}
