+++
title = "Part 4: Users and Authentication"
description = "Add the users app, session-backed authentication server functions, and a nav shell that reflects login state."
weight = 40

[extra]
sidebar_weight = 40
+++

# Part 4: Users and Authentication

The poll app can list, display, and vote. Now add accounts: a `users` app, a minimal `User` model, login/register/logout/current-user server functions, session middleware, auth routes, and a shared navigation bar.

This is the first basis tutorial part that uses dependency injection directly. Keep the DI surface small here. The REST tutorial's dependency-injection part goes deeper into factories, scopes, caching, and type keys.

## Create the Users App

Generate the app:

```bash
reinhardt-admin startapp users --template pages
```

Register it next to `polls`:

```rust
use reinhardt::installed_apps;

installed_apps! {
    polls: "polls",
    users: "users",
}
```

## Define the User Model

Open `src/apps/users/server/models.rs`. The example uses a minimal user model, not `full = true`:

```rust
#[user(hasher = reinhardt::Argon2Hasher, username_field = "username", manager = false)]
#[model(app_label = "users", table_name = "users")]
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct User {
    #[field(primary_key = true)]
    pub id: i64,

    #[field(max_length = 150, unique = true)]
    pub username: String,

    #[field(max_length = 255, skip_info = true)]
    pub password_hash: Option<String>,

    #[field(default = true)]
    pub is_active: bool,

    #[field(default = false, skip_info = true)]
    pub is_superuser: bool,

    #[field(include_in_new = false, skip_info = true)]
    pub last_login: Option<DateTime<Utc>>,

    #[field(auto_now_add = true, skip_info = true)]
    pub created_at: DateTime<Utc>,
}
```

`manager = false` opts out of the generated user manager because this tutorial keeps a project-local manager that owns password hashing, uniqueness checks, and persistence.

## Add the Auth User Manager

The manager is server-only. It stores a `DatabaseConnection` and is registered through an injectable factory:

```rust
#[derive(Clone)]
pub struct AuthUserManager {
    db: DatabaseConnection,
}

#[injectable_factory(scope = "transient")]
async fn auth_user_manager_factory(
    #[inject] db: Depends<DatabaseConnection>,
) -> AuthUserManager {
    AuthUserManager { db: (*db).clone() }
}
```

The `Depends<DatabaseConnection>` wrapper dereferences to the inner connection. Cloning here is cheap because the connection handle is internally shared.

The manager implements `BaseUserManager<User>`:

```rust
#[async_trait]
impl BaseUserManager<User> for AuthUserManager {
    async fn create_user(
        &mut self,
        username: &str,
        password: Option<&str>,
        extra: HashMap<String, Value>,
    ) -> Result<User, Error> {
        let new_user = self.build_user(username, password, &extra).await?;
        User::objects()
            .create_with_conn(&self.db, &new_user)
            .await
            .map_err(|e| Error::Database(e.to_string()))
    }
}
```

`build_user` trims usernames, rejects empty or overlong names, checks uniqueness, enforces password length, and calls `set_password`. Keep that logic in the manager so server functions stay focused on request/session flow.

## Share Auth DTOs

Update `src/shared/types.rs` to add `UserInfo` next to the poll DTOs and define login/register DTOs:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub is_active: bool,
}

impl InfoModel for UserInfo {
    type PrimaryKey = i64;
}

#[cfg(server)]
impl From<crate::apps::users::server::models::User> for UserInfo {
    fn from(user: crate::apps::users::server::models::User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            is_active: user.is_active,
        }
    }
}
```

```rust
#[dto]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    #[validate(length(
        min = 1,
        max = 150,
        message = "Username must be between 1 and 150 characters"
    ))]
    pub username: String,

    #[validate(length(min = 1, message = "Password must not be empty"))]
    pub password: String,
}
```

`RegisterRequest` follows the same pattern and adds `password_confirmation`. The example validates password equality with a helper instead of relying on validator crate field-name matching:

```rust
#[cfg(server)]
impl RegisterRequest {
    pub fn validate_passwords_match(&self) -> Result<(), &'static str> {
        if self.password == self.password_confirmation {
            Ok(())
        } else {
            Err("Passwords do not match")
        }
    }
}
```

## Add Auth Server Functions

Create `src/apps/users/server_fn.rs`. Login validates the request, checks the password, and rotates the session:

```rust
#[server_fn]
pub async fn login(
    username: String,
    password: String,
    #[inject] _db: DatabaseConnection,
    #[inject] session: SessionData,
    #[inject] store: Depends<SessionStore>,
) -> std::result::Result<UserInfo, ServerFnError> {
    let mut session = session;

    let request = LoginRequest { username, password };
    request
        .validate()
        .map_err(|e| ServerFnError::application(format!("Validation failed: {}", e)))?;

    let manager = User::objects();
    let user = manager
        .filter(User::field_username().eq(request.username.trim().to_string()))
        .first()
        .await
        .map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?
        .ok_or_else(|| ServerFnError::server(401, "Invalid credentials"))?;

    let valid = user
        .check_password(&request.password)
        .map_err(|e| ServerFnError::application(format!("Password check failed: {}", e)))?;

    if !valid {
        return Err(ServerFnError::server(401, "Invalid credentials"));
    }

    if !user.is_active() {
        return Err(ServerFnError::server(403, "User account is inactive"));
    }

    session
        .login(&store, user.id())
        .map_err(|e| ServerFnError::application(format!("Session error: {}", e)))?;

    Ok(UserInfo::from(user))
}
```

Register creates the user through the injected `AuthUserManager` and then logs the new account in:

```rust
#[server_fn]
pub async fn register(
    username: String,
    password: String,
    password_confirmation: String,
    #[inject] user_manager: Depends<AuthUserManager>,
    #[inject] session: SessionData,
    #[inject] store: Depends<SessionStore>,
) -> std::result::Result<UserInfo, ServerFnError> {
    let mut session = session;

    let request = RegisterRequest {
        username,
        password,
        password_confirmation,
    };

    request
        .validate()
        .map_err(|e| ServerFnError::application(format!("Validation failed: {}", e)))?;
    request
        .validate_passwords_match()
        .map_err(ServerFnError::application)?;

    let mut user_manager: AuthUserManager = (*user_manager).clone();
    let saved = user_manager
        .create_user(
            request.username.trim(),
            Some(&request.password),
            HashMap::new(),
        )
        .await
        .map_err(|e| ServerFnError::application(e.to_string()))?;

    session
        .login(&store, saved.id())
        .map_err(|e| ServerFnError::application(format!("Session error: {}", e)))?;

    Ok(UserInfo::from(saved))
}
```

Add logout and current-user lookup:

```rust
#[server_fn]
pub async fn logout(
    #[inject] session: SessionData,
    #[inject] store: Depends<SessionStore>,
) -> std::result::Result<(), ServerFnError> {
    let mut session = session;

    if session.get::<i64>(USER_ID_SESSION_KEY).is_none() {
        return Err(ServerFnError::server(401, "Not authenticated"));
    }

    session.logout(&store);
    Ok(())
}

#[server_fn]
pub async fn current_user(
    #[inject] _db: DatabaseConnection,
    #[inject] session: SessionData,
) -> std::result::Result<Option<UserInfo>, ServerFnError> {
    let user_id = match session.get::<i64>(USER_ID_SESSION_KEY) {
        Some(id) => id,
        None => return Ok(None),
    };

    let user = User::objects()
        .filter(User::field_id().eq(user_id))
        .first()
        .await
        .map_err(|e| ServerFnError::application(format!("Database error: {}", e)))?;

    Ok(user.map(UserInfo::from))
}
```

## Register Auth Routes

The users app follows the same target-neutral route surface as polls. `src/apps/users/urls.rs` aggregates the split router modules:

```rust
use reinhardt::{ClientRouter, ServerRouter};

pub mod client_router;

#[cfg(server)]
pub mod server_router;

pub fn server_url_patterns() -> ServerRouter {
    #[cfg(server)]
    {
        server_router::server_url_patterns()
    }
    #[cfg(not(server))]
    {
        ServerRouter::new()
    }
}

pub fn client_url_patterns() -> ClientRouter {
    client_router::client_url_patterns()
}
```

Client routes live in `src/apps/users/urls/client_router.rs`:

```rust
use reinhardt::ClientRouter;

use crate::apps::users::pages;

pub fn client_url_patterns() -> ClientRouter {
    ClientRouter::new()
        .route("login", "/login/", pages::login_page)
        .route("logout", "/logout/", pages::logout_page)
        .route("signup", "/signup/", pages::signup_page)
}
```

Server routes register server-function markers in `src/apps/users/urls/server_router.rs`:

```rust
use crate::apps::users::server_fn::{current_user, login, logout, register};
use reinhardt::ServerRouter;
use reinhardt::pages::server_fn::ServerFnRouterExt;

pub fn server_url_patterns() -> ServerRouter {
    ServerRouter::new()
        .server_fn(login::marker)
        .server_fn(logout::marker)
        .server_fn(register::marker)
        .server_fn(current_user::marker)
}
```

App-local page entry points in `src/apps/users/pages.rs` wrap the users client components on WASM and return `Page::Empty` on native:

```rust
pub fn login_page() -> Page {
    #[cfg(client)]
    {
        with_nav(crate::apps::users::client::components::login_form())
    }
    #[cfg(not(client))]
    {
        Page::Empty
    }
}
```

Mount the users app in `src/config/urls.rs` on both targets:

```rust
#[cfg(server)]
let router = router.server(|s| {
    s.mount("/", crate::apps::polls::urls::server_url_patterns())
        .mount("/", crate::apps::users::urls::server_url_patterns())
});
```

```rust
let router = router
    .mount_unified(
        "/",
        UnifiedRouter::new().client(|_| crate::apps::polls::urls::client_url_patterns()),
    )
    .mount_unified(
        "/",
        UnifiedRouter::new().client(|_| crate::apps::users::urls::client_url_patterns()),
    );
```

## Add Session Middleware

The session middleware owns the `SessionStore` DI registration. Add it once in `src/config/urls.rs`:

```rust
#[cfg(server)]
fn create_session_middleware() -> SessionMiddleware {
    let config = SessionConfig::new("sessionid".to_string(), Duration::from_secs(1_209_600))
        .with_http_only(true)
        .with_same_site("Lax".to_string())
        .with_path("/".to_string());
    SessionMiddleware::new(config)
}
```

Then attach it to the project router:

```rust
#[cfg(server)]
let router = router.with_middleware(create_session_middleware());
```

After this, server functions can inject `SessionData` and `Depends<SessionStore>`.

## Build the Auth Pages

The users client module hosts forms:

```rust
pub mod components;
```

The login form binds directly to the `login` server function:

```rust
pub fn login_form() -> Page {
    let login_form = form! {
        name: LoginForm,
        server_fn: login,
        method: Post,
        redirect_on_success: "/",
        fields: {
            username: CharField {
                label: "Username",
                placeholder: "your-username",
                max_length: 150,
                class: "form-control",
            }
            password: PasswordField {
                label: "Password",
                placeholder: "Enter your password",
                class: "form-control",
            }
        }
    };
```

The signup form uses `register`; the logout form uses `logout` with no fields. The generated `#[server_fn]` client stubs handle the HTTP call and CSRF header.

The page aggregator wraps each auth form in the shared nav:

```rust
pub fn login_page() -> Page {
    with_nav(crate::apps::users::client::components::login_form())
}

pub fn logout_page() -> Page {
    with_nav(crate::apps::users::client::components::logout_form())
}

pub fn signup_page() -> Page {
    with_nav(crate::apps::users::client::components::signup_form())
}
```

## Show Login State in the Nav

The nav calls `current_user()` as a reactive action:

```rust
pub fn nav_bar() -> Page {
    let load_user =
        use_action(|_: ()| async move { current_user().await.map_err(|e| e.to_string()) });
    load_user.dispatch(());
```

When a user is present, show their username and a logout link:

```rust
if load_user.is_pending() {
    div {
        class: "flex items-center gap-3",
        aria_busy: "true",
        span {
            class: "sr-only",
            "Checking sign-in status"
        }
    }
} else if let Some(Some(user)) = load_user.result() {
    div {
        class: "flex items-center gap-3",
        span {
            class: "text-sm text-muted",
            {
                format!("Signed in as {}", user.username)
            }
        }
        a {
            href: logout_href.clone(),
            class: "btn-secondary",
            "Logout"
        }
    }
}
```

The unauthenticated branch links to signup and login:

```rust
let login_href = users_routes::reverse("login", &[]);
let logout_href = users_routes::reverse("logout", &[]);
let signup_href = users_routes::reverse("signup", &[]);
```

## Migrate and Check

Generate and apply the users migration:

```bash
cargo make makemigrations
cargo make migrate
```

Run the app:

```bash
cargo make dev
```

Open `/signup/`, create an account, and confirm that the nav changes from Sign up/Login to `Signed in as <username>` plus Logout. Then sign out and sign back in at `/login/`.

## Checkpoint

Before continuing:

- `User` uses `#[user(..., manager = false)]` plus `#[model(...)]`.
- `AuthUserManager` is registered with `#[injectable_factory(scope = "transient")]`.
- `login`, `register`, `logout`, and `current_user` are registered in the users server router.
- Session middleware is attached once in `src/config/urls.rs`.
- The nav resolves auth links through `users_routes::reverse(...)`.
