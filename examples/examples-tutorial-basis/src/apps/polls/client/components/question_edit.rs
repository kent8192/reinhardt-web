//! Route-backed edit-question component.

use reinhardt::pages::Path;
use reinhardt::pages::component;
use reinhardt::pages::component::Page;

use crate::client::components::nav::with_nav;

#[component("/polls/{question_id}/edit/", "question_edit")]
pub fn question_edit(Path(question_id): Path<i64>) -> Page {
	with_nav(super::question_edit(question_id))
}
