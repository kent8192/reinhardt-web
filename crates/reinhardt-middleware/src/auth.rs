use async_trait::async_trait;
use reinhardt_apps::{Handler, Middleware, Request, Response, Result};
#[cfg(feature = "session")]
use reinhardt_auth::session::{SessionStore, SESSION_KEY_USER_ID};
use reinhardt_auth::{AnonymousUser, AuthenticationBackend, User};
use std::sync::Arc;

/// Authentication middleware
/// Extracts user information from session and attaches it to request extensions
///
/// This middleware integrates with tower/hyper to provide Django-style authentication
/// for Reinhardt applications. It automatically:
/// - Extracts session ID from cookies
/// - Loads user information from the session store
/// - Attaches user authentication state to request extensions
/// - Supports any authentication backend implementing `AuthenticationBackend`
///
/// # Examples
///
/// Basic usage with in-memory session store:
///
/// ```ignore
/// use std::sync::Arc;
/// use reinhardt_middleware::AuthenticationMiddleware;
/// use reinhardt_auth::session::InMemorySessionStore;
/// use reinhardt_types::MiddlewareChain;
/// # use reinhardt_apps::{Handler, Request, Response, Result};
/// # use reinhardt_auth::{AuthenticationBackend, AuthenticationError, User, SimpleUser};
/// # use async_trait::async_trait;
/// # use uuid::Uuid;
/// #
/// # struct MyHandler;
/// # #[async_trait]
/// # impl Handler for MyHandler {
/// #     async fn handle(&self, _request: Request) -> Result<Response> {
/// #         Ok(Response::ok())
/// #     }
/// # }
/// #
/// # // Simple test authentication backend
/// # struct TestAuthBackend;
/// # impl AuthenticationBackend for TestAuthBackend {
/// #     fn authenticate(&self, _request: &Request) -> std::result::Result<Option<Box<dyn User>>, AuthenticationError> {
/// #         Ok(Some(Box::new(SimpleUser {
/// #             id: Uuid::new_v4(),
/// #             username: "testuser".to_string(),
/// #             email: "test@example.com".to_string(),
/// #             is_active: true,
/// #             is_admin: false,
/// #             is_staff: false,
/// #             is_superuser: false,
/// #         })))
/// #     }
/// #     fn get_user(&self, _user_id: &str) -> std::result::Result<Option<Box<dyn User>>, AuthenticationError> {
/// #         Ok(None)
/// #     }
/// # }
///
/// // Create session store and authentication backend
/// let session_store = Arc::new(InMemorySessionStore::new());
/// let auth_backend = Arc::new(TestAuthBackend);
///
/// // Create authentication middleware
/// let auth_middleware = AuthenticationMiddleware::new(session_store, auth_backend);
///
/// // Wrap your handler with the middleware using MiddlewareChain
/// # let handler = Arc::new(MyHandler);
/// let app = MiddlewareChain::new(handler)
///     .with_middleware(Arc::new(auth_middleware));
/// ```
///
/// Accessing authentication state in handlers:
///
/// ```
/// # use reinhardt_apps::{Handler, Request, Response, Result};
/// # use async_trait::async_trait;
/// struct ProtectedHandler;
///
/// #[async_trait]
/// impl Handler for ProtectedHandler {
///     async fn handle(&self, request: Request) -> Result<Response> {
///         // Extract authentication state from request extensions
///         let is_authenticated: Option<bool> = request.extensions.get();
///         let user_id: Option<String> = request.extensions.get();
///         let is_admin: Option<bool> = request.extensions.get();
///
///         if !is_authenticated.unwrap_or(false) {
///             return Ok(Response::new(hyper::StatusCode::UNAUTHORIZED));
///         }
///
///         Ok(Response::ok().with_body(format!("Welcome user: {:?}", user_id)))
///     }
/// }
/// ```
#[cfg(feature = "session")]
pub struct AuthenticationMiddleware<S: SessionStore, A: AuthenticationBackend> {
    session_store: Arc<S>,
    auth_backend: Arc<A>,
}

#[cfg(feature = "session")]
impl<S: SessionStore, A: AuthenticationBackend> AuthenticationMiddleware<S, A> {
    /// Create a new authentication middleware
    ///
    /// # Arguments
    ///
    /// * `session_store` - Session storage backend
    /// * `auth_backend` - Authentication backend for user lookup
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use std::sync::Arc;
    /// use reinhardt_middleware::AuthenticationMiddleware;
    /// use reinhardt_auth::session::InMemorySessionStore;
    /// # use reinhardt_apps::Request;
    /// # use reinhardt_auth::{AuthenticationBackend, AuthenticationError, User, SimpleUser};
    /// # use uuid::Uuid;
    /// #
    /// # // Simple test authentication backend
    /// # struct TestAuthBackend;
/// # impl AuthenticationBackend for TestAuthBackend {
/// #     fn authenticate<'a>(&'a self, _request: &'a Request) -> std::result::Result<Option<Box<dyn User>>, AuthenticationError> {
/// #         Ok(Some(Box::new(SimpleUser {
/// #             id: Uuid::new_v4(),
/// #             username: "testuser".to_string(),
/// #             email: "test@example.com".to_string(),
/// #             is_active: true,
/// #             is_admin: false,
/// #             is_staff: false,
/// #             is_superuser: false,
/// #         })))
/// #     }
/// #     fn get_user<'a>(&'a self, _user_id: &'a str) -> std::result::Result<Option<Box<dyn User>>, AuthenticationError> {
/// #         Ok(None)
/// #     }
/// # }
    ///
    /// let session_store = Arc::new(InMemorySessionStore::new());
    /// let auth_backend = Arc::new(TestAuthBackend);
    /// let middleware = AuthenticationMiddleware::new(session_store, auth_backend);
    /// ```
    pub fn new(session_store: Arc<S>, auth_backend: Arc<A>) -> Self {
        Self {
            session_store,
            auth_backend,
        }
    }

    /// Extract session ID from cookies
    fn extract_session_id(&self, request: &Request) -> Option<String> {
        const SESSION_COOKIE_NAME: &str = "sessionid";
        request
            .headers
            .get("cookie")
            .and_then(|v| v.to_str().ok())
            .and_then(|cookies| {
                cookies.split(';').find_map(|cookie| {
                    let mut parts = cookie.trim().split('=');
                    if parts.next()? == SESSION_COOKIE_NAME {
                        Some(parts.next()?.to_string())
                    } else {
                        None
                    }
                })
            })
    }

    /// Get user from session
    async fn get_user_from_session(&self, session_id: &String) -> Option<Box<dyn User>> {
        if let Some(session) = self.session_store.load(session_id).await {
            if let Some(user_id_value) = session.get(SESSION_KEY_USER_ID) {
                if let Some(user_id) = user_id_value.as_str() {
                    if let Ok(Some(user)) = self.auth_backend.get_user(user_id).await {
                        return Some(user);
                    }
                }
            }
        }
        None
    }
}

#[cfg(feature = "session")]
#[async_trait]
impl<S: SessionStore + 'static, A: AuthenticationBackend + 'static> Middleware
    for AuthenticationMiddleware<S, A>
{
    async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response> {
        let user: Box<dyn User> = if let Some(ref session_id) = self.extract_session_id(&request) {
            self.get_user_from_session(session_id)
                .await
                .unwrap_or_else(|| Box::new(AnonymousUser))
        } else {
            Box::new(AnonymousUser)
        };

        let is_authenticated = user.is_authenticated();
        let is_admin = user.is_admin();
        let is_active = user.is_active();
        let user_id = user.id();

        request.extensions.insert(user_id);
        request.extensions.insert(is_authenticated);
        request.extensions.insert(is_admin);
        request.extensions.insert(is_active);

        next.handle(request).await
    }
}

/// Helper struct to store authentication state in request extensions
#[derive(Clone, Debug)]
pub struct AuthState {
    pub user_id: String,
    pub is_authenticated: bool,
    pub is_admin: bool,
    pub is_active: bool,
}

impl AuthState {
    /// Create auth state from request extensions
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_middleware::auth::AuthState;
    /// use reinhardt_http::Extensions;
    ///
    /// let extensions = Extensions::new();
    /// extensions.insert("user123".to_string());
    /// extensions.insert(true); // is_authenticated
    /// extensions.insert(false); // is_admin
    /// extensions.insert(true); // is_active
    ///
    /// let auth_state = AuthState::from_extensions(&extensions);
    /// assert!(auth_state.is_some());
    /// ```
    pub fn from_extensions(extensions: &reinhardt_http::Extensions) -> Option<Self> {
        Some(Self {
            user_id: extensions.get::<String>()?,
            is_authenticated: extensions.get::<bool>()?,
            is_admin: false,
            is_active: false,
        })
    }

    /// Check if user is anonymous
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_middleware::auth::AuthState;
    ///
    /// let anon_state = AuthState {
    ///     user_id: String::new(),
    ///     is_authenticated: false,
    ///     is_admin: false,
    ///     is_active: false,
    /// };
    ///
    /// assert!(anon_state.is_anonymous());
    /// ```
    pub fn is_anonymous(&self) -> bool {
        !self.is_authenticated
    }
}

#[cfg(all(test, feature = "session"))]
mod tests {
    use super::*;
    use bytes::Bytes;
    use hyper::{HeaderMap, Method, Uri, Version};
    use reinhardt_auth::session::{InMemorySessionStore, Session};
    use reinhardt_auth::user::SimpleUser;
    use reinhardt_auth::AuthenticationError;
    use uuid::Uuid;

    struct TestHandler;

    #[async_trait]
    impl Handler for TestHandler {
        async fn handle(&self, request: Request) -> Result<Response> {
            let user_id: Option<String> = request.extensions.get();
            let is_authenticated: Option<bool> = request.extensions.get();

            Ok(Response::ok().with_json(&serde_json::json!({
                "user_id": user_id.unwrap_or_default(),
                "is_authenticated": is_authenticated.unwrap_or(false)
            }))?)
        }
    }

    struct TestAuthBackend {
        user: Option<SimpleUser>,
    }

    #[async_trait::async_trait]
    impl AuthenticationBackend for TestAuthBackend {
        async fn authenticate(
            &self,
            _request: &Request,
        ) -> std::result::Result<Option<Box<dyn User>>, AuthenticationError> {
            Ok(self
                .user
                .as_ref()
                .map(|u| Box::new(u.clone()) as Box<dyn User>))
        }

        async fn get_user(
            &self,
            _user_id: &str,
        ) -> std::result::Result<Option<Box<dyn User>>, AuthenticationError> {
            Ok(self
                .user
                .as_ref()
                .map(|u| Box::new(u.clone()) as Box<dyn User>))
        }
    }

    #[tokio::test]
    async fn test_auth_middleware_with_valid_session() {
        let session_store = Arc::new(InMemorySessionStore::new());
        let user = SimpleUser {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            is_active: true,
            is_admin: false,
            is_staff: false,
            is_superuser: false,
        };
        let auth_backend = Arc::new(TestAuthBackend { user: Some(user) });

        let session_id = session_store.create_session_id();
        let mut session = Session::new();
        session.set(SESSION_KEY_USER_ID, serde_json::json!("user123"));
        session_store.save(&session_id, &session).await;

        let middleware = AuthenticationMiddleware::new(session_store, auth_backend);
        let handler = Arc::new(TestHandler);

        let mut headers = HeaderMap::new();
        headers.insert(
            "cookie",
            format!("sessionid={}", session_id).parse().unwrap(),
        );

        let request = Request::new(
            Method::GET,
            Uri::from_static("/test"),
            Version::HTTP_11,
            headers,
            Bytes::new(),
        );

        let response = middleware.process(request, handler).await.unwrap();
        assert_eq!(response.status, reinhardt_http::Response::ok().status);
    }

    #[tokio::test]
    async fn test_auth_middleware_without_session() {
        let session_store = Arc::new(InMemorySessionStore::new());
        let auth_backend = Arc::new(TestAuthBackend { user: None });

        let middleware = AuthenticationMiddleware::new(session_store, auth_backend);
        let handler = Arc::new(TestHandler);

        let request = Request::new(
            Method::GET,
            Uri::from_static("/test"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let response = middleware.process(request, handler).await.unwrap();
        assert_eq!(response.status, reinhardt_http::Response::ok().status);

        let body_str = String::from_utf8(response.body.to_vec()).unwrap();
        assert!(body_str.contains("\"is_authenticated\":false"));
    }

    #[test]
    fn test_auth_state_from_extensions() {
        let extensions = reinhardt_http::Extensions::new();
        extensions.insert("user123".to_string());
        extensions.insert(true);

        let auth_state = AuthState::from_extensions(&extensions);
        assert!(auth_state.is_some());
        assert!(!auth_state.unwrap().is_anonymous());
    }

    #[test]
    fn test_auth_state_is_anonymous() {
        let anon_state = AuthState {
            user_id: String::new(),
            is_authenticated: false,
            is_admin: false,
            is_active: false,
        };

        assert!(anon_state.is_anonymous());

        let auth_state = AuthState {
            user_id: "user123".to_string(),
            is_authenticated: true,
            is_admin: false,
            is_active: true,
        };

        assert!(!auth_state.is_anonymous());
    }
}
