//! WASM client application launcher.

mod launcher;
mod link_interceptor;
mod spa_router;

#[allow(deprecated)]
// (Refs #4234) Importing deprecated routing types intentionally during the deprecation cycle.
use crate::router::Router;
use spa_router::SpaRouter;
use std::cell::RefCell;

pub use launcher::{ClientLauncher, LaunchCtx, PathCtx, PathParams};

thread_local! {
	/// Globally stored SPA router used by [`ClientLauncher::launch`] and
	/// the public deprecated [`with_router`] helper. Holds a
	/// `Box<dyn SpaRouter>` so the same slot can back either the
	/// deprecated `Router`-based API or the canonical `ClientRouter`-
	/// based API. (Refs #4234)
	static APP_ROUTER: RefCell<Option<Box<dyn SpaRouter>>> = const { RefCell::new(None) };
}

/// Access the globally registered client router.
///
/// # Panics
///
/// Panics if `ClientLauncher::launch` has not been called yet, or if
/// the application initialised the launcher with the new
/// [`ClientLauncher::router_client`] builder rather than the
/// deprecated [`ClientLauncher::router`] builder. The `router_client`
/// path stores a [`reinhardt_urls::routers::ClientRouter`] which
/// cannot be downcast to a `Router`.
///
/// New code should access reactive routing state through the
/// [`reinhardt_urls::routers::ClientRouter`] returned by the
/// `router_client` builder closure (capture it in a local + clone its
/// `Signal`s) rather than relying on the global `with_router` helper.
/// (Refs #4234)
#[deprecated(
	since = "0.1.0-rc.27",
	note = "If you used `ClientLauncher::router_client(...)`, capture the \
	        `urls::ClientRouter` locally instead — `with_router` panics in that \
	        case. Refs #4234, cloud#578 Phase E."
)]
#[allow(deprecated)] // (Refs #4234) Body operates on the deprecated `Router` by design.
pub fn with_router<F, R>(f: F) -> R
where
	F: FnOnce(&Router) -> R,
{
	APP_ROUTER.with(|r| {
		let borrow = r.borrow();
		let spa = borrow
			.as_ref()
			.expect("Router not initialized. Call ClientLauncher::launch() first.");
		let router = spa.as_any().downcast_ref::<Router>().expect(
			"with_router() requires the deprecated `ClientLauncher::router` builder; \
			 the `router_client` builder stores a `ClientRouter` which cannot be \
			 downcast to `Router`. See ClientLauncher::router_client docs.",
		);
		f(router)
	})
}

/// Internal helper: access the globally registered SPA router as a
/// trait object.
///
/// Mirrors [`with_router`] but operates against `&dyn SpaRouter` so
/// internal launcher code (render mount, link interceptor, history
/// listener wiring) stays agnostic of which builder was used. (Refs
/// #4234)
///
/// # Panics
///
/// Panics if `ClientLauncher::launch` has not been called yet.
#[cfg_attr(not(wasm), allow(dead_code))]
pub(crate) fn with_spa_router<F, R>(f: F) -> R
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
pub(crate) fn try_with_spa_router<F, R>(f: F) -> Option<R>
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

/// Hidden API for installing a [`crate::router::Router`] in the per-thread
/// `APP_ROUTER` slot from integration tests on native targets.
///
/// On wasm the launcher's `launch()` does this through the private
/// `store_spa_router` above; on native the launcher's render path is
/// behind `#[cfg(wasm)]`, so integration tests that exercise the imperative
/// navigation API (`use_router`, `navigate`) need a way to seed the slot
/// without going through the full launcher. Marked `#[doc(hidden)]` so it
/// stays out of the SemVer surface and the published documentation —
/// mirrors the `__diag_*` testing pattern in `router::core::Router`.
///
/// Tests MUST clear the slot at the end of the test (see
/// [`__clear_spa_router_for_test`]) and SHOULD use `#[serial(router)]` to
/// avoid interleaving with other tests that touch the same thread-local.
///
/// Refs #4610.
#[doc(hidden)]
#[allow(deprecated)] // (Refs #4234) Bridge for the deprecated `Router` path used by tests.
pub fn __install_router_for_test(router: crate::router::Router) {
	APP_ROUTER.with(|slot| {
		*slot.borrow_mut() = Some(Box::new(router));
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

#[cfg(test)]
#[allow(deprecated)] // (Refs #4234) Tests exercise deprecated `pages::Router` directly.
mod tests {
	use super::*;
	use rstest::*;

	#[rstest]
	fn test_with_router_panics_before_init() {
		let result = std::panic::catch_unwind(|| with_router(|_r| ()));

		assert!(result.is_err());
	}
}
