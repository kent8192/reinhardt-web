//! Client-side routing for the {{ app_name }} SPA.
//!
//! Routes are declared with `#[url_patterns(InstalledApp::{{ app_name }}, mode = client)]`,
//! which auto-registers the router via inventory. The WASM entry point
//! consumes this builder through `ClientLauncher::router_client(...)`.

use reinhardt::ClientRouter;
use reinhardt::url_patterns;

use crate::config::apps::InstalledApp;

#[url_patterns(InstalledApp::{{ app_name }}, mode = client)]
pub fn client_url_patterns() -> ClientRouter {
	ClientRouter::new()
	// Register client-side routes here, e.g.:
	//     .route("/", index_page)
}
