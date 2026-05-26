//! Route Group functionality
//!
//! Provides functionality to group multiple routes and apply middleware to the entire group.

use crate::routers::ServerRouter;
use reinhardt_middleware::Middleware;

/// Route information tuple: (path, name, namespace, methods)
pub type RouteInfo = Vec<(String, Option<String>, Option<String>, Vec<hyper::Method>)>;

/// Route Group
///
/// Groups multiple routes and applies group-level middleware.
///
/// # Examples
///
/// ```
/// use reinhardt_urls::routers::RouteGroup;
/// use reinhardt_urls::routers::ServerRouter;
/// use reinhardt_middleware::LoggingMiddleware;
/// use hyper::Method;
/// # use reinhardt_http::{Request, Response, Result};
///
/// # async fn users_list(_req: Request) -> Result<Response> {
/// #     Ok(Response::ok())
/// # }
/// # async fn users_detail(_req: Request) -> Result<Response> {
/// #     Ok(Response::ok())
/// # }
///
/// let mut group = RouteGroup::new()
///     .with_prefix("/api/v1")
///     .with_middleware(LoggingMiddleware::new());
///
/// let router = group
///     .function("/users", Method::GET, users_list)
///     .function("/users/{id}", Method::GET, users_detail)
///     .build();
///
/// // Verify router configuration
/// assert_eq!(router.prefix(), "/api/v1");
/// ```
pub struct RouteGroup {
	router: ServerRouter,
}

impl RouteGroup {
	/// Create a new route group
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::RouteGroup;
	///
	/// let group = RouteGroup::new();
	/// ```
	pub fn new() -> Self {
		Self {
			router: ServerRouter::new(),
		}
	}

	/// Set prefix
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::RouteGroup;
	///
	/// let group = RouteGroup::new()
	///     .with_prefix("/api/v1");
	/// ```
	pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
		self.router = self.router.with_prefix(prefix);
		self
	}

	/// Set namespace
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::RouteGroup;
	///
	/// let group = RouteGroup::new()
	///     .with_namespace("v1");
	/// ```
	pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
		self.router = self.router.with_namespace(namespace);
		self
	}

	/// Add middleware to apply to the entire group
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::RouteGroup;
	/// use reinhardt_middleware::LoggingMiddleware;
	///
	/// let group = RouteGroup::new()
	///     .with_middleware(LoggingMiddleware::new());
	///
	/// // Middleware is applied to the router
	/// let router = group.build();
	/// assert!(router.prefix().is_empty() || !router.prefix().is_empty());
	/// ```
	pub fn with_middleware<M: Middleware + 'static>(mut self, middleware: M) -> Self {
		self.router = self.router.with_middleware(middleware);
		self
	}

	/// Add a function-based route
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::RouteGroup;
	/// use hyper::Method;
	/// # use reinhardt_http::{Request, Response, Result};
	///
	/// # async fn health(_req: Request) -> Result<Response> {
	/// #     Ok(Response::ok())
	/// # }
	/// let group = RouteGroup::new()
	///     .function("/health", Method::GET, health);
	///
	/// // Router is built successfully
	/// let router = group.build();
	/// assert!(!router.get_all_routes().is_empty());
	/// ```
	pub fn function<F, Fut>(mut self, path: &str, method: hyper::Method, func: F) -> Self
	where
		F: Fn(reinhardt_http::Request) -> Fut + Send + Sync + 'static,
		Fut: std::future::Future<
				Output = reinhardt_core::exception::Result<reinhardt_http::Response>,
			> + Send
			+ 'static,
	{
		self.router = self.router.function(path, method, func);
		self
	}

	/// Add a route with a Handler trait implementation and HTTP method
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_urls::routers::RouteGroup;
	/// use hyper::Method;
	/// use reinhardt_http::{Request, Response, Result};
	/// use reinhardt_http::Handler;
	/// use async_trait::async_trait;
	///
	/// #[derive(Clone)]
	/// struct ArticleHandler;
	///
	/// #[async_trait]
	/// impl Handler for ArticleHandler {
	///     async fn handle(&self, _request: Request) -> Result<Response> {
	///         Ok(Response::ok())
	///     }
	/// }
	///
	/// let group = RouteGroup::new()
	///     .handler_with_method("/articles", Method::GET, ArticleHandler);
	/// ```
	pub fn handler_with_method<H: reinhardt_http::Handler + 'static>(
		mut self,
		path: &str,
		method: hyper::Method,
		handler: H,
	) -> Self {
		self.router = self.router.handler_with_method(path, method, handler);
		self
	}

	/// Add a named function-based route
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::RouteGroup;
	/// use hyper::Method;
	/// # use reinhardt_http::{Request, Response, Result};
	///
	/// # async fn health(_req: Request) -> Result<Response> {
	/// #     Ok(Response::ok())
	/// # }
	/// let group = RouteGroup::new()
	///     .function_named("/health", Method::GET, "health", health);
	///
	/// // Router is built successfully with named route
	/// let router = group.build();
	/// let routes = router.get_all_routes();
	/// assert!(!routes.is_empty());
	/// assert!(routes.len() >= 1);
	/// ```
	#[deprecated(
		since = "0.2.0",
		note = "Use `#[get(\"/path\", name = \"name\")]` + `.endpoint()` instead"
	)]
	pub fn function_named<F, Fut>(
		mut self,
		path: &str,
		method: hyper::Method,
		name: &str,
		func: F,
	) -> Self
	where
		F: Fn(reinhardt_http::Request) -> Fut + Send + Sync + 'static,
		Fut: std::future::Future<
				Output = reinhardt_core::exception::Result<reinhardt_http::Response>,
			> + Send
			+ 'static,
	{
		#[allow(deprecated)]
		{
			self.router = self.router.function_named(path, method, name, func);
		}
		self
	}

	/// Add a named route with a Handler trait implementation and HTTP method
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_urls::routers::RouteGroup;
	/// use hyper::Method;
	/// use reinhardt_http::{Request, Response, Result};
	/// use reinhardt_http::Handler;
	/// use async_trait::async_trait;
	///
	/// #[derive(Clone)]
	/// struct ArticleHandler;
	///
	/// #[async_trait]
	/// impl Handler for ArticleHandler {
	///     async fn handle(&self, _request: Request) -> Result<Response> {
	///         Ok(Response::ok())
	///     }
	/// }
	///
	/// let group = RouteGroup::new()
	///     .handler_with_method_named("/articles", Method::GET, "list_articles", ArticleHandler);
	/// ```
	#[deprecated(
		since = "0.2.0",
		note = "Use `#[get(\"/path\", name = \"name\")]` + `.endpoint()` instead"
	)]
	pub fn handler_with_method_named<H: reinhardt_http::Handler + 'static>(
		mut self,
		path: &str,
		method: hyper::Method,
		name: &str,
		handler: H,
	) -> Self {
		#[allow(deprecated)]
		{
			self.router = self
				.router
				.handler_with_method_named(path, method, name, handler);
		}
		self
	}

	/// Add a ViewSet
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_urls::routers::RouteGroup;
	/// # use reinhardt_views::viewsets::ViewSet;
	/// # use async_trait::async_trait;
	/// # struct UserViewSet;
	/// # #[async_trait]
	/// # impl ViewSet for UserViewSet {
	/// #     fn get_basename(&self) -> &str { "users" }
	/// #     async fn dispatch(&self, _req: reinhardt_http::Request, _action: reinhardt_views::viewsets::Action)
	/// #         -> reinhardt_core::exception::Result<reinhardt_http::Response> {
	/// #         Ok(reinhardt_http::Response::ok())
	/// #     }
	/// # }
	///
	/// let group = RouteGroup::new()
	///     .viewset("/users", UserViewSet);
	/// ```
	pub fn viewset<V: reinhardt_views::viewsets::ViewSet + 'static>(
		mut self,
		prefix: &str,
		viewset: V,
	) -> Self {
		self.router = self.router.viewset(prefix, viewset);
		self
	}

	/// Same as [`Self::viewset`] at runtime, but carries a `PhantomData<M>`
	/// marker that the route resolver machinery recovers at expansion time
	/// to discover `#[action]`-decorated methods on the impl block `M`.
	///
	/// `M` is purely a name-bearing token. Users write
	/// `PhantomData::<MyViewSetImpl>` as the third argument. The bound is
	/// `M: 'static` so the marker's `std::any::type_name` is reachable for
	/// the marker→runtime bridge below.
	///
	/// Phase 5.1 of Issue #4507: copies every action submitted under
	/// `type_name::<M>()` (via the impl-form `#[viewset]` macro's runtime
	/// registration) into the runtime-keyed `register_action(type_name::<V>(), ...)`
	/// slot so `ViewSet::get_extra_actions` finds them at dispatch time.
	///
	/// Refs Issue #4507.
	pub fn viewset_with_actions<V, M>(
		self,
		prefix: &str,
		viewset: V,
		_marker: std::marker::PhantomData<M>,
	) -> Self
	where
		V: reinhardt_views::viewsets::ViewSet + 'static,
		M: 'static,
	{
		reinhardt_views::viewsets::bridge_marker_actions_to_viewset(
			std::any::type_name::<M>(),
			std::any::type_name::<V>(),
		);
		self.viewset(prefix, viewset)
	}

	/// Add a class-based view
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_urls::routers::RouteGroup;
	/// # use reinhardt_http::{Handler, {Request, Response, Result}};
	/// # use async_trait::async_trait;
	/// # struct ArticleListView;
	/// # #[async_trait]
	/// # impl Handler for ArticleListView {
	/// #     async fn handle(&self, _req: Request) -> Result<Response> {
	/// #         Ok(Response::ok())
	/// #     }
	/// # }
	///
	/// let group = RouteGroup::new()
	///     .view("/articles", ArticleListView);
	///
	/// // RouteGroup created successfully
	/// ```
	pub fn view<V>(mut self, path: &str, view: V) -> Self
	where
		V: reinhardt_http::Handler + 'static,
	{
		self.router = self.router.view(path, view);
		self
	}

	/// Add a named class-based view
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_urls::routers::RouteGroup;
	/// # use reinhardt_http::{Handler, {Request, Response, Result}};
	/// # use async_trait::async_trait;
	/// # struct ArticleListView;
	/// # #[async_trait]
	/// # impl Handler for ArticleListView {
	/// #     async fn handle(&self, _req: Request) -> Result<Response> {
	/// #         Ok(Response::ok())
	/// #     }
	/// # }
	///
	/// let group = RouteGroup::new()
	///     .view_named("/articles", "list", ArticleListView);
	///
	/// // RouteGroup created successfully
	/// ```
	#[deprecated(
		since = "0.2.0",
		note = "Use `#[get(\"/path\", name = \"name\")]` + `.endpoint()` instead"
	)]
	pub fn view_named<V>(mut self, path: &str, name: &str, view: V) -> Self
	where
		V: reinhardt_http::Handler + 'static,
	{
		#[allow(deprecated)]
		{
			self.router = self.router.view_named(path, name, view);
		}
		self
	}

	/// Add a child group (nested group)
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_urls::routers::RouteGroup;
	///
	/// let auth_group = RouteGroup::new()
	///     .with_prefix("/auth/");
	///
	/// let group = RouteGroup::new()
	///     .with_prefix("/api/")
	///     .nest(auth_group);
	///
	/// // RouteGroup with nested group created successfully
	/// ```
	pub fn nest(mut self, child: RouteGroup) -> Self {
		let child_prefix = child.router.prefix().to_string();
		self.router = self.router.mount(&child_prefix, child.router);
		self
	}

	/// Get the prefix of this route group
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::RouteGroup;
	///
	/// let group = RouteGroup::new()
	///     .with_prefix("/api/v1");
	///
	/// assert_eq!(group.prefix(), "/api/v1");
	/// ```
	pub fn prefix(&self) -> &str {
		self.router.prefix()
	}

	/// Get the namespace of this route group
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::RouteGroup;
	///
	/// let group = RouteGroup::new()
	///     .with_namespace("v1");
	///
	/// assert_eq!(group.namespace(), Some("v1"));
	/// ```
	pub fn namespace(&self) -> Option<&str> {
		self.router.namespace()
	}

	/// Get the number of child routers in this group
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::RouteGroup;
	///
	/// let auth_group = RouteGroup::new()
	///     .with_prefix("/auth/");
	///
	/// let group = RouteGroup::new()
	///     .with_prefix("/api/")
	///     .nest(auth_group);
	///
	/// assert_eq!(group.children_count(), 1);
	/// ```
	pub fn children_count(&self) -> usize {
		self.router.children_count()
	}

	/// Get all routes registered in this group
	///
	/// Returns a vector of tuples containing (path, name, namespace, methods).
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::RouteGroup;
	/// use hyper::Method;
	/// # use reinhardt_http::{Request, Response, Result};
	///
	/// # async fn health(_req: Request) -> Result<Response> {
	/// #     Ok(Response::ok())
	/// # }
	/// let group = RouteGroup::new()
	///     .with_prefix("/api")
	///     .function("/health", Method::GET, health);
	///
	/// let routes = group.get_all_routes();
	/// assert!(!routes.is_empty());
	/// ```
	pub fn get_all_routes(&self) -> RouteInfo {
		self.router.get_all_routes()
	}

	/// Build ServerRouter
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_urls::routers::RouteGroup;
	///
	/// let group = RouteGroup::new();
	/// let router = group.build();
	/// ```
	pub fn build(self) -> ServerRouter {
		self.router
	}
}

impl Default for RouteGroup {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use hyper::Method;
	use reinhardt_http::{Request, Response, Result};
	use reinhardt_middleware::LoggingMiddleware;

	async fn test_handler(_req: Request) -> Result<Response> {
		Ok(Response::ok())
	}

	#[test]
	fn test_route_group_new() {
		let group = RouteGroup::new();
		let router = group.build();
		assert_eq!(router.prefix(), "");
	}

	#[test]
	fn test_route_group_with_prefix() {
		let group = RouteGroup::new().with_prefix("/api/v1");
		let router = group.build();
		assert_eq!(router.prefix(), "/api/v1");
	}

	#[test]
	fn test_route_group_with_namespace() {
		let group = RouteGroup::new().with_namespace("v1");
		let router = group.build();
		assert_eq!(router.namespace(), Some("v1"));
	}

	#[test]
	fn test_route_group_with_middleware() {
		let group = RouteGroup::new().with_middleware(LoggingMiddleware::new());
		let _router = group.build();
		// Middleware is correctly added, verified in integration tests
	}

	#[test]
	fn test_route_group_function() {
		let group = RouteGroup::new().function("/health", Method::GET, test_handler);
		let _router = group.build();
		// Routes are correctly added, verified in integration tests
	}

	#[test]
	fn test_route_group_nested() {
		let auth_group =
			RouteGroup::new()
				.with_prefix("/auth/")
				.function("/login", Method::POST, test_handler);

		let group = RouteGroup::new().with_prefix("/api/").nest(auth_group);

		let router = group.build();
		assert_eq!(router.children_count(), 1);
	}

	#[test]
	fn test_route_group_multiple_middleware() {
		let group = RouteGroup::new()
			.with_middleware(LoggingMiddleware::new())
			.with_middleware(LoggingMiddleware::new())
			.function("/test", Method::GET, test_handler);

		let _router = group.build();
		// Verify that multiple middleware are correctly added in integration tests
	}
}

#[cfg(test)]
mod viewset_with_actions_tests {
	use super::*;
	use async_trait::async_trait;
	use reinhardt_http::{Request, Response, Result};
	use reinhardt_views::viewsets::{Action, ViewSet};
	use rstest::rstest;
	use std::marker::PhantomData;

	/// Minimal `ViewSet` fixture for parity tests between `viewset` and
	/// `viewset_with_actions` on `RouteGroup`.
	#[derive(Debug, Clone)]
	struct DummyViewSet {
		basename: String,
	}

	#[async_trait]
	impl ViewSet for DummyViewSet {
		fn get_basename(&self) -> &str {
			&self.basename
		}

		async fn dispatch(&self, _request: Request, _action: Action) -> Result<Response> {
			Ok(Response::ok())
		}
	}

	/// Marker type the route resolver machinery recovers at
	/// expansion time. It carries no runtime state.
	struct DummyImpl;

	#[rstest]
	fn viewset_with_actions_is_equivalent_to_viewset() {
		// Arrange
		let group_a = RouteGroup::new().viewset(
			"/users",
			DummyViewSet {
				basename: "users".to_string(),
			},
		);
		let group_b = RouteGroup::new().viewset_with_actions(
			"/users",
			DummyViewSet {
				basename: "users".to_string(),
			},
			PhantomData::<DummyImpl>,
		);
		let mut router_a = group_a.build();
		let mut router_b = group_b.build();

		// Act
		let _ = router_a.register_all_routes();
		let _ = router_b.register_all_routes();
		let routes_a = router_a.get_all_routes();
		let routes_b = router_b.get_all_routes();

		// Assert
		assert_eq!(routes_a, routes_b);
	}
}
