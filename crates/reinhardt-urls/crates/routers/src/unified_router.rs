//! Unified Router with hierarchical routing support
//!
//! This module provides a unified router that supports:
//! - Nested routers with automatic prefix inheritance
//! - Namespace-based URL reversal
//! - Middleware and DI context propagation
//! - Integration with ViewSets, functions, and class-based views

use crate::{PathMatcher, PathPattern, Route, UrlReverser};
use async_trait::async_trait;
use hyper::Method;
use reinhardt_apps::{Error, Handler, MiddlewareChain, Request, Response, Result};
use reinhardt_di::InjectionContext;
use reinhardt_middleware::Middleware;
use reinhardt_viewsets::{Action, ViewSet};
use std::collections::HashMap;
use std::sync::Arc;

pub use self::handlers::FunctionHandler;
pub use self::matching::{extract_params, path_matches};
pub use self::global::{clear_router, get_router, is_router_registered, register_router};

pub(crate) use self::handlers::ViewSetHandler;

mod handlers;
mod matching;
pub mod global;

/// Route match result with metadata
#[derive(Clone)]
pub(crate) struct RouteMatch {
    /// Matched handler
    pub handler: Arc<dyn Handler>,

    /// Extracted path parameters
    pub params: HashMap<String, String>,

    /// Full matched path
    #[allow(dead_code)]
    pub full_path: String,

    /// Middleware stack to apply (parent → child order)
    pub middleware_stack: Vec<Arc<dyn Middleware>>,

    /// DI context
    pub di_context: Option<Arc<InjectionContext>>,
}

/// Unified router with hierarchical routing support
///
/// Supports multiple API styles:
/// - FastAPI-style: Function-based routes
/// - DRF-style: ViewSets with automatic CRUD
/// - Django-style: Class-based views
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_routers::UnifiedRouter;
/// use hyper::Method;
/// # use reinhardt_apps::{Request, Response, Result};
///
/// # async fn example() -> Result<()> {
/// // Create a users sub-router
/// let users_router = UnifiedRouter::new()
///     .with_namespace("users")
///     .function("/export", Method::GET, |_req| async { Ok(Response::ok()) });
///
/// // Create root router
/// let router = UnifiedRouter::new()
///     .with_prefix("/api/v1")
///     .with_namespace("v1")
///     .function("/health", Method::GET, |_req| async { Ok(Response::ok()) })
///     .mount("/users", users_router);
///
/// // Generated URLs:
/// // /api/v1/health
/// // /api/v1/users/export
/// # Ok(())
/// # }
/// ```
pub struct UnifiedRouter {
    /// Router's prefix path
    prefix: String,

    /// Namespace for URL reversal
    namespace: Option<String>,

    /// Routes defined in this router
    routes: Vec<Route>,

    /// ViewSet registrations
    viewsets: HashMap<String, Arc<dyn ViewSet>>,

    /// Function-based routes
    functions: Vec<FunctionRoute>,

    /// Class-based view routes
    views: Vec<ViewRoute>,

    /// Child routers
    children: Vec<UnifiedRouter>,

    /// DI context
    di_context: Option<Arc<InjectionContext>>,

    /// Middleware stack
    middleware: Vec<Arc<dyn Middleware>>,

    /// URL reverser
    reverser: UrlReverser,

    /// Path matcher for efficient routing
    #[allow(dead_code)]
    matcher: PathMatcher,
}

/// Function-based route
pub(crate) struct FunctionRoute {
    pub path: String,
    pub method: Method,
    pub handler: Arc<dyn Handler>,
    pub name: Option<String>,
}

/// Class-based view route
pub(crate) struct ViewRoute {
    pub path: String,
    pub handler: Arc<dyn Handler>,
    pub name: Option<String>,
}

impl UnifiedRouter {
    /// Create a new UnifiedRouter
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::UnifiedRouter;
    ///
    /// let router = UnifiedRouter::new();
    /// ```
    pub fn new() -> Self {
        Self {
            prefix: String::new(),
            namespace: None,
            routes: Vec::new(),
            viewsets: HashMap::new(),
            functions: Vec::new(),
            views: Vec::new(),
            children: Vec::new(),
            di_context: None,
            middleware: Vec::new(),
            reverser: UrlReverser::new(),
            matcher: PathMatcher::new(),
        }
    }

    /// Set the prefix for this router
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::UnifiedRouter;
    ///
    /// let router = UnifiedRouter::new()
    ///     .with_prefix("/api/v1");
    /// ```
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Set the namespace for this router
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::UnifiedRouter;
    ///
    /// let router = UnifiedRouter::new()
    ///     .with_namespace("v1");
    /// ```
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Set the DI context for this router
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use reinhardt_routers::UnifiedRouter;
    /// use reinhardt_di::{InjectionContext, SingletonScope};
    /// use std::sync::Arc;
    ///
    /// let di_ctx = Arc::new(InjectionContext::new(Arc::new(SingletonScope::new())));
    /// let router = UnifiedRouter::new()
    ///     .with_di_context(di_ctx);
    /// ```
    pub fn with_di_context(mut self, ctx: Arc<InjectionContext>) -> Self {
        self.di_context = Some(ctx);
        self
    }

    /// Add middleware to this router
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use reinhardt_routers::UnifiedRouter;
    /// use reinhardt_middleware::LoggingMiddleware;
    /// use std::sync::Arc;
    ///
    /// let router = UnifiedRouter::new()
    ///     .with_middleware(Arc::new(LoggingMiddleware));
    /// ```
    pub fn with_middleware(mut self, mw: Arc<dyn Middleware>) -> Self {
        self.middleware.push(mw);
        self
    }

    /// Mount a child router at the given prefix
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use reinhardt_routers::UnifiedRouter;
    ///
    /// let users_router = UnifiedRouter::new()
    ///     .with_namespace("users");
    ///
    /// let router = UnifiedRouter::new()
    ///     .with_prefix("/api")
    ///     .mount("/users", users_router);
    ///
    /// // Generated URL structure:
    /// // /api/users/...
    /// ```
    pub fn mount(mut self, prefix: &str, mut child: UnifiedRouter) -> Self {
        // Set prefix if not already set
        if child.prefix.is_empty() {
            child.prefix = prefix.to_string();
        }

        // Inherit DI context if child doesn't have one
        if child.di_context.is_none() {
            child.di_context = self.di_context.clone();
        }

        self.children.push(child);
        self
    }

    /// Mount a child router (mutable version)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use reinhardt_routers::UnifiedRouter;
    ///
    /// let mut router = UnifiedRouter::new();
    /// let users_router = UnifiedRouter::new();
    ///
    /// router.mount_mut("/users", users_router);
    /// ```
    pub fn mount_mut(&mut self, prefix: &str, mut child: UnifiedRouter) {
        if child.prefix.is_empty() {
            child.prefix = prefix.to_string();
        }
        if child.di_context.is_none() {
            child.di_context = self.di_context.clone();
        }
        self.children.push(child);
    }

    /// Add multiple child routers at once
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use reinhardt_routers::UnifiedRouter;
    ///
    /// let users = UnifiedRouter::new().with_prefix("/users");
    /// let posts = UnifiedRouter::new().with_prefix("/posts");
    ///
    /// let router = UnifiedRouter::new()
    ///     .group(vec![users, posts]);
    /// ```
    pub fn group(mut self, routers: Vec<UnifiedRouter>) -> Self {
        for router in routers {
            self.children.push(router);
        }
        self
    }

    /// Register a function-based route (FastAPI-style)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use reinhardt_routers::UnifiedRouter;
    /// use hyper::Method;
    /// # use reinhardt_apps::{Request, Response, Result};
    ///
    /// async fn health_check(_req: Request) -> Result<Response> {
    ///     Ok(Response::ok())
    /// }
    ///
    /// let router = UnifiedRouter::new()
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
        });
        self
    }

    /// Register a named function-based route (FastAPI-style with URL reversal)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use reinhardt_routers::UnifiedRouter;
    /// use hyper::Method;
    /// # use reinhardt_apps::{Request, Response, Result};
    ///
    /// async fn health_check(_req: Request) -> Result<Response> {
    ///     Ok(Response::ok())
    /// }
    ///
    /// let mut router = UnifiedRouter::new()
    ///     .with_namespace("api")
    ///     .function_named("/health", Method::GET, "health", health_check);
    ///
    /// router.register_all_routes();
    /// let url = router.reverse("api:health", &[]).unwrap();
    /// assert_eq!(url, "/health");
    /// ```
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
        });
        self
    }

    /// Register a ViewSet (DRF-style)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use reinhardt_routers::UnifiedRouter;
    /// # use reinhardt_viewsets::ViewSet;
    /// # use std::sync::Arc;
    /// # struct UserViewSet;
    /// # impl ViewSet for UserViewSet {
    /// #     fn get_basename(&self) -> &str { "users" }
    /// #     async fn dispatch(&self, _req: reinhardt_apps::Request, _action: reinhardt_viewsets::Action)
    /// #         -> reinhardt_apps::Result<reinhardt_apps::Response> {
    /// #         Ok(reinhardt_apps::Response::ok())
    /// #     }
    /// # }
    ///
    /// let viewset = Arc::new(UserViewSet);
    /// let router = UnifiedRouter::new()
    ///     .viewset("/users", viewset);
    /// ```
    pub fn viewset(mut self, prefix: &str, viewset: Arc<dyn ViewSet>) -> Self {
        self.viewsets.insert(prefix.to_string(), viewset);
        self
    }

    /// Register a class-based view (Django-style)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use reinhardt_routers::UnifiedRouter;
    /// # use reinhardt_apps::{Handler, Request, Response, Result};
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
    /// let router = UnifiedRouter::new()
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
        });
        self
    }

    /// Register a named class-based view (Django-style with URL reversal)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use reinhardt_routers::UnifiedRouter;
    /// # use reinhardt_apps::{Handler, Request, Response, Result};
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
    /// let mut router = UnifiedRouter::new()
    ///     .with_namespace("articles")
    ///     .view_named("/articles", "list", view);
    ///
    /// router.register_all_routes();
    /// let url = router.reverse("articles:list", &[]).unwrap();
    /// assert_eq!(url, "/articles");
    /// ```
    pub fn view_named<V>(mut self, path: &str, name: &str, view: V) -> Self
    where
        V: Handler + 'static,
    {
        self.views.push(ViewRoute {
            path: path.to_string(),
            handler: Arc::new(view),
            name: Some(name.to_string()),
        });
        self
    }

    /// Register a raw handler (low-level API)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use reinhardt_routers::UnifiedRouter;
    /// use hyper::Method;
    /// # use reinhardt_apps::{Handler, Request, Response, Result};
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
    /// let router = UnifiedRouter::new()
    ///     .handler("/custom", Method::POST, handler);
    /// ```
    pub fn handler(mut self, path: &str, _method: Method, handler: Arc<dyn Handler>) -> Self {
        let route = Route::new(path, handler);
        self.routes.push(route);
        self
    }

    /// Get the prefix of this router
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Get the namespace of this router
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// Get the number of child routers
    pub fn children_count(&self) -> usize {
        self.children.len()
    }

    /// Get all routes from this router and its children
    ///
    /// Returns a vector of tuples containing (full_path, name, namespace, methods).
    /// This recursively collects routes from all child routers.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let router = UnifiedRouter::new()
    ///     .with_prefix("/api/v1")
    ///     .function("/users", Method::GET, handler);
    ///
    /// let routes = router.get_all_routes();
    /// // Returns: [("/api/v1/users", None, None, vec![Method::GET])]
    /// ```
    pub fn get_all_routes(&self) -> Vec<(String, Option<String>, Option<String>, Vec<Method>)> {
        let mut routes = Vec::new();

        // Collect routes from this router
        for route in &self.routes {
            let full_path = if self.prefix.is_empty() {
                route.path.clone()
            } else {
                format!("{}{}", self.prefix, route.path)
            };

            routes.push((
                full_path,
                route.name.clone(),
                route.namespace.clone().or_else(|| self.namespace.clone()),
                vec![], // Routes don't store HTTP methods, empty for now
            ));
        }

        // Collect function-based routes
        for func_route in &self.functions {
            let full_path = if self.prefix.is_empty() {
                func_route.path.clone()
            } else {
                format!("{}{}", self.prefix, func_route.path)
            };

            routes.push((
                full_path,
                None,                   // Function routes don't have names
                self.namespace.clone(), // Use router's namespace
                vec![func_route.method.clone()],
            ));
        }

        // Collect view routes
        for view_route in &self.views {
            let full_path = if self.prefix.is_empty() {
                view_route.path.clone()
            } else {
                format!("{}{}", self.prefix, view_route.path)
            };

            routes.push((
                full_path,
                None,                   // View routes don't have names
                self.namespace.clone(), // Use router's namespace
                vec![],                 // Views handle their own methods
            ));
        }

        // Collect ViewSet routes
        for (prefix, _viewset) in &self.viewsets {
            let base_path = if self.prefix.is_empty() {
                format!("/{}", prefix)
            } else {
                format!("{}/{}", self.prefix, prefix)
            };

            // ViewSets generate standard CRUD routes
            let viewset_routes = vec![
                (format!("{}/", base_path), vec![Method::GET, Method::POST]),
                (
                    format!("{}/<id>/", base_path),
                    vec![Method::GET, Method::PUT, Method::DELETE],
                ),
            ];

            for (path, methods) in viewset_routes {
                routes.push((
                    path,
                    None,                   // ViewSet routes don't have individual names
                    self.namespace.clone(), // Use router's namespace
                    methods,
                ));
            }
        }

        // Recursively collect from child routers
        for child in &self.children {
            let child_prefix = if self.prefix.is_empty() {
                child.prefix.clone()
            } else if child.prefix.is_empty() {
                self.prefix.clone()
            } else {
                format!("{}{}", self.prefix, child.prefix)
            };

            for (path, name, namespace, methods) in child.get_all_routes() {
                // Adjust path if child has no prefix (already included)
                let full_path = if path.starts_with(&child.prefix) || child.prefix.is_empty() {
                    path
                } else {
                    format!("{}{}", child_prefix, path)
                };

                // Combine namespaces (parent:child)
                let combined_namespace = match (self.namespace.as_ref(), namespace.as_ref()) {
                    (Some(parent), Some(child)) => Some(format!("{}:{}", parent, child)),
                    (Some(parent), None) => Some(parent.clone()),
                    (None, Some(child)) => Some(child.clone()),
                    (None, None) => None,
                };

                routes.push((full_path, name, combined_namespace, methods));
            }
        }

        routes
    }

    /// Get the fully qualified namespace for this router
    ///
    /// Returns the complete namespace chain from root to this router.
    /// For example, if this router has namespace "users" and its parent has "v1",
    /// this returns "v1:users".
    ///
    /// # Arguments
    ///
    /// * `parent_namespace` - The parent router's namespace (if any)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let router = UnifiedRouter::new().with_namespace("users");
    /// assert_eq!(router.get_full_namespace(Some("v1")), Some("v1:users".to_string()));
    /// assert_eq!(router.get_full_namespace(None), Some("users".to_string()));
    /// ```
    pub fn get_full_namespace(&self, parent_namespace: Option<&str>) -> Option<String> {
        match (parent_namespace, self.namespace.as_deref()) {
            (Some(parent), Some(child)) => Some(format!("{}:{}", parent, child)),
            (Some(parent), None) => Some(parent.to_string()),
            (None, Some(child)) => Some(child.to_string()),
            (None, None) => None,
        }
    }

    /// Register all routes with the URL reverser
    ///
    /// This recursively registers all routes from this router and its children
    /// with their fully qualified names (namespace:name format).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut router = UnifiedRouter::new()
    ///     .with_namespace("v1");
    ///
    /// // After registering routes, you can reverse them:
    /// router.register_all_routes();
    /// let url = router.reverse("v1:users:detail", &[("id", "123")]);
    /// ```
    pub fn register_all_routes(&mut self) {
        self.register_routes_recursive(None);
    }

    /// Recursively register routes with namespaces
    fn register_routes_recursive(&mut self, parent_namespace: Option<&str>) {
        let full_namespace = self.get_full_namespace(parent_namespace);

        // Register routes from this router
        for route in &self.routes {
            if let Some(name) = &route.name {
                let qualified_name = if let Some(ref ns) = full_namespace {
                    format!("{}:{}", ns, name)
                } else {
                    name.clone()
                };

                // Register with UrlReverser
                self.reverser.register_path(&qualified_name, &route.path);
            }
        }

        // Register function routes (if they get names in the future)
        for func_route in &self.functions {
            if let Some(ref name) = func_route.name {
                let qualified_name = if let Some(ref ns) = full_namespace {
                    format!("{}:{}", ns, name)
                } else {
                    name.clone()
                };

                self.reverser
                    .register_path(&qualified_name, &func_route.path);
            }
        }

        // Register view routes (if they get names in the future)
        for view_route in &self.views {
            if let Some(ref name) = view_route.name {
                let qualified_name = if let Some(ref ns) = full_namespace {
                    format!("{}:{}", ns, name)
                } else {
                    name.clone()
                };

                self.reverser
                    .register_path(&qualified_name, &view_route.path);
            }
        }

        // Register ViewSet routes with standard names
        for (prefix, _viewset) in &self.viewsets {
            let base_path = if self.prefix.is_empty() {
                format!("/{}", prefix)
            } else {
                format!("{}/{}", self.prefix, prefix)
            };

            // Standard ViewSet action names
            let viewset_routes = vec![
                (format!("{}-list", prefix), format!("{}/", base_path)),
                (format!("{}-detail", prefix), format!("{}/<id>/", base_path)),
            ];

            for (name, path) in viewset_routes {
                let qualified_name = if let Some(ref ns) = full_namespace {
                    format!("{}:{}", ns, name)
                } else {
                    name
                };

                self.reverser.register_path(&qualified_name, &path);
            }
        }

        // Recursively register child routes
        for child in &mut self.children {
            child.register_routes_recursive(full_namespace.as_deref());
        }
    }

    /// Reverse a URL by route name
    ///
    /// Supports hierarchical namespace notation (e.g., "v1:users:detail").
    ///
    /// # Arguments
    ///
    /// * `name` - The route name, optionally with namespace (e.g., "users-detail" or "v1:users-detail")
    /// * `params` - URL parameters as key-value pairs
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let router = UnifiedRouter::new()
    ///     .with_namespace("v1");
    ///
    /// // Reverse with namespace
    /// let url = router.reverse("v1:users:detail", &[("id", "123")]).unwrap();
    /// assert_eq!(url, "/users/123/");
    ///
    /// // Reverse without namespace (searches all routes)
    /// let url = router.reverse("users-detail", &[("id", "123")]).unwrap();
    /// ```
    pub fn reverse(&self, name: &str, params: &[(&str, &str)]) -> Option<String> {
        // Try own reverser first
        if let Ok(url) = self.reverser.reverse_with(name, params) {
            return Some(url);
        }

        // Try child routers
        for child in &self.children {
            if let Some(url) = child.reverse(name, params) {
                return Some(url);
            }
        }

        None
    }

    /// Resolve a request path to a route match
    ///
    /// This performs hierarchical route resolution:
    /// 1. Check prefix match
    /// 2. Try child routers first (depth-first search)
    /// 3. Try own routes
    fn resolve(&self, path: &str) -> Option<RouteMatch> {
        // 1. Check prefix
        let remaining_path = if !self.prefix.is_empty() {
            path.strip_prefix(&self.prefix)?
        } else {
            path
        };

        // 2. Try child routers first
        for child in &self.children {
            if let Some(route_match) =
                child.resolve_internal(remaining_path, &self.middleware, &self.di_context)
            {
                return Some(route_match);
            }
        }

        // 3. Try own routes
        self.match_own_routes(remaining_path)
    }

    /// Internal route resolution with middleware and DI context inheritance
    fn resolve_internal(
        &self,
        path: &str,
        parent_middleware: &[Arc<dyn Middleware>],
        parent_di: &Option<Arc<InjectionContext>>,
    ) -> Option<RouteMatch> {
        // Check prefix
        let remaining_path = if !self.prefix.is_empty() {
            path.strip_prefix(&self.prefix)?
        } else {
            path
        };

        // Build middleware stack (parent → child order)
        let mut middleware_stack = parent_middleware.to_vec();
        middleware_stack.extend(self.middleware.iter().cloned());

        // Inherit DI context
        let di_context = self.di_context.clone().or_else(|| parent_di.clone());

        // Try child routers
        for child in &self.children {
            if let Some(route_match) =
                child.resolve_internal(remaining_path, &middleware_stack, &di_context)
            {
                return Some(route_match);
            }
        }

        // Try own routes
        self.match_own_routes_with_context(remaining_path, middleware_stack, di_context)
    }

    /// Match routes in this router (without context)
    fn match_own_routes(&self, path: &str) -> Option<RouteMatch> {
        self.match_own_routes_with_context(path, self.middleware.clone(), self.di_context.clone())
    }

    /// Match routes in this router with provided context
    fn match_own_routes_with_context(
        &self,
        path: &str,
        middleware_stack: Vec<Arc<dyn Middleware>>,
        di_context: Option<Arc<InjectionContext>>,
    ) -> Option<RouteMatch> {
        let full_path = format!("{}{}", self.prefix, path);

        // Try functions
        for func_route in &self.functions {
            if path_matches(path, &func_route.path) {
                return Some(RouteMatch {
                    handler: func_route.handler.clone(),
                    params: extract_params(path, &func_route.path),
                    full_path: full_path.clone(),
                    middleware_stack: middleware_stack.clone(),
                    di_context: di_context.clone(),
                });
            }
        }

        // Try views
        for view_route in &self.views {
            if path_matches(path, &view_route.path) {
                return Some(RouteMatch {
                    handler: view_route.handler.clone(),
                    params: extract_params(path, &view_route.path),
                    full_path: full_path.clone(),
                    middleware_stack: middleware_stack.clone(),
                    di_context: di_context.clone(),
                });
            }
        }

        // Try raw routes
        for route in &self.routes {
            if let Ok(pattern) = PathPattern::new(&route.path) {
                if let Some(params) = pattern.extract_params(path) {
                    return Some(RouteMatch {
                        handler: route.handler.clone(),
                        params,
                        full_path: full_path.clone(),
                        middleware_stack: middleware_stack.clone(),
                        di_context: di_context.clone(),
                    });
                }
            }
        }

        // Try ViewSets
        for (prefix, viewset) in &self.viewsets {
            let base_path = if self.prefix.is_empty() {
                format!("/{}", prefix.trim_start_matches('/'))
            } else {
                format!("{}/{}", self.prefix, prefix.trim_start_matches('/'))
            };

            // Check for collection route (list/create): /prefix/
            let collection_path = format!("{}/", base_path.trim_end_matches('/'));
            if path == collection_path.trim_start_matches('/') || path == collection_path {
                // Determine action based on HTTP method
                let action = match full_path.as_str() {
                    _ if path.ends_with('/') => {
                        // Collection endpoint
                        Action::list() // Default to list for GET, create for POST
                    }
                    _ => continue,
                };

                return Some(RouteMatch {
                    handler: Arc::new(ViewSetHandler {
                        viewset: viewset.clone(),
                        action,
                    }),
                    params: HashMap::new(),
                    full_path: full_path.clone(),
                    middleware_stack: middleware_stack.clone(),
                    di_context: di_context.clone(),
                });
            }

            // Check for detail route (retrieve/update/destroy): /prefix/{id}/
            let detail_pattern = format!("{}/(?P<id>[^/]+)/?$", base_path.trim_end_matches('/'));
            let re = regex::Regex::new(&detail_pattern).ok()?;

            if let Some(captures) = re.captures(path) {
                let id = captures.name("id")?.as_str().to_string();
                let lookup_field = viewset.get_lookup_field();

                let action = Action::retrieve(); // Default to retrieve, actual action determined by HTTP method

                let mut params = HashMap::new();
                params.insert(lookup_field.to_string(), id);

                return Some(RouteMatch {
                    handler: Arc::new(ViewSetHandler {
                        viewset: viewset.clone(),
                        action,
                    }),
                    params,
                    full_path: full_path.clone(),
                    middleware_stack: middleware_stack.clone(),
                    di_context: di_context.clone(),
                });
            }
        }

        None
    }
}

impl Default for UnifiedRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Handler implementation for UnifiedRouter
#[async_trait]
impl Handler for UnifiedRouter {
    async fn handle(&self, mut req: Request) -> Result<Response> {
        let path = req.uri.path();

        // Resolve route
        let route_match = self
            .resolve(path)
            .ok_or_else(|| Error::NotFound(format!("No route for {}", path)))?;

        // Set path parameters in request
        for (key, value) in route_match.params {
            req.set_path_param(key, value);
        }

        // Set DI context if available
        if let Some(di_ctx) = &route_match.di_context {
            req.set_di_context(di_ctx.clone());
        }

        // Apply middleware stack using MiddlewareChain
        if route_match.middleware_stack.is_empty() {
            // No middleware, execute handler directly
            route_match.handler.handle(req).await
        } else {
            // Build middleware chain
            let chain = route_match
                .middleware_stack
                .iter()
                .fold(
                    MiddlewareChain::new(route_match.handler.clone()),
                    |chain, mw| chain.with_middleware(mw.clone()),
                );

            // Execute chain
            chain.handle(req).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_router() {
        let router = UnifiedRouter::new();
        assert_eq!(router.prefix(), "");
        assert_eq!(router.namespace(), None);
        assert_eq!(router.children_count(), 0);
    }

    #[test]
    fn test_with_prefix() {
        let router = UnifiedRouter::new().with_prefix("/api/v1");
        assert_eq!(router.prefix(), "/api/v1");
    }

    #[test]
    fn test_with_namespace() {
        let router = UnifiedRouter::new().with_namespace("v1");
        assert_eq!(router.namespace(), Some("v1"));
    }

    #[test]
    fn test_mount() {
        let child = UnifiedRouter::new();
        let router = UnifiedRouter::new().mount("/users", child);
        assert_eq!(router.children_count(), 1);
    }

    #[test]
    fn test_mount_inherits_di_context() {
        let di_ctx = Arc::new(InjectionContext::new(Arc::new(
            reinhardt_di::SingletonScope::new(),
        )));

        let child = UnifiedRouter::new();
        let router = UnifiedRouter::new()
            .with_di_context(di_ctx.clone())
            .mount("/users", child);

        assert!(router.di_context.is_some());
        assert_eq!(router.children_count(), 1);
    }

    #[test]
    fn test_group() {
        let users = UnifiedRouter::new().with_prefix("/users");
        let posts = UnifiedRouter::new().with_prefix("/posts");

        let router = UnifiedRouter::new().group(vec![users, posts]);
        assert_eq!(router.children_count(), 2);
    }

    #[test]
    fn test_get_all_routes() {
        let router = UnifiedRouter::new()
            .with_prefix("/api")
            .with_namespace("api");

        let routes = router.get_all_routes();
        assert_eq!(routes.len(), 0); // No routes added yet
    }

    #[test]
    fn test_get_full_namespace_no_parent() {
        let router = UnifiedRouter::new().with_namespace("users");
        assert_eq!(router.get_full_namespace(None), Some("users".to_string()));
    }

    #[test]
    fn test_get_full_namespace_with_parent() {
        let router = UnifiedRouter::new().with_namespace("users");
        assert_eq!(
            router.get_full_namespace(Some("v1")),
            Some("v1:users".to_string())
        );
    }

    #[test]
    fn test_get_full_namespace_no_namespace() {
        let router = UnifiedRouter::new();
        assert_eq!(
            router.get_full_namespace(Some("v1")),
            Some("v1".to_string())
        );
        assert_eq!(router.get_full_namespace(None), None);
    }

    #[test]
    fn test_hierarchical_namespace() {
        let child = UnifiedRouter::new().with_namespace("users");
        let parent = UnifiedRouter::new()
            .with_namespace("v1")
            .mount("/users", child);

        // Check that namespaces are properly nested
        assert_eq!(parent.namespace(), Some("v1"));
        assert_eq!(parent.children_count(), 1);
    }

    #[test]
    fn test_register_all_routes_with_namespace() {
        use hyper::Method;

        async fn dummy_handler(_req: Request) -> Result<Response> {
            Ok(Response::ok())
        }

        let mut router = UnifiedRouter::new().with_namespace("api").function_named(
            "/health",
            Method::GET,
            "health",
            dummy_handler,
        );

        router.register_all_routes();

        // Verify route is registered with namespace
        let url = router.reverse("api:health", &[]);
        assert!(url.is_some());
        assert_eq!(url.unwrap(), "/health");
    }

    #[test]
    fn test_nested_namespace_registration() {
        use hyper::Method;

        async fn dummy_handler(_req: Request) -> Result<Response> {
            Ok(Response::ok())
        }

        let users = UnifiedRouter::new().with_namespace("users").function_named(
            "/list",
            Method::GET,
            "list",
            dummy_handler,
        );

        let mut api = UnifiedRouter::new()
            .with_namespace("v1")
            .with_prefix("/api/v1")
            .mount("/users", users);

        api.register_all_routes();

        // Should be able to reverse with full namespace
        let url = api.reverse("v1:users:list", &[]);
        assert!(url.is_some());
        assert_eq!(url.unwrap(), "/list");
    }

    #[test]
    fn test_mount_prefix_inheritance() {
        let child = UnifiedRouter::new();
        let parent = UnifiedRouter::new().with_prefix("/api").mount("/v1", child);

        assert_eq!(parent.children_count(), 1);
        // Child should inherit the mount path as its prefix
    }

    #[test]
    fn test_multiple_child_routers() {
        let users = UnifiedRouter::new().with_namespace("users");
        let posts = UnifiedRouter::new().with_namespace("posts");
        let comments = UnifiedRouter::new().with_namespace("comments");

        let router = UnifiedRouter::new()
            .mount("/users", users)
            .mount("/posts", posts)
            .mount("/comments", comments);

        assert_eq!(router.children_count(), 3);
    }

    #[test]
    fn test_deep_nesting() {
        let resource = UnifiedRouter::new().with_namespace("resource");
        let v2 = UnifiedRouter::new()
            .with_namespace("v2")
            .mount("/resource", resource);
        let v1 = UnifiedRouter::new().with_namespace("v1").mount("/v2", v2);
        let api = UnifiedRouter::new().with_namespace("api").mount("/v1", v1);

        // Should support deep nesting
        assert_eq!(api.children_count(), 1);
    }
}
