use reinhardt::UnifiedRouter;

use super::views;

pub fn url_patterns() -> UnifiedRouter {
	UnifiedRouter::new()
		.endpoint(views::index)
		.endpoint(views::detail)
		.endpoint(views::results)
		.endpoint(views::vote)
}
