//! # Reinhardt Auth
//!
//! Authentication and authorization system for Reinhardt framework.
//!
//! ## Features
//!
//! - **DjangoModelPermissions**: Django-style model permissions with `app_label.action_model` format
//! - **DjangoModelPermissionsOrAnonReadOnly**: Anonymous read access for unauthenticated users
//! - **Object-Level Permissions**: Fine-grained access control on individual objects
//! - **User Management**: CRUD operations for users with password hashing
//! - **Group Management**: User groups and permission assignment
//! - **DRF-Compatible Authentication**: Multiple authentication backends (JWT, Token, Session, OAuth2)
//! - **DRF-Compatible Permissions**: Permission classes for common authorization scenarios
//! - **createsuperuser Command**: CLI tool for creating admin users

pub mod advanced_permissions;
pub mod backend;
pub mod basic;
pub mod di_support;
pub mod drf_authentication;
pub mod drf_permissions;
pub mod group_management;
#[cfg(feature = "session")]
pub mod handlers;
pub mod ip_permission;
#[cfg(feature = "jwt")]
pub mod jwt;
pub mod mfa;
pub mod model_permissions;
pub mod object_permissions;
#[cfg(feature = "oauth")]
pub mod oauth2;
pub mod permission_operators;
pub mod permissions;
pub mod rate_limit_permission;
pub mod remote_user;
#[cfg(feature = "session")]
pub mod session;
pub mod time_based_permission;
#[cfg(any(feature = "jwt", feature = "token"))]
pub mod token_blacklist;
#[cfg(any(feature = "jwt", feature = "token"))]
pub mod token_rotation;
#[cfg(any(feature = "jwt", feature = "token"))]
pub mod token_storage;
pub mod user;
pub mod user_management;

pub use advanced_permissions::{ObjectPermission as AdvancedObjectPermission, RoleBasedPermission};
pub use backend::{Argon2Hasher, AuthBackend, CompositeAuthBackend, PasswordHasher};
pub use basic::BasicAuthentication as HttpBasicAuth;
pub use drf_authentication::{
    Authentication, BasicAuthConfig, CompositeAuthentication, RemoteUserAuthentication,
    SessionAuthConfig, SessionAuthentication, TokenAuthConfig, TokenAuthentication,
};
pub use drf_permissions::{
    DrfAllowAny, DrfIsAdminUser, DrfIsAuthenticated, DrfIsAuthenticatedOrReadOnly,
};
pub use group_management::{
    CreateGroupData, Group, GroupManagementError, GroupManagementResult, GroupManager,
};
#[cfg(feature = "session")]
pub use handlers::{LoginCredentials, LoginHandler, LogoutHandler, SESSION_COOKIE_NAME};
pub use ip_permission::{CidrRange, IpBlacklistPermission, IpWhitelistPermission};
#[cfg(feature = "jwt")]
pub use jwt::{Claims, JwtAuth};
pub use mfa::MFAAuthentication as MfaManager;
pub use model_permissions::{
    DjangoModelPermissions, DjangoModelPermissionsOrAnonReadOnly, ModelPermission,
};
pub use object_permissions::{ObjectPermission, ObjectPermissionChecker, ObjectPermissionManager};
#[cfg(feature = "oauth")]
pub use oauth2::{
    AccessToken, AuthorizationCode, GrantType, InMemoryOAuth2Store, OAuth2Application,
    OAuth2Authentication, OAuth2TokenStore,
};
pub use permission_operators::{AndPermission, NotPermission, OrPermission};
pub use permissions::{
    AllowAny, IsActiveUser, IsAdminUser, IsAuthenticated, IsAuthenticatedOrReadOnly, Permission,
    PermissionContext,
};
pub use rate_limit_permission::{
    RateLimitConfig, RateLimitConfigBuilder, RateLimitKeyStrategy, RateLimitPermission,
    RateLimitPermissionBuilder,
};
pub use remote_user::RemoteUserAuthentication as RemoteUserAuth;
#[cfg(feature = "session")]
pub use session::{InMemorySessionStore, Session, SessionId, SessionStore, SESSION_KEY_USER_ID};
pub use time_based_permission::{DateRange, TimeBasedPermission, TimeWindow};
#[cfg(any(feature = "jwt", feature = "token"))]
pub use token_blacklist::{
    BlacklistReason, BlacklistStats, BlacklistedToken, InMemoryRefreshTokenStore,
    InMemoryTokenBlacklist, RefreshToken, RefreshTokenStore, TokenBlacklist, TokenRotationManager,
};
#[cfg(any(feature = "jwt", feature = "token"))]
pub use token_rotation::{AutoTokenRotationManager, TokenRotationConfig, TokenRotationRecord};
#[cfg(any(feature = "jwt", feature = "token"))]
pub use token_storage::{
    InMemoryTokenStorage, StoredToken, TokenStorage, TokenStorageError, TokenStorageResult,
};
pub use user::{AnonymousUser, SimpleUser, User};
pub use user_management::{
    CreateUserData, UpdateUserData, UserManagementError, UserManagementResult, UserManager,
};

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
///
/// All authentication operations are asynchronous to support various backends
/// including database lookups, external API calls, and distributed systems.
#[async_trait::async_trait]
pub trait AuthenticationBackend: Send + Sync {
    /// Authenticate a request and return a user if successful
    ///
    /// # Arguments
    ///
    /// * `request` - The incoming HTTP request
    ///
    /// # Returns
    ///
    /// - `Ok(Some(user))` if authentication succeeded
    /// - `Ok(None)` if authentication failed but should try next backend
    /// - `Err(error)` if a fatal error occurred
    async fn authenticate(
        &self,
        request: &reinhardt_apps::Request,
    ) -> Result<Option<Box<dyn User>>, AuthenticationError>;

    /// Get a user by their ID
    ///
    /// # Arguments
    ///
    /// * `user_id` - The user's unique identifier
    ///
    /// # Returns
    ///
    /// - `Ok(Some(user))` if user was found
    /// - `Ok(None)` if user doesn't exist
    /// - `Err(error)` if an error occurred
    async fn get_user(&self, user_id: &str) -> Result<Option<Box<dyn User>>, AuthenticationError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    #[cfg(feature = "jwt")]
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
            user: None,
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
            user: None,
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
            user: None,
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
            user: None,
        };
        assert!(permission.has_permission(&context).await);

        // Non-admin user
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
            user: None,
        };
        assert!(permission.has_permission(&context).await);

        // Inactive user
        let context = PermissionContext {
            request: &request,
            is_authenticated: true,
            is_admin: false,
            is_active: false,
            user: None,
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
            user: None,
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
            user: None,
        };
        assert!(!permission.has_permission(&context).await);

        // Authenticated POST should be allowed
        let context = PermissionContext {
            request: &request,
            is_authenticated: true,
            is_admin: false,
            is_active: true,
            user: None,
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
