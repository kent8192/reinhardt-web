//! Core Router Implementation.
//!
//! This module provides the main Router struct and routing logic.

use super::handler::{RouteHandler, no_params_handler, result_handler, with_params_handler};
#[cfg(wasm)]
use super::history::setup_popstate_listener;
use super::history::{HistoryState, NavigationType, current_path, push_state, replace_state};
use super::params::{FromPath, ParamContext, PathParams};
use super::pattern::PathPattern;
use crate::component::Page;
use crate::reactive::Signal;
use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for route guard functions.
pub(super) type RouteGuard = Arc<dyn Fn(&RouteMatch) -> bool + Send + Sync>;

/// Error type for path parameter extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
	/// Failed to parse a parameter value.
	ParseError {
		/// Index of the parameter that failed to parse.
		param_index: Option<usize>,
		/// Expected type name.
		param_type: &'static str,
		/// Raw string value that failed to parse.
		raw_value: String,
		/// Error message from parsing.
		source: String,
	},
	/// Parameter count mismatch.
	CountMismatch {
		/// Expected number of parameters.
		expected: usize,
		/// Actual number of parameters.
		actual: usize,
	},
	/// Custom error message.
	Custom(String),
}

impl std::fmt::Display for PathError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::ParseError {
				param_index,
				param_type,
				raw_value,
				source,
			} => {
				if let Some(idx) = param_index {
					write!(
						f,
						"Failed to parse parameter[{}] '{}' as {}: {}",
						idx, raw_value, param_type, source
					)
				} else {
					write!(
						f,
						"Failed to parse parameter '{}' as {}: {}",
						raw_value, param_type, source
					)
				}
			}
			Self::CountMismatch { expected, actual } => {
				write!(
					f,
					"Parameter count mismatch: expected {}, got {}",
					expected, actual
				)
			}
			Self::Custom(msg) => write!(f, "{}", msg),
		}
	}
}

impl std::error::Error for PathError {}

/// Error type for router operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterError {
	/// Route not found.
	NotFound(String),
	/// Invalid route name.
	InvalidRouteName(String),
	/// Missing parameter for reverse URL.
	MissingParameter(String),
	/// Navigation failed.
	NavigationFailed(String),
	/// Path parameter extraction failed.
	PathExtraction(PathError),
}

impl std::fmt::Display for RouterError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::NotFound(path) => write!(f, "Route not found: {}", path),
			Self::InvalidRouteName(name) => write!(f, "Invalid route name: {}", name),
			Self::MissingParameter(param) => write!(f, "Missing parameter: {}", param),
			Self::NavigationFailed(msg) => write!(f, "Navigation failed: {}", msg),
			Self::PathExtraction(err) => write!(f, "Path extraction error: {}", err),
		}
	}
}

impl std::error::Error for RouterError {}

/// A matched route with extracted parameters.
#[derive(Debug, Clone)]
pub struct RouteMatch {
	/// The matched route.
	pub route: Route,
	/// Extracted path parameters.
	pub params: HashMap<String, String>,
	/// Parameter values in the order they appear in the pattern.
	///
	/// This guarantees that tuple extraction works correctly by index,
	/// matching the order of parameters in the URL pattern.
	pub(crate) param_values: Vec<String>,
}

/// A single route definition.
#[derive(Clone)]
pub struct Route {
	/// The path pattern.
	pattern: PathPattern,
	/// Optional route name for reverse lookups.
	name: Option<String>,
	/// The route handler.
	handler: Arc<dyn RouteHandler>,
	/// Optional guard function.
	guard: Option<RouteGuard>,
}

impl std::fmt::Debug for Route {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Route")
			.field("pattern", &self.pattern)
			.field("name", &self.name)
			.field("has_guard", &self.guard.is_some())
			.finish()
	}
}

impl Route {
	/// Creates a new route.
	pub fn new<F>(pattern: &str, component: F) -> Self
	where
		F: Fn() -> Page + Send + Sync + 'static,
	{
		Self {
			pattern: PathPattern::new(pattern),
			name: None,
			handler: no_params_handler(component),
			guard: None,
		}
	}

	/// Creates a named route.
	pub fn named<F>(name: impl Into<String>, pattern: &str, component: F) -> Self
	where
		F: Fn() -> Page + Send + Sync + 'static,
	{
		Self {
			pattern: PathPattern::new(pattern),
			name: Some(name.into()),
			handler: no_params_handler(component),
			guard: None,
		}
	}

	/// Adds a guard to this route.
	pub fn with_guard<G>(mut self, guard: G) -> Self
	where
		G: Fn(&RouteMatch) -> bool + Send + Sync + 'static,
	{
		self.guard = Some(Arc::new(guard));
		self
	}

	/// Returns the route name.
	pub fn name(&self) -> Option<&str> {
		self.name.as_deref()
	}

	/// Returns the pattern.
	pub fn pattern(&self) -> &PathPattern {
		&self.pattern
	}

	/// Checks if the guard allows access.
	pub fn check_guard(&self, route_match: &RouteMatch) -> bool {
		self.guard.as_ref().map(|g| g(route_match)).unwrap_or(true)
	}
}

/// The main router.
pub struct Router {
	/// Registered routes.
	routes: Vec<Route>,
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
	/// Navigation observers registered via `on_navigate`.
	///
	/// Inspired by React Router's `router.subscribe(listener)` design:
	/// listeners are registered explicitly and notified on every successful
	/// navigation, independent of any reactive Effect / Signal tracking.
	/// Refs #4088, #4075, #3348.
	navigation_observers: NavigationObservers,
}

/// Type alias for the navigation observer storage.
///
/// `Rc<RefCell<...>>` because Routers are not `Send` on wasm32 anyway,
/// and the borrow is released before listeners run (see `navigate`).
type NavigationObservers = std::rc::Rc<std::cell::RefCell<Vec<std::rc::Weak<NavigationListener>>>>;

/// Boxed closure stored behind a `Weak<...>` so a dropped `NavigationSubscription`
/// drops its strong `Rc`, after which `navigate` filters out the dead `Weak`.
type NavigationListener = dyn Fn(&str, &HashMap<String, String>) + 'static;

/// RAII handle returned by [`Router::on_navigate`].
///
/// While alive, the registered listener fires on every [`Router::push`] /
/// [`Router::replace`]. Dropping this handle removes the listener (no
/// explicit `unsubscribe` call needed).
pub struct NavigationSubscription {
	#[allow(dead_code)] // Dropped automatically; presence keeps the Weak alive.
	listener: std::rc::Rc<NavigationListener>,
}

impl std::fmt::Debug for Router {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Router")
			.field("routes_count", &self.routes.len())
			.field(
				"named_routes",
				&self.named_routes.keys().collect::<Vec<_>>(),
			)
			.finish()
	}
}

impl Default for Router {
	fn default() -> Self {
		Self::new()
	}
}

impl Router {
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
			navigation_observers: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
		}
	}

	/// Adds a route to the router.
	pub fn route<F>(mut self, pattern: &str, component: F) -> Self
	where
		F: Fn() -> Page + Send + Sync + 'static,
	{
		self.routes.push(Route::new(pattern, component));
		self
	}

	/// Adds a named route to the router.
	pub fn named_route<F>(mut self, name: &str, pattern: &str, component: F) -> Self
	where
		F: Fn() -> Page + Send + Sync + 'static,
	{
		let index = self.routes.len();
		self.routes.push(Route::named(name, pattern, component));
		self.named_routes.insert(name.to_string(), index);
		self
	}

	/// Adds a route with typed path parameters.
	///
	/// # Example
	///
	/// ```ignore
	/// use reinhardt_pages::router::{Router, PathParams};
	///
	/// let router = Router::new()
	///     .route_params("/users/{id}/", |PathParams(id): PathParams<i64>| {
	///         Page::text(format!("User ID: {}", id))
	///     });
	/// ```
	pub fn route_params<F, T>(mut self, pattern: &str, handler: F) -> Self
	where
		F: Fn(PathParams<T>) -> Page + Send + Sync + 'static,
		T: FromPath + Send + Sync + 'static,
	{
		self.routes.push(Route {
			pattern: PathPattern::new(pattern),
			name: None,
			handler: with_params_handler(handler),
			guard: None,
		});
		self
	}

	/// Adds a named route with typed path parameters.
	pub fn named_route_params<F, T>(mut self, name: &str, pattern: &str, handler: F) -> Self
	where
		F: Fn(PathParams<T>) -> Page + Send + Sync + 'static,
		T: FromPath + Send + Sync + 'static,
	{
		let index = self.routes.len();
		self.routes.push(Route {
			pattern: PathPattern::new(pattern),
			name: Some(name.to_string()),
			handler: with_params_handler(handler),
			guard: None,
		});
		self.named_routes.insert(name.to_string(), index);
		self
	}

	/// Adds a route with typed path parameters that returns a Result.
	///
	/// # Example
	///
	/// ```ignore
	/// use reinhardt_pages::router::{Router, PathParams, RouterError};
	///
	/// let router = Router::new()
	///     .route_result("/users/{id}/", |PathParams(id): PathParams<i64>| {
	///         if id > 0 {
	///             Ok(Page::text(format!("User ID: {}", id)))
	///         } else {
	///             Err(RouterError::NotFound("Invalid ID".to_string()))
	///         }
	///     });
	/// ```
	pub fn route_result<F, T, E>(mut self, pattern: &str, handler: F) -> Self
	where
		F: Fn(PathParams<T>) -> Result<Page, E> + Send + Sync + 'static,
		T: FromPath + Send + Sync + 'static,
		E: Into<RouterError> + Send + Sync + 'static,
	{
		self.routes.push(Route {
			pattern: PathPattern::new(pattern),
			name: None,
			handler: result_handler(handler),
			guard: None,
		});
		self
	}

	/// Adds a named route with typed path parameters that returns a Result.
	pub fn named_route_result<F, T, E>(mut self, name: &str, pattern: &str, handler: F) -> Self
	where
		F: Fn(PathParams<T>) -> Result<Page, E> + Send + Sync + 'static,
		T: FromPath + Send + Sync + 'static,
		E: Into<RouterError> + Send + Sync + 'static,
	{
		let index = self.routes.len();
		self.routes.push(Route {
			pattern: PathPattern::new(pattern),
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
		G: Fn(&RouteMatch) -> bool + Send + Sync + 'static,
	{
		self.routes
			.push(Route::new(pattern, component).with_guard(guard));
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
	pub fn match_path(&self, path: &str) -> Option<RouteMatch> {
		for route in &self.routes {
			if let Some((params, param_values)) = route.pattern.matches(path) {
				let route_match = RouteMatch {
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

	/// Registers a navigation observer.
	///
	/// Inspired by React Router's `router.subscribe(listener)` design.
	/// The listener is invoked synchronously on every successful navigation
	/// ([`Self::push`] or [`Self::replace`]) with the new path and matched
	/// params. The listener fires **after** the path / params Signals have
	/// been updated, so calling `Signal::get` from within the listener
	/// returns the new values.
	///
	/// Returns a [`NavigationSubscription`] handle. Drop the handle to
	/// unregister the listener. The router itself does not retain a strong
	/// reference; if all subscriptions are dropped, the listener is freed
	/// at the next `navigate` call (see `navigate` body which prunes dead
	/// Weak references on every invocation).
	///
	/// Robust against nested reactive nodes spawned during view rendering
	/// because this subscription is independent of the Effect / Signal
	/// auto-tracking system.
	///
	/// Refs #4088, #4075, #3348.
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

		// Invoke registered navigation observers AFTER signal updates so
		// listeners reading `Signal::get` from inside their closure see the
		// new state. Refs #4088.
		let params_for_observers = route_match
			.as_ref()
			.map(|m| m.params.clone())
			.unwrap_or_default();
		// Snapshot strong refs before invoking listeners; this avoids
		// holding the `RefCell` borrow across user-supplied closure code
		// (which could re-enter `on_navigate` and panic on RefCell reentry).
		let listeners_snapshot: Vec<std::rc::Rc<NavigationListener>> = {
			let mut observers = self.navigation_observers.borrow_mut();
			// Prune dropped subscriptions on every navigate.
			observers.retain(|w| w.strong_count() > 0);
			observers.iter().filter_map(|w| w.upgrade()).collect()
		};
		for listener in listeners_snapshot {
			listener(path, &params_for_observers);
		}

		Ok(())
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
	pub fn render_current(&self) -> Page {
		let path = self.current_path.get();

		if let Some(route_match) = self.match_path(&path) {
			let ctx =
				ParamContext::new(route_match.params.clone(), route_match.param_values.clone());

			match route_match.route.handler.handle(&ctx) {
				Ok(view) => view,
				Err(err) => Page::text(format!("Error: {}", err)),
			}
		} else if let Some(not_found) = &self.not_found {
			not_found()
		} else {
			Page::Empty
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
	/// # Example
	///
	/// ```no_run
	/// let router = Router::new()
	///     .route("/", home_page)
	///     .route("/users/{id}/", user_detail);
	///
	/// // Call after routes are configured
	/// router.setup_history_listener();
	/// ```
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

		let closure = setup_popstate_listener(move |path, state| {
			// Update path signal
			path_signal.set(path);

			// Update params and route name from history state if available
			if let Some(hist_state) = state {
				params_signal.set(hist_state.params);
				route_name_signal.set(hist_state.route_name);
			} else {
				// Clear params when no state is available
				params_signal.set(HashMap::new());
				route_name_signal.set(None);
			}
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

#[cfg(test)]
mod tests {
	use super::*;

	fn test_view() -> Page {
		Page::text("Test")
	}

	fn home_view() -> Page {
		Page::text("Home")
	}

	fn user_view() -> Page {
		Page::text("User")
	}

	fn not_found_view() -> Page {
		Page::text("404")
	}

	#[test]
	fn test_route_new() {
		let route = Route::new("/", test_view);
		assert!(route.name().is_none());
	}

	#[test]
	fn test_route_named() {
		let route = Route::named("home", "/", test_view);
		assert_eq!(route.name(), Some("home"));
	}

	#[test]
	fn test_router_new() {
		let router = Router::new();
		assert_eq!(router.route_count(), 0);
	}

	#[test]
	fn test_router_add_route() {
		let router = Router::new()
			.route("/", home_view)
			.route("/users/", user_view);

		assert_eq!(router.route_count(), 2);
	}

	#[test]
	fn test_router_named_route() {
		let router = Router::new()
			.named_route("home", "/", home_view)
			.named_route("users", "/users/", user_view);

		assert!(router.has_route("home"));
		assert!(router.has_route("users"));
		assert!(!router.has_route("nonexistent"));
	}

	#[test]
	fn test_router_match_exact() {
		let router = Router::new()
			.route("/", home_view)
			.route("/users/", user_view);

		assert!(router.match_path("/").is_some());
		assert!(router.match_path("/users/").is_some());
		assert!(router.match_path("/nonexistent/").is_none());
	}

	#[test]
	fn test_router_match_params() {
		let router = Router::new().route("/users/{id}/", user_view);

		let route_match = router.match_path("/users/42/");
		assert!(route_match.is_some());

		let route_match = route_match.unwrap();
		assert_eq!(route_match.params.get("id"), Some(&"42".to_string()));
	}

	#[test]
	fn test_router_reverse() {
		let router = Router::new()
			.named_route("home", "/", home_view)
			.named_route("user_detail", "/users/{id}/", user_view);

		assert_eq!(router.reverse("home", &[]).unwrap(), "/");
		assert_eq!(
			router.reverse("user_detail", &[("id", "42")]).unwrap(),
			"/users/42/"
		);
	}

	#[test]
	fn test_router_reverse_invalid_name() {
		let router = Router::new();
		let result = router.reverse("nonexistent", &[]);
		assert!(matches!(result, Err(RouterError::InvalidRouteName(_))));
	}

	#[test]
	fn test_router_not_found() {
		let router = Router::new().not_found(not_found_view);

		let view = router.render_current();
		let html = view.render_to_string();
		assert_eq!(html, "404");
	}

	#[test]
	fn test_router_with_guard() {
		let router = Router::new()
			.guarded_route("/admin/", test_view, |_| false)
			.route("/public/", test_view);

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
		let router = Router::new()
			.route("/", home_view)
			.route("/users/", user_view);

		// Non-WASM push should succeed
		assert!(router.push("/users/").is_ok());
	}

	#[test]
	fn test_router_replace_non_wasm() {
		let router = Router::new().route("/", home_view);

		// Non-WASM replace should succeed
		assert!(router.replace("/").is_ok());
	}

	use std::cell::RefCell;
	use std::rc::Rc;

	#[test]
	#[serial_test::serial]
	fn on_navigate_fires_listener_after_router_push() {
		// Arrange
		let calls: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
		let calls_clone = calls.clone();
		let router = Router::new().route("/", home_view).route("/foo", user_view);
		let _sub = router.on_navigate(
			move |path: &str, _params: &std::collections::HashMap<String, String>| {
				calls_clone.borrow_mut().push(path.to_string());
			},
		);

		// Act
		router.push("/foo").expect("push /foo");

		// Assert
		assert_eq!(*calls.borrow(), vec!["/foo".to_string()]);
	}

	#[test]
	#[serial_test::serial]
	fn on_navigate_subscription_drop_unregisters_listener() {
		// Arrange
		let calls = Rc::new(RefCell::new(0_usize));
		let calls_clone = calls.clone();
		let router = Router::new().route("/", home_view).route("/foo", user_view);
		let sub = router.on_navigate(move |_p, _ps| {
			*calls_clone.borrow_mut() += 1;
		});

		// Act
		router.push("/foo").expect("push /foo");
		drop(sub);
		router.push("/").expect("push /");

		// Assert: listener fired exactly once (before drop), not twice
		assert_eq!(*calls.borrow(), 1);
	}
}
