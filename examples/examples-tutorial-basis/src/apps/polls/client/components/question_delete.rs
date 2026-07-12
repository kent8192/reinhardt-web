//! Route-backed delete-question component.

use reinhardt::pages::Path;
use reinhardt::pages::component;
use reinhardt::pages::component::Page;

use crate::client::components::nav::with_nav;

#[component("/polls/{question_id}/delete/", "question_delete")]
pub fn question_delete(Path(question_id): Path<i64>) -> Page {
	with_nav(super::question_delete_confirm(question_id))
}
