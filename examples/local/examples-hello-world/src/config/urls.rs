//! URL configuration for examples-hello-world project
//!
//! The `url_patterns` routes URLs to handlers.

use reinhardt::prelude::*;
use reinhardt::register_url_patterns;
use std::sync::Arc;

pub fn url_patterns() -> Arc<UnifiedRouter> {
	let router = UnifiedRouter::new();

	// Mount hello app routes
	let router = router.mount("/", crate::apps::hello::urls::url_patterns());

	Arc::new(router)
}

// Register URL patterns for automatic discovery by the framework
register_url_patterns!();
