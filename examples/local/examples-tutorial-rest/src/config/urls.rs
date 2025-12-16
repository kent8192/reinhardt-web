use reinhardt::prelude::*;
use reinhardt::register_url_patterns;
use std::sync::Arc;

pub fn url_patterns() -> Arc<UnifiedRouter> {
	let router = UnifiedRouter::new().mount(
		"/api/snippets/",
		crate::apps::snippets::urls::url_patterns(),
	);

	Arc::new(router)
}

// Register URL patterns for automatic discovery by the framework
register_url_patterns!();
