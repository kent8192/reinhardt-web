//! Route-registration methods for [`ServerRouter`].
//!
//! Covers function routes, handler routes, named routes, ViewSets,
//! endpoint-trait registration, class-based views, and per-route
//! middleware attachment.

use super::ServerRouter;
use super::handlers::FunctionHandler;
use super::types::{FunctionRoute, ViewRoute};
use crate::routers::Route;
use hyper::Method;
use reinhardt_core::endpoint::EndpointInfo;
use reinhardt_http::{Handler, Request, Response, Result};
use reinhardt_middleware::Middleware;
use reinhardt_views::viewsets::ViewSet;
use std::sync::Arc;

impl ServerRouter {
	/// Register a function-based route (FastAPI-style)
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_urls::routers::ServerRouter;
	/// use hyper::Method;
	/// # use reinhardt_http::{Request, Response, Result};
	///
	/// async fn health_check(_req: Request) -> Result<Response> {
	///     Ok(Response::ok())
	/// }
	///
	/// let router = ServerRouter::new()
	///     .function("/health", Method::GET, health_check);
	/// ```
	pub fn function<F, Fut>(mut self, path: &str, method: Method, func: F) -> Self
	where
		F: Fn(Request) -> Fut + Send + Sync + 'static,
		Fut: std::future::Future<Output = Result<Response>> + Send + 'static,
	{
		let handler = Arc::new(FunctionHandler { func });
		self.functions.push(FunctionRoute {
			path: path.to_string(),
			method,
			handler,
			name: None,
			middleware: Vec::new(),
		});
		self
	}

	/// Register a route with a Handler trait implementation and HTTP method
	///
	/// This method accepts a type that implements the `Handler` trait,
	/// allowing for stateful handlers and a more object-oriented approach.
	/// Unlike `handler()`, this method requires specifying an HTTP method.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_urls::routers::ServerRouter;
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
	/// let router = ServerRouter::new()
	///     .handler_with_method("/articles", Method::GET, ArticleHandler);
	/// ```
	pub fn handler_with_method<H: Handler + 'static>(
		mut self,
		path: &str,
		method: Method,
		handler: H,
	) -> Self {
		self.functions.push(FunctionRoute {
			path: path.to_string(),
			method,
			handler: Arc::new(handler),
			name: None,
			middleware: Vec::new(),
		});
		self
	}

	/// Register a route (alias for `function`)
	///
	/// This method is an alias for `function` and provides the same functionality.
	/// Use it when you prefer the `route` naming convention.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_urls::routers::ServerRouter;
	/// use hyper::Method;
	/// # use reinhardt_http::{Request, Response, Result};
	///
	/// async fn health_check(_req: Request) -> Result<Response> {
	///     Ok(Response::ok())
	/// }
	///
	/// let router = ServerRouter::new()
	///     .route("/health", Method::GET, health_check);
	/// ```
	#[inline]
	pub fn route<F, Fut>(self, path: &str, method: Method, func: F) -> Self
	where
		F: Fn(Request) -> Fut + Send + Sync + 'static,
		Fut: std::future::Future<Output = Result<Response>> + Send + 'static,
	{
		self.function(path, method, func)
	}

	/// Register a named function-based route (FastAPI-style with URL reversal)
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_urls::routers::ServerRouter;
	/// use hyper::Method;
	/// # use reinhardt_http::{Request, Response, Result};
	///
	/// # async fn health_check(_req: Request) -> Result<Response> {
	/// #     Ok(Response::ok())
	/// # }
	/// let mut router = ServerRouter::new()
	///     .with_namespace("api")
	///     .function_named("/health", Method::GET, "health", health_check);
	///
	/// router.register_all_routes();
	/// let url = router.reverse("api:health", &[]).unwrap();
	/// assert_eq!(url, "/health");
	/// ```
	#[deprecated(
		since = "0.2.0",
		note = "Use `#[get(\"/path\", name = \"name\")]` + `.endpoint()` instead"
	)]
	pub fn function_named<F, Fut>(mut self, path: &str, method: Method, name: &str, func: F) -> Self
	where
		F: Fn(Request) -> Fut + Send + Sync + 'static,
		Fut: std::future::Future<Output = Result<Response>> + Send + 'static,
	{
		let handler = Arc::new(FunctionHandler { func });
		self.functions.push(FunctionRoute {
			path: path.to_string(),
			method,
			handler,
			name: Some(name.to_string()),
			middleware: Vec::new(),
		});
		self
	}

	/// Register a named route with a Handler trait implementation and HTTP method
	///
	/// This method accepts a type that implements the `Handler` trait,
	/// allowing for stateful handlers with URL reversal support.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_urls::routers::ServerRouter;
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
	/// let mut router = ServerRouter::new()
	///     .with_namespace("api")
	///     .handler_with_method_named("/articles", Method::GET, "list_articles", ArticleHandler);
	///
	/// router.register_all_routes();
	/// let url = router.reverse("api:list_articles", &[]).unwrap();
	/// assert_eq!(url, "/articles");
	/// ```
	#[deprecated(
		since = "0.2.0",
		note = "Use `#[get(\"/path\", name = \"name\")]` + `.endpoint()` instead"
	)]
	pub fn handler_with_method_named<H: Handler + 'static>(
		mut self,
		path: &str,
		method: Method,
		name: &str,
		handler: H,
	) -> Self {
		self.functions.push(FunctionRoute {
			path: path.to_string(),
			method,
			handler: Arc::new(handler),
			name: Some(name.to_string()),
			middleware: Vec::new(),
		});
		self
	}

	/// Register a named route (alias for `function_named`)
	///
	/// This method is an alias for `function_named` and provides the same functionality.
	/// Use it when you prefer the `route` naming convention.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_urls::routers::ServerRouter;
	/// use hyper::Method;
	/// # use reinhardt_http::{Request, Response, Result};
	///
	/// # async fn health_check(_req: Request) -> Result<Response> {
	/// #     Ok(Response::ok())
	/// # }
	/// let mut router = ServerRouter::new()
	///     .with_namespace("api")
	///     .route_named("/health", Method::GET, "health", health_check);
	///
	/// router.register_all_routes();
	/// let url = router.reverse("api:health", &[]).unwrap();
	/// assert_eq!(url, "/health");
	/// ```
	#[deprecated(
		since = "0.2.0",
		note = "Use `#[get(\"/path\", name = \"name\")]` + `.endpoint()` instead"
	)]
	#[inline]
	pub fn route_named<F, Fut>(self, path: &str, method: Method, name: &str, func: F) -> Self
	where
		F: Fn(Request) -> Fut + Send + Sync + 'static,
		Fut: std::future::Future<Output = Result<Response>> + Send + 'static,
	{
		#[allow(deprecated)]
		self.function_named(path, method, name, func)
	}

	/// Register a ViewSet (DRF-style)
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_urls::routers::ServerRouter;
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
	/// let router = ServerRouter::new()
	///     .viewset("/users", UserViewSet);
	/// ```
	pub fn viewset<V: ViewSet + 'static>(mut self, prefix: &str, viewset: V) -> Self {
		self.viewsets.insert(prefix.to_string(), Arc::new(viewset));
		self
	}

	/// Same as [`Self::viewset`] at runtime, but carries a `PhantomData<M>`
	/// marker that `#[url_patterns]` recovers at expansion time to discover
	/// `#[action]`-decorated methods on the impl block `M`.
	///
	/// `M` is unconstrained at the type level — it is purely a name-bearing
	/// token. Users write `PhantomData::<MyViewSetImpl>` as the third argument.
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
	{
		self.viewset(prefix, viewset)
	}

	/// Register an endpoint using EndpointInfo trait
	///
	/// This method accepts a factory function that returns a View type implementing
	/// both `EndpointInfo` and `Handler` traits. The path, HTTP method, and name
	/// are automatically extracted from the `EndpointInfo` implementation.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_urls::routers::ServerRouter;
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
	/// // Pass the function directly (no () needed)
	/// let router = ServerRouter::new()
	///     .endpoint(list_users);
	/// ```
	pub fn endpoint<F, E>(mut self, f: F) -> Self
	where
		F: FnOnce() -> E,
		E: EndpointInfo + Handler + 'static,
	{
		let view = f();
		let path = E::path().to_string();
		let method = E::method();
		let name = E::name().to_string();

		self.functions.push(FunctionRoute {
			path,
			method,
			handler: Arc::new(view),
			name: Some(name),
			middleware: Vec::new(),
		});
		self
	}

	/// Register a class-based view (Django-style)
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_urls::routers::ServerRouter;
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
	/// let view = ArticleListView;
	/// let router = ServerRouter::new()
	///     .view("/articles", view);
	/// ```
	pub fn view<V>(mut self, path: &str, view: V) -> Self
	where
		V: Handler + 'static,
	{
		self.views.push(ViewRoute {
			path: path.to_string(),
			handler: Arc::new(view),
			name: None,
			middleware: Vec::new(),
		});
		self
	}

	/// Register a named class-based view (Django-style with URL reversal)
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_urls::routers::ServerRouter;
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
	/// let view = ArticleListView;
	/// let mut router = ServerRouter::new()
	///     .with_namespace("articles")
	///     .view_named("/articles", "list", view);
	///
	/// router.register_all_routes();
	/// let url = router.reverse("articles:list", &[]).unwrap();
	/// assert_eq!(url, "/articles");
	/// ```
	#[deprecated(
		since = "0.2.0",
		note = "Use `#[get(\"/path\", name = \"name\")]` + `.endpoint()` instead"
	)]
	pub fn view_named<V>(mut self, path: &str, name: &str, view: V) -> Self
	where
		V: Handler + 'static,
	{
		self.views.push(ViewRoute {
			path: path.to_string(),
			handler: Arc::new(view),
			name: Some(name.to_string()),
			middleware: Vec::new(),
		});
		self
	}

	/// Register a handler directly (recommended method)
	///
	/// This method allows you to pass a handler directly without wrapping it in `Arc`.
	/// The `Arc` wrapping is handled internally for you.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_urls::routers::ServerRouter;
	/// # use reinhardt_http::{Handler, {Request, Response, Result}};
	/// # use async_trait::async_trait;
	/// # struct CustomHandler;
	/// # #[async_trait]
	/// # impl Handler for CustomHandler {
	/// #     async fn handle(&self, _req: Request) -> Result<Response> {
	/// #         Ok(Response::ok())
	/// #     }
	/// # }
	///
	/// // No Arc::new() needed!
	/// let router = ServerRouter::new()
	///     .handler("/custom", CustomHandler);
	/// ```
	pub fn handler<H>(mut self, path: &str, handler: H) -> Self
	where
		H: Handler + 'static,
	{
		let route = Route::from_handler(path, handler);
		self.routes.push(route);
		self
	}

	/// Register a handler that is already wrapped in Arc (low-level API)
	///
	/// This is provided for cases where you already have an `Arc<dyn Handler>`.
	/// In most cases, you should use `handler()` instead.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_urls::routers::ServerRouter;
	/// # use reinhardt_http::{Handler, {Request, Response, Result}};
	/// # use async_trait::async_trait;
	/// # use std::sync::Arc;
	/// # struct CustomHandler;
	/// # #[async_trait]
	/// # impl Handler for CustomHandler {
	/// #     async fn handle(&self, _req: Request) -> Result<Response> {
	/// #         Ok(Response::ok())
	/// #     }
	/// # }
	///
	/// let handler = Arc::new(CustomHandler);
	/// let router = ServerRouter::new()
	///     .handler_arc("/custom", handler);
	/// ```
	pub fn handler_arc(mut self, path: &str, handler: Arc<dyn Handler>) -> Self {
		let route = Route::new(path, handler);
		self.routes.push(route);
		self
	}

	/// Add middleware to the last registered function route
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_urls::routers::ServerRouter;
	/// use reinhardt_middleware::LoggingMiddleware;
	/// use hyper::Method;
	/// # use reinhardt_http::{Request, Response, Result};
	///
	/// # async fn health(_req: Request) -> Result<Response> {
	/// #     Ok(Response::ok())
	/// # }
	/// let router = ServerRouter::new()
	///     .function("/health", Method::GET, health)
	///     .with_route_middleware(LoggingMiddleware::new());
	/// ```
	pub fn with_route_middleware<M: Middleware + 'static>(mut self, middleware: M) -> Self {
		let middleware = Arc::new(middleware);
		if let Some(route) = self.functions.last_mut() {
			route.middleware.push(middleware.clone());
		} else if let Some(route) = self.views.last_mut() {
			route.middleware.push(middleware.clone());
		} else if let Some(route) = self.routes.last_mut() {
			route.middleware.push(middleware);
		}
		self
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
	/// `viewset_with_actions`. The dispatch body is irrelevant — these tests
	/// only inspect what routes get registered.
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

	/// Marker type the future `#[url_patterns]` macro will recover at
	/// expansion time. It carries no runtime state.
	struct DummyImpl;

	#[rstest]
	fn viewset_with_actions_is_equivalent_to_viewset() {
		// Arrange
		let mut router_a = ServerRouter::new().viewset(
			"/users",
			DummyViewSet {
				basename: "users".to_string(),
			},
		);
		let mut router_b = ServerRouter::new().viewset_with_actions(
			"/users",
			DummyViewSet {
				basename: "users".to_string(),
			},
			PhantomData::<DummyImpl>,
		);

		// Act
		let _ = router_a.register_all_routes();
		let _ = router_b.register_all_routes();
		let routes_a = router_a.get_all_routes();
		let routes_b = router_b.get_all_routes();

		// Assert
		assert_eq!(routes_a, routes_b);
	}
}
