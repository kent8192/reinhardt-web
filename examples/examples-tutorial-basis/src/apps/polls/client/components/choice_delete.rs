//! Route-backed delete-choice component.

use reinhardt::pages::Path;
use reinhardt::pages::component;
use reinhardt::pages::component::Page;

use crate::client::components::nav::with_nav;

#[component("/polls/{question_id}/choices/{choice_id}/delete/", "choice_delete")]
pub fn choice_delete(Path(question_id): Path<i64>, Path(choice_id): Path<i64>) -> Page {
	with_nav(super::choice_delete_confirm(question_id, choice_id))
}
