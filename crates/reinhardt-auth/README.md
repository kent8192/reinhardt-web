# reinhardt-auth

Authentication and authorization system for Reinhardt framework.

## Overview

Comprehensive authentication and authorization system inspired by Django and
Django REST Framework. Provides JWT tokens, permission classes, user models, and
password hashing with Argon2.

## Installation

Add `reinhardt` to your `Cargo.toml`:

<!-- reinhardt-version-sync:3 -->
```toml
[dependencies]
reinhardt = { version = "0.4.0-alpha.15", features = ["auth"] }

# Or use a preset:
# reinhardt = { version = "0.4.0-alpha.15", features = ["standard"] }  # Recommended
# reinhardt = { version = "0.4.0-alpha.15", features = ["full"] }      # All features
```

Then import authentication features:

```rust
use reinhardt::auth::{User, SimpleUser, AnonymousUser};
use reinhardt::auth::{JwtAuth, HttpBasicAuth, AuthenticationBackend};
use reinhardt::auth::{AllowAny, IsAuthenticated, IsAuthenticatedOrReadOnly};
```

**Note:** Authentication features are included in the `standard` and `full` feature presets.

## Implemented ✓

### Core Authentication

#### JWT (JSON Web Token) Authentication

- **Claims Management**: `Claims` struct with user identification, expiration,
  and issue times
- **Token Generation**: Automatic 24-hour expiration by default
- **Token Verification**: Built-in expiration checking and signature validation
- **Encode/Decode**: Full JWT token encoding and decoding support

```rust
use reinhardt::auth::jwt::{JwtAuth, Claims};
use chrono::Duration;

let jwt_auth = JwtAuth::new(b"my-secret-key");
let token = jwt_auth.generate_token("user123".to_string(), "john_doe".to_string(), false, false).unwrap();
let claims = jwt_auth.verify_token(&token).unwrap();
```

#### HTTP Basic Authentication

- **BasicAuthentication**: HTTP Basic auth backend with user management
- **Base64 Encoding/Decoding**: Standard HTTP Basic auth header parsing
- **User Registration**: Add users with username/password pairs, with a
  fallible path for custom password policies that can reject input
- **Request Authentication**: Extract and verify credentials from Authorization
  headers

```rust
use reinhardt::auth::{HttpBasicAuth, AuthenticationBackend};

let mut auth = HttpBasicAuth::new();
auth.add_user("alice", "secret123");
auth.try_add_user("bob", "password456").unwrap();

// Request with Basic auth header will be authenticated
let result = auth.authenticate(&request).unwrap();
```

### User Management

#### User Trait (Deprecated)

> **Deprecated since `0.1.0-rc.15`**: The `User` trait is deprecated. Use
> `AuthIdentity` + `BaseUser`/`FullUser` + `PermissionsMixin` instead.
> The trait remains available for backward compatibility but will be removed
> in a future release.

- **Core User Interface**: Unified trait for authenticated and anonymous users
- **User Identification**: `id()`, `username()`, `get_username()` methods
- **Authentication Status**: `is_authenticated()`, `is_active()`, `is_admin()`,
  `is_staff()`, `is_superuser()` checks
- **Django Compatibility**: Methods compatible with Django's user interface

#### User Implementations

- **SimpleUser**: Fully-featured user with UUID, username, email, active/admin
  flags
- **AnonymousUser**: Zero-sized type representing unauthenticated visitors
- **Serialization Support**: Serde integration for SimpleUser

```rust
use reinhardt::auth::{User, SimpleUser, AnonymousUser};
use uuid::Uuid;

let user = SimpleUser {
    id: Uuid::new_v4(),
    username: "john".to_string(),
    email: "john@example.com".to_string(),
    is_active: true,
    is_admin: false,
    is_staff: false,
    is_superuser: false,
};

assert!(user.is_authenticated());
assert!(!user.is_admin());
```

### Django-Style User Models

#### BaseUser Trait (AbstractBaseUser equivalent)

- **Minimal Authentication Interface**: Minimal set of fields for user
  authentication
- **Automatic Password Hashing**: Argon2id hashing by default, fully
  customizable
- **Associated Type Default**:
  `type Hasher: PasswordHasher + Default = Argon2Hasher`
- **Password Management**:
  - `set_password()`: Automatically hashes with configured hasher
  - `check_password()`: Verifies password against hash
  - `set_unusable_password()`: Marks password as unusable (for OAuth-only
    accounts)
  - `has_usable_password()`: Checks if user can log in with password
- **Session Authentication**: `get_session_auth_hash()` for session invalidation
  on password change
- **Username Normalization**: NFKC Unicode normalization to prevent homograph
  attacks
- **Django Compatibility**: Method names and behavior match Django's
  AbstractBaseUser

```rust
use reinhardt::auth::BaseUser;
use uuid::Uuid;
use chrono::Utc;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct MyUser {
    id: Uuid,
    email: String,
    password_hash: Option<String>,
    last_login: Option<chrono::DateTime<Utc>>,
    is_active: bool,
}

impl BaseUser for MyUser {
    type PrimaryKey = Uuid;
    // type Hasher = Argon2Hasher; // Default, can be omitted or customized

    fn get_username_field() -> &'static str { "email" }
    fn get_username(&self) -> &str { &self.email }
    fn password_hash(&self) -> Option<&str> { self.password_hash.as_deref() }
    fn set_password_hash(&mut self, hash: String) { self.password_hash = Some(hash); }
    fn last_login(&self) -> Option<chrono::DateTime<Utc>> { self.last_login }
    fn set_last_login(&mut self, time: chrono::DateTime<Utc>) { self.last_login = Some(time); }
    fn is_active(&self) -> bool { self.is_active }
}

let mut user = MyUser {
    id: Uuid::new_v4(),
    email: "alice@example.com".to_string(),
    password_hash: None,
    last_login: None,
    is_active: true,
};

// Password is automatically hashed with Argon2id
user.set_password("securepass123").unwrap();
assert!(user.check_password("securepass123").unwrap());
```

#### FullUser Trait (AbstractUser equivalent)

- **Complete User Model**: Extends BaseUser with additional fields
- **User Profile Fields**:
  - `username()`: Unique username for login
  - `email()`: Email address
  - `first_name()` / `last_name()`: User's name
  - `is_staff()`: Can access admin interface
  - `is_superuser()`: Has all permissions
  - `date_joined()`: Account creation timestamp
- **Helper Methods**:
  - `get_full_name()`: Combines first and last name
  - `get_short_name()`: Returns first name only
- **Django Compatibility**: Matches Django's AbstractUser interface

```rust
use reinhardt::auth::{BaseUser, FullUser, DefaultUser};
use uuid::Uuid;
use chrono::Utc;

let mut user = DefaultUser {
    id: Uuid::new_v4(),
    username: "alice".to_string(),
    email: "alice@example.com".to_string(),
    first_name: "Alice".to_string(),
    last_name: "Smith".to_string(),
    password_hash: None,
    last_login: None,
    is_active: true,
    is_staff: false,
    is_superuser: false,
    date_joined: Utc::now(),
    user_permissions: Vec::new(),
    groups: Vec::new(),
};

assert_eq!(user.get_full_name(), "Alice Smith");
assert_eq!(user.get_short_name(), "Alice");
```

#### PermissionsMixin Trait

- **Authorization Interface**: Permission and group management
- **Permission Checking**:
  - `has_perm(perm)`: Check if user has specific permission
  - `has_module_perms(app_label)`: Check if user has any permission for app
  - `get_all_permissions()`: Get all user and group permissions
- **Superuser Bypass**: Superusers automatically pass all permission checks
- **Group Support**: Users can belong to multiple groups
- **Django Compatibility**: Permission format `"app_label.permission_name"`

```rust
use reinhardt::auth::{DefaultUser, PermissionsMixin};
use uuid::Uuid;
use chrono::Utc;

let user = DefaultUser {
    id: Uuid::new_v4(),
    username: "bob".to_string(),
    email: "bob@example.com".to_string(),
    first_name: "Bob".to_string(),
    last_name: "Johnson".to_string(),
    password_hash: None,
    last_login: None,
    is_active: true,
    is_staff: true,
    is_superuser: false,
    date_joined: Utc::now(),
    user_permissions: vec![
        "blog.add_post".to_string(),
        "blog.change_post".to_string(),
    ],
    groups: vec!["editors".to_string()],
};

assert!(user.has_perm("blog.add_post"));
assert!(user.has_module_perms("blog"));
```

#### DefaultUser Struct

- **Ready-to-Use Implementation**: Combines BaseUser, FullUser, and
  PermissionsMixin
- **Database Model**: `#[derive(Model)]` for ORM integration
- **Table Name**: `auth_user` (Django-compatible)
- **All Fields Included**: Username, email, names, passwords, permissions,
  groups, flags, timestamps
- **Zero Configuration**: Works out of the box with automatic Argon2id hashing

```rust
use reinhardt::auth::{BaseUser, DefaultUser, DefaultUserManager};
use std::collections::HashMap;

# tokio_test::block_on(async {
let mut manager = DefaultUserManager::new();

// Create a regular user
let user = manager.create_user(
    "alice",
    Some("securepass123"),
    HashMap::new()
).await.unwrap();

assert_eq!(user.username, "alice");
assert!(user.is_active);
assert!(!user.is_staff);

// Create a superuser
let admin = manager.create_superuser(
    "admin",
    Some("adminsecret"),
    HashMap::new()
).await.unwrap();

assert!(admin.is_staff);
assert!(admin.is_superuser);
# })
```

#### BaseUserManager Trait

- **User Creation Interface**: `create_user()` and `create_superuser()` methods
- **Async Support**: Full async/await integration
- **Extra Fields**: Accept arbitrary extra data via HashMap
- **Email Normalization**: `normalize_email()` static method
- **Django Compatibility**: Matches Django's UserManager interface

#### DefaultUserManager

- **In-Memory Implementation**: Built-in manager for DefaultUser
- **Thread-Safe**: Uses `Arc<RwLock<HashMap>>` for concurrent access
- **User Lookup**: `get_by_id()` and `get_by_username()` methods
- **Demonstration Purpose**: For testing and prototyping (use ORM-based manager
  in production)

#### `#[user]` Macro

The `#[user]` attribute macro generates the full user model implementation from
a plain struct definition. It is the recommended approach for defining custom
user models in reinhardt-web applications.

**Parameters:**

- `hasher`: Password hasher type (e.g., `Argon2Hasher`)
- `username_field`: Name of the field used as the login identifier (e.g.,
  `"username"`, `"email"`)
- `full`: When `true`, generates the complete user interface including
  `FullUser` and `PermissionsMixin` implementations
- `manager` (default `true`): When `true`, also emits a `<Name>Manager` struct
  that implements `BaseUserManager<Name>`. Pass `manager = false` to opt out.
- `manager_name` (optional): Override the generated manager type name. Defaults
  to `<Name>Manager` (e.g., `UserManager`).

**Auto-generated user manager**

By default `#[user]` also generates an in-memory user manager so that the
common case "I just need a `BaseUserManager` for my user type" requires zero
boilerplate. The generated manager is backed by `Mutex<HashMap<PK, User>>` and
provides:

- `pub fn new() -> Self` (also `Default`)
- `BaseUserManager<Name>::create_user(username, password, extra)` — builds the
  user from `<Name>::default()`, re-seeds Uuid primary keys with `Uuid::now_v7`,
  applies known `extra` keys (`email`, `first_name`, `last_name`, `is_active`)
  when the corresponding `#[user_field]` is present, and calls
  `set_password` when a password is provided.
- `BaseUserManager<Name>::create_superuser(...)` — same as `create_user` plus
  `is_superuser = true` (and `is_staff = true` under `full = true`).

The user struct MUST implement the following traits for the generator to work:

- `Default` (e.g., via `#[derive(Default)]`) — the generator constructs the user
  from `<User as Default>::default()` before applying field values. This also
  matches the requirement for `SuperuserInit` under `full = true`.
- `Clone` (e.g., via `#[derive(Clone)]`) — the generated `HashMap`-backed store
  clones both the user (`user.clone()`) and the primary-key value
  (`user.<pk_field>.clone()`) on insertion. Non-`Clone` user types must opt out
  via `manager = false`.

**Primary-key restriction (Uuid-only).** Auto-manager (`manager = true`, the
default) only supports `Uuid` (or `Option<Uuid>`) primary keys. The generator
re-seeds the PK with `Uuid::now_v7()` on every `create_user`, so collisions are
not possible in practice. For any other PK type (e.g. `i64`, `String`) the
generator emits a compile-time error pointing at the PK field: either change
the primary key to `Uuid` / `Option<Uuid>`, or set `manager = false` and
provide your own `BaseUserManager` implementation. This restriction prevents
silent map-overwrite bugs in the in-memory store, where repeated `create_user`
calls would otherwise collide on the `<User as Default>::default()` PK value.
See [reinhardt-web#4455](https://github.com/kent8192/reinhardt-web/issues/4455)
for the rationale.

**When to opt out (`manager = false`)**

Opt out and hand-write the implementation when you need any of:

- Database-backed persistence with custom queries
- Custom uniqueness checks (e.g., case-insensitive email collision detection
  with a precise error type)
- Multi-step user creation (welcome email, audit log, MFA enrollment, etc.)
- Integration with `reinhardt-di` `#[injectable]` for DI-driven lifetime

```rust
// Opt out and provide a DB-backed manager yourself.
#[user(hasher = Argon2Hasher, username_field = "email", manager = false)]
#[derive(Default, Serialize, Deserialize)]
pub struct User { /* ... */ }

pub struct UserManager { /* DI-injected db handle */ }

#[async_trait::async_trait]
impl reinhardt_auth::BaseUserManager<User> for UserManager {
    // your DB-backed create_user / create_superuser
}
```

**Notes:**

- `#[user]` does NOT auto-derive `Serialize` or `Deserialize`; add
  `#[derive(...)]` explicitly for those traits. `Default` is now required
  whenever the auto-manager is enabled (the default).
- `#[model]` is still required for database integration; `app_label` must be
  configured there, while `table_name` defaults to the app label plus the
  singular snake_case struct name and remains available for existing or custom
  schemas
- Each field uses `#[field(...)]` attributes to declare constraints such as
  `primary_key`, `unique`, `max_length`, `default`, and `include_in_new`

```rust
use reinhardt::Argon2Hasher;
use reinhardt::macros::user;
use reinhardt::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[user(hasher = Argon2Hasher, username_field = "username", full = true)]
#[derive(Default, Serialize, Deserialize)]
#[model(app_label = "auth", table_name = "users")]
pub struct User {
	// `#[user]` also auto-generates `UserManager` with an in-memory
	// `BaseUserManager<User>` impl. Construct it via `UserManager::new()`.
	// Opt out with `manager = false` or rename via `manager_name = MyMgr`.
	#[field(primary_key = true, include_in_new = false)]
	pub id: Uuid,

	#[field(max_length = 150, unique = true)]
	pub username: String,

	#[field(max_length = 254, unique = true)]
	pub email: String,

	#[field(max_length = 512)]
	pub password_hash: Option<String>,

	#[field(default = true)]
	pub is_active: bool,

	#[field(default = false)]
	pub is_staff: bool,

	#[field(default = false)]
	pub is_superuser: bool,
}
```

### Password Security

#### Password Hashing

- **PasswordHasher Trait**: Composable password hashing interface
- **PasswordHashPolicy**: Ordered preferred and legacy hasher policy for login-time upgrades
- **PasswordVerification / PasswordCheck**: Status values that report invalid,
  valid, and rehash-required password checks
- **Argon2Hasher**: Production-ready Argon2id implementation (recommended)
- **BcryptHasher**: Optional compatibility hasher behind the `bcrypt-hasher`
  feature for migrations and interoperability
- **Hash Generation**: Secure salt generation using OS random number generator
- **Password Verification**: Constant-time comparison for security

```rust
use reinhardt::auth::{Argon2Hasher, PasswordHasher};

let hasher = Argon2Hasher::new();
let hash = hasher.hash("my_password").unwrap();
assert!(hasher.verify("my_password", &hash).unwrap());
```

#### Password hash policy upgrades

`PasswordHashPolicy` defines one preferred hasher for new passwords and optional
legacy hashers for stored hashes from older deployments. During a successful
password check, legacy matches and outdated preferred hashes can be rewritten
with the preferred algorithm or cost parameters. `PasswordVerification` reports
whether a replacement hash is needed, and `BaseUser` helpers return
`PasswordCheck::ValidUpdated` after updating the in-memory user value.

```rust
use reinhardt::auth::{Argon2Hasher, PasswordHashPolicy, PasswordVerification};

let policy = PasswordHashPolicy::new(Argon2Hasher::default());
let hash = policy.hash("my_password").unwrap();
let verification = policy
    .verify_with_update("my_password", &hash)
    .unwrap();

assert!(matches!(verification, PasswordVerification::Valid));
```

`BcryptHasher` is useful when migrating from bcrypt-backed systems or
interoperating with existing bcrypt hashes, but bcrypt still retains its
72-byte input limit.

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
use reinhardt::auth::CompositeAuthBackend;

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
- **IsAuthenticatedOrReadOnly**: Authenticated for write, read-only for
  anonymous users

```rust
use reinhardt::auth::{Permission, IsAuthenticated, PermissionContext};

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

### Session-Based Authentication

#### SessionAuthentication

- **Session Management**: `Session` struct with HashMap-based data storage
- **SessionStore Trait**: Async interface for session persistence
  - `load()`: Retrieve session by ID
  - `save()`: Persist session data
  - `delete()`: Remove session
- **InMemorySessionStore**: Built-in in-memory session storage
- **SessionId**: Type-safe session identifier wrapper
- **Cookie Integration**: Secure session cookie handling

```rust
use reinhardt::auth::{SessionAuthentication, AuthenticationBackend};
use reinhardt::auth::sessions::backends::InMemorySessionBackend;

// SessionAuthentication is generic over B: SessionBackend.
// Pass the backend to new(), or use Default when B: Default.
let session_backend = InMemorySessionBackend::new();
let auth = SessionAuthentication::new(session_backend);

// Authenticate user from request (checks session cookie)
let user = auth.authenticate(&request).await?;

// Get user by ID
if let Some(user) = auth.get_user("user_id").await? {
    println!("User: {}", user.get_username());
}
```

### Multi-Factor Authentication (MFA)

#### TOTP-Based MFA

- **MFAAuthentication**: Time-based one-time password (TOTP) authentication
- **Secret Management**: Secure per-user secret storage
- **QR Code Generation**: Generate TOTP URLs for authenticator apps (Google
  Authenticator, Authy)
- **Code Verification**: Verify TOTP codes with configurable time window
- **Registration Flow**: User enrollment with secret generation
- **Time Window**: Configurable tolerance for time skew (default: 1 time step)

```rust
use reinhardt::auth::MFAAuthentication;

let mfa = MFAAuthentication::new("MyApp");

// Register user for MFA
let totp_url = mfa.register_user("alice").await?;
// User scans QR code generated from totp_url

// Verify code during login
let code = "123456"; // from user's authenticator app
assert!(mfa.verify_code("alice", code).await?);
```

### OAuth2 Authorization Server

Enable `oauth` for the authorization server and `database` for its PostgreSQL
store. The server supports Authorization Code with mandatory PKCE `S256` and
Client Credentials. It issues audience-bound opaque Bearer tokens. Refresh and
Implicit grants are unavailable; no refresh token is issued.

`OAuthServer` provides administrative client and resource registration, typed
pending authorization requests, explicit host approval, token inspection,
revocation, and introspection. `OAuthHandler` implements Reinhardt's `Handler`
for the authorization, token, revocation, introspection, and metadata routes.
Mount those handlers at the exact HTTPS URLs configured in `OAuthServerConfig`;
mount metadata at the URL returned by `metadata_url()`.
The host must insert `OAuthBrowserSession` from its browser
session into authorization requests and implement `OAuthConsentPresenter` for
login and consent. The session binding must be unpredictable and retained
across login redirects. Approval must call `complete_authorization` with that same
browser-session binding and an authenticated active user ID. The host controls
remembered consent and calls `revoke_user` after account security events. That
operation invalidates outstanding authorization codes and tokens together,
returns only the number of newly revoked tokens, and does not retire the user or
OIDC subject. Code exchange rechecks the user's current authenticated, active
status; a missing or inactive user, or failed user lookup, cannot consume a code.

Register resource servers before clients. A confidential client receives a
secret once at registration; store it securely. A public client has no secret.
Secrets are Argon2 hashes at rest. Codes and tokens are SHA-256 lookup digests.
New registrations use atomic insert-if-absent; re-registration compares the
complete stored snapshot before replacing it. Concurrent registration or
rotation can return `ServerError`; retry using current state rather than
distributing a secret from a failed operation. A resource identifier keeps its
original audience when re-registered. Custom stores used for registration must
implement `insert_client_if_absent`, `insert_resource_if_absent`, and the
corresponding compare-and-swap methods atomically.
Code exchange commits redemption and token insertion together; custom stores
must implement `redeem_code_and_store_token` atomically. `put_token` accepts
client-credentials tokens only. Authorization completion separately prepares its
validated pending snapshot and code, then calls `complete_pending` to persist
both atomically. Client-secret rotation uses `compare_and_swap_client` so a
concurrent administrative update cannot be overwritten. A conflicting rotation,
previous-secret revocation, or disable operation returns `ServerError`; retry
with fresh state rather than using an uncommitted credential.
Resource servers authenticate separately to introspection. Use
`PostgresOAuthStore::migration()` in the host's Reinhardt migration graph before
serving requests; enable `reinhardt-db/postgres` in the host that runs it.
Schedule `PostgresOAuthStore::purge_expired(now)` from a host maintenance job,
passing the current UNIX time in seconds. It deletes expired pending requests,
authorization codes, and tokens in one transaction and returns the deleted row count.
Codes referenced by unexpired tokens remain until those tokens can be removed.
Production construction uses `OAuthServer::for_production` with
a PostgreSQL store and a host-provided shared `OAuthRateLimiter`. The
`for_development` constructor accepts an in-memory store and limiter.

```rust
use reinhardt_auth::UserRepository;
use reinhardt_auth::oauth2_server::{
    OAuthError, OAuthServer, OAuthServerConfig, PostgresOAuthStore,
    SharedOAuthRateLimiter,
};
use sqlx::PgPool;
use std::sync::Arc;

fn create_server<L: SharedOAuthRateLimiter + 'static>(
    pool: PgPool,
    users: Arc<dyn UserRepository>,
    limiter: Arc<L>,
) -> Result<OAuthServer, OAuthError> {
    let config = OAuthServerConfig::new(
        "https://auth.example.com",
        "https://auth.example.com/oauth/authorize",
        "https://auth.example.com/oauth/token",
        "https://auth.example.com/oauth/revoke",
        "https://auth.example.com/oauth/introspect",
    )?;
    OAuthServer::for_production(config, PostgresOAuthStore::new(pool), users, limiter)
}
```

Mount the handlers in the host router (the example uses a root-path issuer):

```rust
use reinhardt_auth::oauth2_server::{
    OAuthConsentPresenter, OAuthEndpoint, OAuthHandler, OAuthServer,
};
use reinhardt_urls::routers::ServerRouter;
use std::sync::Arc;

fn oauth_routes(
    server: Arc<OAuthServer>,
    presenter: Arc<dyn OAuthConsentPresenter>,
) -> ServerRouter {
    ServerRouter::new()
        .handler_arc("/oauth/authorize", Arc::new(OAuthHandler::authorization(server.clone(), presenter)))
        .handler_arc("/oauth/token", Arc::new(OAuthHandler::new(server.clone(), OAuthEndpoint::Token)))
        .handler_arc("/oauth/revoke", Arc::new(OAuthHandler::new(server.clone(), OAuthEndpoint::Revocation)))
        .handler_arc("/oauth/introspect", Arc::new(OAuthHandler::new(server.clone(), OAuthEndpoint::Introspection)))
        .handler_arc("/.well-known/oauth-authorization-server", Arc::new(OAuthHandler::new(server, OAuthEndpoint::Metadata)))
}
```

The previous `OAuth2Authentication`, `OAuth2Application`, and `OAuth2TokenStore`
remain in-process compatibility helpers. They do not implement a routable
authorization server and cannot be supplied as the new server's store. Their
code flow validates registered clients and redirect URIs, expires in-memory
tokens, and does not issue refresh tokens. Existing client registrations must
be explicitly recreated with the typed `ClientRegistration` API, including
redirect URIs, scopes, audience, grant permissions, and a new secret.
Legacy codes and tokens are not imported.

For migration, apply the OAuth server migration first, register each intended
resource audience, then recreate each client with its allowed grants, exact
redirect URIs, scopes, default scopes, audiences, default audience, and browser
origins. Distribute each new confidential-client secret once. Update clients to
use the mounted HTTPS endpoints and PKCE `S256`; users must authorize again.
Retire the legacy helper only after its existing callers have moved.

### OpenID Provider

Enable `oidc-op` on `reinhardt-auth` (or `auth-oidc-op` on the root
`reinhardt` crate) to serve an opt-in OpenID Connect issuer on the OAuth
authorization server. The first profile supports first-party confidential web
clients, Authorization Code with mandatory PKCE `S256`, `client_secret_basic`,
the `openid` scope, RS256 ID Tokens, a UserInfo-only opaque access token,
Discovery, and JWKS. It does not issue refresh tokens or expose a logout,
dynamic registration, or claims endpoint. The RP redirect URI must match its
registration exactly and use HTTPS. `OidcConfig::for_loopback_development`
permits HTTP only for explicit loopback development.

Use one root-path HTTPS issuer and mount these handlers at the exact URLs in
`OidcConfig`. Discovery is always at `config.discovery_url()`.

```rust,ignore
use reinhardt_auth::oidc_op::{OidcEndpoint, OidcHandler, OidcInteraction, OidcProvider};
use reinhardt_urls::routers::ServerRouter;
use std::sync::Arc;

fn oidc_routes(
    provider: Arc<OidcProvider>,
    interaction: Arc<dyn OidcInteraction>,
) -> ServerRouter {
    ServerRouter::new()
        .handler_arc("/oidc/authorize", Arc::new(OidcHandler::authorization(provider.clone(), interaction)))
        .handler_arc("/oidc/token", Arc::new(OidcHandler::new(provider.clone(), OidcEndpoint::Token)))
        .handler_arc("/oidc/userinfo", Arc::new(OidcHandler::new(provider.clone(), OidcEndpoint::UserInfo)))
        .handler_arc("/oidc/jwks", Arc::new(OidcHandler::new(provider.clone(), OidcEndpoint::Jwks)))
        .handler_arc("/.well-known/openid-configuration", Arc::new(OidcHandler::new(provider, OidcEndpoint::Discovery)))
}
```

Apply `PostgresOAuthStore::migration()` and then
`PostgresOidcStore::migration()` with the host's Reinhardt migration executor.
Use the same database for both stores. Schedule
`PostgresOidcStore::purge_expired(now)` after the OAuth purge job. It removes
expired OIDC pending requests, but retains code contexts while their OAuth code
still exists. This preserves replay-triggered revocation until linked tokens
expire and OAuth maintenance removes their code.
Production construction requires `OAuthServer::for_production` and
`OidcProvider::for_production` with PostgreSQL-backed state, a shared OAuth
rate limiter, an active signing key, an `OidcSigner` that has the matching
private key on every signing node, and a host `OidcAccountStatus` adapter.
The adapter must return the current active status and fail closed when account
lookup fails. The built-in `RsaPemKeyRing` accepts RSA private PEM keys of at
least 2048 bits; a KMS or non-exportable key can implement `OidcSigner`.
Provision a public key in the OIDC store before constructing a production
provider, and keep private key material outside the database.

Register the UserInfo URL as an OAuth resource audience before registering an
OIDC client. Set `ClientRegistration.oidc_enabled = true`, use
`ClientKind::Confidential`, allow Authorization Code, register the exact HTTPS
redirect URI, and allow `openid` and the UserInfo audience. The OAuth and
OIDC flows share this registration and token store. The ordinary OAuth
authorization and token endpoints cannot complete or redeem an OIDC code.
Rotate a client secret with `rotate_client_secret_with_overlap` for at most
24 hours of overlap; `revoke_previous_client_secret` ends the overlap, and
`disable_client` invalidates its access tokens.

The host supplies `OAuthBrowserSession` on authorization requests and
implements `OidcInteraction` to handle login, reauthentication, consent, and
account selection. Use the same session binding when calling
`OidcProvider::complete_authorization`. Completion consumes both pending rows and
stores both code records in one PostgreSQL transaction (or under coordinated
in-memory locks without an intervening await). Host or storage failures leave
both continuations retryable until expiry. Custom stores must implement the
corresponding atomic coordination hook; mixed in-memory/PostgreSQL backends
are rejected rather than risking a partial commit. For `prompt=none`, avoid presenting UI
and deny with `login_required`, `consent_required`, or another applicable OIDC
error when silent completion is impossible. `auth_time` must be the time of
the active host authentication. The provider enforces `prompt=login`,
`prompt=consent`, `prompt=select_account`, and `max_age` against the decision
supplied by the host. An opaque public `sub` remains stable for a live account;
call `retire_user` during account deletion to invalidate unredeemed codes and
tokens, remove OIDC code contexts, and permanently reserve its former subject.

ID Tokens default to five minutes and can be configured up to fifteen;
UserInfo access tokens default to ten minutes and can be configured up to one
hour. `signing_key_rotation_due` reports the configured rotation interval
(thirty days by default); schedule provisioning of a new key and call
`rotate_signing_key` when due. Retired public keys remain in JWKS for the
maximum ID Token lifetime plus clock skew. For a compromise, provision and
activate a replacement, then call `compromise_signing_key` on the old key and
notify RPs: an RP with a cached old JWKS key may still accept an unexpired
ID Token. Never log client secrets, codes, access tokens, or private keys.

#### Browser-Bound Social OAuth State

Enable the `social-auth` feature to bind a social OAuth callback to a
high-entropy browser or session value and to carry opaque application context
through the provider redirect. Only a SHA-256 digest of the binding is stored;
the binding itself remains in the application-controlled cookie or session.
State is consumed before the provider exchange, so a replay, provider swap,
expired state, or binding mismatch cannot be retried with the same state.

```rust,ignore
use reinhardt::auth::{ContextualCallbackResult, SocialAuthBackend};

let authorization = backend
    .begin_auth_with_context(
        "github",
        None,
        None,
        browser_cookie_value.as_bytes(),
        serialized_link_intent,
    )
    .await?;

let ContextualCallbackResult { callback, context } = backend
    .handle_callback_with_context(
        "github",
        code,
        state,
        browser_cookie_value.as_bytes(),
    )
    .await?;
```

Use a per-browser, unpredictable binding with appropriate `Secure`,
`HttpOnly`, and `SameSite` cookie settings. The context is opaque and is not
encrypted by the state store, so do not place secrets in it.

### Token Blacklist & Rotation

#### Token Blacklist

- **TokenBlacklist Trait**: Interface for token invalidation
- **BlacklistReason**: Categorized revocation reasons
  - `Logout`: User-initiated logout
  - `Compromised`: Security incident
  - `ManualRevoke`: Admin revocation
  - `Rotated`: Automatic token rotation
- **InMemoryTokenBlacklist**: Built-in in-memory blacklist storage
- **Cleanup**: Automatic removal of expired blacklist entries
- **Statistics**: Usage tracking and monitoring

#### Token Rotation

- **TokenRotationManager**: Automatic refresh token rotation
- **RefreshTokenStore Trait**: Persistent refresh token storage
- **Rotation Flow**: Invalidate old token when issuing new one
- **Security**: Prevents refresh token reuse attacks
- **InMemoryRefreshTokenStore**: Built-in in-memory refresh token storage

```rust
use reinhardt::auth::{
    TokenBlacklist, InMemoryTokenBlacklist, BlacklistReason,
    TokenRotationManager, InMemoryRefreshTokenStore
};

// Token blacklist
let blacklist = InMemoryTokenBlacklist::new();
use chrono::{Utc, Duration};
let expires_at = Utc::now() + Duration::hours(24);
blacklist.blacklist("old_token", expires_at, BlacklistReason::Logout).await?;
assert!(blacklist.is_blacklisted("old_token").await?);

// Token rotation
let refresh_store = InMemoryRefreshTokenStore::new();
let rotation_manager = TokenRotationManager::new(blacklist, refresh_store);

let new_token = rotation_manager.rotate_token("old_refresh_token", "user123").await?;
```

### Remote User Authentication

#### Header-Based Authentication

- **RemoteUserAuthentication**: Authenticate via trusted HTTP headers
- **Reverse Proxy Integration**: Support for authentication proxies (nginx,
  Apache, etc.)
- **Header Configuration**: Configurable header name (default: `REMOTE_USER`)
- **Header Validation**: Verify header presence and format
- **Automatic Logout**: Optional force logout when header is missing
- **SSO Support**: Single sign-on integration

```rust
use reinhardt::auth::RemoteUserAuthentication;

// Standard configuration
let auth = RemoteUserAuthentication::new("REMOTE_USER");

// With force logout
let auth = RemoteUserAuthentication::new("REMOTE_USER").force_logout_if_no_header(true);

// Authenticate from request
let user = auth.authenticate(&request).await?;
```

## Usage Examples

### Complete Authentication Flow

```rust
use reinhardt::auth::{
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
    user.username().to_string(),
    false,
    false,
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
use reinhardt::auth::{AuthBackend, SimpleUser, Argon2Hasher, PasswordHasher};
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
    ) -> Result<Option<Self::User>, reinhardt_exception::Error> {
        if let Some((hash, user)) = self.users.get(username) {
            if self.hasher.verify(password, hash)? {
                return Ok(Some(user.clone()));
            }
        }
        Ok(None)
    }

    async fn get_user(&self, user_id: &str)
        -> Result<Option<Self::User>, reinhardt_exception::Error> {
        Ok(self.users.values()
            .find(|(_, u)| u.id.to_string() == user_id)
            .map(|(_, u)| u.clone()))
    }
}
```


## sessions

### Features

### Implemented ✓

#### Core Session Backend

- **SessionBackend Trait** - Async trait defining session storage operations (load, save, delete, exists)
- **SessionError** - Error types for session operations (cache errors, serialization errors)
- **Generic Session Storage** - Type-safe session data storage with `serde` support

#### Cache-Based Backends

- **InMemorySessionBackend** - In-memory session storage using `InMemoryCache`
  - Fast, volatile storage (sessions lost on restart)
  - TTL (Time-To-Live) support for automatic expiration
  - Suitable for development and single-instance deployments
- **CacheSessionBackend** - Generic cache-based session backend
  - Works with any `Cache` trait implementation
  - Supports external cache systems (Redis, Memcached, etc.)
  - Configurable TTL for session expiration
  - Horizontal scalability for distributed systems

#### Dependency Injection Support

- Integration with `reinhardt-di` for dependency injection
- Session backend registration and resolution

#### High-Level Session API

- **Session<B>** struct - Django-style session object with dictionary-like interface
  - Type-safe with generic backend parameter `B: SessionBackend`
  - Dictionary-like methods: `get()`, `set()`, `delete()`, `contains_key()`
  - Session iteration methods: `keys()`, `values()`, `items()`
  - Manual session clearing: `clear()`
  - Manual modification tracking: `mark_modified()`, `mark_unmodified()`
  - Session modification tracking: `is_modified()`, `is_accessed()`
  - Session key management: `get_or_create_key()`, `generate_key()`
  - Session lifecycle: `flush()` (clear and new key), `cycle_key()` (keep data, new key)
  - Automatic persistence: `save()` method with TTL support (default: 3600 seconds)
  - Comprehensive doctests and unit tests (36 total tests)

#### Storage Backends

- **DatabaseSessionBackend** (feature: `database`) - Persistent session storage in database
  - Uses the connection supplied to `from_connection()` for load, save, delete, and existence checks
  - Session model with expiration timestamps
  - Automatic session cleanup with `cleanup_expired()`
  - SQLite, PostgreSQL, and MySQL support via sqlx
  - Table creation with `create_table()`
  - Indexed expiration dates for efficient cleanup
  - 9 comprehensive tests
- **FileSessionBackend** (feature: `file`) - File-based session storage
  - Session files stored in configurable directory (default: `/tmp/reinhardt_sessions`)
  - File locking using `fs2` for concurrent access safety
  - JSON serialization with TTL support
  - Automatic expired session cleanup on access
  - 11 comprehensive tests
- **CookieSessionBackend** (feature: `cookie`) - Encrypted session data in cookies
  - AES-256-GCM encryption for session data
  - HMAC-SHA256 signing for tamper detection
  - Automatic size limitation checking (4KB max)
  - Secure client-side storage
  - 11 comprehensive tests

#### HTTP Middleware

- **SessionMiddleware** (feature: `middleware`) - HTTP middleware for session management
  - Automatic session loading from cookies
  - Automatic session saving on response
  - Cookie configuration: name, path, domain
  - Security settings: secure, httponly, samesite
  - TTL and max-age support
- **HttpSessionConfig** - Comprehensive middleware configuration
- **SameSite** enum - Cookie SameSite attribute (Strict, Lax, None)

#### Session Management Features

- **Session expiration and cleanup** - Implemented via `cleanup_expired()` in DatabaseSessionBackend
- **Session key rotation** - Implemented via `cycle_key()` and `flush()` in Session API
- **Cross-site request forgery (CSRF) protection integration** - CSRF module available
- **Session serialization formats** - JSON via serde_json, MessagePack, CBOR, Bincode
- **Session storage migration tools** - Migration module available

#### Session Serialization Formats

- **JSON** (always available) - Human-readable, widely compatible via `serde_json`
- **MessagePack** (feature: `messagepack`) - Compact binary format, cross-platform via `rmp-serde`
- **CBOR** (feature: `cbor`) - RFC 7049 compliant binary format via `ciborium`
- **Bincode** (feature: `bincode`) - Fastest for Rust-to-Rust communication

#### Session Compression

- **CompressedSessionBackend** (feature: `compression`) - Automatic compression wrapper
  - Threshold-based compression (default: 512 bytes, configurable)
  - Only compresses data exceeding threshold to avoid overhead
  - **Zstd compression** (feature: `compression-zstd`) - Best balance of speed and ratio
  - **Gzip compression** (feature: `compression-gzip`) - Wide compatibility
  - **Brotli compression** (feature: `compression-brotli`) - Best compression ratio

#### Session Replication

- **ReplicatedSessionBackend** (feature: `replication`) - High availability with multi-backend replication
  - **AsyncReplication** - Eventual consistency, highest throughput
  - **SyncReplication** - Strong consistency, both backends updated in parallel
  - **AcknowledgedReplication** - Primary first, then secondary with acknowledgment
  - Configurable retry attempts and delays for failure handling

#### Session Analytics

- **InstrumentedSessionBackend** - Automatic session event tracking wrapper
- **LoggerAnalytics** - Tracing-based logging (always available)
- **PrometheusAnalytics** (feature: `analytics-prometheus`) - Prometheus metrics export
  - `session_created_total` - Total sessions created
  - `session_accessed_total` - Total session accesses
  - `session_access_latency_seconds` - Access latency histogram
  - `session_size_bytes` - Session data size histogram
  - `session_deleted_total` - Deletions by reason (explicit, expired, flushed)
  - `session_expired_total` - Total expired sessions

#### Multi-Tenant Session Isolation

- **TenantSessionBackend** (feature: `tenant`) - Tenant-specific session namespacing
  - Prefix-based keying: `tenant:{tenant_id}:session:{session_id}`
  - Configurable key prefix pattern
  - Maximum sessions per tenant limit
  - Strict isolation mode for security
  - **TenantSessionOperations** trait: `list_sessions()`, `count_sessions()`, `delete_all_sessions()`

## License

Licensed under the BSD 3-Clause License.

Client disabling invalidates pending approvals and unused authorization codes together
with the registration and its tokens; re-registering the same client identifier does
not reactivate those grants. Client secret rotation uses a compare-and-swap snapshot;
a conflicting administrative update fails rather than returning an unusable secret.

For local public SPAs, an explicit HTTP loopback development issuer permits canonical
HTTP origins on localhost, 127.0.0.1, and [::1]. HTTPS issuer configurations still reject
HTTP browser origins. Origins must not contain credentials, paths, queries, or fragments.

Code exchange authenticates the client and checks bound replay state before account,
resource, or signing dependencies. A correctly bound replay permanently revokes linked
tokens even while the account or resource is disabled. An unused code remains available
when a temporary host validation or signing failure prevents issuance.
