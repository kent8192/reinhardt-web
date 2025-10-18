/// URL reverse resolution
/// Inspired by Django's django.urls.reverse() function
///
/// This module provides both string-based (runtime) and type-safe (compile-time)
/// URL reversal mechanisms.
use crate::path;
use crate::{PathPattern, Route};
use reinhardt_exception::{Error, Result};
use std::collections::HashMap;
use std::marker::PhantomData;

// NOTE: エラーは`reinhardt_exception::Error`に統一
pub type ReverseError = Error;
pub type ReverseResult<T> = Result<T>;

/// URL reverser for resolving names back to URLs
/// Similar to Django's URLResolver reverse functionality
pub struct UrlReverser {
    /// Map of route names (including namespace) to routes
    routes: HashMap<String, Route>,
}

impl UrlReverser {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    /// Register a route for reverse lookup
    pub fn register(&mut self, route: Route) {
        if let Some(full_name) = route.full_name() {
            self.routes.insert(full_name, route);
        }
    }

    /// Reverse a URL name to a path with parameters
    /// Similar to Django's reverse() function
    ///
    /// # Arguments
    ///
    /// * `name` - The route name, optionally with namespace (e.g., "users:detail")
    /// * `params` - Map of parameter names to values
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::{UrlReverser, Route};
    /// use reinhardt_apps::Handler;
    /// use std::sync::Arc;
    /// use std::collections::HashMap;
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
    /// let mut reverser = UrlReverser::new();
    /// let route = Route::new("/users/{id}/", handler)
    ///     .with_name("detail")
    ///     .with_namespace("users");
    /// reverser.register(route);
    ///
    /// let mut params = HashMap::new();
    /// params.insert("id".to_string(), "123".to_string());
    ///
    /// let url = reverser.reverse("users:detail", &params).unwrap();
    /// assert_eq!(url, "/users/123/");
    /// ```
    pub fn reverse(&self, name: &str, params: &HashMap<String, String>) -> ReverseResult<String> {
        let route = self
            .routes
            .get(name)
            .ok_or_else(|| Error::NotFound(name.to_string()))?;

        // Parse the path pattern to find parameters
        let pattern = PathPattern::new(&route.path)
            .map_err(|e| Error::Validation(format!("pattern: {}", e)))?;

        let mut result = route.path.clone();

        // Replace each parameter in the path
        for param_name in pattern.param_names() {
            let value = params
                .get(param_name)
                .ok_or_else(|| Error::Validation(format!("missing param: {}", param_name)))?;

            // Replace {param_name} with the value
            let placeholder = format!("{{{}}}", param_name);
            result = result.replace(&placeholder, value);
        }

        Ok(result)
    }

    /// Reverse a URL name to a path with positional parameters
    /// Convenience method that takes a slice of key-value pairs
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_routers::{UrlReverser, Route};
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
    /// let mut reverser = UrlReverser::new();
    /// let route = Route::new("/users/{id}/", handler)
    ///     .with_name("detail");
    /// reverser.register(route);
    ///
    /// let url = reverser.reverse_with("detail", &[("id", "123")]).unwrap();
    /// assert_eq!(url, "/users/123/");
    /// ```
    pub fn reverse_with<S: AsRef<str>>(
        &self,
        name: &str,
        params: &[(S, S)],
    ) -> ReverseResult<String> {
        let params_map: HashMap<String, String> = params
            .iter()
            .map(|(k, v)| (k.as_ref().to_string(), v.as_ref().to_string()))
            .collect();

        self.reverse(name, &params_map)
    }

    /// Check if a route name is registered
    pub fn has_route(&self, name: &str) -> bool {
        self.routes.contains_key(name)
    }

    /// Get all registered route names
    pub fn route_names(&self) -> Vec<String> {
        self.routes.keys().cloned().collect()
    }
}

impl Default for UrlReverser {
    fn default() -> Self {
        Self::new()
    }
}

/// Standalone reverse function for convenience
/// Similar to Django's reverse() function
///
/// This requires routes to be registered with a global reverser.
/// For more control, use UrlReverser directly.
pub fn reverse(
    name: &str,
    params: &HashMap<String, String>,
    reverser: &UrlReverser,
) -> ReverseResult<String> {
    reverser.reverse(name, params)
}

// ============================================================================
// Type-safe URL reversal (compile-time checked)
// ============================================================================

/// Trait for URL patterns that can be reversed at compile time
///
/// Implement this trait for each URL pattern in your application.
/// The compiler will ensure that only valid URL patterns can be reversed.
///
/// # Example
///
/// ```rust
/// use reinhardt_routers::reverse::UrlPattern;
///
/// pub struct UserListUrl;
/// impl UrlPattern for UserListUrl {
///     const NAME: &'static str = "user-list";
///     const PATTERN: &'static str = "/users/";
/// }
/// ```
pub trait UrlPattern {
    /// The unique name for this URL pattern
    const NAME: &'static str;

    /// The URL pattern string
    const PATTERN: &'static str;
}

/// Trait for URL patterns with parameters
///
/// Use this for URLs that require path parameters.
///
/// # Example
///
/// ```rust
/// use reinhardt_routers::reverse::{UrlPattern, UrlPatternWithParams};
///
/// pub struct UserDetailUrl;
/// impl UrlPattern for UserDetailUrl {
///     const NAME: &'static str = "user-detail";
///     const PATTERN: &'static str = "/users/{id}/";
/// }
/// impl UrlPatternWithParams for UserDetailUrl {
///     const PARAMS: &'static [&'static str] = &["id"];
/// }
/// ```
pub trait UrlPatternWithParams: UrlPattern {
    /// The parameter names in order
    const PARAMS: &'static [&'static str];
}

/// Type-safe reverse for simple URL patterns (no parameters)
///
/// This function takes a type parameter implementing `UrlPattern`
/// and returns the URL string. Invalid patterns will fail at compile time.
///
/// # Example
///
/// ```rust
/// use reinhardt_routers::reverse::{reverse_typed, UrlPattern};
///
/// pub struct HomeUrl;
/// impl UrlPattern for HomeUrl {
///     const NAME: &'static str = "home";
///     const PATTERN: &'static str = "/";
/// }
///
/// let url = reverse_typed::<HomeUrl>();
/// assert_eq!(url, "/");
/// ```
pub fn reverse_typed<U: UrlPattern>() -> String {
    U::PATTERN.to_string()
}

/// Type-safe reverse for URL patterns with parameters
///
/// This function takes a type parameter and a HashMap of parameters,
/// substituting them into the URL pattern. Missing parameters will
/// result in a runtime error, but the pattern itself is compile-time checked.
///
/// # Example
///
/// ```rust
/// use reinhardt_routers::reverse::{reverse_typed_with_params, UrlPattern, UrlPatternWithParams};
/// use std::collections::HashMap;
///
/// pub struct UserDetailUrl;
/// impl UrlPattern for UserDetailUrl {
///     const NAME: &'static str = "user-detail";
///     const PATTERN: &'static str = "/users/{id}/";
/// }
/// impl UrlPatternWithParams for UserDetailUrl {
///     const PARAMS: &'static [&'static str] = &["id"];
/// }
///
/// let mut params = HashMap::new();
/// params.insert("id", "123");
/// let url = reverse_typed_with_params::<UserDetailUrl>(&params).unwrap();
/// assert_eq!(url, "/users/123/");
/// ```
pub fn reverse_typed_with_params<U: UrlPatternWithParams>(
    params: &HashMap<&str, &str>,
) -> ReverseResult<String> {
    let mut pattern = U::PATTERN.to_string();

    // Validate that all required parameters are provided
    for param_name in U::PARAMS {
        if !params.contains_key(param_name) {
            return Err(ReverseError::MissingParameter(param_name.to_string()));
        }

        let value = params[param_name];
        let placeholder = format!("{{{}}}", param_name);
        pattern = pattern.replace(&placeholder, value);
    }

    Ok(pattern)
}

/// Type-safe URL parameter builder
///
/// Provides a fluent API for building URL parameters with compile-time checking
/// of parameter names.
///
/// # Example
///
/// ```rust
/// use reinhardt_routers::reverse::{UrlParams, UrlPattern, UrlPatternWithParams};
///
/// pub struct UserDetailUrl;
/// impl UrlPattern for UserDetailUrl {
///     const NAME: &'static str = "user-detail";
///     const PATTERN: &'static str = "/users/{id}/";
/// }
/// impl UrlPatternWithParams for UserDetailUrl {
///     const PARAMS: &'static [&'static str] = &["id"];
/// }
///
/// let params = UrlParams::<UserDetailUrl>::new()
///     .param("id", "123")
///     .build()
///     .unwrap();
///
/// assert_eq!(params, "/users/123/");
/// ```
pub struct UrlParams<U: UrlPatternWithParams> {
    _phantom: PhantomData<U>,
    params: HashMap<String, String>,
}

impl<U: UrlPatternWithParams> UrlParams<U> {
    /// Create a new URL parameter builder
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
            params: HashMap::new(),
        }
    }

    /// Add a parameter (note: parameter name is not compile-time checked currently,
    /// but the pattern itself is)
    pub fn param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(name.into(), value.into());
        self
    }

    /// Build the URL string, checking that all required parameters are present
    pub fn build(self) -> ReverseResult<String> {
        let params_ref: HashMap<&str, &str> = self
            .params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        reverse_typed_with_params::<U>(&params_ref)
    }
}

impl<U: UrlPatternWithParams> Default for UrlParams<U> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Route;
    use async_trait::async_trait;
    use reinhardt_apps::{Handler, Request, Response, Result as CoreResult};
    use std::sync::Arc;

    /// // Simple test handler
    struct TestHandler;

    #[async_trait]
    impl Handler for TestHandler {
        async fn handle(&self, _request: Request) -> CoreResult<Response> {
            Ok(Response::ok())
        }
    }

    #[test]
    fn test_reverse_simple_path() {
        let mut reverser = UrlReverser::new();

        let route = Route::new(path!("/users/"), Arc::new(TestHandler)).with_name("users-list");

        reverser.register(route);

        let url = reverser.reverse("users-list", &HashMap::new()).unwrap();
        assert_eq!(url, path!("/users/"));
    }

    #[test]
    fn test_reverse_with_parameters() {
        let mut reverser = UrlReverser::new();

        let route =
            Route::new(path!("/users/{id}/"), Arc::new(TestHandler)).with_name("users-detail");

        reverser.register(route);

        let mut params = HashMap::new();
        params.insert("id".to_string(), "123".to_string());

        let url = reverser.reverse("users-detail", &params).unwrap();
        assert_eq!(url, "/users/123/");
    }

    #[test]
    fn test_reverse_with_namespace() {
        let mut reverser = UrlReverser::new();

        let route = Route::new(path!("/users/{id}/"), Arc::new(TestHandler))
            .with_name("detail")
            .with_namespace("users");

        reverser.register(route);

        let mut params = HashMap::new();
        params.insert("id".to_string(), "456".to_string());

        let url = reverser.reverse("users:detail", &params).unwrap();
        assert_eq!(url, "/users/456/");
    }

    #[test]
    fn test_reverse_missing_parameter() {
        let mut reverser = UrlReverser::new();

        let route =
            Route::new(path!("/users/{id}/"), Arc::new(TestHandler)).with_name("users-detail");

        reverser.register(route);

        let result = reverser.reverse("users-detail", &HashMap::new());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ReverseError::Validation(_)));
    }

    #[test]
    fn test_reverse_not_found() {
        let reverser = UrlReverser::new();

        let result = reverser.reverse("nonexistent", &HashMap::new());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ReverseError::NotFound(_)));
    }

    #[test]
    fn test_reverse_with_helper() {
        let mut reverser = UrlReverser::new();

        let route = Route::new(path!("/users/{id}/posts/{post_id}/"), Arc::new(TestHandler))
            .with_name("user-posts");

        reverser.register(route);

        let url = reverser
            .reverse_with("user-posts", &[("id", "123"), ("post_id", "456")])
            .unwrap();

        assert_eq!(url, "/users/123/posts/456/");
    }

    #[test]
    fn test_has_route() {
        let mut reverser = UrlReverser::new();

        let route = Route::new(path!("/users/"), Arc::new(TestHandler)).with_name("users-list");

        reverser.register(route);

        assert!(reverser.has_route("users-list"));
        assert!(!reverser.has_route("nonexistent"));
    }

    /// // Type-safe URL reversal tests
    struct HomeUrl;
    impl UrlPattern for HomeUrl {
        const NAME: &'static str = "home";
        const PATTERN: &'static str = reinhardt_routers_macros::path!("/");
    }

    struct UserListUrl;
    impl UrlPattern for UserListUrl {
        const NAME: &'static str = "user-list";
        const PATTERN: &'static str = reinhardt_routers_macros::path!("/users/");
    }

    struct UserDetailUrl;
    impl UrlPattern for UserDetailUrl {
        const NAME: &'static str = "user-detail";
        const PATTERN: &'static str = reinhardt_routers_macros::path!("/users/{id}/");
    }
    impl UrlPatternWithParams for UserDetailUrl {
        const PARAMS: &'static [&'static str] = &["id"];
    }

    struct PostDetailUrl;
    impl UrlPattern for PostDetailUrl {
        const NAME: &'static str = "post-detail";
        const PATTERN: &'static str =
            reinhardt_routers_macros::path!("/users/{user_id}/posts/{post_id}/");
    }
    impl UrlPatternWithParams for PostDetailUrl {
        const PARAMS: &'static [&'static str] = &["user_id", "post_id"];
    }

    #[test]
    fn test_typed_reverse_simple() {
        let url = reverse_typed::<HomeUrl>();
        assert_eq!(url, path!("/"));
    }

    #[test]
    fn test_typed_reverse_user_list() {
        let url = reverse_typed::<UserListUrl>();
        assert_eq!(url, path!("/users/"));
    }

    #[test]
    fn test_typed_reverse_with_params() {
        let mut params = HashMap::new();
        params.insert("id", "123");

        let url = reverse_typed_with_params::<UserDetailUrl>(&params).unwrap();
        assert_eq!(url, "/users/123/");
    }

    #[test]
    fn test_typed_reverse_with_multiple_params() {
        let mut params = HashMap::new();
        params.insert("user_id", "42");
        params.insert("post_id", "100");

        let url = reverse_typed_with_params::<PostDetailUrl>(&params).unwrap();
        assert_eq!(url, "/users/42/posts/100/");
    }

    #[test]
    fn test_typed_reverse_missing_param() {
        let params = HashMap::new();

        let result = reverse_typed_with_params::<UserDetailUrl>(&params);
        assert!(result.is_err());

        if let Err(ReverseError::MissingParameter(param)) = result {
            assert_eq!(param, "id");
        }
    }

    #[test]
    fn test_url_params_builder() {
        let url = UrlParams::<UserDetailUrl>::new()
            .param("id", "456")
            .build()
            .unwrap();

        assert_eq!(url, "/users/456/");
    }

    #[test]
    fn test_url_params_builder_multiple() {
        let url = UrlParams::<PostDetailUrl>::new()
            .param("user_id", "42")
            .param("post_id", "100")
            .build()
            .unwrap();

        assert_eq!(url, "/users/42/posts/100/");
    }

    #[test]
    fn test_url_params_builder_missing() {
        let result = UrlParams::<UserDetailUrl>::new().build();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ReverseError::MissingParameter(_)
        ));
    }
}
