//! Lightweight router with minimal overhead.
//!
//! `SimpleRouter` provides basic routing functionality without the advanced features
//! of `DefaultRouter`. It's designed for applications that need simple routing
//! without the overhead of URL reversal, namespace support, or ViewSet registration.
//!
//! # Features
//!
//! - Basic route registration and matching
//! - Path parameter extraction
//! - Minimal memory footprint
//! - Fast route lookup
//!
//! # Examples
//!
//! ```
//! use reinhardt_routers::{SimpleRouter, Router, path};
//! use reinhardt_apps::Handler;
//! use std::sync::Arc;
//!
//! # use async_trait::async_trait;
//! # use reinhardt_apps::{Request, Response, Result};
//! # struct DummyHandler;
//! # #[async_trait]
//! # impl Handler for DummyHandler {
//! #     async fn handle(&self, _req: Request) -> Result<Response> {
//! #         Ok(Response::ok())
//! #     }
//! # }
//! let handler = Arc::new(DummyHandler);
//! let mut router = SimpleRouter::new();
//! router.add_route(path("/users/", handler.clone()));
//! router.add_route(path("/users/{id}/", handler));
//!
//! assert_eq!(router.get_routes().len(), 2);
//! ```

use crate::{PathMatcher, PathPattern, Route, Router};
use async_trait::async_trait;
use reinhardt_apps::{Handler, Request, Response, Result};

/// Simple router implementation with minimal overhead
///
/// Unlike `DefaultRouter`, `SimpleRouter` does not support:
/// - URL reversal (reverse routing by name)
/// - Namespace-based routing
/// - ViewSet registration
/// - Version extraction
///
/// Use `SimpleRouter` when:
/// - You only need basic routing functionality
/// - Memory footprint is a concern
/// - You don't need URL reversal or namespaces
/// - You're building a microservice with simple routing needs
pub struct SimpleRouter {
    routes: Vec<Route>,
    matcher: PathMatcher,
}

impl SimpleRouter {
    /// Create a new SimpleRouter
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::SimpleRouter;
    ///
    /// let router = SimpleRouter::new();
    /// assert_eq!(router.get_routes().len(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            matcher: PathMatcher::new(),
        }
    }

    /// Get all registered routes
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::{SimpleRouter, Router, path};
    /// use reinhardt_apps::Handler;
    /// use std::sync::Arc;
    ///
    /// # use async_trait::async_trait;
    /// # use reinhardt_apps::{Request, Response, Result};
    /// # struct DummyHandler;
    /// # #[async_trait]
    /// # impl Handler for DummyHandler {
    /// #     async fn handle(&self, _req: Request) -> Result<Response> {
    /// #         Ok(Response::ok())
    /// #     }
    /// # }
    /// let handler = Arc::new(DummyHandler);
    /// let mut router = SimpleRouter::new();
    /// router.add_route(path("/users/", handler));
    ///
    /// assert_eq!(router.get_routes().len(), 1);
    /// assert_eq!(router.get_routes()[0].path, "/users/");
    /// ```
    pub fn get_routes(&self) -> &[Route] {
        &self.routes
    }
}

impl Default for SimpleRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::router::Router for SimpleRouter {
    fn add_route(&mut self, route: Route) {
        let pattern = PathPattern::new(&route.path).expect("Invalid path pattern");
        let handler_id = route
            .name
            .clone()
            .unwrap_or_else(|| format!("route_{}", self.routes.len()));

        self.matcher.add_pattern(pattern, handler_id);
        self.routes.push(route);
    }

    /// Include routes with a prefix
    ///
    /// Note: SimpleRouter ignores namespaces as it doesn't support URL reversal
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::{SimpleRouter, Router, path};
    /// use reinhardt_apps::Handler;
    /// use std::sync::Arc;
    ///
    /// # use async_trait::async_trait;
    /// # use reinhardt_apps::{Request, Response, Result};
    /// # struct DummyHandler;
    /// # #[async_trait]
    /// # impl Handler for DummyHandler {
    /// #     async fn handle(&self, _req: Request) -> Result<Response> {
    /// #         Ok(Response::ok())
    /// #     }
    /// # }
    /// let handler = Arc::new(DummyHandler);
    /// let users_routes = vec![
    ///     path("/", handler.clone()),
    ///     path("/{id}/", handler),
    /// ];
    ///
    /// let mut router = SimpleRouter::new();
    /// router.include("/users", users_routes, None);
    ///
    /// // Routes are prefixed: /users/ and /users/{id}/
    /// assert_eq!(router.get_routes().len(), 2);
    /// assert_eq!(router.get_routes()[0].path, "/users/");
    /// ```
    fn include(&mut self, prefix: &str, routes: Vec<Route>, _namespace: Option<String>) {
        let prefix = prefix.trim_end_matches('/');

        for mut route in routes {
            // Prepend the prefix to the route path
            let new_path = if route.path.starts_with('/') {
                format!("{}{}", prefix, route.path)
            } else {
                format!("{}/{}", prefix, route.path)
            };
            route.path = new_path;

            self.add_route(route);
        }
    }

    async fn route(&self, mut request: Request) -> Result<Response> {
        let path = request.path().to_string();

        if let Some((handler_id, params)) = self.matcher.match_path(&path) {
            // Find the route by matching handler_id with route name or index
            for (idx, route) in self.routes.iter().enumerate() {
                let route_id = route.name.as_ref().map(|n| n.as_str()).unwrap_or_else(|| {
                    // Use a temporary owned string for comparison
                    ""
                });

                let expected_id = if route_id.is_empty() {
                    format!("route_{}", idx)
                } else {
                    route_id.to_string()
                };

                if expected_id == handler_id {
                    // Add path parameters to request
                    request.path_params = params;
                    return route.handler.handle(request).await;
                }
            }
        }

        Err(reinhardt_apps::Error::NotFound(format!(
            "No route found for {}",
            path
        )))
    }
}

#[async_trait]
impl Handler for SimpleRouter {
    async fn handle(&self, request: Request) -> Result<Response> {
        self.route(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Router, path};
    use async_trait::async_trait;
    use bytes::Bytes;
    use hyper::{HeaderMap, Method, Uri, Version};
    use reinhardt_apps::{Request, Response, Result};

    struct DummyHandler;

    #[async_trait]
    impl Handler for DummyHandler {
        async fn handle(&self, _req: Request) -> Result<Response> {
            Ok(Response::ok())
        }
    }

    #[test]
    fn test_simple_router_new() {
        let router = SimpleRouter::new();
        assert_eq!(router.get_routes().len(), 0);
    }

    #[test]
    fn test_simple_router_add_route() {
        let mut router = SimpleRouter::new();
        let handler = std::sync::Arc::new(DummyHandler);

        router.add_route(path("/users/", handler.clone()));
        router.add_route(path("/users/{id}/", handler));

        assert_eq!(router.get_routes().len(), 2);
        assert_eq!(router.get_routes()[0].path, "/users/");
        assert_eq!(router.get_routes()[1].path, "/users/{id}/");
    }

    #[test]
    fn test_simple_router_include() {
        let mut router = SimpleRouter::new();
        let handler = std::sync::Arc::new(DummyHandler);

        let users_routes = vec![path("/", handler.clone()), path("/{id}/", handler)];

        router.include("/users", users_routes, None);

        assert_eq!(router.get_routes().len(), 2);
        assert_eq!(router.get_routes()[0].path, "/users/");
        assert_eq!(router.get_routes()[1].path, "/users/{id}/");
    }

    #[tokio::test]
    async fn test_simple_router_route() {
        let mut router = SimpleRouter::new();
        let handler = std::sync::Arc::new(DummyHandler);

        router.add_route(path("/users/", handler.clone()).with_name("users"));
        router.add_route(path("/users/{id}/", handler).with_name("user-detail"));

        let uri = Uri::from_static("/users/");
        let req = Request::new(
            Method::GET,
            uri,
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let response = router.route(req).await.unwrap();
        assert_eq!(response.status, 200);
    }

    #[tokio::test]
    async fn test_simple_router_route_with_params() {
        let mut router = SimpleRouter::new();
        let handler = std::sync::Arc::new(DummyHandler);

        router.add_route(path("/users/{id}/", handler).with_name("user-detail"));

        let uri = Uri::from_static("/users/123/");
        let req = Request::new(
            Method::GET,
            uri,
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let response = router.route(req).await.unwrap();
        assert_eq!(response.status, 200);
    }

    #[tokio::test]
    async fn test_simple_router_not_found() {
        let router = SimpleRouter::new();

        let uri = Uri::from_static("/nonexistent/");
        let req = Request::new(
            Method::GET,
            uri,
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let result = router.route(req).await;
        assert!(result.is_err());
    }
}
