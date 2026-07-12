//! Route-backed edit-choice component.

use reinhardt::pages::Path;
use reinhardt::pages::component;
use reinhardt::pages::component::Page;

use crate::client::components::nav::with_nav;

#[component("/polls/{question_id}/choices/{choice_id}/edit/", "choice_edit")]
pub fn choice_edit(Path(question_id): Path<i64>, Path(choice_id): Path<i64>) -> Page {
	with_nav(super::choice_edit(question_id, choice_id))
}
