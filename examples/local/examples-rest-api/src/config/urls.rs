//! URL configuration for example-rest-api project (RESTful)
//!
//! The `url_patterns` routes URLs to handlers.

use reinhardt::UnifiedRouter;
use reinhardt::register_url_patterns;
use std::sync::Arc;

use super::views;

pub fn url_patterns() -> Arc<UnifiedRouter> {
	Arc::new(
		UnifiedRouter::new()
			.endpoint(views::root)
			.endpoint(views::health)
			.endpoint(views::list_users)
			.mount("/api/", crate::apps::api::urls::url_patterns()),
	)
}

// Register URL patterns for automatic discovery by the framework
register_url_patterns!();
