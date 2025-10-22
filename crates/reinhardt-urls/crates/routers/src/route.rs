use reinhardt_apps::Handler;
use std::sync::Arc;

/// Route definition
/// Uses composition to combine path patterns with handlers
/// Similar to Django's URLPattern
#[derive(Clone)]
pub struct Route {
    pub path: String,
    pub handler: Arc<dyn Handler>,
    pub name: Option<String>,
    /// Namespace for this route (e.g., "users", "api")
    /// When combined with name, forms "namespace:name"
    pub namespace: Option<String>,
}

impl Route {
    /// Create a new route
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::Route;
    /// use reinhardt_apps::Handler;
    /// use std::sync::Arc;
    ///
    // Create a simple route (using a dummy handler for demonstration)
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
    /// let route = Route::new("/users/", handler);
    /// assert_eq!(route.path, "/users/");
    /// ```
    pub fn new(path: impl Into<String>, handler: Arc<dyn Handler>) -> Self {
        Self {
            path: path.into(),
            handler,
            name: None,
            namespace: None,
        }
    }
    /// Set the name of the route
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::Route;
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
    /// let route = Route::new("/users/", handler)
    ///     .with_name("user-list");
    /// assert_eq!(route.name, Some("user-list".to_string()));
    /// ```
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    /// Set the namespace of the route
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::Route;
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
    /// let route = Route::new("/users/", handler)
    ///     .with_namespace("api");
    /// assert_eq!(route.namespace, Some("api".to_string()));
    /// ```
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }
    /// Get the full name including namespace (e.g., "users:list")
    /// Similar to Django's view_name in ResolverMatch
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::Route;
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
    ///
    // With namespace and name
    /// let route = Route::new("/users/", handler.clone())
    ///     .with_namespace("api")
    ///     .with_name("list");
    /// assert_eq!(route.full_name(), Some("api:list".to_string()));
    ///
    // With only name
    /// let route = Route::new("/users/", handler.clone())
    ///     .with_name("list");
    /// assert_eq!(route.full_name(), Some("list".to_string()));
    ///
    // Without name
    /// let route = Route::new("/users/", handler);
    /// assert_eq!(route.full_name(), None);
    /// ```
    pub fn full_name(&self) -> Option<String> {
        match (&self.namespace, &self.name) {
            (Some(ns), Some(name)) => Some(format!("{}:{}", ns, name)),
            (None, Some(name)) => Some(name.clone()),
            _ => None,
        }
    }

    /// Check if this route matches a namespace pattern
    /// Used for namespace-based versioning
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::Route;
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
    /// let route = Route::new("/v1/users/", handler)
    ///     .with_namespace("v1");
    ///
    /// assert!(route.matches_namespace_pattern("/v{version}/"));
    /// assert!(!route.matches_namespace_pattern("/api/{version}/"));
    /// ```
    pub fn matches_namespace_pattern(&self, pattern: &str) -> bool {
        // Convert pattern like "/v{version}/" to regex
        let regex_pattern = pattern.replace("{version}", r"([^/]+)").replace("/", r"\/");
        let full_pattern = format!("^{}", regex_pattern);

        if let Ok(regex) = regex::Regex::new(&full_pattern) {
            regex.is_match(&self.path)
        } else {
            false
        }
    }

    /// Extract version from namespace pattern
    /// Returns the version string if the route matches the pattern
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::Route;
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
    /// let route = Route::new("/v1/users/", handler)
    ///     .with_namespace("v1");
    ///
    /// assert_eq!(route.extract_version_from_pattern("/v{version}/"), Some("1"));
    /// assert_eq!(route.extract_version_from_pattern("/api/{version}/"), None);
    /// ```
    pub fn extract_version_from_pattern(&self, pattern: &str) -> Option<&str> {
        // Convert pattern like "/v{version}/" to regex with capture group
        let regex_pattern = pattern.replace("{version}", r"([^/]+)").replace("/", r"\/");
        let full_pattern = format!("^{}", regex_pattern);

        if let Ok(regex) = regex::Regex::new(&full_pattern) {
            if let Some(captures) = regex.captures(&self.path) {
                if let Some(version_match) = captures.get(1) {
                    return Some(version_match.as_str());
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use reinhardt_apps::{Request, Response, Result};

    struct DummyHandler;

    #[async_trait]
    impl Handler for DummyHandler {
        async fn handle(&self, _req: Request) -> Result<Response> {
            Ok(Response::ok())
        }
    }

    #[test]
    fn test_matches_namespace_pattern() {
        let handler = std::sync::Arc::new(DummyHandler);
        let route = Route::new("/v1/users/", handler).with_namespace("v1");

        assert!(route.matches_namespace_pattern("/v{version}/"));
        assert!(!route.matches_namespace_pattern("/api/{version}/"));
        assert!(!route.matches_namespace_pattern("/users/"));
    }

    #[test]
    fn test_extract_version_from_pattern() {
        let handler = std::sync::Arc::new(DummyHandler);
        let route = Route::new("/v1/users/", handler).with_namespace("v1");

        assert_eq!(
            route.extract_version_from_pattern("/v{version}/"),
            Some("1")
        );
        assert_eq!(route.extract_version_from_pattern("/api/{version}/"), None);
        assert_eq!(route.extract_version_from_pattern("/users/"), None);
    }

    #[test]
    fn test_extract_version_with_custom_pattern() {
        let handler = std::sync::Arc::new(DummyHandler);
        let route = Route::new("/api/v2/users/", handler).with_namespace("v2");

        assert_eq!(
            route.extract_version_from_pattern("/api/v{version}/"),
            Some("2")
        );
        assert_eq!(route.extract_version_from_pattern("/v{version}/"), None);
    }
}
