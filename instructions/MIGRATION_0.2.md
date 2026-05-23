# Migration Guide: 0.1.0 → 0.2.0

Umbrella tracker: [#4520](https://github.com/kent8192/reinhardt-web/issues/4520).
Companion: [#4652](https://github.com/kent8192/reinhardt-web/issues/4652).

> Filled in incrementally by each per-crate PR.

## Quick removal index

| Crate | Status |
|---|---|
| reinhardt-core / -query / -di / -conf (partial) / -db | shipped via PRs #4713 / #4717 / #4722 / #4728 / #4729 |
| reinhardt-auth + #4652 | 🔄 this PR |
| (others) | ⏳ pending |

---

## reinhardt-db

### `DatabaseConnection::get_database_url_from_env_or_settings(base_dir)` removed

Deprecated since `0.1.0-rc.29` per Issue #4520. Use
`DatabaseConnection::database_url_from(settings, env_override)` with a
pre-built `ProjectSettings`.

```rust
// Before
let url = DatabaseConnection::get_database_url_from_env_or_settings(None)?;

// After
let settings: ProjectSettings = /* build via SettingsBuilder */;
let url = DatabaseConnection::database_url_from(&settings, env_override)?;
```

The new API is cheaper (no per-call disk re-read) and surfaces the
settings dependency at the call site instead of hiding it behind a
fresh `SettingsBuilder::new()`.

#### Follow-up consumer migration

`reinhardt-commands/src/builtin.rs::get_database_url_from_settings`
still references the removed entry point and will be migrated in a
follow-up `chore(commands)!` PR.

---

## reinhardt-auth (closes #4652)

### `CurrentUser<U>` → `AuthUser<U>` (closes #4652)

Deprecated since `0.1.0-rc.12`. The `current_user` module is removed
entirely. **`CurrentUser` is not a type alias** — its shape differs
from `AuthUser`, so pattern-match call sites need restructuring.

```rust
// Before
async fn handler(current_user: CurrentUser<DefaultUser>) -> Response {
    if current_user.is_authenticated() {
        let user = current_user.user()?;
        let id = current_user.id()?;
        // ...
    }
}

// After
async fn handler(auth_user: AuthUser<MyUser>) -> Response {
    let user: &MyUser = &auth_user.0;
    let id = user.id();
    // ...
}
```

For anonymous-user handling, branch on the `AuthUser<U>` extractor
result at the framework level (return 401 / redirect via guards)
rather than carrying an `Option<U>` payload inside the extractor.

### `DefaultUser` → `#[user]` macro

Deprecated since `0.1.0-rc.15`. Define your own user struct:

```rust
// Before
use reinhardt_auth::DefaultUser;

// After
use reinhardt_auth::user;

#[user]
pub struct MyUser {
    pub username: String,
    pub email: String,
    // ...
}
```

### `User` trait + `SimpleUser` + `AnonymousUser` → composable trait stack

Deprecated since `0.1.0-rc.15`. The `core::user` module is gone. Use:

- `AuthIdentity` for the identity claim
- `BaseUser` / `FullUser` for user model traits
- `PermissionsMixin` for authorization checks

### Consumer migration follow-up

The following workspace crates still reference the removed symbols and
need a follow-up PR to migrate:

- `crates/reinhardt-middleware/src/auth.rs`
- `crates/reinhardt-rest/src/serializers/model_serializer.rs`
- `crates/reinhardt-http/src/auth_state.rs`
- `crates/reinhardt-views/src/viewsets/handler/model_view_set_handler.rs`
- `crates/reinhardt-di/src/lib.rs` (User-related re-export, if any)
- `examples/examples-tutorial-basis/apps/polls/di.rs` (per #4652
  companion-PR section)

CI on this PR is expected to surface those compile errors so the
follow-up PR has a complete punch list.
