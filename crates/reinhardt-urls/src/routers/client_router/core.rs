//! Core ClientRouter Implementation.
//!
//! This module provides the main ClientRouter struct and routing logic.
//! The router uses `Page` type for all view rendering.

use super::error::RouterError;
use super::handler::{
	RouteHandler, no_params_handler, result_handler, single_path_handler, three_path_handler,
	two_path_handler, with_params_handler,
};
#[cfg(wasm)]
use super::history::setup_popstate_listener;
use super::history::{HistoryState, NavigationType, current_path, push_state, replace_state};
use super::params::{FromPath, ParamContext, Path, SingleFromPath};
use super::pattern::ClientPathPattern;
use reinhardt_core::page::Page;
use reinhardt_core::reactive::Signal;
use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for route guard functions.
pub(super) type RouteGuard = Arc<dyn Fn(&ClientRouteMatch) -> bool + Send + Sync>;

// (Refs #4234, Fixes #4258) Mirrors `pages::Router NavigationObservers /
// NavigationListener`. Gated `#[cfg(wasm)]` so `ClientRouter` stays
// `Send + Sync` on native targets — `Rc<RefCell<_>>` is `!Send + !Sync`
// and would otherwise propagate up through `UnifiedRouter` and break
// multi-threaded DI registration on native.
//
// `Rc<RefCell<...>>` because Routers are not `Send` on wasm32 anyway,
// and the borrow is released before listeners run (see `notify_observers`).
#[cfg(wasm)]
type NavigationObservers = std::rc::Rc<std::cell::RefCell<Vec<std::rc::Weak<NavigationListener>>>>;

/// Boxed closure stored behind a `Weak<...>` so a dropped
/// [`NavigationSubscription`] drops its strong `Rc`, after which
/// [`ClientRouter::notify_observers`] filters out the dead `Weak`.
type NavigationListener = dyn Fn(&str, &HashMap<String, String>) + 'static;

/// RAII handle returned by [`ClientRouter::on_navigate`].
///
/// While alive, the registered listener fires on every
/// [`ClientRouter::push`] / [`ClientRouter::replace`] and on browser
/// back/forward navigation handled by
/// [`ClientRouter::setup_history_listener`]. Dropping this handle
/// removes the listener (no explicit `unsubscribe` call needed).
///
/// Mirrors `reinhardt_pages::router::NavigationSubscription`. (Refs #4234)
pub struct NavigationSubscription {
	#[allow(dead_code)] // Dropped automatically; presence keeps the Weak alive.
	listener: std::rc::Rc<NavigationListener>,
}

/// A matched route with extracted parameters.
#[derive(Debug, Clone)]
pub struct ClientRouteMatch {
	/// The matched route.
	pub route: ClientRoute,
	/// Extracted path parameters.
	pub params: HashMap<String, String>,
	/// Parameter values in the order they appear in the pattern.
	///
	/// This guarantees that tuple extraction works correctly by index,
	/// matching the order of parameters in the URL pattern.
	pub(crate) param_values: Vec<String>,
}

/// A single route definition.
pub struct ClientRoute {
	/// The path pattern.
	pattern: ClientPathPattern,
	/// Optional route name for reverse lookups.
	name: Option<String>,
	/// The route handler.
	handler: Arc<dyn RouteHandler>,
	/// Optional guard function.
	guard: Option<RouteGuard>,
}

impl Clone for ClientRoute {
	fn clone(&self) -> Self {
		Self {
			pattern: self.pattern.clone(),
			name: self.name.clone(),
			handler: Arc::clone(&self.handler),
			guard: self.guard.clone(),
		}
	}
}

impl std::fmt::Debug for ClientRoute {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ClientRoute")
			.field("pattern", &self.pattern)
			.field("name", &self.name)
			.field("has_guard", &self.guard.is_some())
			.finish()
	}
}

impl ClientRoute {
	/// Creates a new route.
	///
	/// # Panics
	///
	/// Panics if the pattern is invalid (exceeds length/segment limits or invalid regex).
	/// Use [`ClientPathPattern::new`] directly for fallible construction.
	pub fn new<F>(pattern: &str, component: F) -> Self
	where
		F: Fn() -> Page + Send + Sync + 'static,
	{
		Self {
			pattern: ClientPathPattern::new(pattern)
				.unwrap_or_else(|e| panic!("Invalid route pattern '{}': {}", pattern, e)),
			name: None,
			handler: no_params_handler(component),
			guard: None,
		}
	}

	/// Creates a named route.
	///
	/// # Panics
	///
	/// Panics if the pattern is invalid (exceeds length/segment limits or invalid regex).
	/// Use [`ClientPathPattern::new`] directly for fallible construction.
	pub fn named<F>(name: impl Into<String>, pattern: &str, component: F) -> Self
	where
		F: Fn() -> Page + Send + Sync + 'static,
	{
		Self {
			pattern: ClientPathPattern::new(pattern)
				.unwrap_or_else(|e| panic!("Invalid route pattern '{}': {}", pattern, e)),
			name: Some(name.into()),
			handler: no_params_handler(component),
			guard: None,
		}
	}

	/// Adds a guard to this route.
	pub fn with_guard<G>(mut self, guard: G) -> Self
	where
		G: Fn(&ClientRouteMatch) -> bool + Send + Sync + 'static,
	{
		self.guard = Some(Arc::new(guard));
		self
	}

	/// Returns the route name.
	pub fn name(&self) -> Option<&str> {
		self.name.as_deref()
	}

	/// Returns the pattern.
	pub fn pattern(&self) -> &ClientPathPattern {
		&self.pattern
	}

	/// Checks if the guard allows access.
	pub fn check_guard(&self, route_match: &ClientRouteMatch) -> bool {
		self.guard.as_ref().map(|g| g(route_match)).unwrap_or(true)
	}
}

/// The main client-side router.
///
/// `ClientRouter` renders views using the [`Page`] type.
pub struct ClientRouter {
	/// Registered routes.
	routes: Vec<ClientRoute>,
	/// Named routes for reverse lookups.
	named_routes: HashMap<String, usize>,
	/// Current path signal.
	current_path: Signal<String>,
	/// Current params signal.
	current_params: Signal<HashMap<String, String>>,
	/// Current matched route name signal.
	current_route_name: Signal<Option<String>>,
	/// Not found handler.
	not_found: Option<Arc<dyn Fn() -> Page + Send + Sync>>,
	// (Refs #4234, Fixes #4258) Mirrors `pages::Router::navigation_observers`.
	// Navigation observers registered via `on_navigate`. Held as `Weak`
	// so dropping the returned `NavigationSubscription` deregisters the
	// listener.
	//
	// Gated `#[cfg(wasm)]` because `Rc<RefCell<_>>` is `!Send + !Sync`
	// and the reactive observation pattern only fires on WASM (the popstate
	// listener is wasm-only and `notify_observers` is a no-op on native).
	// Without this gate `ClientRouter` becomes `!Send + !Sync` on native,
	// breaking `UnifiedRouter` registration in multi-threaded DI containers.
	#[cfg(wasm)]
	navigation_observers: NavigationObservers,
	// (Refs #4234, Fixes #4258) Mirrors `pages::Router::dispatch_count`.
	// Cumulative count of `notify_observers` invocations since this
	// Router was constructed. Used by tests to assert invariants that
	// DOM-only assertions cannot reach.
	//
	// Gated `#[cfg(wasm)]` for the same reason as `navigation_observers`:
	// `Rc<Cell<u64>>` is `!Send + !Sync` on native.
	#[cfg(wasm)]
	dispatch_count: std::rc::Rc<std::cell::Cell<u64>>,
}

impl std::fmt::Debug for ClientRouter {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ClientRouter")
			.field("routes_count", &self.routes.len())
			.field(
				"named_routes",
				&self.named_routes.keys().collect::<Vec<_>>(),
			)
			.finish()
	}
}

impl Default for ClientRouter {
	fn default() -> Self {
		Self::new()
	}
}

impl ClientRouter {
	/// Creates a new router.
	pub fn new() -> Self {
		let initial_path = current_path().unwrap_or_else(|_| "/".to_string());

		Self {
			routes: Vec::new(),
			named_routes: HashMap::new(),
			current_path: Signal::new(initial_path),
			current_params: Signal::new(HashMap::new()),
			current_route_name: Signal::new(None),
			not_found: None,
			// (Fixes #4258) Reactive observation state is wasm-only; see field
			// definitions on `ClientRouter`.
			#[cfg(wasm)]
			navigation_observers: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
			#[cfg(wasm)]
			dispatch_count: std::rc::Rc::new(std::cell::Cell::new(0)),
		}
	}

	/// Merges routes from another router into this one.
	///
	/// Routes and named route mappings from `other` are appended.
	/// Signals and not_found handler from `other` are discarded.
	// Used by UnifiedRouter::mount_unified() on WASM and native targets.
	pub(crate) fn merge(mut self, other: ClientRouter) -> Self {
		let offset = self.routes.len();
		for (name, idx) in other.named_routes {
			self.named_routes.insert(name, idx + offset);
		}
		self.routes.extend(other.routes);
		self
	}

	/// Prefix all named route keys with `"namespace:"`.
	///
	/// This is the client-side equivalent of `ServerRouter::with_namespace()`.
	/// Called by `#[url_patterns(InstalledApp::<variant>, mode = client)]` (or
	/// `mode = unified`) to ensure registered names match the `"app:route"`
	/// format used by per-app resolver structs.
	pub fn with_namespace(mut self, namespace: &str) -> Self {
		let old = std::mem::take(&mut self.named_routes);
		for (name, idx) in old {
			self.named_routes.insert(format!("{namespace}:{name}"), idx);
		}
		// Also update route names stored inside ClientRoute
		for route in &mut self.routes {
			if let Some(ref old_name) = route.name {
				route.name = Some(format!("{namespace}:{old_name}"));
			}
		}
		self
	}

	/// Adds a route to the router.
	pub fn route<F>(mut self, pattern: &str, component: F) -> Self
	where
		F: Fn() -> Page + Send + Sync + 'static,
	{
		self.routes.push(ClientRoute::new(pattern, component));
		self
	}

	/// Adds a named route to the router.
	pub fn named_route<F>(mut self, name: &str, pattern: &str, component: F) -> Self
	where
		F: Fn() -> Page + Send + Sync + 'static,
	{
		let index = self.routes.len();
		self.routes
			.push(ClientRoute::named(name, pattern, component));
		self.named_routes.insert(name.to_string(), index);
		self
	}

	/// Adds a route with typed path parameters.
	pub fn route_params<F, T>(mut self, pattern: &str, handler: F) -> Self
	where
		F: Fn(Path<T>) -> Page + Send + Sync + 'static,
		T: FromPath + Send + Sync + 'static,
	{
		self.routes.push(ClientRoute {
			pattern: ClientPathPattern::new(pattern)
				.unwrap_or_else(|e| panic!("Invalid route pattern '{}': {}", pattern, e)),
			name: None,
			handler: with_params_handler(handler),
			guard: None,
		});
		self
	}

	/// Adds a named route with typed path parameters.
	pub fn named_route_params<F, T>(mut self, name: &str, pattern: &str, handler: F) -> Self
	where
		F: Fn(Path<T>) -> Page + Send + Sync + 'static,
		T: FromPath + Send + Sync + 'static,
	{
		let index = self.routes.len();
		self.routes.push(ClientRoute {
			pattern: ClientPathPattern::new(pattern)
				.unwrap_or_else(|e| panic!("Invalid route pattern '{}': {}", pattern, e)),
			name: Some(name.to_string()),
			handler: with_params_handler(handler),
			guard: None,
		});
		self.named_routes.insert(name.to_string(), index);
		self
	}

	/// Adds a route with typed path parameters that returns a Result.
	pub fn route_result<F, T, E>(mut self, pattern: &str, handler: F) -> Self
	where
		F: Fn(Path<T>) -> Result<Page, E> + Send + Sync + 'static,
		T: FromPath + Send + Sync + 'static,
		E: Into<RouterError> + Send + Sync + 'static,
	{
		self.routes.push(ClientRoute {
			pattern: ClientPathPattern::new(pattern)
				.unwrap_or_else(|e| panic!("Invalid route pattern '{}': {}", pattern, e)),
			name: None,
			handler: result_handler(handler),
			guard: None,
		});
		self
	}

	/// Adds a named route with typed path parameters that returns a Result.
	pub fn named_route_result<F, T, E>(mut self, name: &str, pattern: &str, handler: F) -> Self
	where
		F: Fn(Path<T>) -> Result<Page, E> + Send + Sync + 'static,
		T: FromPath + Send + Sync + 'static,
		E: Into<RouterError> + Send + Sync + 'static,
	{
		let index = self.routes.len();
		self.routes.push(ClientRoute {
			pattern: ClientPathPattern::new(pattern)
				.unwrap_or_else(|e| panic!("Invalid route pattern '{}': {}", pattern, e)),
			name: Some(name.to_string()),
			handler: result_handler(handler),
			guard: None,
		});
		self.named_routes.insert(name.to_string(), index);
		self
	}

	/// Adds a route with a guard.
	pub fn guarded_route<F, G>(mut self, pattern: &str, component: F, guard: G) -> Self
	where
		F: Fn() -> Page + Send + Sync + 'static,
		G: Fn(&ClientRouteMatch) -> bool + Send + Sync + 'static,
	{
		self.routes
			.push(ClientRoute::new(pattern, component).with_guard(guard));
		self
	}

	/// Adds a route with a single path parameter using `Path<T>` extractor.
	///
	/// # Example
	///
	/// ```ignore
	/// let router = ClientRouter::new()
	///     .route_path("/users/{id}/", |Path(id): Path<i64>| {
	///         user_detail(id)
	///     });
	/// ```
	pub fn route_path<F, T>(mut self, pattern: &str, handler: F) -> Self
	where
		F: Fn(Path<T>) -> Page + Send + Sync + 'static,
		T: SingleFromPath + Send + Sync + 'static,
	{
		self.routes.push(ClientRoute {
			pattern: ClientPathPattern::new(pattern)
				.unwrap_or_else(|e| panic!("Invalid route pattern '{}': {}", pattern, e)),
			name: None,
			handler: single_path_handler(handler),
			guard: None,
		});
		self
	}

	/// Adds a named route with a single path parameter using `Path<T>` extractor.
	pub fn named_route_path<F, T>(mut self, name: &str, pattern: &str, handler: F) -> Self
	where
		F: Fn(Path<T>) -> Page + Send + Sync + 'static,
		T: SingleFromPath + Send + Sync + 'static,
	{
		let index = self.routes.len();
		self.routes.push(ClientRoute {
			pattern: ClientPathPattern::new(pattern)
				.unwrap_or_else(|e| panic!("Invalid route pattern '{}': {}", pattern, e)),
			name: Some(name.to_string()),
			handler: single_path_handler(handler),
			guard: None,
		});
		self.named_routes.insert(name.to_string(), index);
		self
	}

	/// Adds a route with two path parameters using multiple `Path<T>` extractors.
	///
	/// # Example
	///
	/// ```ignore
	/// let router = ClientRouter::new()
	///     .route_path2("/users/{user_id}/posts/{post_id}/",
	///         |Path(user_id): Path<i64>, Path(post_id): Path<i64>| {
	///             user_post_detail(user_id, post_id)
	///         });
	/// ```
	pub fn route_path2<F, T1, T2>(mut self, pattern: &str, handler: F) -> Self
	where
		F: Fn(Path<T1>, Path<T2>) -> Page + Send + Sync + 'static,
		T1: SingleFromPath + Send + Sync + 'static,
		T2: SingleFromPath + Send + Sync + 'static,
	{
		self.routes.push(ClientRoute {
			pattern: ClientPathPattern::new(pattern)
				.unwrap_or_else(|e| panic!("Invalid route pattern '{}': {}", pattern, e)),
			name: None,
			handler: two_path_handler(handler),
			guard: None,
		});
		self
	}

	/// Adds a named route with two path parameters.
	pub fn named_route_path2<F, T1, T2>(mut self, name: &str, pattern: &str, handler: F) -> Self
	where
		F: Fn(Path<T1>, Path<T2>) -> Page + Send + Sync + 'static,
		T1: SingleFromPath + Send + Sync + 'static,
		T2: SingleFromPath + Send + Sync + 'static,
	{
		let index = self.routes.len();
		self.routes.push(ClientRoute {
			pattern: ClientPathPattern::new(pattern)
				.unwrap_or_else(|e| panic!("Invalid route pattern '{}': {}", pattern, e)),
			name: Some(name.to_string()),
			handler: two_path_handler(handler),
			guard: None,
		});
		self.named_routes.insert(name.to_string(), index);
		self
	}

	/// Adds a route with three path parameters using multiple `Path<T>` extractors.
	///
	/// # Example
	///
	/// ```ignore
	/// let router = ClientRouter::new()
	///     .route_path3("/org/{org}/repos/{repo}/issues/{issue}/",
	///         |Path(org): Path<String>, Path(repo): Path<String>, Path(issue): Path<i32>| {
	///             issue_detail(org, repo, issue)
	///         });
	/// ```
	pub fn route_path3<F, T1, T2, T3>(mut self, pattern: &str, handler: F) -> Self
	where
		F: Fn(Path<T1>, Path<T2>, Path<T3>) -> Page + Send + Sync + 'static,
		T1: SingleFromPath + Send + Sync + 'static,
		T2: SingleFromPath + Send + Sync + 'static,
		T3: SingleFromPath + Send + Sync + 'static,
	{
		self.routes.push(ClientRoute {
			pattern: ClientPathPattern::new(pattern)
				.unwrap_or_else(|e| panic!("Invalid route pattern '{}': {}", pattern, e)),
			name: None,
			handler: three_path_handler(handler),
			guard: None,
		});
		self
	}

	/// Adds a named route with three path parameters.
	pub fn named_route_path3<F, T1, T2, T3>(mut self, name: &str, pattern: &str, handler: F) -> Self
	where
		F: Fn(Path<T1>, Path<T2>, Path<T3>) -> Page + Send + Sync + 'static,
		T1: SingleFromPath + Send + Sync + 'static,
		T2: SingleFromPath + Send + Sync + 'static,
		T3: SingleFromPath + Send + Sync + 'static,
	{
		let index = self.routes.len();
		self.routes.push(ClientRoute {
			pattern: ClientPathPattern::new(pattern)
				.unwrap_or_else(|e| panic!("Invalid route pattern '{}': {}", pattern, e)),
			name: Some(name.to_string()),
			handler: three_path_handler(handler),
			guard: None,
		});
		self.named_routes.insert(name.to_string(), index);
		self
	}

	/// Sets the not found handler.
	pub fn not_found<F>(mut self, component: F) -> Self
	where
		F: Fn() -> Page + Send + Sync + 'static,
	{
		self.not_found = Some(Arc::new(component));
		self
	}

	/// Returns the current path signal.
	pub fn current_path(&self) -> &Signal<String> {
		&self.current_path
	}

	/// Returns the current params signal.
	pub fn current_params(&self) -> &Signal<HashMap<String, String>> {
		&self.current_params
	}

	/// Returns the current route name signal.
	pub fn current_route_name(&self) -> &Signal<Option<String>> {
		&self.current_route_name
	}

	/// Matches a path against registered routes.
	pub fn match_path(&self, path: &str) -> Option<ClientRouteMatch> {
		for route in &self.routes {
			if let Some((params, param_values)) = route.pattern.matches(path) {
				let route_match = ClientRouteMatch {
					route: route.clone(),
					params,
					param_values,
				};

				// Check guard if present
				if route.check_guard(&route_match) {
					return Some(route_match);
				}
			}
		}
		None
	}

	/// Navigates to a path using pushState.
	pub fn push(&self, path: &str) -> Result<(), RouterError> {
		self.navigate(path, NavigationType::Push)
	}

	/// Navigates to a path using replaceState.
	pub fn replace(&self, path: &str) -> Result<(), RouterError> {
		self.navigate(path, NavigationType::Replace)
	}

	/// Internal navigation implementation.
	fn navigate(&self, path: &str, nav_type: NavigationType) -> Result<(), RouterError> {
		let route_match = self.match_path(path);

		let state = HistoryState::new(path)
			.with_params(
				route_match
					.as_ref()
					.map(|m| m.params.clone())
					.unwrap_or_default(),
			)
			.with_route_name(
				route_match
					.as_ref()
					.and_then(|m| m.route.name())
					.unwrap_or(""),
			);

		let result = match nav_type {
			NavigationType::Push => push_state(&state),
			NavigationType::Replace => replace_state(&state),
			_ => Ok(()),
		};

		result.map_err(RouterError::NavigationFailed)?;

		// Update reactive signals
		self.current_path.set(path.to_string());
		self.current_params.set(
			route_match
				.as_ref()
				.map(|m| m.params.clone())
				.unwrap_or_default(),
		);
		self.current_route_name.set(
			route_match
				.as_ref()
				.and_then(|m| m.route.name().map(|s| s.to_string())),
		);

		// (Refs #4234, Inv-1, Inv-5) Invoke registered navigation observers
		// AFTER the history mutation succeeds and AFTER signal updates so
		// listeners reading `Signal::get` from inside their closure see the
		// new state. Mirrors `pages::Router::navigate`.
		let params_for_observers = route_match
			.as_ref()
			.map(|m| m.params.clone())
			.unwrap_or_default();
		self.notify_observers(path, &params_for_observers);

		Ok(())
	}

	/// Register a listener for navigation events.
	///
	/// Returns a [`NavigationSubscription`] handle. Drop the handle to
	/// deregister the listener. The router itself only retains a `Weak`
	/// reference, so dropping the subscription frees the listener
	/// closure immediately. The stale `Weak` entry in
	/// `navigation_observers` is pruned lazily on the next
	/// `notify_observers` call (the listener itself is already gone by
	/// then).
	///
	/// Robust against nested reactive nodes spawned during view rendering
	/// because this subscription is independent of the reactive
	/// `Effect` / `Signal` auto-tracking system.
	///
	/// Mirrors `pages::Router::on_navigate`. (Refs #4234)
	///
	/// Gated `#[cfg(wasm)]` (Fixes #4258) because the underlying
	/// `navigation_observers` field is wasm-only — registering a listener
	/// only makes sense when the popstate listener can fire it.
	#[cfg(wasm)]
	pub fn on_navigate<F>(&self, listener: F) -> NavigationSubscription
	where
		F: Fn(&str, &HashMap<String, String>) + 'static,
	{
		let listener: std::rc::Rc<NavigationListener> = std::rc::Rc::new(listener);
		self.navigation_observers
			.borrow_mut()
			.push(std::rc::Rc::downgrade(&listener));
		NavigationSubscription { listener }
	}

	/// Dispatch the registered `on_navigate` listeners with the given path
	/// and params.
	///
	/// Both `ClientRouter::navigate` (after a programmatic push/replace) and
	/// the popstate listener (after a browser-driven back/forward) end up
	/// calling [`dispatch_navigation_observers`] after the `Signal` updates
	/// so listeners always see the new state when they read `Signal::get`
	/// from inside their closure.
	///
	/// (Refs #4234, Inv-4)
	///
	/// Wasm-only (Fixes #4258): the reactive observer state lives only on
	/// wasm. The native no-op stub immediately below preserves the
	/// cross-target call site in `ClientRouter::navigate`.
	#[cfg(wasm)]
	fn notify_observers(&self, path: &str, params: &HashMap<String, String>) {
		// (Refs #4234) `nav_diag_dom!` invocation from
		// `pages::Router::notify_observers` is intentionally not mirrored
		// here. The nav-diag-dom feature can be added separately in a
		// follow-up if downstream consumers need urls-side runtime
		// diagnostics.
		dispatch_navigation_observers(
			&self.navigation_observers,
			&self.dispatch_count,
			path,
			params,
		);
	}

	/// Native no-op stub for `notify_observers` (Fixes #4258).
	///
	/// On native targets there is no popstate listener and no reactive
	/// observation state, so navigation cannot dispatch listeners. This
	/// stub keeps the call site in `ClientRouter::navigate` cross-target
	/// without leaking `Rc<...>` reactive state into the native
	/// `ClientRouter` (which would break `Send + Sync`).
	#[cfg(native)]
	fn notify_observers(&self, _path: &str, _params: &HashMap<String, String>) {}

	/// Diagnostic counter: number of currently-alive navigation observers.
	///
	/// Returns the count of `Weak<NavigationListener>` entries in
	/// `navigation_observers` whose `strong_count() > 0`. Used by tests in
	/// `tests/wasm/` to assert observer-lifecycle invariants.
	///
	/// Internal diagnostic API. `#[doc(hidden)]` removes this from the
	/// rendered documentation, but it remains technically part of the
	/// public API surface. Treat it as unstable: callers outside this
	/// crate's own tests should not depend on it. (Refs #4234)
	///
	/// Wasm-only (Fixes #4258): backed by `navigation_observers` which
	/// is itself wasm-only. Consumed solely by `tests/wasm/*` (see
	/// `required-features = ["wasm-diag-test"]` in `Cargo.toml`).
	#[cfg(wasm)]
	#[doc(hidden)]
	pub fn __diag_observer_count(&self) -> usize {
		self.navigation_observers
			.borrow()
			.iter()
			.filter(|w| w.strong_count() > 0)
			.count()
	}

	/// Diagnostic counter: cumulative `notify_observers` invocation count.
	///
	/// Includes invocations from `ClientRouter::push`,
	/// `ClientRouter::replace`, and the popstate listener.
	///
	/// Hidden API for testing only. (Refs #4234)
	///
	/// Wasm-only (Fixes #4258): see `__diag_observer_count`.
	#[cfg(wasm)]
	#[doc(hidden)]
	pub fn __diag_dispatch_count(&self) -> u64 {
		self.dispatch_count.get()
	}

	/// Stable per-instance router id for diagnostic correlation.
	///
	/// Returns the pointer of the `Rc` backing `navigation_observers`.
	/// Two `ClientRouter` values share an id iff they share the same
	/// observer list, which only happens within the same logical instance:
	/// the `Rc` is constructed fresh in `ClientRouter::new` and never
	/// reseated.
	///
	/// Hidden API for testing only. (Refs #4234)
	///
	/// Wasm-only (Fixes #4258): see `__diag_observer_count`.
	#[cfg(wasm)]
	#[doc(hidden)]
	pub fn __diag_router_id(&self) -> usize {
		std::rc::Rc::as_ptr(&self.navigation_observers) as usize
	}

	/// Generates a URL by route name with parameters.
	pub fn reverse(&self, name: &str, params: &[(&str, &str)]) -> Result<String, RouterError> {
		let index = self
			.named_routes
			.get(name)
			.ok_or_else(|| RouterError::InvalidRouteName(name.to_string()))?;

		let route = &self.routes[*index];
		let params_map: HashMap<String, String> = params
			.iter()
			.map(|(k, v)| (k.to_string(), v.to_string()))
			.collect();

		route
			.pattern
			.reverse(&params_map)
			.ok_or_else(|| RouterError::MissingParameter("unknown".to_string()))
	}

	/// Renders the current route's component.
	///
	/// Returns the registered `not_found` page when no route matches, or a
	/// default 404 page if no `not_found` handler has been set.
	pub fn render_current(&self) -> Page {
		let path = self.current_path.get();

		if let Some(route_match) = self.match_path(&path) {
			let ctx =
				ParamContext::new(route_match.params.clone(), route_match.param_values.clone());

			match route_match.route.handler.handle(&ctx) {
				Ok(view) => view,
				Err(_err) => self.not_found.as_ref().map(|f| f()).unwrap_or(Page::Empty),
			}
		} else {
			self.not_found.as_ref().map(|f| f()).unwrap_or(Page::Empty)
		}
	}

	/// Returns the number of registered routes.
	pub fn route_count(&self) -> usize {
		self.routes.len()
	}

	/// Checks if a route name exists.
	pub fn has_route(&self, name: &str) -> bool {
		self.named_routes.contains_key(name)
	}

	/// Extract a lightweight, thread-safe URL reverser.
	///
	/// The returned `ClientUrlReverser` contains only the
	/// named-route-to-pattern mapping and can be shared across threads
	/// (unlike `ClientRouter` itself which holds reactive signals).
	pub fn to_reverser(&self) -> super::reverser::ClientUrlReverser {
		let named_patterns = self
			.named_routes
			.iter()
			.map(|(name, &idx)| {
				let pattern = self.routes[idx].pattern.pattern().to_string();
				(name.clone(), pattern)
			})
			.collect();
		super::reverser::ClientUrlReverser::new(named_patterns)
	}

	/// Sets up a popstate event listener for browser back/forward navigation.
	///
	/// This method registers a listener for the browser's `popstate` event,
	/// which fires when the user navigates using the back/forward buttons.
	/// When triggered, it updates the router's reactive signals to reflect
	/// the new URL state.
	///
	/// # WASM Only
	///
	/// This method only has effect on WASM targets. On non-WASM targets,
	/// it's a no-op that always returns `Ok(())`.
	///
	/// # Note
	///
	/// The listener closure is kept alive using `.forget()`, meaning it will
	/// persist for the lifetime of the page. This is intentional for SPA
	/// navigation handling.
	#[cfg(wasm)]
	pub fn setup_history_listener(&self) {
		let path_signal = self.current_path.clone();
		let params_signal = self.current_params.clone();
		let route_name_signal = self.current_route_name.clone();
		let navigation_observers = self.navigation_observers.clone();
		let dispatch_count = self.dispatch_count.clone();

		let closure = setup_popstate_listener(move |path, state| {
			// (Refs #4234, Inv-1, Inv-5) Update Signals first, then notify
			// observers, so listeners that read `Signal::get` from inside
			// their closure see the new state. Mirrors
			// `ClientRouter::navigate`.
			path_signal.set(path.clone());

			let params_for_observers = if let Some(hist_state) = state {
				let params = hist_state.params.clone();
				params_signal.set(hist_state.params);
				route_name_signal.set(hist_state.route_name);
				params
			} else {
				// Clear params when no state is available.
				params_signal.set(HashMap::new());
				route_name_signal.set(None);
				HashMap::new()
			};

			// (Refs #4234, Inv-4, Inv-5, Inv-6) Bump the diagnostic counter
			// and dispatch on_navigate observers via the shared helper so
			// popstate-driven dispatches are counted and ordered
			// identically to push/replace-driven ones.
			dispatch_navigation_observers(
				&navigation_observers,
				&dispatch_count,
				&path,
				&params_for_observers,
			);
		});

		if let Ok(c) = closure {
			// Keep the closure alive for the lifetime of the page
			c.forget();
		}
	}

	/// Non-WASM version of `setup_history_listener`.
	#[cfg(native)]
	pub fn setup_history_listener(&self) {
		// No-op on non-WASM targets
	}
}

/// Snapshot, prune, and invoke navigation observers.
///
/// Bumps `dispatch_count` first so even a no-listener dispatch is
/// counted (Inv-5), then collects strong `Rc` references to live
/// listeners while pruning dead `Weak` entries (Inv-6). The `RefCell`
/// borrow is released before any user-supplied closure runs (Inv-4),
/// which lets listeners call `ClientRouter::push` /
/// `ClientRouter::replace` reentrantly, register new listeners via
/// `on_navigate`, or drop existing `NavigationSubscription` handles
/// without panicking on `RefCell` reentry.
///
/// Used by both `ClientRouter::notify_observers` (programmatic
/// push/replace) and the popstate listener (browser back/forward) so
/// the two code paths stay observably identical. (Refs #4234, Inv-4)
///
/// Wasm-only (Fixes #4258): touches `NavigationObservers` /
/// `dispatch_count` which are themselves wasm-only.
#[cfg(wasm)]
fn dispatch_navigation_observers(
	navigation_observers: &NavigationObservers,
	dispatch_count: &std::rc::Rc<std::cell::Cell<u64>>,
	path: &str,
	params: &HashMap<String, String>,
) {
	dispatch_count.set(dispatch_count.get() + 1);
	let listeners_snapshot: Vec<std::rc::Rc<NavigationListener>> = {
		let mut observers = navigation_observers.borrow_mut();
		observers.retain(|w| w.strong_count() > 0);
		observers.iter().filter_map(|w| w.upgrade()).collect()
	};
	for listener in listeners_snapshot {
		listener(path, params);
	}
}

// (Fixes #4258) Compile-time guard: `ClientRouter` MUST be `Send + Sync`
// on native targets so `UnifiedRouter` (which always contains it) can
// be registered with multi-threaded DI containers. Regression of #4258
// — for example, re-introducing an unguarded `Rc<...>` or `RefCell<...>`
// field — would fail this assertion at native build time.
#[cfg(all(test, native))]
const _: fn() = || {
	fn assert_send_sync<T: Send + Sync>() {}
	assert_send_sync::<ClientRouter>();
};

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::*;

	fn test_page() -> Page {
		Page::Empty
	}

	fn page_with_text(s: &str) -> Page {
		Page::Text(s.to_string().into())
	}

	fn home_page() -> Page {
		page_with_text("Home")
	}

	fn user_page() -> Page {
		page_with_text("User")
	}

	fn not_found_page() -> Page {
		page_with_text("NotFound")
	}

	#[test]
	fn test_route_new() {
		let route = ClientRoute::new("/", test_page);
		assert!(route.name().is_none());
	}

	#[test]
	fn test_route_named() {
		let route = ClientRoute::named("home", "/", test_page);
		assert_eq!(route.name(), Some("home"));
	}

	#[test]
	fn test_router_new() {
		let router = ClientRouter::new();
		assert_eq!(router.route_count(), 0);
	}

	#[test]
	fn test_router_add_route() {
		let router = ClientRouter::new()
			.route("/", home_page)
			.route("/users/", user_page);

		assert_eq!(router.route_count(), 2);
	}

	#[test]
	fn test_router_named_route() {
		let router = ClientRouter::new()
			.named_route("home", "/", home_page)
			.named_route("users", "/users/", user_page);

		assert!(router.has_route("home"));
		assert!(router.has_route("users"));
		assert!(!router.has_route("nonexistent"));
	}

	#[test]
	fn test_router_match_exact() {
		let router = ClientRouter::new()
			.route("/", home_page)
			.route("/users/", user_page);

		assert!(router.match_path("/").is_some());
		assert!(router.match_path("/users/").is_some());
		assert!(router.match_path("/nonexistent/").is_none());
	}

	#[test]
	fn test_router_match_params() {
		let router = ClientRouter::new().route("/users/{id}/", user_page);

		let route_match = router.match_path("/users/42/");
		assert!(route_match.is_some());

		let route_match = route_match.unwrap();
		assert_eq!(route_match.params.get("id"), Some(&"42".to_string()));
	}

	#[test]
	fn test_router_reverse() {
		let router = ClientRouter::new()
			.named_route("home", "/", home_page)
			.named_route("user_detail", "/users/{id}/", user_page);

		assert_eq!(router.reverse("home", &[]).unwrap(), "/");
		assert_eq!(
			router.reverse("user_detail", &[("id", "42")]).unwrap(),
			"/users/42/"
		);
	}

	#[test]
	fn test_router_reverse_invalid_name() {
		let router = ClientRouter::new();
		let result = router.reverse("nonexistent", &[]);
		assert!(matches!(result, Err(RouterError::InvalidRouteName(_))));
	}

	#[test]
	fn test_router_not_found() {
		let router = ClientRouter::new().not_found(not_found_page);

		let _view = router.render_current();
	}

	#[rstest]
	fn test_render_current_returns_page_without_not_found() {
		// Arrange
		let router = ClientRouter::new().route("/home/", home_page);

		// Act — path does not match, no not_found registered
		let page = router.render_current();

		// Assert — returns Page::Empty as default fallback
		assert!(matches!(page, Page::Empty));
	}

	#[test]
	fn test_router_with_guard() {
		let router = ClientRouter::new()
			.guarded_route("/admin/", test_page, |_| false)
			.route("/public/", test_page);

		// Guard rejects
		assert!(router.match_path("/admin/").is_none());
		// No guard
		assert!(router.match_path("/public/").is_some());
	}

	#[test]
	fn test_router_error_display() {
		assert_eq!(
			RouterError::NotFound("/test/".to_string()).to_string(),
			"Route not found: /test/"
		);
		assert_eq!(
			RouterError::InvalidRouteName("test".to_string()).to_string(),
			"Invalid route name: test"
		);
	}

	#[test]
	fn test_router_push_non_wasm() {
		let router = ClientRouter::new()
			.route("/", home_page)
			.route("/users/", user_page);

		// Non-WASM push should succeed
		assert!(router.push("/users/").is_ok());
	}

	#[test]
	fn test_router_replace_non_wasm() {
		let router = ClientRouter::new().route("/", home_page);

		// Non-WASM replace should succeed
		assert!(router.replace("/").is_ok());
	}

	// ============================================================================
	// route_path* tests
	// ============================================================================

	#[test]
	fn test_route_path_single() {
		let router = ClientRouter::new().route_path("/users/{id}/", |Path(_id): Path<i64>| {
			page_with_text("User")
		});

		assert_eq!(router.route_count(), 1);

		// Match and verify handler works
		let route_match = router.match_path("/users/42/");
		assert!(route_match.is_some());

		let route_match = route_match.unwrap();
		assert_eq!(route_match.params.get("id"), Some(&"42".to_string()));
	}

	#[test]
	fn test_route_path2_two_params() {
		let router = ClientRouter::new().route_path2(
			"/users/{user_id}/posts/{post_id}/",
			|Path(_user_id): Path<i64>, Path(_post_id): Path<i64>| page_with_text("UserPost"),
		);

		assert_eq!(router.route_count(), 1);

		let route_match = router.match_path("/users/123/posts/456/");
		assert!(route_match.is_some());

		let route_match = route_match.unwrap();
		assert_eq!(route_match.params.get("user_id"), Some(&"123".to_string()));
		assert_eq!(route_match.params.get("post_id"), Some(&"456".to_string()));
	}

	#[test]
	fn test_route_path3_three_params() {
		let router = ClientRouter::new().route_path3(
			"/orgs/{org_id}/teams/{team_id}/members/{member_id}/",
			|Path(_org_id): Path<String>,
			 Path(_team_id): Path<i64>,
			 Path(_member_id): Path<i64>| page_with_text("Member"),
		);

		assert_eq!(router.route_count(), 1);

		let route_match = router.match_path("/orgs/acme/teams/10/members/100/");
		assert!(route_match.is_some());

		let route_match = route_match.unwrap();
		assert_eq!(route_match.params.get("org_id"), Some(&"acme".to_string()));
		assert_eq!(route_match.params.get("team_id"), Some(&"10".to_string()));
		assert_eq!(
			route_match.params.get("member_id"),
			Some(&"100".to_string())
		);
	}

	#[test]
	fn test_named_route_path() {
		let router = ClientRouter::new().named_route_path(
			"user_detail",
			"/users/{id}/",
			|Path(_id): Path<i64>| page_with_text("User"),
		);

		assert!(router.has_route("user_detail"));
		assert_eq!(
			router.reverse("user_detail", &[("id", "42")]).unwrap(),
			"/users/42/"
		);
	}

	#[test]
	fn test_named_route_path2() {
		let router = ClientRouter::new().named_route_path2(
			"user_post",
			"/users/{user_id}/posts/{post_id}/",
			|Path(_user_id): Path<i64>, Path(_post_id): Path<i64>| page_with_text("UserPost"),
		);

		assert!(router.has_route("user_post"));
		assert_eq!(
			router
				.reverse("user_post", &[("user_id", "10"), ("post_id", "20")])
				.unwrap(),
			"/users/10/posts/20/"
		);
	}

	#[test]
	fn test_named_route_path3() {
		let router = ClientRouter::new().named_route_path3(
			"org_team_member",
			"/orgs/{org}/teams/{team}/members/{member}/",
			|Path(_org): Path<String>, Path(_team): Path<i64>, Path(_member): Path<i64>| {
				page_with_text("Member")
			},
		);

		assert!(router.has_route("org_team_member"));
		assert_eq!(
			router
				.reverse(
					"org_team_member",
					&[("org", "acme"), ("team", "5"), ("member", "42")]
				)
				.unwrap(),
			"/orgs/acme/teams/5/members/42/"
		);
	}

	#[test]
	fn test_route_path_with_string_param() {
		let router = ClientRouter::new()
			.route_path("/posts/{slug}/", |Path(_slug): Path<String>| {
				page_with_text("Post")
			});

		let route_match = router.match_path("/posts/hello-world/");
		assert!(route_match.is_some());
		assert_eq!(
			route_match.unwrap().params.get("slug"),
			Some(&"hello-world".to_string())
		);
	}
}
