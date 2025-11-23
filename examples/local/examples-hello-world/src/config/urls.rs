//! URL configuration for examples-hello-world project
//!
//! The `url_patterns` routes URLs to handlers.

use reinhardt::prelude::*;
use std::sync::Arc;

pub fn url_patterns() -> Arc<UnifiedRouter> {
	let router = UnifiedRouter::new();

	// Include hello app routes
	let router = router.include("/", crate::apps::hello::urls::url_patterns());

	Arc::new(router)
}
