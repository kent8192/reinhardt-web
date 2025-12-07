//! URL routing for hello app

use reinhardt::UnifiedRouter;

use crate::apps::hello::views;

pub fn url_patterns() -> UnifiedRouter {
	UnifiedRouter::new()
		.endpoint(views::hello_world)
		.endpoint(views::health_check)
}
