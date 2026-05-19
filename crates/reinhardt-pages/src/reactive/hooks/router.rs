//! Imperative router hook (`use_router`).
//!
//! Issue #4610: until now, the only way to navigate programmatically from a
//! component or reactive context was to either (a) hard-code
//! `web_sys::window().unwrap().location().set_href(...)` (which triggers a
//! full document reload and defeats the SPA router), or (b) reach into the
//! deprecated [`crate::with_router`] helper which is in a deprecation
//! window. Neither shape composes with the canonical
//! [`reinhardt_urls::routers::ClientRouter`] path that `ClientLauncher::router_client`
//! installs.
//!
//! `use_router()` returns a zero-sized [`RouterHandle`] that dispatches into
//! whichever router builder the application picked, by re-entering the
//! `APP_ROUTER` thread-local on every call. Using a handle rather than a
//! cloned `Rc<Router>` matches the shape of `use_state` / `use_effect`
//! (zero-cost wrapper, no strong reference bookkeeping) and avoids pinning
//! the router for the lifetime of any captured closure.
//!
//! See also the free [`crate::router::navigate`] function for one-shot
//! navigation calls outside a hook context.

use crate::app::with_spa_router;
use crate::router::NavigationType;

/// Public navigation error returned by [`RouterHandle::push`],
/// [`RouterHandle::replace`], and [`RouterHandle::navigate`].
///
/// The inner SPA router translates its concrete error (either
/// `crate::router::RouterError` for the deprecated `Router` path or
/// `reinhardt_urls::routers::client_router::error::RouterError` for the
/// canonical `ClientRouter` path) into a stringly-typed message so the
/// public API does not couple to either crate's error enum.
#[derive(Debug)]
pub enum NavigateError {
	/// The underlying SPA router rejected the navigation.
	///
	/// The string is the inner router's `Display` representation.
	RouterRejected(String),
}

impl core::fmt::Display for NavigateError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::RouterRejected(msg) => write!(f, "router rejected navigation: {}", msg),
		}
	}
}

impl std::error::Error for NavigateError {}

/// Imperative navigation handle returned by [`use_router`].
///
/// Zero-sized: every method re-enters the `APP_ROUTER` thread-local rather
/// than caching a strong reference to the underlying router. That keeps
/// captured closures free of router-lifetime bookkeeping and lets the
/// launcher swap the router out at teardown without dangling clones.
///
/// # Panics
///
/// All methods on `RouterHandle` panic if `ClientLauncher::launch()` has not
/// installed an SPA router on the current thread. This mirrors the contract
/// of `use_state`/`use_effect`: hooks are only callable from inside a
/// mounted component tree.
#[derive(Clone, Copy, Debug)]
pub struct RouterHandle;

impl RouterHandle {
	/// Pushes a new entry onto the browser history stack and dispatches
	/// the SPA router's navigation observers.
	///
	/// This is the SPA equivalent of setting `window.location.href`: the
	/// route changes, registered observers fire, and the matching route
	/// component re-renders — all without a document reload.
	pub fn push(&self, path: impl Into<String>) -> Result<(), NavigateError> {
		let path = path.into();
		with_spa_router(|router| router.push(&path))
			.map_err(|e| NavigateError::RouterRejected(e.to_string()))
	}

	/// Replaces the current browser history entry and dispatches the SPA
	/// router's navigation observers.
	///
	/// Useful when the new path should not appear as a separate entry in
	/// the user's back/forward history (e.g. post-login redirect).
	pub fn replace(&self, path: impl Into<String>) -> Result<(), NavigateError> {
		let path = path.into();
		with_spa_router(|router| router.replace(&path))
			.map_err(|e| NavigateError::RouterRejected(e.to_string()))
	}

	/// Dispatches navigation based on the supplied [`NavigationType`].
	///
	/// `NavigationType::Push` and `NavigationType::Replace` delegate to
	/// [`Self::push`] and [`Self::replace`] respectively. `Pop` and
	/// `Initial` are interpreted as "do not modify history" — they are
	/// produced by the browser itself (popstate / first paint) and are
	/// accepted here as no-ops so callers can pass through a value
	/// received from a navigation observer without filtering.
	pub fn navigate(
		&self,
		path: impl Into<String>,
		nav: NavigationType,
	) -> Result<(), NavigateError> {
		match nav {
			NavigationType::Push => self.push(path),
			NavigationType::Replace => self.replace(path),
			// `Pop` and `Initial` are browser-originated events; the
			// imperative API has nothing to do for them.
			NavigationType::Pop | NavigationType::Initial => Ok(()),
		}
	}
}

/// Returns a [`RouterHandle`] for imperative navigation from the current
/// component or reactive context.
///
/// # Panics
///
/// `RouterHandle`'s methods panic if `ClientLauncher::launch()` has not
/// installed an SPA router on the current thread (same contract as
/// `use_state` and `use_effect`).
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::reactive::hooks::{use_effect, use_router, use_state};
///
/// // Navigate to `/welcome` once `should_redirect` flips to `true`.
/// let (should_redirect, _set) = use_state(false);
/// let router = use_router();
/// use_effect({
///     let should_redirect = should_redirect.clone();
///     move || {
///         if should_redirect.get() {
///             let _ = router.push("/welcome");
///         }
///     }
/// });
/// ```
pub fn use_router() -> RouterHandle {
	RouterHandle
}
