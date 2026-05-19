//! Free-standing imperative navigation entry point.
//!
//! Issue #4610: the form! macro's WASM-side codegen needs an imperative
//! navigation primitive it can splice into the generated `submit()` body
//! without going through a hook (hooks must be called from a reactive
//! context, which the generated `async fn submit(&self)` is not). This free
//! function is a thin wrapper over [`crate::reactive::hooks::RouterHandle`]
//! so the macro can call `#pages_crate::navigate(__url, NavigationType::Push)`
//! from anywhere on wasm.
//!
//! Outside the macro, prefer [`crate::reactive::hooks::use_router`] from
//! component bodies so the call site documents that it expects an SPA
//! context.

use crate::reactive::hooks::router::{NavigateError, RouterHandle};
use crate::router::NavigationType;

/// One-shot imperative SPA navigation.
///
/// Equivalent to `use_router().navigate(path, nav)` — see
/// [`crate::reactive::hooks::use_router`] for the hook form.
///
/// # Errors
///
/// - `Err(NavigateError::RouterNotInstalled)` — `ClientLauncher::launch()`
///   has not installed an SPA router on the current thread. The form!
///   macro's WASM-side codegen uses this discriminant to fall back to a
///   hard navigation; component / hook callers SHOULD treat it as a
///   programmer error.
/// - `Err(NavigateError::RouterRejected(_))` — the installed router
///   rejected the navigation (e.g. unknown route, invalid path). The
///   inner string is the router's error message, suitable for logging
///   but not for direct user display.
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::{navigate, router::NavigationType};
///
/// let _ = navigate("/welcome", NavigationType::Push);
/// ```
pub fn navigate(path: impl Into<String>, nav: NavigationType) -> Result<(), NavigateError> {
	RouterHandle.navigate(path, nav)
}
