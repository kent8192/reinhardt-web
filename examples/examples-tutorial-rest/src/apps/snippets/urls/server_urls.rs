//! Server-side URL patterns for the snippets app.
//!
//! This module demonstrates two approaches for defining URL patterns:
//! 1. Function-based views (Tutorial 1-5) - Explicit endpoint registration
//! 2. ViewSet-based (Tutorial 6) - Automatic CRUD endpoint generation
//!
//! Switch between approaches using the USE_VIEWSET environment variable:
//! - Default: Function-based views
//! - USE_VIEWSET=1: ViewSet-based views
//!
//! The `#[url_patterns(InstalledApp::snippets, mode = server)]` attribute
//! (the typed URL-patterns macro introduced in rc.18, see reinhardt-web
//! discussion #3770) pairs this router with its owning app at compile time
//! via the `AppLabel` trait — if `InstalledApp::snippets` is removed from
//! `installed_apps! { ... }`, this file stops compiling. The macro also
//! applies `.with_namespace("snippets")` to the returned `ServerRouter`,
//! scoping the named-URL reversal table (e.g. `"snippets:snippets_list"`)
//! without changing the actual request paths — the literal `/api/` prefix
//! is still attached explicitly in `src/config/urls.rs`.

use reinhardt::ServerRouter;
use reinhardt::url_patterns;

use crate::apps::snippets::views;
use crate::config::apps::InstalledApp;

#[url_patterns(InstalledApp::snippets, mode = server)]
pub fn server_url_patterns() -> ServerRouter {
	// Check which approach to use
	if std::env::var("USE_VIEWSET").is_ok() {
		// Option 2: ViewSet-based approach (Tutorial 6)
		// Automatically generates all CRUD endpoints with pagination, filtering, and ordering
		// - GET    /api/snippets-viewset/         - List all snippets (with pagination)
		// - POST   /api/snippets-viewset/         - Create a new snippet
		// - GET    /api/snippets-viewset/{id}/    - Retrieve a specific snippet
		// - PUT    /api/snippets-viewset/{id}/    - Update a snippet
		// - PATCH  /api/snippets-viewset/{id}/    - Partially update a snippet
		// - DELETE /api/snippets-viewset/{id}/    - Delete a snippet
		//
		// Additional query parameters:
		// - ?page=1&page_size=10                  - Pagination
		// - ?language=rust&title=hello            - Filtering
		// - ?ordering=created_at,-title           - Ordering (- for descending)
		ServerRouter::new().viewset("/snippets-viewset", views::viewset())
	} else {
		// Option 1: Function-based approach (Tutorial 1-5)
		// Explicitly register each endpoint
		ServerRouter::new()
			.endpoint(views::list)
			.endpoint(views::create)
			.endpoint(views::retrieve)
			.endpoint(views::update)
			.endpoint(views::delete)
	}
}
