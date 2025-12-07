//! URL configuration for snippets app

use reinhardt::UnifiedRouter;

use super::views;

pub fn url_patterns() -> UnifiedRouter {
	UnifiedRouter::new()
		.endpoint(views::list)
		.endpoint(views::create)
		.endpoint(views::retrieve)
		.endpoint(views::update)
		.endpoint(views::delete)
}
