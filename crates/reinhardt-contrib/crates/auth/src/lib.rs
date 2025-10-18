pub mod advanced_permissions;
pub mod backend;
pub mod basic;
pub mod di_support;
pub mod drf_authentication;
pub mod drf_permissions;
pub mod jwt;
pub mod mfa;
pub mod model_permissions;
pub mod oauth2;
pub mod permission_operators;
pub mod permissions;
pub mod remote_user;
pub mod session;
pub mod token_blacklist;
pub mod user;

// pub use advanced_permissions::{
//     InMemoryRateLimitStore, IpBlacklistPermission, IpWhitelistPermission, RateLimitAction,
//     RateLimitConfig, RateLimitPermission, RateLimitStore, TimeBasedPermission, TimeWindow,
// };
pub use backend::{Argon2Hasher, AuthBackend, CompositeAuthBackend, PasswordHasher};
pub use basic::BasicAuthentication as HttpBasicAuth;
// pub use drf_authentication::{
//     AuthUser, Authentication, BasicAuthConfig, BasicAuthentication, JWTAuthentication,
//     RemoteUserAuthentication, SessionAuthConfig, SessionAuthentication, TokenAuthConfig,
//     TokenAuthentication,
// };
// pub use drf_permissions::{
//     AllowAny as DrfAllowAny, IsAdminUser as DrfIsAdminUser, IsAuthenticated as DrfIsAuthenticated,
//     Permission as DrfPermission,
// };
pub use jwt::{Claims, JwtAuth};
// pub use mfa::{BackupCode, InMemoryMfaStore, MfaDeviceStore, MfaManager, MfaMethod, TotpDevice};
// pub use model_permissions::{
//     DjangoModelPermissions, DjangoModelPermissionsOrAnonReadOnly, ModelPermission, PermissionError,
//     PermissionResult,
// };
// pub use oauth2::{
//     AccessToken, AuthorizationCode, GrantType, InMemoryOAuth2Store, OAuth2Application,
//     OAuth2Authentication, OAuth2Token, OAuth2TokenStore,
// };
// pub use permission_operators::{combinators, AndPermission, NotPermission, OrPermission};
pub use permissions::{
    AllowAny, IsActiveUser, IsAdminUser, IsAuthenticated, IsAuthenticatedOrReadOnly, Permission,
    PermissionContext,
};
// pub use remote_user::{
//     PersistentRemoteUserAuthentication, RemoteUserAuthentication as RemoteUserAuth,
// };
// pub use session::{SessionAuthentication as SessionAuth, SessionStore};
// pub use token_blacklist::{
//     BlacklistReason, BlacklistStats, BlacklistedToken, InMemoryRefreshTokenStore,
//     InMemoryTokenBlacklist, RefreshToken, RefreshTokenStore, TokenBlacklist, TokenRotationManager,
// };
pub use user::{AnonymousUser, SimpleUser, User};

/// Authentication errors
#[derive(Debug, Clone)]
pub enum AuthenticationError {
    InvalidCredentials,
    UserNotFound,
    SessionExpired,
    InvalidToken,
    Unknown(String),
}

impl std::fmt::Display for AuthenticationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthenticationError::InvalidCredentials => write!(f, "Invalid credentials"),
            AuthenticationError::UserNotFound => write!(f, "User not found"),
            AuthenticationError::SessionExpired => write!(f, "Session expired"),
            AuthenticationError::InvalidToken => write!(f, "Invalid token"),
            AuthenticationError::Unknown(msg) => write!(f, "Authentication error: {}", msg),
        }
    }
}

impl std::error::Error for AuthenticationError {}

/// Authentication backend trait
pub trait AuthenticationBackend: Send + Sync {
    fn authenticate(
        &self,
        request: &reinhardt_apps::Request,
    ) -> Result<Option<Box<dyn User>>, AuthenticationError>;
    fn get_user(&self, user_id: &str) -> Result<Option<Box<dyn User>>, AuthenticationError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_auth_jwt_generate_unit() {
        let jwt_auth = JwtAuth::new(b"test_secret_key");
        let user_id = "user123".to_string();
        let username = "testuser".to_string();

        let token = jwt_auth.generate_token(user_id, username).unwrap();

        assert!(!token.is_empty());
    }

    #[tokio::test]
    async fn test_permission_allow_any() {
        use bytes::Bytes;
        use hyper::{HeaderMap, Method, Uri, Version};
        use reinhardt_types::Request;

        let permission = AllowAny;
        let request = Request::new(
            Method::GET,
            Uri::from_static("/test"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let context = PermissionContext {
            request: &request,
            is_authenticated: false,
            is_admin: false,
            is_active: false,
        };

        assert!(permission.has_permission(&context).await);
    }

    #[tokio::test]
    async fn test_permission_is_authenticated_with_auth() {
        use bytes::Bytes;
        use hyper::{HeaderMap, Method, Uri, Version};
        use reinhardt_types::Request;

        let permission = IsAuthenticated;
        let request = Request::new(
            Method::GET,
            Uri::from_static("/test"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let context = PermissionContext {
            request: &request,
            is_authenticated: true,
            is_admin: false,
            is_active: true,
        };

        assert!(permission.has_permission(&context).await);
    }

    #[tokio::test]
    async fn test_permission_is_authenticated_without_auth() {
        use bytes::Bytes;
        use hyper::{HeaderMap, Method, Uri, Version};
        use reinhardt_types::Request;

        let permission = IsAuthenticated;
        let request = Request::new(
            Method::GET,
            Uri::from_static("/test"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        let context = PermissionContext {
            request: &request,
            is_authenticated: false,
            is_admin: false,
            is_active: false,
        };

        assert!(!permission.has_permission(&context).await);
    }

    #[tokio::test]
    async fn test_permission_is_admin_user() {
        use bytes::Bytes;
        use hyper::{HeaderMap, Method, Uri, Version};
        use reinhardt_types::Request;

        let permission = IsAdminUser;
        let request = Request::new(
            Method::GET,
            Uri::from_static("/test"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        // Admin user
        let context = PermissionContext {
            request: &request,
            is_authenticated: true,
            is_admin: true,
            is_active: true,
        };
        assert!(permission.has_permission(&context).await);

        // Non-admin user
        let context = PermissionContext {
            request: &request,
            is_authenticated: true,
            is_admin: false,
            is_active: true,
        };
        assert!(!permission.has_permission(&context).await);
    }

    #[tokio::test]
    async fn test_permission_is_active_user() {
        use bytes::Bytes;
        use hyper::{HeaderMap, Method, Uri, Version};
        use reinhardt_types::Request;

        let permission = IsActiveUser;
        let request = Request::new(
            Method::GET,
            Uri::from_static("/test"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        // Active user
        let context = PermissionContext {
            request: &request,
            is_authenticated: true,
            is_admin: false,
            is_active: true,
        };
        assert!(permission.has_permission(&context).await);

        // Inactive user
        let context = PermissionContext {
            request: &request,
            is_authenticated: true,
            is_admin: false,
            is_active: false,
        };
        assert!(!permission.has_permission(&context).await);
    }

    #[tokio::test]
    async fn test_permission_is_authenticated_or_read_only_get() {
        use bytes::Bytes;
        use hyper::{HeaderMap, Method, Uri, Version};
        use reinhardt_types::Request;

        let permission = IsAuthenticatedOrReadOnly;
        let request = Request::new(
            Method::GET,
            Uri::from_static("/test"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        // Unauthenticated GET should be allowed
        let context = PermissionContext {
            request: &request,
            is_authenticated: false,
            is_admin: false,
            is_active: false,
        };
        assert!(permission.has_permission(&context).await);
    }

    #[tokio::test]
    async fn test_permission_is_authenticated_or_read_only_post() {
        use bytes::Bytes;
        use hyper::{HeaderMap, Method, Uri, Version};
        use reinhardt_types::Request;

        let permission = IsAuthenticatedOrReadOnly;
        let request = Request::new(
            Method::POST,
            Uri::from_static("/test"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        );

        // Unauthenticated POST should be denied
        let context = PermissionContext {
            request: &request,
            is_authenticated: false,
            is_admin: false,
            is_active: false,
        };
        assert!(!permission.has_permission(&context).await);

        // Authenticated POST should be allowed
        let context = PermissionContext {
            request: &request,
            is_authenticated: true,
            is_admin: false,
            is_active: true,
        };
        assert!(permission.has_permission(&context).await);
    }

    #[test]
    fn test_simple_user_implementation() {
        let user = SimpleUser {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            is_active: true,
            is_admin: false,
        };

        assert!(!user.id().is_empty());
        assert_eq!(user.username(), "testuser");
        assert!(user.is_authenticated());
        assert!(user.is_active());
        assert!(!user.is_admin());
    }

    #[test]
    fn test_anonymous_user() {
        let user = AnonymousUser;

        assert_eq!(user.id(), "");
        assert_eq!(user.username(), "");
        assert!(!user.is_authenticated());
        assert!(!user.is_active());
        assert!(!user.is_admin());
    }
}
