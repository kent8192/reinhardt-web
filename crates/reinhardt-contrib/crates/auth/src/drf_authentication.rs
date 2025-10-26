//! Django REST Framework-style Authentication
//!
//! Provides DRF-compatible authentication wrappers and combinators.

use crate::{AuthenticationBackend, AuthenticationError, SimpleUser, User};
use reinhardt_apps::Request;
use std::sync::Arc;

/// DRF-style authentication trait wrapper
///
/// Provides a Django REST Framework-compatible interface for authentication.
#[async_trait::async_trait]
pub trait Authentication: Send + Sync {
    /// Authenticate a request and return a user if successful
    async fn authenticate(
        &self,
        request: &Request,
    ) -> Result<Option<Box<dyn User>>, AuthenticationError>;
}

/// Basic authentication configuration
#[derive(Debug, Clone)]
pub struct BasicAuthConfig {
    /// Realm for WWW-Authenticate header
    pub realm: String,
}

impl Default for BasicAuthConfig {
    fn default() -> Self {
        Self {
            realm: "api".to_string(),
        }
    }
}

/// Session authentication configuration
#[derive(Debug, Clone)]
pub struct SessionAuthConfig {
    /// Session cookie name
    pub cookie_name: String,
    /// Whether to enforce CSRF protection
    pub enforce_csrf: bool,
}

impl Default for SessionAuthConfig {
    fn default() -> Self {
        Self {
            cookie_name: "sessionid".to_string(),
            enforce_csrf: true,
        }
    }
}

/// Token authentication configuration
#[derive(Debug, Clone)]
pub struct TokenAuthConfig {
    /// Token header name (default: "Authorization")
    pub header_name: String,
    /// Token prefix (default: "Token")
    pub prefix: String,
}

impl Default for TokenAuthConfig {
    fn default() -> Self {
        Self {
            header_name: "Authorization".to_string(),
            prefix: "Token".to_string(),
        }
    }
}

/// Composite authentication backend
///
/// Tries multiple authentication methods in sequence, similar to Django REST Framework.
///
/// # Examples
///
/// ```
/// use reinhardt_auth::{CompositeAuthentication, SessionAuthentication, TokenAuthentication};
/// use std::sync::Arc;
///
/// let mut auth = CompositeAuthentication::new();
/// auth.add_backend(Arc::new(SessionAuthentication::new()));
/// auth.add_backend(Arc::new(TokenAuthentication::new()));
/// ```
pub struct CompositeAuthentication {
    backends: Vec<Arc<dyn Authentication>>,
}

impl CompositeAuthentication {
    /// Create a new composite authentication backend
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_auth::CompositeAuthentication;
    ///
    /// let auth = CompositeAuthentication::new();
    /// ```
    pub fn new() -> Self {
        Self {
            backends: Vec::new(),
        }
    }

    /// Add an authentication backend
    ///
    /// Backends are tried in the order they are added.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_auth::{CompositeAuthentication, TokenAuthentication};
    /// use std::sync::Arc;
    ///
    /// let mut auth = CompositeAuthentication::new();
    /// auth.add_backend(Arc::new(TokenAuthentication::new()));
    /// ```
    pub fn add_backend(&mut self, backend: Arc<dyn Authentication>) {
        self.backends.push(backend);
    }

    /// Add multiple backends at once
    pub fn add_backends(&mut self, backends: Vec<Arc<dyn Authentication>>) {
        self.backends.extend(backends);
    }
}

impl Default for CompositeAuthentication {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Authentication for CompositeAuthentication {
    async fn authenticate(
        &self,
        request: &Request,
    ) -> Result<Option<Box<dyn User>>, AuthenticationError> {
        // Try each backend in order
        for backend in &self.backends {
            match backend.authenticate(request).await {
                Ok(Some(user)) => return Ok(Some(user)),
                Ok(None) => continue,
                Err(e) => {
                    // Log error but continue to next backend
                    eprintln!("Authentication backend error: {}", e);
                    continue;
                }
            }
        }
        Ok(None)
    }
}

#[async_trait::async_trait]
impl AuthenticationBackend for CompositeAuthentication {
    async fn authenticate(
        &self,
        request: &Request,
    ) -> Result<Option<Box<dyn User>>, AuthenticationError> {
        Authentication::authenticate(self, request).await
    }

    async fn get_user(&self, _user_id: &str) -> Result<Option<Box<dyn User>>, AuthenticationError> {
        // For composite auth, we can't determine which backend to use
        // In a real implementation, we'd need to track which backend authenticated the user
        Ok(None)
    }
}

/// Token authentication using custom tokens
pub struct TokenAuthentication {
    /// Token store (token -> user_id)
    tokens: std::collections::HashMap<String, String>,
    /// Configuration
    config: TokenAuthConfig,
}

impl TokenAuthentication {
    /// Create a new token authentication backend
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_auth::TokenAuthentication;
    ///
    /// let auth = TokenAuthentication::new();
    /// ```
    pub fn new() -> Self {
        Self {
            tokens: std::collections::HashMap::new(),
            config: TokenAuthConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: TokenAuthConfig) -> Self {
        Self {
            tokens: std::collections::HashMap::new(),
            config,
        }
    }

    /// Add a token for a user
    pub fn add_token(&mut self, token: impl Into<String>, user_id: impl Into<String>) {
        self.tokens.insert(token.into(), user_id.into());
    }
}

impl Default for TokenAuthentication {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Authentication for TokenAuthentication {
    async fn authenticate(
        &self,
        request: &Request,
    ) -> Result<Option<Box<dyn User>>, AuthenticationError> {
        let auth_header = request
            .headers
            .get(&self.config.header_name)
            .and_then(|h| h.to_str().ok());

        if let Some(header) = auth_header {
            let prefix = format!("{} ", self.config.prefix);
            if let Some(token) = header.strip_prefix(&prefix) {
                if let Some(user_id) = self.tokens.get(token) {
                    return Ok(Some(Box::new(SimpleUser {
                        id: uuid::Uuid::new_v4(),
                        username: user_id.clone(),
                        email: format!("{}@example.com", user_id),
                        is_active: true,
                        is_admin: false,
                    })));
                }
            }
        }

        Ok(None)
    }
}

#[async_trait::async_trait]
impl AuthenticationBackend for TokenAuthentication {
    async fn authenticate(
        &self,
        request: &Request,
    ) -> Result<Option<Box<dyn User>>, AuthenticationError> {
        Authentication::authenticate(self, request).await
    }

    async fn get_user(&self, user_id: &str) -> Result<Option<Box<dyn User>>, AuthenticationError> {
        if self.tokens.values().any(|id| id == user_id) {
            Ok(Some(Box::new(SimpleUser {
                id: uuid::Uuid::new_v4(),
                username: user_id.to_string(),
                email: format!("{}@example.com", user_id),
                is_active: true,
                is_admin: false,
            })))
        } else {
            Ok(None)
        }
    }
}

/// Remote user authentication (from upstream proxy)
pub struct RemoteUserAuthentication {
    /// Header name to check
    header_name: String,
}

impl RemoteUserAuthentication {
    /// Create a new remote user authentication backend
    pub fn new() -> Self {
        Self {
            header_name: "REMOTE_USER".to_string(),
        }
    }

    /// Set custom header name
    pub fn with_header(mut self, header: impl Into<String>) -> Self {
        self.header_name = header.into();
        self
    }
}

impl Default for RemoteUserAuthentication {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Authentication for RemoteUserAuthentication {
    async fn authenticate(
        &self,
        request: &Request,
    ) -> Result<Option<Box<dyn User>>, AuthenticationError> {
        let header_value = request
            .headers
            .get(&self.header_name)
            .and_then(|v| v.to_str().ok());

        if let Some(username) = header_value {
            if !username.is_empty() {
                return Ok(Some(Box::new(SimpleUser {
                    id: uuid::Uuid::new_v4(),
                    username: username.to_string(),
                    email: format!("{}@example.com", username),
                    is_active: true,
                    is_admin: false,
                })));
            }
        }

        Ok(None)
    }
}

#[async_trait::async_trait]
impl AuthenticationBackend for RemoteUserAuthentication {
    async fn authenticate(
        &self,
        request: &Request,
    ) -> Result<Option<Box<dyn User>>, AuthenticationError> {
        Authentication::authenticate(self, request).await
    }

    async fn get_user(&self, _user_id: &str) -> Result<Option<Box<dyn User>>, AuthenticationError> {
        Ok(None)
    }
}

/// Session-based authentication
pub struct SessionAuthentication {
    /// Configuration
    config: SessionAuthConfig,
}

impl SessionAuthentication {
    /// Create a new session authentication backend
    pub fn new() -> Self {
        Self {
            config: SessionAuthConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: SessionAuthConfig) -> Self {
        Self { config }
    }
}

impl Default for SessionAuthentication {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Authentication for SessionAuthentication {
    async fn authenticate(
        &self,
        request: &Request,
    ) -> Result<Option<Box<dyn User>>, AuthenticationError> {
        // Check for session cookie
        let cookie_header = request.headers.get("Cookie").and_then(|h| h.to_str().ok());

        if let Some(cookies) = cookie_header {
            for cookie in cookies.split(';') {
                let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                if parts.len() == 2 && parts[0] == self.config.cookie_name {
                    // In a real implementation, we'd validate the session
                    // For now, just return a user if session exists
                    return Ok(Some(Box::new(SimpleUser {
                        id: uuid::Uuid::new_v4(),
                        username: "session_user".to_string(),
                        email: "session@example.com".to_string(),
                        is_active: true,
                        is_admin: false,
                    })));
                }
            }
        }

        Ok(None)
    }
}

#[async_trait::async_trait]
impl AuthenticationBackend for SessionAuthentication {
    async fn authenticate(
        &self,
        request: &Request,
    ) -> Result<Option<Box<dyn User>>, AuthenticationError> {
        Authentication::authenticate(self, request).await
    }

    async fn get_user(&self, _user_id: &str) -> Result<Option<Box<dyn User>>, AuthenticationError> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JwtAuth;
    use crate::basic::BasicAuthentication;
    use bytes::Bytes;
    use hyper::{HeaderMap, Method, Uri, Version};

    #[tokio::test]
    async fn test_composite_authentication() {
        let mut composite = CompositeAuthentication::new();

        let mut basic = BasicAuthentication::new();
        basic.add_user("user1", "pass1");

        let jwt = JwtAuth::new(b"secret");

        composite.add_backend(Arc::new(jwt));
        composite.add_backend(Arc::new(basic));

        // Test with basic auth
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            "Basic dXNlcjE6cGFzczE=".parse().unwrap(), // user1:pass1
        );

        let request = Request::new(
            Method::GET,
            Uri::from_static("/"),
            Version::HTTP_11,
            headers,
            Bytes::new(),
        );

        let result = Authentication::authenticate(&composite, &request)
            .await
            .unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().get_username(), "user1");
    }

    #[tokio::test]
    async fn test_composite_authentication_with_jwt() {
        let mut composite = CompositeAuthentication::new();

        let mut basic = BasicAuthentication::new();
        basic.add_user("user1", "pass1");

        let jwt_secret = b"test_secret_key";
        let jwt = JwtAuth::new(jwt_secret);
        let jwt_for_backend = JwtAuth::new(jwt_secret);

        composite.add_backend(Arc::new(jwt_for_backend));
        composite.add_backend(Arc::new(basic));

        // Generate a JWT token
        let token = jwt
            .generate_token("user123".to_string(), "testuser".to_string())
            .unwrap();

        // Test with JWT Bearer token
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );

        let request = Request::new(
            Method::GET,
            Uri::from_static("/"),
            Version::HTTP_11,
            headers,
            Bytes::new(),
        );

        let result = Authentication::authenticate(&composite, &request)
            .await
            .unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().get_username(), "testuser");
    }

    #[tokio::test]
    async fn test_token_authentication() {
        let mut auth = TokenAuthentication::new();
        auth.add_token("secret_token", "alice");

        let mut headers = HeaderMap::new();
        headers.insert("Authorization", "Token secret_token".parse().unwrap());

        let request = Request::new(
            Method::GET,
            Uri::from_static("/"),
            Version::HTTP_11,
            headers,
            Bytes::new(),
        );

        let result = Authentication::authenticate(&auth, &request).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().get_username(), "alice");
    }

    #[tokio::test]
    async fn test_remote_user_authentication() {
        let auth = RemoteUserAuthentication::new();

        let mut headers = HeaderMap::new();
        headers.insert("REMOTE_USER", "bob".parse().unwrap());

        let request = Request::new(
            Method::GET,
            Uri::from_static("/"),
            Version::HTTP_11,
            headers,
            Bytes::new(),
        );

        let result = Authentication::authenticate(&auth, &request).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().get_username(), "bob");
    }

    #[tokio::test]
    async fn test_session_authentication() {
        let auth = SessionAuthentication::new();

        let mut headers = HeaderMap::new();
        headers.insert("Cookie", "sessionid=abc123".parse().unwrap());

        let request = Request::new(
            Method::GET,
            Uri::from_static("/"),
            Version::HTTP_11,
            headers,
            Bytes::new(),
        );

        let result = Authentication::authenticate(&auth, &request).await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_custom_token_config() {
        let config = TokenAuthConfig {
            header_name: "X-API-Key".to_string(),
            prefix: "Bearer".to_string(),
        };

        let mut auth = TokenAuthentication::with_config(config);
        auth.add_token("my_token", "charlie");

        let mut headers = HeaderMap::new();
        headers.insert("X-API-Key", "Bearer my_token".parse().unwrap());

        let request = Request::new(
            Method::GET,
            Uri::from_static("/"),
            Version::HTTP_11,
            headers,
            Bytes::new(),
        );

        let result = Authentication::authenticate(&auth, &request).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().get_username(), "charlie");
    }
}
