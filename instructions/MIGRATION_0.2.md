# Migration Guide: 0.1.0 → 0.2.0

Umbrella tracker: [#4520](https://github.com/kent8192/reinhardt-web/issues/4520).
Companion: [#4652](https://github.com/kent8192/reinhardt-web/issues/4652).

## Quick removal index

| Crate | Status |
|---|---|
| reinhardt-core | ✅ documented ([PR #4713](https://github.com/kent8192/reinhardt-web/pull/4713)) |
| reinhardt-query | ✅ documented ([PR #4717](https://github.com/kent8192/reinhardt-web/pull/4717)) |
| reinhardt-di | ✅ documented ([PR #4722](https://github.com/kent8192/reinhardt-web/pull/4722)) |
| reinhardt-conf (partial) | ✅ documented ([PR #4728](https://github.com/kent8192/reinhardt-web/pull/4728)) |
| reinhardt-db | ✅ documented ([PR #4729](https://github.com/kent8192/reinhardt-web/pull/4729)) |
| reinhardt-auth | ✅ documented ([PR #4652](https://github.com/kent8192/reinhardt-web/issues/4652)) |

---

## reinhardt-core

All removals shipped in [PR #4713](https://github.com/kent8192/reinhardt-web/pull/4713).
These are macro-emitted items — the `#[routes]` and `#[viewset]` macros no
longer generate the deprecated codegen paths.

### Flat 2-level URL accessor codegen (deprecated since `0.1.0-rc.16`)

The `#[routes]` macro previously generated flat accessor methods on
`ResolvedUrls`. These are removed; use the namespaced gateway instead.

```rust
// Before (server routes)
let url = urls.myapp();

// After
let url = urls.server().myapp();
```

```rust
// Before (client routes)
let url = urls.myapp_client();

// After
let url = urls.client().myapp();
```

### Per-route resolver-trait codegen (deprecated since `0.1.0-rc.16`)

The `#[get(name = "...")]` / `#[post(name = "...")]` macros previously
generated a `Resolve<Name>` blanket-impl trait that produced flat
`urls.<name>(...)` calls. This codegen is removed.

```rust
// Before
let url = urls.article_detail(id);

// After
let url = urls.server().blog().article_detail(id);
```

### Flat ViewSet accessor codegen (deprecated since `0.1.0-rc.29`)

The `#[viewset]` macro previously generated `Resolve<Pascal>List` /
`Resolve<Pascal>Detail` blanket-impl traits and flat accessor methods.
These are removed (4 items).

```rust
// Before
let list_url = urls.article_list();
let detail_url = urls.article_detail(id);

// After
let list_url = urls.server().blog().article_list();
let detail_url = urls.server().blog().article_detail(id);
```

### `UrlResolverUnprefixed` override

The `impl UrlResolverUnprefixed for ResolvedUrls` override emitted by
`#[routes]` is removed because the flat ViewSet accessor that required
it no longer exists.

---

## reinhardt-query

All removals shipped in [PR #4717](https://github.com/kent8192/reinhardt-web/pull/4717).

### `SeaRc<T>` type alias (deprecated since `0.1.0-rc.16`)

The transitional `SeaRc<T>` type alias (left over from the SeaQuery fork)
is removed. Use `SharedRc<T>` directly — it expands to `Arc<T>` with the
`thread-safe` feature and `Rc<T>` without it.

```rust
// Before
use reinhardt_query::SeaRc;
let iden: SeaRc<dyn Iden> = SeaRc::new(MyTable);

// After
use reinhardt_query::SharedRc;
let iden: SharedRc<dyn Iden> = SharedRc::new(MyTable);
```

`SharedRc<T>` is identical in behavior — it is the underlying type the
alias resolved to. No semantic change, only the spelling.

---

## reinhardt-di

All removals shipped in [PR #4722](https://github.com/kent8192/reinhardt-web/pull/4722).

### `Injected<T>` struct (deprecated since `0.1.0-rc.16`)

The FastAPI-inspired `Injected<T>` wrapper is removed. All injection
codegen now goes through `Depends<T>` exclusively.

### `OptionalInjected<T>` type alias (deprecated since `0.1.0-rc.16`)

Use `Option<Depends<T>>` instead.

### Combined migration example

```rust
// Before
use reinhardt_di::{Injected, OptionalInjected, Injectable};

#[injectable]
struct Handler {
    #[inject]
    db: Injected<Database>,
    #[inject]
    cache: OptionalInjected<Cache>,
}

// After
use reinhardt_di::{Depends, Injectable};

#[injectable]
struct Handler {
    #[inject]
    db: Depends<Database>,
    #[inject]
    cache: Option<Depends<Cache>>,
}
```

Field-access semantics are unchanged — `Depends<T>` derefs to `&T` the
same way `Injected<T>` did.

### Macro behavior change

`#[injectable]` no longer accepts `Injected<T>` / `OptionalInjected<T>`
fields. If you have not yet migrated, the compile error reads:

```text
#[inject] field must have type Depends<T> or Option<Depends<T>>
```

---

## reinhardt-conf

Partial removals shipped in [PR #4728](https://github.com/kent8192/reinhardt-web/pull/4728).
The remaining `Settings` struct removals are tracked in a follow-up PR.

### `AdvancedSettings` struct (deprecated since `0.1.0-rc.16`)

The monolithic `AdvancedSettings` is removed. Use the individual fragment
types (`CacheSettings`, `SessionSettings`, etc.) composed via
`ProjectSettings` instead.

```rust
// Before
use reinhardt_conf::AdvancedSettings;

let settings = AdvancedSettings::from_file("settings.toml")?;
let cache_ttl = settings.cache.ttl;

// After
use reinhardt_conf::{SettingsBuilder, TomlFileSource, CacheSettings, SessionSettings};

let settings = SettingsBuilder::new()
    .add_source(TomlFileSource::new("settings.toml"))
    .build_composed::<(CacheSettings, SessionSettings)>()?;
let cache_ttl = settings.get::<CacheSettings>().ttl;
```

### `JsonFileSource` struct (deprecated since `0.1.0-rc.26`)

TOML is the canonical Reinhardt configuration format. `JsonFileSource`
and its `ConfigSource` impl are removed.

```rust
// Before
use reinhardt_conf::JsonFileSource;

let source = JsonFileSource::new("config.json");

// After
use reinhardt_conf::TomlFileSource;

let source = TomlFileSource::new("config.toml");
```

Migrate your `.json` configuration files to `.toml` format. If you must
keep JSON support out-of-tree, implement `ConfigSource` against
`serde_json` directly.

### `auto_source(path)` function (deprecated since `0.1.0-rc.26`)

The magic format-detection function is removed. Construct the source
explicitly so the configuration format is visible at the call site.

```rust
// Before
use reinhardt_conf::auto_source;

let source = auto_source("config/settings.toml");

// After
use reinhardt_conf::TomlFileSource;

let source = TomlFileSource::new("config/settings.toml");
```

### `TomlFileSource::set_interpolation(bool)` method (deprecated since `0.1.0-rc.27`)

The boolean setter is removed. Use the builder methods instead.

```rust
// Before
let mut source = TomlFileSource::new("settings.toml");
source.set_interpolation(true);

// After (interpolation is ON by default since 0.1.0)
let source = TomlFileSource::new("settings.toml");

// To explicitly enable (no-op, but documents intent):
let source = TomlFileSource::new("settings.toml").with_interpolation();

// To disable:
let source = TomlFileSource::new("settings.toml").without_interpolation();
```

---

## reinhardt-db

All removals shipped in [PR #4729](https://github.com/kent8192/reinhardt-web/pull/4729).

### `get_database_url_from_env_or_settings(base_dir)` (deprecated since `0.1.0-rc.29`)

This method on `DatabaseConnection` reloaded `settings/<profile>.toml`
from disk on every call, duplicating the framework's settings-loading
logic. Use `database_url_from(settings, env_override)` with a pre-built
`ProjectSettings` instead.

```rust
// Before
let url = DatabaseConnection::get_database_url_from_env_or_settings(None)?;

// After
use reinhardt_conf::SettingsBuilder;

let settings = SettingsBuilder::new()
    .add_source(TomlFileSource::new("settings/development.toml"))
    .build_composed::<ProjectSettings>()?;
let url = DatabaseConnection::database_url_from(&settings, None)?;
```

The new API integrates with the typed-TOML settings pipeline (including
`${VAR}` interpolation) and avoids redundant disk I/O.

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
