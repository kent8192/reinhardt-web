//! Route-backed poll detail component.

use reinhardt::pages::Path;
use reinhardt::pages::component;
use reinhardt::pages::component::Page;

use crate::client::components::nav::with_nav;

#[component("/polls/{question_id}/", "detail")]
pub fn polls_detail(Path(question_id): Path<i64>) -> Page {
	with_nav(super::polls_detail(question_id))
}
