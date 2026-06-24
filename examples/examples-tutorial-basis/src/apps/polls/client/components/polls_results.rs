//! Route-backed poll results component.

use reinhardt::pages::Path;
use reinhardt::pages::component;
use reinhardt::pages::component::Page;

use crate::client::components::nav::with_nav;

#[component("/polls/{question_id}/results/", "results")]
pub fn polls_results(Path(question_id): Path<i64>) -> Page {
	with_nav(super::polls_results(question_id))
}
