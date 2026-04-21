//! Client-side routing for {{ project_name }}
//!
//! Add routes inside `init_router()` using `.route(path, handler)`.
//! Use `with_router(|r| r.push("/path/"))` to navigate from components.

use reinhardt::pages::router::Router;

/// Re-export for ergonomic access within this module and sub-modules.
pub use reinhardt::pages::with_router;

/// Build the application router.
///
/// Called once by [`super::bootstrap`] via `ClientLauncher::router(init_router)`.
pub fn init_router() -> Router {
	Router::new()
		// Add routes here, e.g.:
		// .route("/", || home_page())
		// .route("/about/", || about_page())
}
