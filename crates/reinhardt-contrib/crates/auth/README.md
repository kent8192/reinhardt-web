# reinhardt-auth

Authentication and authorization system for Reinhardt framework.

## Overview

Comprehensive authentication and authorization system inspired by Django and Django REST Framework. Provides JWT tokens, permission classes, user models, and password hashing with Argon2.

## Implemented ✓

### Core Authentication

#### JWT (JSON Web Token) Authentication
- **Claims Management**: `Claims` struct with user identification, expiration, and issue times
- **Token Generation**: Automatic 24-hour expiration by default
- **Token Verification**: Built-in expiration checking and signature validation
- **Encode/Decode**: Full JWT token encoding and decoding support

```rust
use reinhardt_auth::jwt::{JwtAuth, Claims};
use chrono::Duration;

let jwt_auth = JwtAuth::new(b"my-secret-key");
let token = jwt_auth.generate_token("user123".to_string(), "john_doe".to_string()).unwrap();
let claims = jwt_auth.verify_token(&token).unwrap();
```

#### HTTP Basic Authentication
- **BasicAuthentication**: HTTP Basic auth backend with user management
- **Base64 Encoding/Decoding**: Standard HTTP Basic auth header parsing
- **User Registration**: Add users with username/password pairs
- **Request Authentication**: Extract and verify credentials from Authorization headers

```rust
use reinhardt_auth::{HttpBasicAuth, AuthenticationBackend};

let mut auth = HttpBasicAuth::new();
auth.add_user("alice", "secret123");

// Request with Basic auth header will be authenticated
let result = auth.authenticate(&request).unwrap();
```

### User Management

#### User Trait
- **Core User Interface**: Unified trait for authenticated and anonymous users
- **User Identification**: `id()`, `username()`, `get_username()` methods
- **Authentication Status**: `is_authenticated()`, `is_active()`, `is_admin()` checks
- **Django Compatibility**: Methods compatible with Django's user interface

#### User Implementations
- **SimpleUser**: Fully-featured user with UUID, username, email, active/admin flags
- **AnonymousUser**: Zero-sized type representing unauthenticated visitors
- **Serialization Support**: Serde integration for SimpleUser

```rust
use reinhardt_auth::{User, SimpleUser, AnonymousUser};
use uuid::Uuid;

let user = SimpleUser {
    id: Uuid::new_v4(),
    username: "john".to_string(),
    email: "john@example.com".to_string(),
    is_active: true,
    is_admin: false,
};

assert!(user.is_authenticated());
assert!(!user.is_admin());
```

### Password Security

#### Password Hashing
- **PasswordHasher Trait**: Composable password hashing interface
- **Argon2Hasher**: Production-ready Argon2id implementation (recommended)
- **Hash Generation**: Secure salt generation using OS random number generator
- **Password Verification**: Constant-time comparison for security

```rust
use reinhardt_auth::{Argon2Hasher, PasswordHasher};

let hasher = Argon2Hasher::new();
let hash = hasher.hash("my_password").unwrap();
assert!(hasher.verify("my_password", &hash).unwrap());
```

### Authentication Backends

#### AuthBackend Trait
- **Composable Architecture**: Support for multiple authentication strategies
- **Async Support**: Full async/await integration with `async_trait`
- **User Authentication**: `authenticate(username, password)` method
- **User Lookup**: `get_user(user_id)` for session restoration

#### Composite Authentication
- **CompositeAuthBackend**: Chain multiple authentication backends
- **Fallback Support**: Try backends in order until one succeeds
- **Flexible Configuration**: Add backends dynamically at runtime

```rust
use reinhardt_auth::CompositeAuthBackend;

let mut composite = CompositeAuthBackend::new();
composite.add_backend(Box::new(database_backend));
composite.add_backend(Box::new(ldap_backend));

// Will try database first, then LDAP
let user = composite.authenticate("alice", "password").await;
```

### Permission System

#### Permission Trait
- **Permission Interface**: Async `has_permission()` method with context
- **PermissionContext**: Request-aware context with authentication flags
- **Composable Permissions**: Build complex permission logic

#### Built-in Permission Classes
- **AllowAny**: Allow all requests without authentication
- **IsAuthenticated**: Require authenticated user
- **IsAdminUser**: Require authenticated admin user
- **IsActiveUser**: Require authenticated and active user
- **IsAuthenticatedOrReadOnly**: Authenticated for write, read-only for anonymous users

```rust
use reinhardt_auth::{Permission, IsAuthenticated, PermissionContext};

let permission = IsAuthenticated;
let context = PermissionContext {
    request: &request,
    is_authenticated: true,
    is_admin: false,
    is_active: true,
};

assert!(permission.has_permission(&context).await);
```

### Error Handling

#### AuthenticationError
- **InvalidCredentials**: Wrong username or password
- **UserNotFound**: User does not exist
- **SessionExpired**: Session has expired
- **InvalidToken**: Token is malformed or invalid
- **Unknown**: Generic error with custom message

#### AuthenticationBackend Trait
- **Unified Error Handling**: All backends use `AuthenticationError`
- **Standard Error Trait**: Implements `std::error::Error`
- **Display Implementation**: User-friendly error messages

## Planned

The following features are planned for future releases:

### Session-Based Authentication
- **SessionAuthentication**: Traditional session-based auth
- **SessionStore**: Session data persistence
- **Cookie Management**: Secure cookie handling

### Token-Based Authentication
- **TokenAuthentication**: API token authentication
- **Token Storage**: Token persistence and lookup
- **Token Rotation**: Automatic token rotation for security

### Advanced Permissions
- **RateLimitPermission**: Request rate limiting by IP or user
- **TimeBasedPermission**: Time-of-day access control
- **IpWhitelistPermission**: IP-based access control
- **IpBlacklistPermission**: IP blocking
- **Permission Operators**: AND, OR, NOT combinators for complex logic

### Model Permissions
- **DjangoModelPermissions**: Django-style model permissions
- **DjangoModelPermissionsOrAnonReadOnly**: Anonymous read access
- **ModelPermission**: CRUD permissions per model
- **Permission Checking**: Object-level permission support

### Multi-Factor Authentication (MFA)
- **TotpDevice**: Time-based one-time passwords (Google Authenticator, Authy)
- **BackupCode**: Emergency backup codes
- **MfaManager**: MFA device management
- **MfaMethod**: Support for multiple MFA methods
- **MfaDeviceStore**: Persistent storage for MFA devices

### OAuth2 Support
- **OAuth2Authentication**: OAuth2 provider integration
- **OAuth2Application**: Client application management
- **OAuth2Token**: Access and refresh tokens
- **AccessToken**: OAuth2 access token handling
- **AuthorizationCode**: Authorization code flow
- **GrantType**: Support for multiple grant types
- **OAuth2TokenStore**: Token persistence

### Token Blacklist
- **TokenBlacklist**: Invalidate tokens before expiration
- **RefreshToken**: Refresh token management
- **TokenRotationManager**: Automatic token rotation
- **BlacklistedToken**: Track invalidated tokens
- **BlacklistReason**: Categorize token blacklist reasons
- **BlacklistStats**: Usage statistics and monitoring

### Remote User Authentication
- **RemoteUserAuthentication**: Authenticate via trusted headers
- **PersistentRemoteUserAuthentication**: SSO integration support
- **Header-based Auth**: Reverse proxy authentication

### Django REST Framework Compatibility
- **DRF Authentication Classes**: Compatible authentication interfaces
- **DRF Permission Classes**: Compatible permission interfaces
- **Browsable API Support**: Integration with DRF-style browsable API

### Admin & Management
- **User Management**: CRUD operations for users
- **Group Management**: User groups and permissions
- **Permission Assignment**: Assign permissions to users/groups
- **createsuperuser Command**: CLI tool for creating admin users

## Usage Examples

### Complete Authentication Flow

```rust
use reinhardt_auth::{
    JwtAuth, HttpBasicAuth, AuthBackend,
    SimpleUser, User, Argon2Hasher, PasswordHasher,
    Permission, IsAuthenticated, PermissionContext
};

// 1. Set up JWT authentication
let jwt_auth = JwtAuth::new(b"secret-key");

// 2. Set up Basic authentication with a user
let mut basic_auth = HttpBasicAuth::new();
basic_auth.add_user("alice", "password123");

// 3. Authenticate user and generate JWT
let user = basic_auth.authenticate(&request).unwrap().unwrap();
let token = jwt_auth.generate_token(
    user.id(),
    user.username().to_string()
).unwrap();

// 4. Verify token on subsequent requests
let claims = jwt_auth.verify_token(&token).unwrap();

// 5. Check permissions
let permission = IsAuthenticated;
let context = PermissionContext {
    request: &request,
    is_authenticated: true,
    is_admin: user.is_admin(),
    is_active: user.is_active(),
};

if permission.has_permission(&context).await {
    // Grant access
}
```

### Custom Authentication Backend

```rust
use reinhardt_auth::{AuthBackend, SimpleUser, Argon2Hasher, PasswordHasher};
use async_trait::async_trait;
use std::collections::HashMap;

struct MyAuthBackend {
    users: HashMap<String, (String, SimpleUser)>,
    hasher: Argon2Hasher,
}

#[async_trait]
impl AuthBackend for MyAuthBackend {
    type User = SimpleUser;

    async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> reinhardt_apps::Result<Option<Self::User>> {
        if let Some((hash, user)) = self.users.get(username) {
            if self.hasher.verify(password, hash)? {
                return Ok(Some(user.clone()));
            }
        }
        Ok(None)
    }

    async fn get_user(&self, user_id: &str)
        -> reinhardt_apps::Result<Option<Self::User>> {
        Ok(self.users.values()
            .find(|(_, u)| u.id.to_string() == user_id)
            .map(|(_, u)| u.clone()))
    }
}
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
