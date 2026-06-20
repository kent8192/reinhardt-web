# Reinhardt 0.3 Migration Guide

This guide covers compatibility APIs that were deprecated during the 0.2 line
and removed for 0.3. It assumes the application already follows the 0.2
migration guide.

## Removed API

| Crate | Removed API | Replacement |
|---|---|---|
| `reinhardt-auth` | `AuthUser<U>` | `CurrentUser<U>` |
| `reinhardt-pages` | `create_resource(fetcher)` | `use_resource(fetcher, ())` |
| `reinhardt-pages` | `create_resource_with_deps(...)` | `use_resource(fetcher, deps)` |
| `reinhardt-pages` | `use_effect_event(...)` | `use_callback(f, deps)` or `.get_untracked()` inside the effect |
| `reinhardt-pages` | `use_effect_event_with(...)` | `use_callback_with(f, deps)` or `.get_untracked()` inside the effect |

## Auth Extractor

`CurrentUser<U>` is the only full authenticated-user extractor.

```rust
// Before
use reinhardt_auth::AuthUser;

async fn profile(AuthUser(user): AuthUser<MyUser>) -> Response {
    let id = user.id();
    // ...
}

// After
use reinhardt_auth::CurrentUser;

async fn profile(CurrentUser(user): CurrentUser<MyUser>) -> Response {
    let id = user.id();
    // ...
}
```

## Session Auth Middleware Wiring

Cookie-backed session apps should register `SessionMiddleware` once in
`urls.rs`. The 0.3 line uses Option A from issue #4740: `SessionMiddleware`
derives `AuthState` from `USER_ID_SESSION_KEY` when the active session is
authenticated, while preserving any `AuthState` already inserted by another
auth middleware. This keeps the common cookie-session setup to one middleware
without introducing a separate `SessionAuthMiddleware` bundle type.

`CookieSessionAuthMiddleware` remains available for projects that plug a custom
`AsyncSessionBackend` directly, but it is not required for the standard
`SessionData` + `SessionAuthExt` + `CurrentUser<U>` flow.

## Resource Hooks

Use `use_resource(fetcher, deps)` for both mount-only and dependency-driven
resource loading.

```rust
// Before
let questions = create_resource_with_deps(fetch_questions, (page,));

// After
let questions = use_resource(fetch_questions, (page,));
```

For mount-only loading, pass `()`:

```rust
let user = use_resource(fetch_user, ());
```

## Verification

Search for removed API names before running the full test suite:

```bash
rg -n "AuthUser|create_resource|create_resource_with_deps|use_effect_event|use_effect_event_with" src crates examples
```
