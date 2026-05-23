//! WASM client application launcher.

mod launcher;
mod link_interceptor;
mod spa_router;

use spa_router::SpaRouter;
use std::cell::RefCell;

pub use launcher::{ClientLauncher, LaunchCtx, PathCtx, PathParams};

thread_local! {
	/// Globally stored SPA router used by [`ClientLauncher::launch`] and
	/// the internal [`with_spa_router`] helper. Holds a
	/// `Box<dyn SpaRouter>` backed by a [`reinhardt_urls::routers::ClientRouter`].
	/// (Refs #4234)
	static APP_ROUTER: RefCell<Option<Box<dyn SpaRouter>>> = const { RefCell::new(None) };
}


/// Internal helper: access the globally registered SPA router as a
/// trait object.
///
/// Operates against `&dyn SpaRouter` so internal launcher code (render
/// mount, link interceptor, history listener wiring) stays agnostic of
/// the concrete router type. (Refs #4234)
///
/// # Panics
///
/// Panics if `ClientLauncher::launch` has not been called yet.
#[cfg_attr(not(wasm), allow(dead_code))]
#[doc(hidden)]
pub fn with_spa_router<F, R>(f: F) -> R
where
	F: FnOnce(&dyn SpaRouter) -> R,
{
	APP_ROUTER.with(|r| {
		let borrow = r.borrow();
		let spa = borrow
			.as_ref()
			.expect("Router not initialized. Call ClientLauncher::launch() first.");
		f(&**spa)
	})
}

/// Fallible variant of [`with_spa_router`] used by the public imperative
/// navigation API ([`crate::router::navigate`],
/// [`crate::reactive::hooks::router::RouterHandle`]).
///
/// Returns `None` when the SPA router has not been installed (instead of
/// panicking like [`with_spa_router`]), so the form! macro's WASM-side
/// codegen can fall back to a hard navigation when the form is rendered
/// outside `ClientLauncher::launch` — e.g. in unit tests, dev tooling, or
/// applications that intentionally mount forms without an SPA router.
/// Refs #4610.
// Native builds never instantiate this helper (the form! macro's
// fallback path is gated on `#[cfg(wasm)]`), so silence the dead-code
// warning off-wasm.
#[cfg_attr(not(wasm), allow(dead_code))]
#[doc(hidden)]
pub fn try_with_spa_router<F, R>(f: F) -> Option<R>
where
	F: FnOnce(&dyn SpaRouter) -> R,
{
	APP_ROUTER.with(|r| {
		let borrow = r.borrow();
		borrow.as_ref().map(|spa| f(&**spa))
	})
}

#[cfg(wasm)]
fn store_spa_router(router: Box<dyn SpaRouter>) {
	APP_ROUTER.with(|r| {
		*r.borrow_mut() = Some(router);
	});
}


/// Hidden API for installing a
/// [`reinhardt_urls::routers::ClientRouter`] in the per-thread
/// `APP_ROUTER` slot from integration tests on native targets.
///
/// Companion to [`__install_router_for_test`] for the canonical
/// `router_client` path. Refs #4610.
#[doc(hidden)]
pub fn __install_client_router_for_test(router: reinhardt_urls::routers::ClientRouter) {
	APP_ROUTER.with(|slot| {
		*slot.borrow_mut() = Some(Box::new(router));
	});
}

/// Hidden API for clearing the per-thread `APP_ROUTER` slot at the end of
/// an integration test. Refs #4610.
#[doc(hidden)]
pub fn __clear_spa_router_for_test() {
	APP_ROUTER.with(|slot| {
		*slot.borrow_mut() = None;
	});
}

/// Hidden API that snapshots the installed SPA router's `current_path`
/// signal for assertion in integration tests.
///
/// Returns `None` when no router is installed; otherwise returns the
/// current path as it would be observed by a reactive consumer right
/// now. Uses `get_untracked()` because tests are not run inside a
/// reactive context and we only need a point-in-time value.
///
/// Refs #4610: lets `tests/use_router_integration.rs` verify that
/// `RouterHandle::push` / `RouterHandle::replace` actually move the
/// shared `current_path` signal, rather than just returning `Ok` while
/// silently no-op'ing.
#[doc(hidden)]
pub fn __current_path_for_test() -> Option<String> {
	APP_ROUTER.with(|slot| {
		slot.borrow()
			.as_ref()
			.map(|spa| spa.current_path().get_untracked())
	})
}

