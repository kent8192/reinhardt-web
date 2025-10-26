//! Django REST Framework Compatible Permissions
//!
//! Provides DRF-compatible permission classes for common authorization scenarios.

use crate::permissions::{Permission, PermissionContext};
use async_trait::async_trait;

/// Allow any request (DRF compatible)
///
/// This permission class allows unrestricted access regardless of authentication status.
///
/// # Examples
///
/// ```
/// use reinhardt_auth::drf_permissions::DrfAllowAny;
/// use reinhardt_auth::permissions::{Permission, PermissionContext};
/// use bytes::Bytes;
/// use hyper::{HeaderMap, Method, Uri, Version};
/// use reinhardt_types::Request;
///
/// #[tokio::main]
/// async fn main() {
///     let permission = DrfAllowAny;
///     let request = Request::new(
///         Method::GET,
///         Uri::from_static("/"),
///         Version::HTTP_11,
///         HeaderMap::new(),
///         Bytes::new(),
///     );
///
///     let context = PermissionContext {
///         request: &request,
///         is_authenticated: false,
///         is_admin: false,
///         is_active: false,
///         user: None,
///     };
///
///     assert!(permission.has_permission(&context).await);
/// }
/// ```
pub struct DrfAllowAny;

#[async_trait]
impl Permission for DrfAllowAny {
    async fn has_permission(&self, _context: &PermissionContext<'_>) -> bool {
        true
    }
}

/// Require authenticated user (DRF compatible)
///
/// This permission class requires the user to be authenticated.
///
/// # Examples
///
/// ```
/// use reinhardt_auth::drf_permissions::DrfIsAuthenticated;
/// use reinhardt_auth::permissions::{Permission, PermissionContext};
/// use bytes::Bytes;
/// use hyper::{HeaderMap, Method, Uri, Version};
/// use reinhardt_types::Request;
///
/// #[tokio::main]
/// async fn main() {
///     let permission = DrfIsAuthenticated;
///     let request = Request::new(
///         Method::GET,
///         Uri::from_static("/"),
///         Version::HTTP_11,
///         HeaderMap::new(),
///         Bytes::new(),
///     );
///
///     // Authenticated user
///     let context = PermissionContext {
///         request: &request,
///         is_authenticated: true,
///         is_admin: false,
///         is_active: true,
///         user: None,
///     };
///     assert!(permission.has_permission(&context).await);
///
///     // Unauthenticated user
///     let context = PermissionContext {
///         request: &request,
///         is_authenticated: false,
///         is_admin: false,
///         is_active: false,
///         user: None,
///     };
///     assert!(!permission.has_permission(&context).await);
/// }
/// ```
pub struct DrfIsAuthenticated;

#[async_trait]
impl Permission for DrfIsAuthenticated {
    async fn has_permission(&self, context: &PermissionContext<'_>) -> bool {
        context.is_authenticated
    }
}

/// Require admin user (DRF compatible)
///
/// This permission class requires the user to be both authenticated and an administrator.
///
/// # Examples
///
/// ```
/// use reinhardt_auth::drf_permissions::DrfIsAdminUser;
/// use reinhardt_auth::permissions::{Permission, PermissionContext};
/// use bytes::Bytes;
/// use hyper::{HeaderMap, Method, Uri, Version};
/// use reinhardt_types::Request;
///
/// #[tokio::main]
/// async fn main() {
///     let permission = DrfIsAdminUser;
///     let request = Request::new(
///         Method::GET,
///         Uri::from_static("/"),
///         Version::HTTP_11,
///         HeaderMap::new(),
///         Bytes::new(),
///     );
///
///     // Admin user
///     let context = PermissionContext {
///         request: &request,
///         is_authenticated: true,
///         is_admin: true,
///         is_active: true,
///         user: None,
///     };
///     assert!(permission.has_permission(&context).await);
///
///     // Non-admin user
///     let context = PermissionContext {
///         request: &request,
///         is_authenticated: true,
///         is_admin: false,
///         is_active: true,
///         user: None,
///     };
///     assert!(!permission.has_permission(&context).await);
/// }
/// ```
pub struct DrfIsAdminUser;

#[async_trait]
impl Permission for DrfIsAdminUser {
    async fn has_permission(&self, context: &PermissionContext<'_>) -> bool {
        context.is_authenticated && context.is_admin
    }
}

/// Authenticated for write, read-only for unauthenticated (DRF compatible)
///
/// This permission allows read operations (GET, HEAD, OPTIONS) for any request,
/// but requires authentication for write operations (POST, PUT, PATCH, DELETE).
///
/// # Examples
///
/// ```
/// use reinhardt_auth::drf_permissions::DrfIsAuthenticatedOrReadOnly;
/// use reinhardt_auth::permissions::{Permission, PermissionContext};
/// use bytes::Bytes;
/// use hyper::{HeaderMap, Method, Uri, Version};
/// use reinhardt_types::Request;
///
/// #[tokio::main]
/// async fn main() {
///     let permission = DrfIsAuthenticatedOrReadOnly;
///
///     // GET request - allowed for unauthenticated
///     let get_request = Request::new(
///         Method::GET,
///         Uri::from_static("/"),
///         Version::HTTP_11,
///         HeaderMap::new(),
///         Bytes::new(),
///     );
///     let context = PermissionContext {
///         request: &get_request,
///         is_authenticated: false,
///         is_admin: false,
///         is_active: false,
///         user: None,
///     };
///     assert!(permission.has_permission(&context).await);
///
///     // POST request - requires authentication
///     let post_request = Request::new(
///         Method::POST,
///         Uri::from_static("/"),
///         Version::HTTP_11,
///         HeaderMap::new(),
///         Bytes::new(),
///     );
///     let context = PermissionContext {
///         request: &post_request,
///         is_authenticated: false,
///         is_admin: false,
///         is_active: false,
///         user: None,
///     };
///     assert!(!permission.has_permission(&context).await);
/// }
/// ```
pub struct DrfIsAuthenticatedOrReadOnly;

#[async_trait]
impl Permission for DrfIsAuthenticatedOrReadOnly {
    async fn has_permission(&self, context: &PermissionContext<'_>) -> bool {
        if context.is_authenticated {
            return true;
        }

        matches!(context.request.method.as_str(), "GET" | "HEAD" | "OPTIONS")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use hyper::{HeaderMap, Method, Uri, Version};
    use reinhardt_types::Request;

    #[tokio::test]
    async fn test_drf_allow_any() {
        let permission = DrfAllowAny;
        let request = Request::new(
            Method::GET,
            Uri::from_static("/"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let context = PermissionContext {
            request: &request,
            is_authenticated: false,
            is_admin: false,
            is_active: false,
            user: None,
        };

        assert!(permission.has_permission(&context).await);
    }

    #[tokio::test]
    async fn test_drf_is_authenticated_success() {
        let permission = DrfIsAuthenticated;
        let request = Request::new(
            Method::GET,
            Uri::from_static("/"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let context = PermissionContext {
            request: &request,
            is_authenticated: true,
            is_admin: false,
            is_active: true,
            user: None,
        };

        assert!(permission.has_permission(&context).await);
    }

    #[tokio::test]
    async fn test_drf_is_authenticated_failure() {
        let permission = DrfIsAuthenticated;
        let request = Request::new(
            Method::GET,
            Uri::from_static("/"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let context = PermissionContext {
            request: &request,
            is_authenticated: false,
            is_admin: false,
            is_active: false,
            user: None,
        };

        assert!(!permission.has_permission(&context).await);
    }

    #[tokio::test]
    async fn test_drf_is_admin_user_success() {
        let permission = DrfIsAdminUser;
        let request = Request::new(
            Method::GET,
            Uri::from_static("/"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let context = PermissionContext {
            request: &request,
            is_authenticated: true,
            is_admin: true,
            is_active: true,
            user: None,
        };

        assert!(permission.has_permission(&context).await);
    }

    #[tokio::test]
    async fn test_drf_is_admin_user_not_admin() {
        let permission = DrfIsAdminUser;
        let request = Request::new(
            Method::GET,
            Uri::from_static("/"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let context = PermissionContext {
            request: &request,
            is_authenticated: true,
            is_admin: false,
            is_active: true,
            user: None,
        };

        assert!(!permission.has_permission(&context).await);
    }

    #[tokio::test]
    async fn test_drf_is_admin_user_not_authenticated() {
        let permission = DrfIsAdminUser;
        let request = Request::new(
            Method::GET,
            Uri::from_static("/"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let context = PermissionContext {
            request: &request,
            is_authenticated: false,
            is_admin: true,
            is_active: false,
            user: None,
        };

        assert!(!permission.has_permission(&context).await);
    }

    #[tokio::test]
    async fn test_drf_is_authenticated_or_read_only_get() {
        let permission = DrfIsAuthenticatedOrReadOnly;
        let request = Request::new(
            Method::GET,
            Uri::from_static("/"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let context = PermissionContext {
            request: &request,
            is_authenticated: false,
            is_admin: false,
            is_active: false,
            user: None,
        };

        assert!(permission.has_permission(&context).await);
    }

    #[tokio::test]
    async fn test_drf_is_authenticated_or_read_only_head() {
        let permission = DrfIsAuthenticatedOrReadOnly;
        let request = Request::new(
            Method::HEAD,
            Uri::from_static("/"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let context = PermissionContext {
            request: &request,
            is_authenticated: false,
            is_admin: false,
            is_active: false,
            user: None,
        };

        assert!(permission.has_permission(&context).await);
    }

    #[tokio::test]
    async fn test_drf_is_authenticated_or_read_only_options() {
        let permission = DrfIsAuthenticatedOrReadOnly;
        let request = Request::new(
            Method::OPTIONS,
            Uri::from_static("/"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let context = PermissionContext {
            request: &request,
            is_authenticated: false,
            is_admin: false,
            is_active: false,
            user: None,
        };

        assert!(permission.has_permission(&context).await);
    }

    #[tokio::test]
    async fn test_drf_is_authenticated_or_read_only_post_unauthenticated() {
        let permission = DrfIsAuthenticatedOrReadOnly;
        let request = Request::new(
            Method::POST,
            Uri::from_static("/"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let context = PermissionContext {
            request: &request,
            is_authenticated: false,
            is_admin: false,
            is_active: false,
            user: None,
        };

        assert!(!permission.has_permission(&context).await);
    }

    #[tokio::test]
    async fn test_drf_is_authenticated_or_read_only_post_authenticated() {
        let permission = DrfIsAuthenticatedOrReadOnly;
        let request = Request::new(
            Method::POST,
            Uri::from_static("/"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let context = PermissionContext {
            request: &request,
            is_authenticated: true,
            is_admin: false,
            is_active: true,
            user: None,
        };

        assert!(permission.has_permission(&context).await);
    }
}
