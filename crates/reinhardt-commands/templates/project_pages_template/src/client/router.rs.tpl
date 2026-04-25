//! Client-side router for {{ project_name }}.
//!
//! [`init_router`] is invoked once by `super::lib::main` through
//! `ClientLauncher::router`. From any component, call [`with_router`]
//! (re-exported from `reinhardt::pages`) to inspect or push routing state.

use reinhardt::pages::component::Page;
use reinhardt::pages::page;
use reinhardt::pages::router::Router;

// Re-export so callers can `use crate::client::router::with_router`.
pub use reinhardt::pages::with_router;

/// Build the application router.
///
/// Add new routes here, e.g.:
///
/// ```rust,ignore
/// .route("/", || crate::client::pages::index_page())
/// ```
pub fn init_router() -> Router {
	Router::new()
		// Add routes here
		.not_found(|| not_found_page("Page not found"))
}

/// Default 404 / error page used by `init_router`.
fn not_found_page(message: &str) -> Page {
	let message = message.to_string();
	page!(|message: String| {
		div {
			class: "container mt-5",
			div {
				class: "alert alert-danger",
				{ message }
			}
			a {
				href: "/",
				class: "btn btn-primary",
				"Back to Home"
			}
		}
	})(message)
}
