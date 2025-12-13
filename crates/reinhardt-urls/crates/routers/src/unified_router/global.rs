//! Global router registry for URL inspection (showurls command)

use super::UnifiedRouter;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;

/// Global router registry
static GLOBAL_ROUTER: OnceCell<StdRwLock<Option<Arc<UnifiedRouter>>>> = OnceCell::new();

/// Register the application's main router globally
///
/// This allows commands like `showurls` to inspect registered routes.
/// The router is automatically wrapped in `Arc` internally.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_routers::{UnifiedRouter, register_router};
/// use hyper::Method;
///
/// let router = UnifiedRouter::new()
///     .with_prefix("/api/v1")
///     .function("/health", Method::GET, health_handler);
///
/// // No Arc::new() needed!
/// register_router(router);
/// ```
pub fn register_router(router: UnifiedRouter) {
	register_router_arc(Arc::new(router));
}

/// Register a router that is already wrapped in Arc
///
/// This is provided for cases where you already have an `Arc<UnifiedRouter>`.
/// In most cases, you should use `register_router()` instead.
pub fn register_router_arc(router: Arc<UnifiedRouter>) {
	let cell = GLOBAL_ROUTER.get_or_init(|| StdRwLock::new(None));
	let mut guard = cell.write().unwrap();
	*guard = Some(router);
}

/// Get a reference to the globally registered router
///
/// Returns `None` if no router has been registered.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_routers::get_router;
///
/// if let Some(router) = get_router() {
///     let routes = router.get_all_routes();
///     println!("Registered routes: {}", routes.len());
/// }
/// ```
pub fn get_router() -> Option<Arc<UnifiedRouter>> {
	GLOBAL_ROUTER
		.get()
		.and_then(|cell| cell.read().unwrap().clone())
}

/// Check if a router has been registered
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_routers::is_router_registered;
///
/// if !is_router_registered() {
///     println!("Warning: No router registered");
/// }
/// ```
pub fn is_router_registered() -> bool {
	GLOBAL_ROUTER
		.get()
		.map(|cell| cell.read().unwrap().is_some())
		.unwrap_or(false)
}

/// Clear the registered router (useful for tests)
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_routers::clear_router;
///
/// clear_router();
/// assert!(!is_router_registered());
/// ```
pub fn clear_router() {
	if let Some(cell) = GLOBAL_ROUTER.get() {
		let mut guard = cell.write().unwrap();
		*guard = None;
	}
}
