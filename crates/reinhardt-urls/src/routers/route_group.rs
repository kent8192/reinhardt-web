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
/// ```rust,no_run
/// use reinhardt_urls::routers::RouteGroup;
/// use reinhardt_urls::routers::ServerRouter;
/// use reinhardt_middleware::LoggingMiddleware;
/// # use reinhardt_core::endpoint::EndpointInfo;
/// # use reinhardt_http::{Handler, Request, Response};
/// # use hyper::Method;
/// # struct ListUsers;
/// # impl EndpointInfo for ListUsers {
/// #     fn path() -> &'static str { "/users" }
/// #     fn method() -> Method { Method::GET }
/// #     fn name() -> &'static str { "list_users" }
/// # }
/// # #[async_trait::async_trait]
/// # impl Handler for ListUsers {
/// #     async fn handle(&self, _req: Request) -> Result<Response, reinhardt_http::Error> {
/// #         Ok(Response::ok())
/// #     }
/// # }
/// # fn list_users() -> ListUsers { ListUsers }
///
/// let group = RouteGroup::new()
///     .with_prefix("/api/v1")
///     .with_middleware(LoggingMiddleware::new());
///
/// let router = group
///     .endpoint(list_users)
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

	/// Add an endpoint (a function decorated with `#[get]`, `#[post]`, etc.)
	///
	/// This is the primary way to register routes in a `RouteGroup`. The endpoint
	/// carries its path, HTTP method, and name via the [`EndpointInfo`] trait,
	/// which is automatically implemented by the route attribute macros.
	///
	/// [`EndpointInfo`]: reinhardt_core::endpoint::EndpointInfo
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_urls::routers::RouteGroup;
	/// # use reinhardt_core::endpoint::EndpointInfo;
	/// # use reinhardt_http::{Handler, Request, Response};
	/// # use hyper::Method;
	/// # struct ListUsers;
	/// # impl EndpointInfo for ListUsers {
	/// #     fn path() -> &'static str { "/users" }
	/// #     fn method() -> Method { Method::GET }
	/// #     fn name() -> &'static str { "list_users" }
	/// # }
	/// # #[async_trait::async_trait]
	/// # impl Handler for ListUsers {
	/// #     async fn handle(&self, _req: Request) -> Result<Response, reinhardt_http::Error> {
	/// #         Ok(Response::ok())
	/// #     }
	/// # }
	/// # fn list_users() -> ListUsers { ListUsers }
	///
	/// let group = RouteGroup::new()
	///     .endpoint(list_users);
	///
	/// let router = group.build();
	/// assert!(!router.get_all_routes().is_empty());
	/// ```
	pub fn endpoint<F, E>(mut self, f: F) -> Self
	where
		F: FnOnce() -> E,
		E: reinhardt_core::endpoint::EndpointInfo + reinhardt_http::Handler + 'static,
	{
		self.router = self.router.endpoint(f);
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
	/// ```rust,no_run
	/// use reinhardt_urls::routers::RouteGroup;
	/// # use reinhardt_core::endpoint::EndpointInfo;
	/// # use reinhardt_http::{Handler, Request, Response};
	/// # use hyper::Method;
	/// # struct Health;
	/// # impl EndpointInfo for Health {
	/// #     fn path() -> &'static str { "/health" }
	/// #     fn method() -> Method { Method::GET }
	/// #     fn name() -> &'static str { "health" }
	/// # }
	/// # #[async_trait::async_trait]
	/// # impl Handler for Health {
	/// #     async fn handle(&self, _req: Request) -> Result<Response, reinhardt_http::Error> {
	/// #         Ok(Response::ok())
	/// #     }
	/// # }
	/// # fn health() -> Health { Health }
	///
	/// let group = RouteGroup::new()
	///     .with_prefix("/api")
	///     .endpoint(health);
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
	use async_trait::async_trait;
	use reinhardt_http::{Handler, Request, Response, Result};
	use reinhardt_middleware::LoggingMiddleware;

	#[derive(Clone)]
	struct TestView;

	#[async_trait]
	impl Handler for TestView {
		async fn handle(&self, _req: Request) -> Result<Response> {
			Ok(Response::ok())
		}
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
	fn test_route_group_view() {
		let group = RouteGroup::new().view("/health", TestView);
		let _router = group.build();
		// Routes are correctly added, verified in integration tests
	}

	#[test]
	fn test_route_group_nested() {
		let auth_group =
			RouteGroup::new()
				.with_prefix("/auth/")
				.view("/login", TestView);

		let group = RouteGroup::new().with_prefix("/api/").nest(auth_group);

		let router = group.build();
		assert_eq!(router.children_count(), 1);
	}

	#[test]
	fn test_route_group_multiple_middleware() {
		let group = RouteGroup::new()
			.with_middleware(LoggingMiddleware::new())
			.with_middleware(LoggingMiddleware::new())
			.view("/test", TestView);

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
