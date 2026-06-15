//! URL configuration for examples-tutorial-rest project
//!
//! The `routes` function defines all URL patterns for this project.
//!
//! The `/api/` prefix is a literal path (no `{...}` parameters), which
//! satisfies the rc.24 guard that panics if `ServerRouter::mount()` receives
//! a prefix containing path parameters.

use reinhardt::UnifiedRouter;
use reinhardt::routes;

#[routes]
pub fn routes() -> UnifiedRouter {
	crate::native_runtime::mount_api_routes(UnifiedRouter::new())
}
